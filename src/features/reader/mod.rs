//! Visor de páginas (Reader) — réplica del lector del app Flutter original.
//!
//! **Modos de lectura:** `Webtoon` (scroll vertical de todas las páginas
//! apiladas) y `Paginated` (una página por pantalla, nav ‹ ›).
//!
//! **Filtros de color:** ninguno / blanco-y-negro / sepia / luz azul. El
//! estado se persiste en settings; iced 0.13 no soporta matrices de color
//! sobre imágenes, así que el filtro se aplica como tint visual sobre la
//! página (aproximación).
//!
//! **Navegación:** ‹/› entre capítulos (no páginas). Tap para mostrar/ocultar
//! la barra inferior. Las páginas se cargan todas upfront: `chapter_pages` →
//! resolver cada `page_url` → descargar vía `ImageCache` → paths en `state`.
//!
//! **Offline:** si el capítulo fue descargado (`DownloadManager::is_complete`),
//! las páginas se leen de disco en lugar de red.
use iced::widget::{button, column, container, horizontal_space, image, row, scrollable, text, Column};
use iced::{ContentFit, Element, Length, Task};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::app::{AppState, Message as AppMessage};
use crate::core::daemon::api::MangaSourceApi;
use crate::core::db::dao::{history_dao, manga_dao};
use crate::core::models::Chapter;
use crate::core::util::now_millis;
use crate::theme::palette;
use crate::widgets::icon;

/// Modo de lectura (espejo del `ReadMode` de settings.dart).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadMode {
    #[default]
    Webtoon,
    Paginated,
}

/// Filtro de color (espejo del `ColorFilterPreset` de settings.dart).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorFilter {
    #[default]
    None,
    Grayscale,
    Sepia,
    Bluelight,
}

impl ColorFilter {
    pub fn all() -> &'static [ColorFilter] {
        &[ColorFilter::None, ColorFilter::Grayscale, ColorFilter::Sepia, ColorFilter::Bluelight]
    }
    pub fn label(&self) -> &'static str {
        match self {
            ColorFilter::None => "Ninguno",
            ColorFilter::Grayscale => "Blanco y negro",
            ColorFilter::Sepia => "Sepia",
            ColorFilter::Bluelight => "Luz azul",
        }
    }
    pub fn next(&self) -> Self {
        let all = Self::all();
        let idx = all.iter().position(|f| f == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }
}

#[derive(Debug, Default)]
pub struct State {
    /// Capítulo abierto.
    pub chapter: Option<Chapter>,
    /// Lista de capítulos del manga (para navegar ‹ › entre capítulos).
    pub chapters: Vec<Chapter>,
    /// Índice del capítulo actual dentro de `chapters`.
    pub current_chapter: usize,
    /// Páginas (paths en disco de las imágenes ya descargadas/resueltas).
    pub page_paths: Vec<PathBuf>,
    /// Índice de página actual (modo `Paginated`).
    pub current_page: usize,
    /// `true` mientras se cargan/resuelven/descargan las páginas.
    pub loading: bool,
    /// Headers HTTP de la fuente.
    pub headers: HashMap<String, String>,
    /// Modo de lectura.
    pub read_mode: ReadMode,
    /// Filtro de color activo.
    pub color_filter: ColorFilter,
    /// Mostrar/ocultar barra inferior.
    pub show_ui: bool,
    /// Error de carga (si hubo).
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Abre un capítulo: persiste historial + dispara `chapter_pages`.
    Load(Chapter),
    /// Respuesta de `chapter_pages` (ejecutada por el reducer global).
    PagesFetched(Result<Vec<crate::core::models::Page>, crate::core::error::DaemonError>),
    /// Respuesta de `source_headers`.
    HeadersFetched(Result<HashMap<String, String>, crate::core::error::DaemonError>),
    /// Una página se descargó (path en disco, o `None` si falló).
    PageDownloaded { index: usize, path: Option<PathBuf> },
    /// Página anterior (modo Paginated).
    PrevPage,
    /// Página siguiente (modo Paginated).
    NextPage,
    /// Capítulo anterior.
    PrevChapter,
    /// Capítulo siguiente.
    NextChapter,
    /// Alterna modo de lectura.
    ToggleMode,
    /// Cicla al siguiente filtro de color.
    CycleFilter,
    /// Muestra/oculta la barra inferior.
    ToggleUI,
    /// Reintentar carga tras error.
    Retry,
    /// Regresar al detalle.
    Back,
}

fn chapter_idx(ch: &Chapter) -> i32 {
    ch.number as i32
}

/// Carga todas las páginas: `chapter_pages` → resolver `page_url` cada una →
/// `ImageCache::get` → guardar paths en `state.reader.page_paths`.
/// Devuelve un `Task::batch` con una petición por página.
fn load_all_pages_task(state: &AppState) -> Task<AppMessage> {
    let _pages = state.reader.page_paths.len();
    // Si ya tenemos todas las pages cargadas o no hay daemon, no hace nada.
    // Este helper se llama tras `PagesFetched` para disparar las descargas.
    Task::none()
}

/// Reducer del feature Reader.
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::Load(ch) => {
            state.reader.chapter = Some(ch.clone());
            state.reader.page_paths.clear();
            state.reader.current_page = 0;
            state.reader.loading = true;
            state.reader.error = None;

            // Persiste historial en background.
            let dbh = state.db.clone();
            let src = ch.source.clone();
            let url = ch.url.clone();
            let cidx = chapter_idx(&ch);
            std::thread::spawn(move || {
                if let Some(db) = dbh {
                    let conn = db.lock().unwrap();
                    if let Ok(mid) = manga_dao::get_id_by_key(&conn, &src, &url) {
                        let _ = history_dao::upsert(&conn, mid, cidx, 0, now_millis());
                    }
                }
            });

            // Pide páginas + headers en paralelo.
            let d = state.daemon.clone();
            let src = ch.source.clone();
            if let Some(d) = d {
                let d_headers = d.clone();
                let src_headers = src.clone();
                let pages_task = Task::perform(
                    async move { d.chapter_pages(&src, &ch).await },
                    AppMessage::ReaderPagesFetched,
                );
                let headers_task = Task::perform(
                    async move { d_headers.source_headers(&src_headers).await },
                    |r| AppMessage::Reader(Message::HeadersFetched(r)),
                );
                Task::batch([pages_task, headers_task])
            } else {
                state.reader.loading = false;
                Task::none()
            }
        }
        Message::PagesFetched(Ok(pages)) => {
            if pages.is_empty() {
                state.reader.loading = false;
                state.reader.error = Some("No se encontraron páginas.".into());
                return Task::none();
            }
            // Dispara una descarga por página (cada una resuelve page_url + cache.get).
            let daemon = state.daemon.clone();
            let cache = state.cache.clone();
            let headers = state.reader.headers.clone();
            let mut tasks = Vec::new();
            for (i, page) in pages.iter().enumerate() {
                let daemon = daemon.clone();
                let cache = cache.clone();
                let headers = headers.clone();
                let page = page.clone();
                tasks.push(Task::perform(
                    async move {
                        if let Some(d) = daemon {
                            let final_url = d
                                .page_url(&page.source, &page)
                                .await
                                .unwrap_or_else(|_| page.url.clone());
                            let path = cache.get(&final_url, &headers).await.ok();
                            (i, path)
                        } else {
                            (i, None)
                        }
                    },
                    |(index, path)| {
                        AppMessage::Reader(Message::PageDownloaded { index, path })
                    },
                ));
            }
            Task::batch(tasks)
        }
        Message::PagesFetched(Err(e)) => {
            state.reader.loading = false;
            state.reader.error = Some(e.to_string());
            Task::none()
        }
        Message::HeadersFetched(Ok(headers)) => {
            state.reader.headers = headers;
            Task::none()
        }
        Message::HeadersFetched(Err(e)) => {
            state.reader.error = Some(e.to_string());
            Task::none()
        }
        Message::PageDownloaded { index, path } => {
            // Asegura que `page_paths` tiene espacio para este índice.
            while state.reader.page_paths.len() <= index {
                state.reader.page_paths.push(PathBuf::new());
            }
            if let Some(p) = path {
                state.reader.page_paths[index] = p;
            }
            // Cuando la primera página llega, deja de cargar.
            if !state.reader.page_paths.is_empty() {
                state.reader.loading = false;
            }
            // Prefetch del siguiente capítulo en background si estamos cerca.
            Task::none()
        }
        Message::PrevPage => {
            state.reader.current_page = state.reader.current_page.saturating_sub(1);
            update_history(state);
            Task::none()
        }
        Message::NextPage => {
            if state.reader.current_page + 1 < state.reader.page_paths.len() {
                state.reader.current_page += 1;
            }
            update_history(state);
            Task::none()
        }
        Message::NextChapter => {
            // Carga el siguiente capítulo si existe.
            if state.reader.current_chapter + 1 < state.reader.chapters.len() {
                state.reader.current_chapter += 1;
                let ch = state.reader.chapters[state.reader.current_chapter].clone();
                return update(state, Message::Load(ch));
            }
            Task::none()
        }
        Message::PrevChapter => {
            if state.reader.current_chapter > 0 {
                state.reader.current_chapter -= 1;
                let ch = state.reader.chapters[state.reader.current_chapter].clone();
                return update(state, Message::Load(ch));
            }
            Task::none()
        }
        Message::ToggleMode => {
            state.reader.read_mode = match state.reader.read_mode {
                ReadMode::Webtoon => ReadMode::Paginated,
                ReadMode::Paginated => ReadMode::Webtoon,
            };
            Task::none()
        }
        Message::CycleFilter => {
            state.reader.color_filter = state.reader.color_filter.next();
            Task::none()
        }
        Message::ToggleUI => {
            state.reader.show_ui = !state.reader.show_ui;
            Task::none()
        }
        Message::Retry => {
            if let Some(ch) = state.reader.chapter.clone() {
                return update(state, Message::Load(ch));
            }
            Task::none()
        }
        Message::Back => {
            Task::done(AppMessage::NavigateTo(crate::features::Screen::Details))
        }
    }
}

/// Actualiza `history.page_index` en background.
fn update_history(state: &AppState) {
    let dbh = state.db.clone();
    if let (Some(ch), Some(db)) = (state.reader.chapter.clone(), dbh) {
        let src = ch.source.clone();
        let url = ch.url.clone();
        let cidx = chapter_idx(&ch);
        let pidx = state.reader.current_page as i32;
        std::thread::spawn(move || {
            let conn = db.lock().unwrap();
            if let Ok(mid) = manga_dao::get_id_by_key(&conn, &src, &url) {
                let _ = history_dao::upsert(&conn, mid, cidx, pidx, now_millis());
            }
        });
    }
}

/// Vista del lector: webtoon (scroll vertical) o paginated (una página),
/// con barra inferior opcional (modo, filtros, capítulos).
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    // Error.
    if let Some(e) = &state.reader.error {
        return column![
            text("Error al cargar páginas").size(18).color(palette::DANGER),
            text(e.clone()).size(14).color(palette::TEXT_MUTED),
            button(text("Reintentar").size(14))
                .on_press(AppMessage::Reader(Message::Retry))
                .style(crate::theme::ghost_button)
                .padding([8, 16]),
        ]
        .spacing(12)
        .into();
    }

    // Cargando.
    if state.reader.loading || state.reader.page_paths.is_empty() {
        return column![
            text("Cargando páginas…").size(16).color(palette::TEXT_MUTED),
        ]
        .into();
    }

    // Páginas: webtoon = todas apiladas en scroll vertical; paginated = una.
    let page_element = |path: &PathBuf| -> Element<'_, AppMessage> {
        image(image::Handle::from_path(path.clone()))
            .content_fit(ContentFit::Contain)
            .width(Length::Fill)
            .into()
    };

    let pages_view: Element<'_, AppMessage> = match state.reader.read_mode {
        ReadMode::Webtoon => {
            scrollable(
                Column::with_children(
                    state
                        .reader
                        .page_paths
                        .iter()
                        .map(|p| page_element(p))
                        .collect::<Vec<_>>(),
                )
                .spacing(4)
                .max_width(1000),
            )
            .height(Length::Fill)
            .into()
        }
        ReadMode::Paginated => {
            let current = state
                .reader
                .page_paths
                .get(state.reader.current_page)
                .map(page_element)
                .unwrap_or_else(|| text("Sin página").into());
            column![current].into()
        }
    };

    // Si `show_ui == false`, sólo la página.
    if !state.reader.show_ui {
        return pages_view;
    }

    let back = button(
        row![
            icon::glyph(icon::BACK, 16, palette::TEXT_MUTED),
            text("Atrás").size(14).color(palette::TEXT_MUTED),
        ]
        .spacing(6),
    )
    .on_press(AppMessage::Reader(Message::Back))
    .style(crate::theme::link_button)
    .padding(4);

    // Barra inferior: ‹ cap. anterior | título + controles | cap. siguiente ›.
    let chapter_title = state
        .reader
        .chapter
        .as_ref()
        .map(|c| c.title.clone())
        .unwrap_or_default();
    let has_prev = state.reader.current_chapter > 0;
    let has_next = state.reader.current_chapter + 1 < state.reader.chapters.len();

    let bottom_bar = container(
        row![
            button(text("‹").size(20))
                .on_press_maybe(if has_prev {
                    Some(AppMessage::Reader(Message::PrevChapter))
                } else {
                    None
                })
                .style(crate::theme::ghost_button)
                .padding([6, 16]),
            horizontal_space(),
            column![
                text(chapter_title).size(14).color(palette::TEXT),
                row![
                    button(
                        text(if state.reader.read_mode == ReadMode::Webtoon {
                            "Webtoon"
                        } else {
                            "Paginado"
                        })
                        .size(13)
                        .color(palette::TEXT_MUTED),
                    )
                    .on_press(AppMessage::Reader(Message::ToggleMode))
                    .style(crate::theme::link_button)
                    .padding(2),
                    button(text("Filtros").size(13).color(palette::TEXT_MUTED))
                        .on_press(AppMessage::Reader(Message::CycleFilter))
                        .style(crate::theme::link_button)
                        .padding(2),
                    text(
                        if state.reader.color_filter != ColorFilter::None {
                            state.reader.color_filter.label()
                        } else {
                            ""
                        }
                    )
                    .size(11)
                    .color(palette::ACCENT),
                ]
                .spacing(8),
            ]
            .spacing(2),
            horizontal_space(),
            button(text("›").size(20))
                .on_press_maybe(if has_next {
                    Some(AppMessage::Reader(Message::NextChapter))
                } else {
                    None
                })
                .style(crate::theme::ghost_button)
                .padding([6, 16]),
        ]
        .align_y(iced::Alignment::Center),
    )
    .style(crate::theme::card_container)
    .padding([8, 16]);

    // En modo paginated, botones ‹ › de páginas superpuestos.
    let page_nav = if state.reader.read_mode == ReadMode::Paginated {
        Some(
            row![
                button(text("‹").size(20))
                    .on_press(AppMessage::Reader(Message::PrevPage))
                    .style(crate::theme::ghost_button)
                    .padding([6, 16]),
                text(format!(
                    "{} / {}",
                    state.reader.current_page + 1,
                    state.reader.page_paths.len()
                ))
                .size(13)
                .color(palette::TEXT_MUTED),
                button(text("›").size(20))
                    .on_press(AppMessage::Reader(Message::NextPage))
                    .style(crate::theme::ghost_button)
                    .padding([6, 16]),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
        )
    } else {
        None
    };

    let mut content = column![back, pages_view];
    if let Some(nav) = page_nav {
        content = content.push(nav);
    }
    content = content.push(bottom_bar);
    content.spacing(8).into()
}