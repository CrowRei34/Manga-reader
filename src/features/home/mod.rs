//! Pantalla de Inicio (Home). Réplica del diseño original:
//! - Título "Inicio"
//! - Sección "Continuar leyendo" (historial) con scroll horizontal de covers
//!   + subtítulo "Cap. N"
//! - Sección "Añadidos recientemente" con grid de covers de la biblioteca
use iced::widget::{column, container, row, scrollable, text, Row};
use iced::{Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::db;
use crate::core::db::dao::{history_dao, manga_dao};
use crate::core::error::DbError;
use crate::core::models::Manga;
use crate::features::details;
use crate::core::models::MangaRef;
use crate::theme::palette;
use crate::widgets::cover::{cover_card, cover_grid};

#[derive(Debug, Default)]
pub struct State {
    /// (manga, chapter_index) del historial, ordenado por `updated_at DESC`.
    pub recent: Vec<(Manga, i32)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadRecent,
    RecentLoaded(Result<Vec<(Manga, i32)>, DbError>),
}

pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::LoadRecent => {
            let dbh = state.db.clone();
            if let Some(db) = dbh {
                Task::perform(
                    db::db_blocking(db, |conn| {
                        let ids = history_dao::recent(conn, 10)?;
                        let mut out = Vec::with_capacity(ids.len());
                        for (id, chapter_index, _, _) in ids {
                            if let Some(m) = manga_dao::get_by_id(conn, id)? {
                                out.push((m, chapter_index));
                            }
                        }
                        Ok(out)
                    }),
                    |r| AppMessage::Home(Message::RecentLoaded(r)),
                )
            } else {
                Task::none()
            }
        }
        Message::RecentLoaded(Ok(recent)) => {
            state.home.recent = recent;
            // Trae las portadas del historial.
            let mangas: Vec<Manga> = state.home.recent.iter().map(|(m, _)| m.clone()).collect();
            crate::widgets::cover::fetch_covers(state, &mangas)
        }
        Message::RecentLoaded(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
    }
}

/// Cabecera de sección: label muted + línea divisoria que ocupa el resto.
fn section_header<'a>(label: &str) -> Element<'a, AppMessage> {
    row![
        text(label.to_string()).size(14).color(palette::TEXT_MUTED),
        container(text(""))
            .style(crate::theme::divider)
            .height(Length::Fixed(1.0))
            .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let title = text("Inicio").size(22).color(palette::TEXT);

    // Continuar leyendo: scroll horizontal de covers con "Cap. N".
    let recent_cards: Vec<Element<'_, AppMessage>> = state
        .home
        .recent
        .iter()
        .map(|(m, cidx)| {
            cover_card(
                m,
                &state.covers,
                Some(format!("Cap. {cidx}")),
                AppMessage::Details(details::Message::Load(MangaRef {
                    source: m.source.clone(),
                    url: m.url.clone(),
                    title: m.title.clone(),
                })),
            )
        })
        .collect();
    let recent_row = scrollable(
        Row::with_children(recent_cards).spacing(16),
    )
    .direction(scrollable::Direction::Horizontal(Default::default()));

    // Añadidos recientemente: grid de la biblioteca.
    let grid = cover_grid(&state.library, &state.covers, 5, |m| {
        AppMessage::Details(details::Message::Load(MangaRef {
            source: m.source.clone(),
            url: m.url.clone(),
            title: m.title.clone(),
        }))
    });

    column![
        title,
        section_header("Continuar leyendo"),
        recent_row,
        section_header("Añadidos recientemente"),
        scrollable(grid),
    ]
    .spacing(16)
    .into()
}
