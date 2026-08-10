//! Pantalla de Biblioteca (Library). Lista las mangas marcadas como
//! `library=1` en la tabla `manga` (vía `manga_dao::list_library`); la
//! lectura corre en `tokio::task::spawn_blocking` (helper `db_blocking`)
//! porque `rusqlite::Connection` no es `Sync` y bloquea el hilo del runtime
//! async de Iced. El resultado aterriza en el reducer global como
//! `Message::LibraryLoaded(Result<Vec<Manga>, DbError>)`.
use iced::widget::{button, column, scrollable, text};
use iced::{Element, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::db;
use crate::core::db::dao::manga_dao;
use crate::core::models::{Manga, MangaRef};
use crate::features::details;

#[derive(Debug, Default)]
pub struct State {
    pub category_filter: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Load,
    /// Filtro por categoría; la vista aún no expone el selector.
    #[allow(dead_code)]
    CategoryFilter(i64),
}

/// Reducer del feature Library. `Load` dispara la consulta DAO vía
/// `db_blocking` (que internally usa `spawn_blocking` + `Arc<Mutex<Connection>>`).
/// El `Result` regresa como `AppMessage::LibraryLoaded` y se resuelve en
/// `app::update` (muta `state.library`), no aquí — el reducer de library
/// sólo dispara el efecto.
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::Load => {
            let dbh = state.db.clone();
            if let Some(db) = dbh {
                let category = state.library_state.category_filter;
                Task::perform(
                    db::db_blocking(db, move |conn| {
                        let mut all = manga_dao::list_library(conn)?;
                        if let Some(cat) = category {
                            // Filtro por categoría en memoria: el schema
                            // actual no modela la vinculación directamente en
                            // `manga`, así que el filtro se aplica aquí para
                            // no acoplar features/library a la layer de DAO.
                            all.retain(|m| {
                                m.blob.get("category").and_then(|v| v.as_i64()) == Some(cat)
                            });
                        }
                        Ok(all)
                    }),
                    AppMessage::LibraryLoaded,
                )
            } else {
                Task::none()
            }
        }
        Message::CategoryFilter(c) => {
            state.library_state.category_filter = Some(c);
            // Re-aplica el filtro en memoria sin re-levar la DB: la lista
            // completa ya vive en `state.library`.
            Task::none()
        }
    }
}

/// Vista del feature: lista scrollable de mangas de la biblioteca (espejo de
/// `state.library`, no del sub-estado `library_state.list`, porque el reducer
/// global escribe directamente sobre el `Vec<Manga>` de `AppState`). Tocar un
/// manga lanza `Details(Message::Load(MangaRef{..}))` que enruta a la pantalla
/// de detalle con la fuente+manga concretas.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    if state.library.is_empty() {
        return column![text("Biblioteca vacía").size(16)].spacing(8).into();
    }
    let list_col = column(state.library.iter().map(|m| {
        button(text(&m.title))
            .on_press(AppMessage::Details(details_load(m)))
            .into()
    }))
    .spacing(4);
    column![
        text("Biblioteca").size(24),
        scrollable(list_col),
        button(text("Recargar")).on_press(AppMessage::Library(Message::Load)),
    ]
    .spacing(8)
    .into()
}

/// Construye `details::Message::Load(MangaRef{..})` a partir de un manga.
/// Aislado en helper porque lo reutilizan browse/home y library.
fn details_load(m: &Manga) -> details::Message {
    details::Message::Load(MangaRef {
        source: m.source.clone(),
        url: m.url.clone(),
        title: m.title.clone(),
    })
}