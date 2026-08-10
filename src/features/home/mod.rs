//! Pantalla de Inicio (Home). Muestra "Continuar leyendo" (recientes
//! mapeados de `history_dao::recent` → `manga_dao::get_by_id`) y
//! "Biblioteca" (espejo de `state.library`).
//!
//! `LoadRecent` corre la consulta en `tokio::task::spawn_blocking` (vía
//! `db_blocking`) porque `rusqlite::Connection` es `Send` pero el acceso
//! bloquea — no puede ir en el hilo del async runtime sin ceder. El
//! resultado aterriza en `home::Message::RecentLoaded(Result<Vec<Manga>, DbError>)`.
use iced::widget::{button, column, text};
use iced::{Element, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::db;
use crate::core::db::dao::{history_dao, manga_dao};
use crate::core::error::DbError;
use crate::core::models::Manga;
use crate::features::Screen;

#[derive(Debug, Default)]
pub struct State {
    pub recent: Vec<Manga>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadRecent,
    RecentLoaded(Result<Vec<Manga>, DbError>),
}

/// Reducer del feature Home. El `Result` acarrea `DbError` (mismo Clone
/// manual de error.rs) para mantener tipado el reporte de fallos.
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::LoadRecent => {
            let dbh = state.db.clone();
            if let Some(db) = dbh {
                Task::perform(
                    db::db_blocking(db, |conn| {
                        let ids = history_dao::recent(conn, 10)?;
                        let mut out = Vec::with_capacity(ids.len());
                        for (id, _, _, _) in ids {
                            if let Some(m) = manga_dao::get_by_id(conn, id)? {
                                out.push(m);
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
            Task::none()
        }
        Message::RecentLoaded(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
    }
}

/// Vista del feature: título + "Continuar leyendo" + "Biblioteca". Cada manga
/// lanza `NavigateTo(Screen::Details)` (placeholder; Task 14 reemplaza por
/// `Details(Message::Load(MangaRef{..}))` con el `MangaRef` concreto).
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let header = text("Bakeneko").size(32);

    let recent_row = column(state.home.recent.iter().map(|m| {
        button(text(&m.title))
            .on_press(AppMessage::NavigateTo(Screen::Details))
            .into()
    }))
    .spacing(4);

    let lib_row = column(state.library.iter().map(|m| {
        button(text(&m.title))
            .on_press(AppMessage::NavigateTo(Screen::Details))
            .into()
    }))
    .spacing(4);

    column![
        header,
        text("Continuar leyendo"),
        recent_row,
        text("Biblioteca"),
        lib_row,
    ]
    .spacing(8)
    .into()
}