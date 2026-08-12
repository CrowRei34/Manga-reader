//! Pantalla de Exploración (Browse). Selector de fuentes + buscador +
//! grid scrollable de mangas con **carga infinita** (al nearing bottom del
//! scroll, se pide la siguiente página y se APENDA a la lista — igual que
//! el app Flutter original `browse_controller.loadMore`).
//!
//! Las `Task` devueltas llaman `MangaSourceApi::catalog_list` contra el
//! `DaemonClient`. Los resultados aterrizan en el reducer global:
//! `CatalogListed` (carga inicial, reemplaza) y `CatalogMoreListed`
//! (paginación, agrega).
use iced::widget::{button, column, container, pick_list, row, scrollable, text, text_input};
use iced::{Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::daemon::api::MangaSourceApi;
use bakeneko_core::models::Manga;
use crate::theme::palette;
use crate::widgets::icon;

#[derive(Debug, Default)]
pub struct State {
    pub source: Option<String>,
    pub offset: i32,
    pub query: Option<String>,
    pub list: Vec<Manga>,
    pub loading: bool,
    /// Carga de página siguiente en curso.
    pub loading_more: bool,
    /// `false` cuando la última petición volvió vacía → no hay más.
    pub has_more: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SourceSelected(String),
    SourceSelectedByName(String),
    Refresh,
    QueryChanged(String),
    More,
    /// Disparado por `on_scroll` cuando el viewport se acerca al fondo.
    Scrolled(f32),
}

/// Reducer del feature Browse.
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::SourceSelected(s) => {
            state.browse.source = Some(s.clone());
            state.browse.offset = 0;
            state.browse.query = None;
            state.browse.list.clear();
            state.browse.loading = true;
            state.browse.has_more = true;
            state.browse.loading_more = false;
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
            state.browse.has_more = true;
            state.browse.list.clear();
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
            // No dispara si ya está cargando más, si no hay más, o si
            // está en la carga inicial. (Espejo de `loadMore` del Dart.)
            if state.browse.loading_more || !state.browse.has_more || state.browse.loading {
                return Task::none();
            }
            state.browse.loading_more = true;
            let s = state.browse.source.clone().unwrap_or_default();
            let off = state.browse.list.len() as i32; // offset = lista actual
            let q = state.browse.query.clone();
            let daemon = state.daemon.clone();
            if let Some(d) = daemon {
                Task::perform(
                    async move { d.catalog_list(&s, off, q.as_deref()).await },
                    AppMessage::CatalogMoreListed,
                )
            } else {
                Task::none()
            }
        }
        Message::Scrolled(relative_y) => {
            // Dispara `More` cuando el scroll pasa del 85 % del fondo.
            if relative_y > 0.85 {
                update(state, Message::More)
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
    .icon(text_input::Icon {
        font: icon::ICON_FONT,
        code_point: icon::SEARCH,
        size: Some(iced::Pixels(16.0)),
        spacing: 8.0,
        side: text_input::Side::Left,
    })
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

    // Grid de portadas (título + autor), responsivo al ancho de la ventana.
    let grid = crate::widgets::cover::cover_grid(
        &state.browse.list,
        &state.covers,
        crate::widgets::cover::per_row_for(state.window_size.0),
        crate::widgets::cover::details_msg,
    );

    // Indicador de carga al final de la lista (spinner mientras `loading_more`).
    let footer: Element<'_, AppMessage> = if state.browse.loading_more {
        text("Cargando más…").size(14).color(palette::TEXT_MUTED).into()
    } else if !state.browse.has_more && !state.browse.list.is_empty() {
        text("No hay más mangas").size(14).color(palette::TEXT_DIM).into()
    } else {
        text("").into()
    };

    // Contenido del scroll: grid + footer, o spinner centrado en carga
    // inicial (sin resultados aún). NUNCA se añade nada después de un
    // scrollable con height(Fill) — eso rompe el layout y "tapa" el grid.
    let body: Element<'_, AppMessage> = if state.browse.loading && state.browse.list.is_empty() {
        container(text("Cargando…").size(16).color(palette::TEXT_MUTED))
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(40)
            .into()
    } else {
        column![grid, footer].spacing(16).into()
    };

    let header_container = container(header)
        .style(crate::theme::content_container)
        .padding(iced::Padding { top: 0.0, bottom: 8.0, left: 0.0, right: 0.0 });

    column![
        header_container,
        container(
            scrollable(body)
                .on_scroll(|vp| {
                    let off = vp.relative_offset();
                    AppMessage::Browse(Message::Scrolled(off.y as f32))
                })
                .height(Length::Fill)
        )
        .clip(true)
        .height(Length::Fill),
    ]
    .spacing(8)
    .into()
}