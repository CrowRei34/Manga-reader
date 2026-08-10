//! Pantalla de Exploración (Browse). Selector de fuentes (uno de
//! `state.sources`), buscador opcional, lista scrollable de mangas del
//! catálogo y botón "Más" para paginar (offset += 20).
//!
//! Las `Task` devueltas llaman `MangaSourceApi::catalog_list` contra el
//! `DaemonClient` (`Arc<DaemonClient>`) que vive en `AppState`. Los
//! resultados aterrizan en el reducer global como `Message::CatalogListed`.
use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Element, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::daemon::api::MangaSourceApi;
use crate::core::models::Manga;
use crate::features::Screen;

#[derive(Debug, Default)]
pub struct State {
    pub source: Option<String>,
    pub offset: i32,
    pub query: Option<String>,
    pub list: Vec<Manga>,
    pub loading: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SourceSelected(String),
    Refresh,
    QueryChanged(String),
    More,
}

/// Reducer del feature Browse. Muta `state.browse` y emite `Task<AppMessage>`
/// (nunca `Task<self::Message>` — el catálogo se resuelve asíncrono y el
/// resultado entra por el `Message::CatalogListed` global).
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::SourceSelected(s) => {
            state.browse.source = Some(s.clone());
            state.browse.offset = 0;
            state.browse.query = None;
            state.browse.list.clear();
            state.browse.loading = true;
            let daemon = state.daemon.clone();
            if let Some(d) = daemon {
                Task::perform(
                    async move { d.catalog_list(&s, 0, None).await },
                    AppMessage::CatalogListed,
                )
            } else {
                Task::none()
            }
        }
        Message::Refresh => {
            let s = state.browse.source.clone().unwrap_or_default();
            let q = state.browse.query.clone();
            state.browse.offset = 0;
            state.browse.loading = true;
            let daemon = state.daemon.clone();
            if let Some(d) = daemon {
                Task::perform(
                    async move { d.catalog_list(&s, 0, q.as_deref()).await },
                    AppMessage::CatalogListed,
                )
            } else {
                Task::none()
            }
        }
        Message::QueryChanged(q) => {
            state.browse.query = Some(q);
            Task::none()
        }
        Message::More => {
            let off = state.browse.offset + 20;
            state.browse.offset = off;
            state.browse.loading = true;
            let s = state.browse.source.clone().unwrap_or_default();
            let daemon = state.daemon.clone();
            if let Some(d) = daemon {
                Task::perform(
                    async move { d.catalog_list(&s, off, None).await },
                    AppMessage::CatalogListed,
                )
            } else {
                Task::none()
            }
        }
    }
}

/// Vista del feature: row de fuentes + caja de búsqueda + lista scrollable +
/// botón "Más". Cada manga lanza `NavigateTo(Screen::Details)` (Task 14
/// reemplaza el placeholder por `Details(Message::Load(MangaRef{..}))`).
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    // Fuentes: botón por cada source; el activo se marca con "✓" y se deshabilita.
    let source_row = row(
        state.sources.clone().into_iter().map(|s| {
            let is_sel = Some(&s.id) == state.browse.source.as_ref();
            let label = if is_sel {
                format!("{} ✓", s.name)
            } else {
                s.name.clone()
            };
            let btn = button(text(label));
            if is_sel {
                btn.into()
            } else {
                btn.on_press(AppMessage::Browse(Message::SourceSelected(s.id)))
                    .into()
            }
        }),
    )
    .spacing(8);

    // Buscador: typing → QueryChanged (sin fetch); Enter o "Buscar" → Refresh.
    let search = text_input(
        "Buscar…",
        state.browse.query.as_deref().unwrap_or(""),
    )
    .on_input(|q| AppMessage::Browse(Message::QueryChanged(q)))
    .on_submit(AppMessage::Browse(Message::Refresh));
    let search_row = row![
        search,
        button(text("Buscar")).on_press(AppMessage::Browse(Message::Refresh)),
    ]
    .spacing(8);

    // Lista de mangas: click → NavigateTo(Details) (placeholder Task 14).
    let list_col = column(state.browse.list.iter().map(|m| {
        button(text(&m.title))
            .on_press(AppMessage::NavigateTo(Screen::Details))
            .into()
    }))
    .spacing(4);

    let mut col = column![source_row, search_row, scrollable(list_col)];
    if state.browse.loading {
        col = col.push(text("Cargando…"));
    }
    col = col.push(button(text("Más")).on_press(AppMessage::Browse(Message::More)));
    col.spacing(8).into()
}