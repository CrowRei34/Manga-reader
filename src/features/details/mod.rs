//! Pantalla de Detalle (Details). Recibe un `MangaRef` (source+url+title) —
//! normalmente desde browse/home/library al tocar una manga — y pide al
//! daemon los detalles completos (`MangaSourceApi::manga_details`) que
//! incluyen `description` y `chapters`. El resultado aterriza como
//! `AppMessage::DetailsFetched(Result<Manga, DaemonError>)`, que el reducer
//! global reenvía aquí como `Message::Fetched(...)`.
//!
//! `Fetched(Ok(m))` además persiste el manga en la DB (`manga_dao::upsert`,
//! `library=0` por defecto) vía `db_blocking` para no bloquear el runtime
//! async. `AddToLibrary` hace `upsert` + `set_library_flag(id, true)` y luego
//! dispara `Library(Load)` para refrescar la pantalla de biblioteca.
use iced::widget::{button, column, scrollable, text};
use iced::{Element, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::daemon::api::MangaSourceApi;
use crate::core::db;
use crate::core::db::dao::manga_dao;
use crate::core::error::{DaemonError, DbError};
use crate::core::models::{Chapter, Manga, MangaRef};
use crate::features::library;
use crate::features::reader;
use crate::features::Screen;

#[derive(Debug, Default)]
pub struct State {
    pub manga: Option<Manga>,
    pub chapters: Vec<Chapter>,
    pub loading: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    Load(MangaRef),
    Fetched(Result<Manga, DaemonError>),
    ChapterSelected(Chapter),
    AddToLibrary,
}

/// Reducer del feature Details. Muta `state.details` y devuelve
/// `Task<AppMessage>` (los efectos async resuelven contra el reducer
/// global: `DetailsFetched`, `Library(Load)`, etc.).
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::Load(mref) => {
            state.details.loading = true;
            state.details.manga = None;
            state.details.chapters.clear();
            state.screen = Screen::Details;
            let d = state.daemon.clone();
            let src = mref.source.clone();
            let manga = Manga {
                source: mref.source,
                url: mref.url,
                title: mref.title,
                ..Default::default()
            };
            if let Some(d) = d {
                Task::perform(
                    async move { d.manga_details(&src, &manga).await },
                    AppMessage::DetailsFetched,
                )
            } else {
                Task::none()
            }
        }
        Message::Fetched(Ok(manga)) => {
            state.details.chapters = manga.chapters.clone();
            state.details.manga = Some(manga.clone());
            state.details.loading = false;
            // Persiste en DB (background) — `upsert` con `library=0` para
            // no pisar el flag si ya estaba en biblioteca.
            let dbh = state.db.clone();
            if let Some(db) = dbh {
                return Task::perform(
                    db::db_blocking(db, move |conn| {
                        manga_dao::upsert(conn, &manga, 0)?;
                        Ok::<(), DbError>(())
                    }),
                    |r| match r {
                        Ok(_) => AppMessage::ErrorDismissed,
                        Err(e) => AppMessage::LibraryLoaded(Err(e)),
                    },
                );
            }
            Task::none()
        }
        Message::Fetched(Err(e)) => {
            state.error = Some(e.to_string());
            state.details.loading = false;
            Task::none()
        }
        Message::ChapterSelected(c) => {
            // Dispara `Reader(Load(c))` (que abre el capítulo) y
            // `NavigateTo(Screen::Reader)` en paralelo vía `Task::batch`.
            Task::batch([
                Task::done(AppMessage::Reader(reader::Message::Load(c))),
                Task::done(AppMessage::NavigateTo(Screen::Reader)),
            ])
        }
        Message::AddToLibrary => {
            let m_opt = state.details.manga.clone();
            let dbh = state.db.clone();
            if let (Some(m), Some(db)) = (m_opt, dbh) {
                return Task::perform(
                    db::db_blocking(db, move |conn| {
                        let id = manga_dao::upsert(conn, &m, 0)?;
                        manga_dao::set_library_flag(conn, id, true)?;
                        Ok::<(), DbError>(())
                    }),
                    |r| match r {
                        Ok(_) => AppMessage::Library(library::Message::Load),
                        Err(e) => AppMessage::LibraryLoaded(Err(e)),
                    },
                );
            }
            Task::none()
        }
    }
}

/// Vista del feature: header (título + descripción) + botón "Agregar a
/// biblioteca" + lista scrollable de capítulos. Mientras carga (`loading`
/// y sin manga aún) muestra placeholder "Cargando…".
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    if state.details.loading && state.details.manga.is_none() {
        return column![text("Cargando…").size(16)].spacing(8).into();
    }
    let Some(m) = &state.details.manga else {
        return column![text("Sin datos").size(16)].spacing(8).into();
    };
    let title = text(&m.title).size(28);
    let description = match &m.description {
        Some(d) if !d.is_empty() => text(d),
        _ => text("Sin descripción"),
    };
    let header = column![title, description].spacing(4);

    let chapters = column(state.details.chapters.iter().map(|c| {
        button(text(&c.title))
            .on_press(AppMessage::Details(Message::ChapterSelected(c.clone())))
            .into()
    }))
    .spacing(4);

    column![
        header,
        button(text("Agregar a biblioteca"))
            .on_press(AppMessage::Details(Message::AddToLibrary)),
        scrollable(chapters),
    ]
    .spacing(8)
    .into()
}