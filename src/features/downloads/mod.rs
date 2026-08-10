//! Pantalla de Descargas (Downloads). Lista la cola persistida en SQLite
//! (`download_dao::list`) en una tabla con título del capítulo, estado y
//! barra de progreso. Los eventos en vivo del `DownloadManager`
//! (`Message::DownloadEvent`) se aplican sobre `state.downloads_state.entries`
//! en el reducer global vía `apply_event` — sin necesidad de re-consultar la
//! DB — y `Load` refresca el vector completo desde la base.
//!
//! `download_dao::list` no devuelve títulos (sólo `(manga_id, chapter_url,
//! state, total, done)`), así que al cargar se enriquece con los títulos de
//! capítulo desde `chapter_dao::list_for_manga` (mismo `&mut Connection` del
//! closure `db_blocking`, sin queries adicionales).
use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Length, Task};
use std::collections::HashMap;

use crate::app::{AppState, Message as AppMessage};
use crate::core::db;
use crate::core::db::dao::{chapter_dao, download_dao};
use crate::core::downloads::DownloadEvent;
use crate::core::error::DbError;
use crate::core::models::{DownloadEntry, DownloadState};
use crate::theme::palette;
use crate::widgets::icon;

/// Resultado de la carga en background: entradas DAO + títulos de capítulo
/// resueltos (clave `(manga_id, chapter_url)`).
pub type Loaded =
    Result<(Vec<DownloadEntry>, HashMap<(i64, String), String>), DbError>;

#[derive(Debug, Default)]
pub struct State {
    /// Cola de descargas tal cual sale de `download_dao::list()`.
    pub entries: Vec<DownloadEntry>,
    /// Título del capítulo por `(manga_id, chapter_url)`, resuelto en `Load`
    /// y conservado mientras los `DownloadEvent` mutan `entries`.
    pub titles: HashMap<(i64, String), String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Recarga la cola desde la DB (`download_dao::list` → `DownloadsLoaded`).
    Load,
}

/// Reducer del feature Downloads. `Load` corre la consulta en
/// `spawn_blocking` (helper `db_blocking`) y el resultado aterriza en el
/// reducer global como `Message::DownloadsLoaded`.
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::Load => {
            let dbh = state.db.clone();
            if let Some(db) = dbh {
                Task::perform(
                    db::db_blocking(db, |conn| {
                        let entries = download_dao::list(conn)?;
                        // Enriquece con títulos de capítulo: un lookup por
                        // manga (id único) cubre todas sus entradas.
                        let mut titles = HashMap::new();
                        let mut manga_ids: Vec<i64> =
                            entries.iter().map(|e| e.manga_id).collect();
                        manga_ids.sort_unstable();
                        manga_ids.dedup();
                        for mid in manga_ids {
                            for ch in chapter_dao::list_for_manga(conn, mid)? {
                                titles.insert((mid, ch.url), ch.title);
                            }
                        }
                        Ok((entries, titles))
                    }),
                    AppMessage::DownloadsLoaded,
                )
            } else {
                Task::none()
            }
        }
    }
}

/// Aplica un `DownloadEvent` del `DownloadManager` sobre la cola en memoria.
/// Insert/update in-place: `Queued`/`Progress` crean la entrada si no existe,
/// `Done`/`Errored` sólo mutan el estado de la entrada conocida.
pub fn apply_event(state: &mut AppState, ev: DownloadEvent) {
    let st = &mut state.downloads_state;
    match ev {
        DownloadEvent::Queued(id, url) => {
            upsert_entry(st, id, url, DownloadState::Queued, 0, 0);
        }
        DownloadEvent::Progress {
            manga_id,
            chapter_url,
            done,
            total,
        } => upsert_entry(st, manga_id, chapter_url, DownloadState::Downloading, done, total),
        DownloadEvent::Done(id, url) => {
            if let Some(e) = st
                .entries
                .iter_mut()
                .find(|e| e.manga_id == id && e.chapter_url == url)
            {
                e.state = DownloadState::Done;
            }
        }
        DownloadEvent::Errored(id, url, _) => {
            if let Some(e) = st
                .entries
                .iter_mut()
                .find(|e| e.manga_id == id && e.chapter_url == url)
            {
                e.state = DownloadState::Error;
            }
        }
    }
}

fn upsert_entry(
    st: &mut State,
    id: i64,
    url: String,
    state: DownloadState,
    done: i32,
    total: i32,
) {
    match st
        .entries
        .iter_mut()
        .find(|e| e.manga_id == id && e.chapter_url == url)
    {
        Some(e) => {
            e.state = state;
            if total > 0 {
                e.total_pages = total;
            }
            if done > 0 {
                e.done_pages = done;
            }
        }
        None => st.entries.push(DownloadEntry {
            manga_id: id,
            chapter_url: url,
            state,
            total_pages: total,
            done_pages: done,
        }),
    }
}

fn state_label(s: DownloadState) -> &'static str {
    match s {
        DownloadState::Idle => "Inactivo",
        DownloadState::Queued => "En cola",
        DownloadState::Downloading => "Descargando",
        DownloadState::Done => "Hecho",
        DownloadState::Error => "Error",
    }
}

/// Vista del feature (réplica del diseño original): header "Descargas" +
/// ícono de pausa; filas con ícono de estado (✓ hecho, ! error, ↓ activo),
/// título del manga + capítulo, botón ✕ para quitar de la cola.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let header = row![
        text("Descargas").size(22).color(palette::TEXT),
        iced::widget::horizontal_space(),
        button(icon::glyph(icon::PAUSE, 18, palette::TEXT_MUTED))
            .style(crate::theme::link_button)
            .padding(6),
    ]
    .align_y(iced::Alignment::Center);

    if state.downloads_state.entries.is_empty() {
        return column![
            header,
            text("Sin descargas").size(15).color(palette::TEXT_MUTED),
            button(text("Recargar").size(13))
                .on_press(AppMessage::Download(Message::Load))
                .style(crate::theme::ghost_button)
                .padding([6, 14]),
        ]
        .spacing(12)
        .into();
    }

    let rows = column(state.downloads_state.entries.iter().map(|e| {
        let title = state
            .downloads_state
            .titles
            .get(&(e.manga_id, e.chapter_url.clone()))
            .cloned()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| e.chapter_url.clone());
        let (ic, color) = match e.state {
            DownloadState::Done => (icon::CHECK, palette::SUCCESS),
            DownloadState::Error => (icon::ERROR, palette::DANGER),
            DownloadState::Downloading => (icon::DOWNLOAD, palette::ACCENT),
            DownloadState::Queued => (icon::DOWNLOAD_FOR_OFFLINE, palette::TEXT_MUTED),
            DownloadState::Idle => (icon::DOWNLOAD_FOR_OFFLINE, palette::TEXT_DIM),
        };
        let subtitle = match e.state {
            DownloadState::Downloading => {
                format!("Descargando… {}/{}", e.done_pages, e.total_pages)
            }
            _ => state_label(e.state).to_string(),
        };
        row![
            icon::glyph(ic, 18, color),
            column![
                text(title).size(14).color(palette::TEXT),
                text(subtitle).size(12).color(palette::TEXT_MUTED),
            ]
            .spacing(2)
            .width(Length::Fill),
            button(icon::glyph(icon::CLOSE, 16, palette::TEXT_MUTED))
                .style(crate::theme::link_button)
                .padding(6),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }))
    .spacing(4);

    column![header, scrollable(rows)].spacing(12).into()
}
