//! Pantalla de Exploración (Browse). Selector de fuentes (uno de
//! `state.sources`), buscador opcional, lista scrollable de mangas del
//! catálogo y botón "Más" para paginar (offset += 20).
//!
//! Las `Task` devueltas llaman `MangaSourceApi::catalog_list` contra el
//! `DaemonClient` (`Arc<DaemonClient>`) que vive en `AppState`. Los
//! resultados aterrizan en el reducer global como `Message::CatalogListed`.
use iced::widget::{button, column, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::daemon::api::MangaSourceApi;
use crate::core::models::{Manga, MangaRef};
use crate::features::details;
use crate::theme::palette;
use crate::widgets::cover::cover_grid;
use crate::widgets::icon;

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
    SourceSelectedByName(String),
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
        Message::SourceSelectedByName(name) => {
            let id = state
                .sources
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.id.clone());
            match id {
                Some(id) => update(state, Message::SourceSelected(id)),
                None => Task::none(),
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

/// Vista del feature (réplica del diseño original): header con ícono +
/// título "Explorar" + dropdown de fuentes (1356 — el `pick_list` scrollea)
/// + búsqueda + botón "Buscar" terracota; grid scrollable de cover cards.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    // Header: ícono + título + dropdown fuente + input búsqueda + botón.
    // Las fuentes deshabilitadas en Extensiones no se ofrecen aquí.
    let names: Vec<String> = state
        .sources
        .iter()
        .filter(|s| !state.extensions.disabled.contains(&s.id))
        .map(|s| s.name.clone())
        .collect();
    let selected_name: Option<String> = state.browse.source.as_ref().and_then(|id| {
        state
            .sources
            .iter()
            .find(|s| &s.id == id)
            .map(|s| s.name.clone())
    });
    let picker = pick_list(names, selected_name, |name| {
        AppMessage::Browse(Message::SourceSelectedByName(name))
    })
    .placeholder("Fuente…")
    .style(crate::theme::dropdown)
    .menu_style(crate::theme::dropdown_menu)
    .width(Length::Fixed(180.0));

    let search = text_input(
        "Buscar manga…",
        state.browse.query.as_deref().unwrap_or(""),
    )
    .on_input(|q| AppMessage::Browse(Message::QueryChanged(q)))
    .on_submit(AppMessage::Browse(Message::Refresh))
    .style(crate::theme::search_input)
    .padding([8, 12]);

    let buscar_btn = button(text("Buscar").size(14))
        .on_press(AppMessage::Browse(Message::Refresh))
        .style(crate::theme::primary_button)
        .padding([8, 20]);

    let header = row![
        icon::glyph(icon::EXPLORE, 20, palette::ACCENT),
        text("Explorar").size(22).color(palette::TEXT),
        picker,
        search,
        buscar_btn,
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center);

    // Grid de portadas (título + autor).
    let grid = cover_grid(&state.browse.list, &state.covers, 5, |m| {
        AppMessage::Details(details::Message::Load(MangaRef {
            source: m.source.clone(),
            url: m.url.clone(),
            title: m.title.clone(),
        }))
    });

    let mut col = column![header, scrollable(grid)];
    if state.browse.loading {
        col = col.push(text("Cargando…").size(14).color(palette::TEXT_MUTED));
    }
    if !state.browse.list.is_empty() {
        col = col.push(
            button(text("Más").size(14))
                .on_press(AppMessage::Browse(Message::More))
                .style(crate::theme::ghost_button)
                .padding([8, 20]),
        );
    }
    col.spacing(16).into()
}