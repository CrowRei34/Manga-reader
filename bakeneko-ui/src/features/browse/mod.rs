//! Pantalla de Exploración (Browse). Selector de fuentes + buscador +
//! grid scrollable de mangas con **carga infinita** (al nearing bottom del
//! scroll, se pide la siguiente página y se APENDA a la lista — igual que
//! el app Flutter original `browse_controller.loadMore`).
//!
//! Las `Task` devueltas llaman `MangaSourceApi::catalog_list` contra el
//! `DaemonClient`. Los resultados aterrizan en el reducer global:
//! `CatalogListed` (carga inicial, reemplaza) y `CatalogMoreListed`
//! (paginación, agrega).
use iced::widget::{button, column, container, row, scrollable, text, text_input, toggler, Space, Stack};
use iced::{Element, Length, Task};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::daemon::api::MangaSourceApi;
use bakeneko_core::models::{Manga, Source};
use crate::theme::palette;
use crate::widgets::icon;

const MAX_SEARCH_SOURCES: usize = 24;
const MAX_CONCURRENT_SEARCHES: usize = 6;
const CATEGORIES: [(&str, &str); 17] = [
    ("action", "Acción"), ("adventure", "Aventura"), ("comedy", "Comedia"),
    ("drama", "Drama"), ("fantasy", "Fantasía"), ("romance", "Romance"),
    ("school", "Escolar"), ("mystery", "Misterio"), ("horror", "Terror"),
    ("sci-fi", "Ciencia ficción"), ("sports", "Deportes"), ("isekai", "Isekai"),
    ("slice-of-life", "Slice of Life"), ("yaoi", "Yaoi / BL"), ("yuri", "Yuri / GL"),
    ("ecchi", "Ecchi"), ("hentai", "Hentai / Adulto"),
];

#[derive(Debug, Default)]
pub struct State {
    pub source: Option<String>,
    pub selected_sources: HashSet<String>,
    pub source_offsets: HashMap<String, i32>,
    pub exhausted_sources: HashSet<String>,
    pub source_panel_open: bool,
    pub category_panel_open: bool,
    pub selected_categories: HashSet<String>,
    pub adult_filter: AdultFilter,
    pub source_query: String,
    pub source_language: Option<String>,
    pub category_query: String,
    pub search_generation: u64,
    pub pending_sources: usize,
    pub search_errors: Vec<String>,
    pub failed_sources: HashSet<String>,
    /// Distingue la portada inicial de una exploración que terminó sin obras.
    pub has_searched: bool,
    pub query: Option<String>,
    /// Consulta que produjo la lista actual. A diferencia de `query`, no
    /// cambia mientras el usuario todavía está escribiendo.
    pub active_query: Option<String>,
    pub group_results: bool,
    pub visible_per_source: HashMap<String, usize>,
    pub list: Vec<Manga>,
    pub loading: bool,
    /// Carga de página siguiente en curso.
    pub loading_more: bool,
    /// `false` cuando la última petición volvió vacía → no hay más.
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AdultFilter {
    #[default]
    All,
    Safe,
    Adult,
}

#[derive(Debug, Clone)]
pub enum Message {
    SourceSelected(String),
    QueryChanged(String),
    Search,
    RetryFailed,
    ToggleSourcePanel,
    ToggleCategoryPanel,
    ToggleCategory(String),
    ClearCategories,
    SetAdultFilter(AdultFilter),
    CategoryQueryChanged(String),
    SourceQueryChanged(String),
    SetSourceLanguage(Option<String>),
    ToggleSearchSource(String, bool),
    SelectAllSources,
    ClearSources,
    More,
    MoreSource(String),
    /// Disparado por `on_scroll` cuando el viewport se acerca al fondo.
    Scrolled(f32),
}

/// Reducer del feature Browse.
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::SourceSelected(s) => {
            state.browse.source = Some(s.clone());
            state.browse.selected_sources.clear();
            state.browse.selected_sources.insert(s);
            start_search(state, false)
        }
        Message::QueryChanged(q) => {
            state.browse.query = Some(q);
            Task::none()
        }
        Message::Search => {
            state.browse.source_panel_open = false;
            state.browse.category_panel_open = false;
            start_search(state, false)
        }
        Message::RetryFailed => {
            let failed: Vec<String> = state.browse.failed_sources.iter().cloned().collect();
            if failed.is_empty() {
                Task::none()
            } else {
                start_search_for(state, false, Some(failed))
            }
        }
        Message::ToggleSourcePanel => {
            state.browse.source_panel_open = !state.browse.source_panel_open;
            state.browse.category_panel_open = false;
            Task::none()
        }
        Message::ToggleCategoryPanel => {
            state.browse.category_panel_open = !state.browse.category_panel_open;
            state.browse.source_panel_open = false;
            Task::none()
        }
        Message::ToggleCategory(category) => {
            if !state.browse.selected_categories.remove(&category) {
                state.browse.selected_categories.insert(category);
            }
            Task::none()
        }
        Message::ClearCategories => {
            state.browse.selected_categories.clear();
            Task::none()
        }
        Message::SetAdultFilter(filter) => {
            state.browse.adult_filter = filter;
            Task::none()
        }
        Message::CategoryQueryChanged(q) => {
            state.browse.category_query = q;
            Task::none()
        }
        Message::SourceQueryChanged(q) => {
            state.browse.source_query = q;
            Task::none()
        }
        Message::SetSourceLanguage(language) => {
            state.browse.source_language = language;
            Task::none()
        }
        Message::ToggleSearchSource(id, enabled) => {
            if enabled {
                if state.browse.selected_sources.len() >= MAX_SEARCH_SOURCES {
                    state.browse.search_errors = vec![format!(
                        "Puedes buscar en un máximo de {MAX_SEARCH_SOURCES} fuentes a la vez."
                    )];
                } else {
                    state.browse.selected_sources.insert(id);
                    state.browse.search_errors.clear();
                }
            } else {
                state.browse.selected_sources.remove(&id);
                state.browse.search_errors.clear();
            }
            Task::none()
        }
        Message::SelectAllSources => {
            state.browse.selected_sources = state.sources.iter()
                .filter(|source| !state.extensions.disabled.contains(&source.id))
                .filter(|source| source_matches_filters(
                    source,
                    &state.browse.source_query,
                    state.browse.source_language.as_deref(),
                ))
                .take(MAX_SEARCH_SOURCES)
                .map(|source| source.id.clone())
                .collect();
            state.browse.search_errors.clear();
            Task::none()
        }
        Message::ClearSources => {
            state.browse.selected_sources.clear();
            state.browse.search_errors.clear();
            Task::none()
        }
        Message::More => {
            // No dispara si ya está cargando más, si no hay más, o si
            // está en la carga inicial. (Espejo de `loadMore` del Dart.)
            if state.browse.loading_more || !state.browse.has_more || state.browse.loading {
                return Task::none();
            }
            start_search(state, true)
        }
        Message::MoreSource(source) => {
            if state.browse.loading || state.browse.loading_more {
                return Task::none();
            }
            let loaded = state.browse.list.iter().filter(|manga| manga.source == source).count();
            let visible = state.browse.visible_per_source.entry(source.clone()).or_insert(10);
            *visible += 10;
            if loaded >= *visible || state.browse.exhausted_sources.contains(&source) {
                Task::none()
            } else {
                start_search_for(state, true, Some(vec![source]))
            }
        }
        Message::Scrolled(relative_y) => {
            // Dispara `More` cuando el scroll pasa del 85 % del fondo.
            if relative_y > 0.85 && state.browse.active_query.is_none() {
                update(state, Message::More)
            } else {
                Task::none()
            }
        }
    }
}

fn start_search(state: &mut AppState, append: bool) -> Task<AppMessage> {
    start_search_for(state, append, None)
}

fn start_search_for(
    state: &mut AppState,
    append: bool,
    requested_sources: Option<Vec<String>>,
) -> Task<AppMessage> {
    let Some(daemon) = state.daemon.clone() else { return Task::none() };
    let candidates: Vec<String> = requested_sources.unwrap_or_else(|| {
        state.browse.selected_sources.iter().cloned().collect()
    });
    let sources: Vec<String> = candidates.into_iter()
        .filter(|id| !state.extensions.disabled.contains(id))
        .filter(|id| !append || !state.browse.exhausted_sources.contains(id))
        .collect();
    if sources.is_empty() {
        state.browse.search_errors = vec!["Selecciona al menos una fuente para buscar.".into()];
        state.browse.loading = false;
        state.browse.loading_more = false;
        return Task::none();
    }

    if !append {
        state.browse.search_generation = state.browse.search_generation.wrapping_add(1);
        state.browse.list.clear();
        state.browse.source_offsets.clear();
        state.browse.exhausted_sources.clear();
        state.browse.visible_per_source.clear();
        state.browse.search_errors.clear();
        state.browse.failed_sources.clear();
        state.browse.has_searched = true;
        state.browse.loading = true;
        state.browse.loading_more = false;
    } else {
        state.browse.loading_more = true;
    }
    state.browse.pending_sources = sources.len();
    state.browse.has_more = true;
    let generation = state.browse.search_generation;
    let query = state.browse.query.clone().filter(|q| !q.trim().is_empty());
    if !append {
        state.browse.active_query = query.clone();
        state.browse.group_results = query.is_some() || !state.browse.selected_categories.is_empty();
    }
    let categories: Vec<String> = state.browse.selected_categories.iter().cloned().collect();
    let search_slots = Arc::new(Semaphore::new(MAX_CONCURRENT_SEARCHES));

    Task::batch(sources.into_iter().map(|source| {
        let d = daemon.clone();
        let q = query.clone();
        let categories = categories.clone();
        let search_slots = Arc::clone(&search_slots);
        let offset = if append { *state.browse.source_offsets.get(&source).unwrap_or(&0) } else { 0 };
        Task::perform(
            async move {
                let _slot = search_slots.acquire_owned().await
                    .expect("semaphore de búsqueda cerrado");
                let result = d.catalog_list_filtered(&source, offset, q.as_deref(), &categories).await;
                (source, result)
            },
            move |(source, result)| AppMessage::SearchResult {
                generation, source, result,
            },
        )
    }))
}

pub fn apply_search_result(
    state: &mut AppState,
    generation: u64,
    source: String,
    result: Result<Vec<Manga>, bakeneko_core::error::DaemonError>,
) -> Task<AppMessage> {
    if generation != state.browse.search_generation {
        return Task::none();
    }
    state.browse.pending_sources = state.browse.pending_sources.saturating_sub(1);
    match result {
        Ok(items) => {
            state.browse.failed_sources.remove(&source);
            if items.is_empty() {
                state.browse.exhausted_sources.insert(source.clone());
            } else {
                let offset = state.browse.source_offsets.entry(source).or_insert(0);
                *offset += items.len() as i32;
                let mut known: HashSet<String> = state.browse.list.iter()
                    .map(|m| format!("{}|{}", m.source, m.url)).collect();
                for manga in items {
                    if known.insert(format!("{}|{}", manga.source, manga.url)) {
                        state.browse.list.push(manga);
                    }
                }
            }
        }
        Err(error) => {
            state.browse.exhausted_sources.insert(source.clone());
            state.browse.failed_sources.insert(source.clone());
            state.browse.search_errors.push(friendly_source_error(&source, &error.to_string()));
        }
    }
    if state.browse.pending_sources == 0 {
        state.browse.loading = false;
        state.browse.loading_more = false;
        state.browse.has_more = state.browse.selected_sources.iter()
            .any(|id| !state.browse.exhausted_sources.contains(id));
    }
    crate::widgets::cover::fetch_covers(state, &state.browse.list)
}

/// Convierte errores del parser/HTTP en mensajes útiles para quien busca.
/// Los detalles completos siguen quedando en los logs del daemon.
fn friendly_source_error(source: &str, detail: &str) -> String {
    let lower = detail.to_ascii_lowercase();
    let reason = if lower.contains("google.com") || lower.contains("signin") || lower.contains("401") {
        "requiere iniciar sesión o no está disponible"
    } else if lower.contains("cannot find") || lower.contains("404") || lower.contains("not found") {
        "cambió su sitio y necesita actualización"
    } else if lower.contains("timeout") || lower.contains("unknownhost") || lower.contains("name or service") {
        "no responde en este momento"
    } else {
        "no pudo responder"
    };
    format!("{source}: {reason}. Se omitió esta fuente para continuar.")
}

/// Vista del feature (réplica del diseño original): header con ícono +
/// título "Explorar" + dropdown de fuentes (1356 — el `pick_list` scrollea)
/// + búsqueda + botón "Buscar" terracota; grid scrollable de cover cards.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let accent = crate::theme::accent(&state.settings);
    // Header: ícono + título + dropdown fuente + input búsqueda + botón.
    // Las fuentes deshabilitadas en Extensiones no se ofrecen aquí.
    let source_count = state.browse.selected_sources.len();
    let selected_languages: HashSet<&'static str> = state.sources.iter()
        .filter(|source| state.browse.selected_sources.contains(&source.id))
        .map(|source| language_label(source.language.as_deref()))
        .collect();
    let source_label = match source_count {
        0 if state.sources.is_empty() && state.daemon_ready => "Cargando fuentes…".to_owned(),
        0 => "Elige dónde buscar".to_owned(),
        count if selected_languages.len() == 1 => {
            format!("{} · {count}", selected_languages.iter().next().copied().unwrap_or("Idioma"))
        }
        1 => state.sources.iter()
            .find(|source| state.browse.selected_sources.contains(&source.id))
            .map(|source| format!("En {}", source.name))
            .unwrap_or_else(|| "1 fuente".to_owned()),
        count => format!("En {count} fuentes"),
    };
    let source_button = button(text(format!(
        "{}  {}",
        source_label,
        if state.browse.source_panel_open { "▲" } else { "▼" },
    )).size(13))
        .on_press(AppMessage::Browse(Message::ToggleSourcePanel))
        .style(crate::theme::ghost_button)
        .padding([8, 12]);

    let category_count = state.browse.selected_categories.len();
    let adult_label = match state.browse.adult_filter {
        AdultFilter::All => "",
        AdultFilter::Safe => " · Apto",
        AdultFilter::Adult => " · +18",
    };
    let category_button = button(text(format!(
        "Géneros{}{}  {}",
        if category_count == 0 { String::new() } else { format!(" ({category_count})") },
        adult_label,
        if state.browse.category_panel_open { "▲" } else { "▼" },
    )).size(13))
        .on_press(AppMessage::Browse(Message::ToggleCategoryPanel))
        .style(crate::theme::ghost_button)
        .padding([8, 12]);

    let search = text_input(
        "Escribe el título del manga…",
        state.browse.query.as_deref().unwrap_or(""),
    )
    .on_input(|q| AppMessage::Browse(Message::QueryChanged(q)))
    .on_submit(AppMessage::Browse(Message::Search))
    .icon(text_input::Icon {
        font: icon::ICON_FONT,
        code_point: icon::SEARCH,
        size: Some(iced::Pixels(16.0)),
        spacing: 8.0,
        side: text_input::Side::Left,
    })
    .style(crate::theme::search_input)
    .padding([8, 12]);

    let action_label = if state.browse.query.as_deref().unwrap_or("").trim().is_empty() {
        "Explorar"
    } else {
        "Buscar"
    };
    let buscar_btn = button(text(action_label).size(14))
        .on_press_maybe((source_count > 0).then_some(AppMessage::Browse(Message::Search)))
        .style(crate::theme::primary_button)
        .padding([8, 20]);

    // En ventanas estrechas los filtros bajan a una segunda fila para que el
    // campo de búsqueda conserve un ancho útil y nunca empuje el botón fuera
    // de la ventana. En escritorio amplio se mantiene la barra compacta.
    let heading = row![
        icon::glyph(icon::EXPLORE, 20, accent),
        text("Explorar").size(22).color(palette::TEXT),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);
    let filters: Element<'_, AppMessage> = if state.window_size.0 < 700.0 {
        column![
            row![source_button, category_button].spacing(8),
            row![search, buscar_btn].spacing(8).width(Length::Fill),
        ]
        .spacing(8)
        .width(Length::Fill)
        .into()
    } else {
        row![source_button, category_button, search, buscar_btn]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .width(Length::Fill)
            .into()
    };
    let header: Element<'_, AppMessage> = if state.window_size.0 < 900.0 {
        column![heading, filters].spacing(8).width(Length::Fill).into()
    } else {
        row![heading, filters].spacing(14).align_y(iced::Alignment::Center).into()
    };

    let source_panel: Option<Element<'_, AppMessage>> = state.browse.source_panel_open.then(|| {
        let filter = text_input("Filtrar fuentes…", &state.browse.source_query)
            .on_input(|q| AppMessage::Browse(Message::SourceQueryChanged(q)))
            .style(crate::theme::search_input)
            .padding([7, 10]);
        let mut languages: Vec<(String, &'static str, usize)> = Vec::new();
        for source in state.sources.iter().filter(|s| !state.extensions.disabled.contains(&s.id)) {
            let key = language_key(source.language.as_deref()).to_owned();
            let label = language_label(source.language.as_deref());
            if let Some(entry) = languages.iter_mut().find(|entry| entry.0 == key) {
                entry.2 += 1;
            } else {
                languages.push((key, label, 1));
            }
        }
        languages.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(b.1)));
        let mut language_chips = iced::widget::Row::new()
            .spacing(6)
            .padding([4, 0])
            .align_y(iced::Alignment::Center);
        language_chips = language_chips.push(
            button(text("Todos").size(12))
                .on_press(AppMessage::Browse(Message::SetSourceLanguage(None)))
                .style(crate::theme::chip_button(state.browse.source_language.is_none()))
                .padding([6, 10]),
        );
        for (key, label, count) in languages {
            let selected = state.browse.source_language.as_deref() == Some(key.as_str());
            language_chips = language_chips.push(
                button(text(format!("{label} ({count})")).size(12))
                    .on_press(AppMessage::Browse(Message::SetSourceLanguage(Some(key))))
                    .style(crate::theme::chip_button(selected))
                    .padding([6, 10]),
            );
        }
        let source_rows: Vec<Element<'_, AppMessage>> = state.sources.iter()
            .filter(|s| !state.extensions.disabled.contains(&s.id))
            .filter(|s| source_matches_filters(
                s,
                &state.browse.source_query,
                state.browse.source_language.as_deref(),
            ))
            .map(|source| {
                let selected = state.browse.selected_sources.contains(&source.id);
                let id = source.id.clone();
                row![
                    column![
                        text(source.name.clone()).size(13).color(palette::TEXT),
                        text(format!("{}  ·  {}", source.id, language_label(source.language.as_deref())))
                            .size(11).color(palette::TEXT_DIM),
                    ].spacing(1).width(Length::Fill),
                    toggler(selected)
                        .on_toggle(move |enabled| AppMessage::Browse(
                            Message::ToggleSearchSource(id.clone(), enabled)
                        ))
                        .style(crate::theme::toggle),
                ].align_y(iced::Alignment::Center).padding([5, 8]).into()
            })
            .collect();
        let panel_header = row![
            column![
                text("¿Dónde quieres buscar?").size(16).color(palette::TEXT),
                text(format!("{source_count} de {MAX_SEARCH_SOURCES} fuentes seleccionadas"))
                    .size(11).color(palette::TEXT_MUTED),
            ].spacing(2).width(Length::Fill),
            button(text("Seleccionar visibles").size(12))
                .on_press(AppMessage::Browse(Message::SelectAllSources))
                .style(crate::theme::ghost_button)
                .padding([6, 10]),
            button(text("Ninguna").size(12))
                .on_press(AppMessage::Browse(Message::ClearSources))
                .style(crate::theme::ghost_button)
                .padding([6, 10]),
            button(text("Listo").size(12))
                .on_press(AppMessage::Browse(Message::ToggleSourcePanel))
                .style(crate::theme::primary_button)
                .padding([6, 12]),
        ].spacing(8).align_y(iced::Alignment::Center);
        container(column![
            panel_header,
            row![
                text("Idioma").size(12).color(palette::TEXT_MUTED),
                scrollable(language_chips)
                    .style(crate::theme::scrollable_style)
                    .width(Length::Fill)
                    .direction(scrollable::Direction::Horizontal(Default::default())),
            ].spacing(10).align_y(iced::Alignment::Center),
            filter,
            text(format!(
                "Filtra por idioma o nombre. Se consultan {MAX_CONCURRENT_SEARCHES} fuentes a la vez para mantener la app fluida."
            ))
                .size(11).color(palette::TEXT_MUTED),
            scrollable(column(source_rows).spacing(1)).style(crate::theme::scrollable_style).width(Length::Fill).height(Length::Fill),
        ].spacing(8))
            .style(crate::theme::panel_container)
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .into()
    });

    let category_panel: Option<Element<'_, AppMessage>> = state.browse.category_panel_open.then(|| {
        let category_filter = text_input("Buscar género…", &state.browse.category_query)
            .on_input(|q| AppMessage::Browse(Message::CategoryQueryChanged(q)))
            .style(crate::theme::search_input)
            .padding([7, 10]);
        let category_query = state.browse.category_query.trim().to_lowercase();
        let mut visible_categories: Vec<(&str, &str)> = CATEGORIES.iter().copied()
            .filter(|(_, label)| category_query.is_empty() || label.to_lowercase().contains(&category_query))
            .collect();
        visible_categories.sort_by_key(|(id, _)| !state.browse.selected_categories.contains(*id));
        let mut rows: Vec<Element<'_, AppMessage>> = Vec::new();
        for chunk in visible_categories.chunks(4) {
            let mut category_row = iced::widget::Row::new().spacing(8);
            for (id, label) in chunk {
                let selected = state.browse.selected_categories.contains(*id);
                category_row = category_row.push(
                    button(text(*label).size(13))
                        .on_press(AppMessage::Browse(Message::ToggleCategory((*id).to_owned())))
                        .style(crate::theme::chip_button(selected))
                        .padding([8, 12]),
                );
            }
            rows.push(category_row.into());
        }
        let panel_header = row![
            column![
                text("Filtrar por géneros").size(16).color(palette::TEXT),
                text("Puedes combinar varios; cada fuente usará sus etiquetas equivalentes.")
                    .size(11).color(palette::TEXT_MUTED),
            ].spacing(2).width(Length::Fill),
            button(text("Limpiar").size(12))
                .on_press(AppMessage::Browse(Message::ClearCategories))
                .style(crate::theme::ghost_button)
                .padding([6, 10]),
            button(text("Listo").size(12))
                .on_press(AppMessage::Browse(Message::ToggleCategoryPanel))
                .style(crate::theme::primary_button)
                .padding([6, 12]),
        ].spacing(8).align_y(iced::Alignment::Center);
        container(column![
            panel_header,
            row![
                text("Clasificación").size(12).color(palette::TEXT_MUTED),
                button(text("Todas").size(12))
                    .on_press(AppMessage::Browse(Message::SetAdultFilter(AdultFilter::All)))
                    .style(crate::theme::chip_button(state.browse.adult_filter == AdultFilter::All))
                    .padding([6, 10]),
                button(text("Ocultar +18").size(12))
                    .on_press(AppMessage::Browse(Message::SetAdultFilter(AdultFilter::Safe)))
                    .style(crate::theme::chip_button(state.browse.adult_filter == AdultFilter::Safe))
                    .padding([6, 10]),
                button(text("Solo +18").size(12))
                    .on_press(AppMessage::Browse(Message::SetAdultFilter(AdultFilter::Adult)))
                    .style(crate::theme::chip_button(state.browse.adult_filter == AdultFilter::Adult))
                    .padding([6, 10]),
            ].spacing(8).align_y(iced::Alignment::Center),
            category_filter,
            iced::widget::Column::with_children(rows).spacing(8),
            text("El contenido adulto depende de que la fuente seleccionada lo incluya.")
                .size(11).color(palette::TEXT_DIM),
        ].spacing(14))
            .style(crate::theme::panel_container)
            .padding(14)
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true)
            .into()
    });

    // Grid de portadas (título + autor), responsivo al ancho de la ventana.
    let visible_mangas: Vec<Manga> = state.browse.list.iter()
        .filter(|manga| match state.browse.adult_filter {
            AdultFilter::All => true,
            AdultFilter::Safe => !manga.is_nsfw,
            AdultFilter::Adult => manga.is_nsfw,
        })
        .cloned()
        .collect();
    let (columns, cover_width) = crate::widgets::cover::grid_metrics(
        state.window_size.0, &state.settings.library_view,
    );
    let grid = crate::widgets::cover::search_result_grid_sized(
        &visible_mangas,
        &state.covers,
        columns,
        cover_width,
        &state.sources,
        state.browse.group_results,
        &state.browse.visible_per_source,
        &state.browse.exhausted_sources,
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
            .style(crate::theme::empty_state)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(40)
            .into()
    } else if visible_mangas.is_empty() {
        let message = if !state.browse.list.is_empty() {
            "No hay obras que coincidan con la clasificación seleccionada."
        } else if source_count == 0 {
            "Primero elige una o varias fuentes para comenzar."
        } else if !state.browse.has_searched {
            "Pulsa Explorar para ver el catálogo de las fuentes elegidas."
        } else if !state.browse.search_errors.is_empty() {
            "Las fuentes disponibles no devolvieron obras. Reintenta las que fallaron o elige otras."
        } else {
            "No encontramos obras. Prueba otro título, categoría o grupo de fuentes."
        };
        let mut empty = column![text(message).size(15).color(palette::TEXT_MUTED)]
            .align_x(iced::Alignment::Center)
            .spacing(14);
        if !state.browse.failed_sources.is_empty() {
            empty = empty.push(
                button(text(format!("Reintentar {} fuentes", state.browse.failed_sources.len())))
                    .on_press(AppMessage::Browse(Message::RetryFailed))
                    .style(crate::theme::primary_button),
            );
        }
        container(empty)
            .style(crate::theme::empty_state)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(40)
            .into()
    } else {
        column![grid, footer].spacing(16).width(Length::Fill).into()
    };

    let mut header_content = column![header].spacing(8);
    if !state.browse.search_errors.is_empty() {
        let status = if state.browse.list.is_empty() && !state.browse.loading {
            format!(
                "{} fuente(s) fallaron y las demás no devolvieron obras.",
                state.browse.search_errors.len(),
            )
        } else {
            format!(
                "{} fuente(s) no respondieron; se omitieron y la búsqueda continuó.",
                state.browse.search_errors.len(),
            )
        };
        header_content = header_content.push(
            text(status)
                .size(12).color(accent),
        );
    } else if state.browse.loading {
        header_content = header_content.push(
            text(format!("Buscando en {} fuente(s)…", state.browse.pending_sources))
                .size(12).color(palette::TEXT_MUTED),
        );
    }

    let has_header_status = !state.browse.search_errors.is_empty() || state.browse.loading;
    let compact_header = state.window_size.0 < 900.0;
    let very_compact_header = state.window_size.0 < 700.0;
    let header_height = match (compact_header, has_header_status) {
        (true, true) if very_compact_header => 180.0,
        (true, false) if very_compact_header => 156.0,
        (true, true) => 132.0,
        (true, false) => 108.0,
        (false, true) => 88.0,
        (false, false) => 68.0,
    };
    let header_container = container(header_content)
        .style(crate::theme::content_container)
        .padding(iced::Padding { top: 20.0, bottom: 8.0, left: 0.0, right: 0.0 })
        .width(Length::Fill)
        .height(Length::Fixed(header_height))
        .clip(true);

    // El selector y el grid son vistas mutuamente excluyentes. Mantenerlos
    // juntos hacía que las texturas de las portadas escaparan del clip del
    // scroll y se pintaran sobre las filas de fuentes.
    let main_area: Element<'_, AppMessage> = match (source_panel, category_panel) {
        (Some(panel), _) | (_, Some(panel)) => panel,
        (None, None) => container(
            scrollable(body)
                .style(crate::theme::scrollable_style)
                .width(Length::Fill)
                .on_scroll(|vp| {
                    let off = vp.relative_offset();
                    AppMessage::Browse(Message::Scrolled(off.y as f32))
                })
                .height(Length::Fill)
        )
        .clip(true)
        .height(Length::Fill)
        .into(),
    };

    // Las imágenes usan una capa propia del renderer. En ciertos frames esa
    // capa puede quedar por encima de un hermano anterior aunque el scroll
    // esté recortado. Reservamos el sitio de la barra en la capa base y la
    // dibujamos al final del Stack, garantizando que siempre quede arriba.
    let base: Element<'_, AppMessage> = column![
        Space::new(Length::Fill, Length::Fixed(header_height)),
        main_area,
    ]
    .spacing(8)
    .width(Length::Fill)
    .height(Length::Fill)
    .into();

    Stack::with_children(vec![base, header_container.into()])
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn language_label(locale: Option<&str>) -> &'static str {
    crate::language::label(locale)
}

fn language_key(locale: Option<&str>) -> &'static str {
    crate::language::key(locale)
}

fn source_matches_filters(source: &Source, query: &str, language: Option<&str>) -> bool {
    let query = query.trim().to_lowercase();
    let matches_query = query.is_empty()
        || source.name.to_lowercase().contains(&query)
        || source.id.to_lowercase().contains(&query);
    let matches_language = language
        .map(|selected| language_key(source.language.as_deref()) == selected)
        .unwrap_or(true);
    matches_query && matches_language
}

#[cfg(test)]
mod tests {
    use super::*;
    use bakeneko_core::error::DaemonError;

    fn manga(source: &str, url: &str) -> Manga {
        Manga { source: source.into(), url: url.into(), title: url.into(), ..Default::default() }
    }

    #[test]
    fn federated_results_merge_without_duplicates() {
        let mut state = AppState::default();
        state.browse.search_generation = 7;
        state.browse.pending_sources = 2;
        state.browse.selected_sources.extend(["A".into(), "B".into()]);

        let _ = apply_search_result(&mut state, 7, "A".into(),
            Ok(vec![manga("A", "/same"), manga("A", "/same")]));
        let _ = apply_search_result(&mut state, 7, "B".into(),
            Ok(vec![manga("B", "/same")]));

        assert_eq!(state.browse.list.len(), 2);
        assert_eq!(state.browse.source_offsets["A"], 2);
        assert_eq!(state.browse.source_offsets["B"], 1);
        assert_eq!(state.browse.pending_sources, 0);
        assert!(!state.browse.loading);
    }

    #[test]
    fn stale_results_are_ignored_and_source_errors_are_isolated() {
        let mut state = AppState::default();
        state.browse.search_generation = 3;
        state.browse.pending_sources = 1;

        let _ = apply_search_result(&mut state, 2, "OLD".into(),
            Ok(vec![manga("OLD", "/old")]));
        assert!(state.browse.list.is_empty());

        let _ = apply_search_result(&mut state, 3, "BROKEN".into(),
            Err(DaemonError::Spawn("falló".into())));
        assert_eq!(state.browse.search_errors.len(), 1);
        assert!(state.browse.failed_sources.contains("BROKEN"));
        assert_eq!(state.browse.pending_sources, 0);
    }

    #[test]
    fn source_filter_combines_language_and_text() {
        let source = Source {
            id: "MANGAFIRE_ES".into(),
            name: "MangaFire".into(),
            language: Some("es".into()),
        };
        assert!(source_matches_filters(&source, "fire", Some("es")));
        assert!(!source_matches_filters(&source, "fire", Some("en")));
        assert!(!source_matches_filters(&source, "dex", Some("es")));
    }

    #[test]
    fn language_keys_normalize_locale_variants() {
        assert_eq!(language_key(Some("pt-BR")), "pt");
        assert_eq!(language_key(Some("es-419")), "es");
        assert_eq!(language_key(None), "mixed");
    }
}
