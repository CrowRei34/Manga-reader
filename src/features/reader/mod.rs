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
use iced::{Color, ContentFit, Element, Length, Task};
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
    /// Panel de filtros de color abierto (bottom sheet del original).
    pub show_filters: bool,
    /// Error de carga (si hubo).
    pub error: Option<String>,
    /// Ventana en pantalla completa (botón "Pantalla Completa").
    pub is_fullscreen: bool,
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
    /// Abre/cierra el panel de filtros de color.
    ToggleFilterPanel,
    /// Selecciona un filtro de color desde el panel.
    SetFilter(ColorFilter),
    /// Muestra/oculta la barra inferior.
    ToggleUI,
    /// Alterna pantalla completa (fullscreen real, como el original).
    ToggleFullscreen,
    /// Click dentro del panel (no hace nada; evita que el tap llegue a la
    /// página y esconda la UI).
    Noop,
    /// Reintentar carga tras error.
    Retry,
    /// Regresar al detalle.
    Back,
}

fn chapter_idx(ch: &Chapter) -> i32 {
    ch.number as i32
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
            // La barra se muestra al entrar (descubrible); tap la esconde.
            state.reader.show_ui = true;
            state.reader.show_filters = false;

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
        Message::ToggleFilterPanel => {
            state.reader.show_filters = !state.reader.show_filters;
            Task::none()
        }
        Message::SetFilter(f) => {
            state.reader.color_filter = f;
            Task::none()
        }
        Message::ToggleUI => {
            // Tap sobre la página: primero cierra el panel de filtros si está
            // abierto; el siguiente tap esconde/muestra la barra.
            if state.reader.show_filters {
                state.reader.show_filters = false;
            } else {
                state.reader.show_ui = !state.reader.show_ui;
            }
            Task::none()
        }
        Message::ToggleFullscreen => {
            state.reader.is_fullscreen = !state.reader.is_fullscreen;
            set_window_mode(state.reader.is_fullscreen)
        }
        Message::Noop => Task::none(),
        Message::Retry => {
            if let Some(ch) = state.reader.chapter.clone() {
                return update(state, Message::Load(ch));
            }
            Task::none()
        }
        Message::Back => {
            // Al salir, restaura la ventana si quedó en fullscreen.
            let restore = if state.reader.is_fullscreen {
                state.reader.is_fullscreen = false;
                set_window_mode(false)
            } else {
                Task::none()
            };
            Task::batch([
                restore,
                Task::done(AppMessage::NavigateTo(crate::features::Screen::Details)),
            ])
        }
    }
}

/// Cambia el modo de la ventana (fullscreen real ⇄ ventana).
fn set_window_mode(fullscreen: bool) -> Task<AppMessage> {
    let mode = if fullscreen {
        iced::window::Mode::Fullscreen
    } else {
        iced::window::Mode::Windowed
    };
    iced::window::get_latest().then(move |maybe_id| {
        if let Some(id) = maybe_id {
            iced::window::change_mode(id, mode)
        } else {
            Task::none()
        }
    })
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

/// Tint aproximado del filtro de color (iced no soporta matrices de color
/// sobre imágenes; se superpone una capa translúcida sobre la página).
fn filter_tint(f: ColorFilter) -> Option<Color> {
    match f {
        ColorFilter::None => None,
        ColorFilter::Grayscale => Some(Color::from_rgba(0.55, 0.55, 0.55, 0.30)),
        ColorFilter::Sepia => Some(Color::from_rgba(0.72, 0.53, 0.30, 0.25)),
        ColorFilter::Bluelight => Some(Color::from_rgba(1.00, 0.62, 0.20, 0.20)),
    }
}

/// Vista del lector: webtoon (scroll vertical) o paginated (una página),
/// con overlays flotantes: X para salir, chip contador de páginas, panel
/// inferior translúcido y panel de filtros — espejo del panel del original.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    use iced::widget::{mouse_area, stack};

    // --- Contenido de páginas (área central, fondo negro puro) ---
    let page_element = |path: &PathBuf, fit: ContentFit| -> Element<'_, AppMessage> {
        image(image::Handle::from_path(path.clone()))
            .content_fit(fit)
            .width(Length::Fill)
            .into()
    };

    let pages_view: Element<'_, AppMessage> = if let Some(e) = &state.reader.error {
        // Error
        container(
            column![
                text("Error al cargar páginas").size(18).color(palette::DANGER),
                text(e.clone()).size(13).color(palette::TEXT_MUTED),
                button(text("Reintentar").size(14))
                    .on_press(AppMessage::Reader(Message::Retry))
                    .style(crate::theme::ghost_button)
                    .padding([8, 16]),
            ]
            .spacing(12),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    } else if state.reader.loading || state.reader.page_paths.is_empty() {
        // Cargando
        container(text("Cargando páginas…").size(16).color(palette::TEXT_MUTED))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else {
        match state.reader.read_mode {
            // Tira continua centrada (spacing 0: los webtoons son un strip
            // sin cortes), max 900px de ancho como el original.
            ReadMode::Webtoon => scrollable(
                container(
                    Column::with_children(
                        state
                            .reader
                            .page_paths
                            .iter()
                            .filter(|p| p.as_os_str().len() > 0) // skip placeholders vacíos
                            .map(|p| page_element(p, ContentFit::Contain))
                            .collect::<Vec<_>>(),
                    )
                    .spacing(0)
                    .max_width(900),
                )
                .center_x(Length::Fill),
            )
            .height(Length::Fill)
            .into(),
            ReadMode::Paginated => {
                let current = state
                    .reader
                    .page_paths
                    .get(state.reader.current_page)
                    .map(|p| page_element(p, ContentFit::Contain))
                    .unwrap_or_else(|| text("Sin página").into());
                container(current)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            }
        }
    };

    // Área central: tap para mostrar/ocultar UI.
    let page_area: Element<'_, AppMessage> = mouse_area(pages_view)
        .on_press(AppMessage::Reader(Message::ToggleUI))
        .into();

    // --- Botón X (cerrar) top-left, círculo terracota ---
    let close_btn = container(
        button(icon::glyph(icon::CLOSE, 18, palette::ON_ACCENT))
            .on_press(AppMessage::Reader(Message::Back))
            .style(|_t, _s| button::Style {
                background: Some(palette::ACCENT.into()),
                text_color: palette::ON_ACCENT,
                border: iced::Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: crate::theme::radius(20.0),
                },
                ..Default::default()
            })
            .padding(10),
    )
    .padding(16);

    // --- Panel inferior flotante (translúcido, redondeado, como el original) ---
    let chapter_title = state
        .reader
        .chapter
        .as_ref()
        .map(|c| {
            if c.title.is_empty() {
                format!("Ch. {}", state.reader.current_chapter + 1)
            } else {
                c.title.clone()
            }
        })
        .unwrap_or_default();
    let has_prev = state.reader.current_chapter > 0;
    let has_next = state.reader.current_chapter + 1 < state.reader.chapters.len();
    let filters_on = state.reader.color_filter != ColorFilter::None || state.reader.show_filters;

    let panel_link = |label: &'static str, active: bool, msg: Message| {
        button(
            text(label)
                .size(13)
                .color(if active { palette::ACCENT } else { palette::TEXT_MUTED }),
        )
        .on_press(AppMessage::Reader(msg))
        .style(crate::theme::link_button)
        .padding(2)
    };

    let bottom_panel: Element<'_, AppMessage> = mouse_area(
        container(
            row![
                button(text("‹").size(22).color(palette::TEXT))
                    .on_press_maybe(if has_prev {
                        Some(AppMessage::Reader(Message::PrevChapter))
                    } else {
                        None
                    })
                    .style(crate::theme::link_button)
                    .padding([6, 14]),
                horizontal_space(),
                column![
                    text(chapter_title).size(14).color(palette::TEXT),
                    row![
                        panel_link(
                            if state.reader.read_mode == ReadMode::Webtoon {
                                "Webtoon"
                            } else {
                                "Paginado"
                            },
                            true,
                            Message::ToggleMode,
                        ),
                        panel_link("Filtros", filters_on, Message::ToggleFilterPanel),
                        panel_link("Pantalla Completa", state.reader.is_fullscreen, Message::ToggleFullscreen),
                    ]
                    .spacing(14),
                ]
                .spacing(3)
                .align_x(iced::Alignment::Center),
                horizontal_space(),
                button(text("›").size(22).color(palette::TEXT))
                    .on_press_maybe(if has_next {
                        Some(AppMessage::Reader(Message::NextChapter))
                    } else {
                        None
                    })
                    .style(crate::theme::link_button)
                    .padding([6, 14]),
            ]
            .align_y(iced::Alignment::Center),
        )
        .style(crate::theme::reader_panel)
        .padding([10, 16])
        .max_width(680),
    )
    .on_press(AppMessage::Reader(Message::Noop))
    .into();

    // --- Panel de filtros de color (bottom sheet del original) ---
    let filter_panel: Element<'_, AppMessage> = {
        let mut opts = column![
            text("Filtros de color").size(12).color(palette::TEXT_MUTED),
        ]
        .spacing(6);
        for f in ColorFilter::all() {
            let active = *f == state.reader.color_filter;
            opts = opts.push(
                button(
                    row![
                        icon::glyph(
                            icon::CHECK,
                            15,
                            if active { palette::ACCENT } else { Color::TRANSPARENT },
                        ),
                        text(f.label()).size(13),
                    ]
                    .spacing(10)
                    .align_y(iced::Alignment::Center),
                )
                .style(crate::theme::panel_option(active))
                .width(Length::Fill)
                .padding([7, 10])
                .on_press(AppMessage::Reader(Message::SetFilter(*f))),
            );
        }
        mouse_area(
            container(opts)
                .style(crate::theme::reader_panel)
                .padding(12)
                .width(Length::Fixed(230.0)),
        )
        .on_press(AppMessage::Reader(Message::Noop))
        .into()
    };

    // --- Chip contador de páginas (modo paginado) ---
    let page_chip: Option<Element<'_, AppMessage>> =
        if state.reader.read_mode == ReadMode::Paginated && !state.reader.page_paths.is_empty() {
            let total = state
                .reader
                .page_paths
                .iter()
                .filter(|p| p.as_os_str().len() > 0)
                .count();
            Some(
                container(
                    row![
                        button(text("‹").size(18).color(palette::TEXT))
                            .on_press(AppMessage::Reader(Message::PrevPage))
                            .style(crate::theme::link_button)
                            .padding([2, 10]),
                        text(format!("{} / {}", state.reader.current_page + 1, total))
                            .size(13)
                            .color(palette::TEXT),
                        button(text("›").size(18).color(palette::TEXT))
                            .on_press(AppMessage::Reader(Message::NextPage))
                            .style(crate::theme::link_button)
                            .padding([2, 10]),
                    ]
                    .spacing(4)
                    .align_y(iced::Alignment::Center),
                )
                .style(crate::theme::reader_chip)
                .padding([3, 6])
                .into(),
            )
        } else {
            None
        };

    // --- Ensamblaje: stack de overlays sobre fondo negro ---
    let mut layers = vec![page_area];

    // Tint del filtro (capa no interactiva: los eventos pasan a la página).
    if let Some(tint) = filter_tint(state.reader.color_filter) {
        layers.push(
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_t| container::Style {
                    background: Some(tint.into()),
                    ..Default::default()
                })
                .into(),
        );
    }

    // X arriba-izquierda.
    layers.push(
        container(close_btn)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Left)
            .align_y(iced::alignment::Vertical::Top)
            .into(),
    );

    // Chip de páginas: bottom-center, elevado sobre el panel si está visible
    // (antes ambos caían en el mismo sitio y el panel tapaba el contador).
    // Con el panel de filtros abierto se omite (ocuparía su lugar).
    if let (Some(chip), false) = (page_chip, state.reader.show_ui && state.reader.show_filters) {
        let lift = if state.reader.show_ui { 100.0 } else { 18.0 };
        layers.push(
            container(chip)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: lift,
                    left: 0.0,
                })
                .into(),
        );
    }

    // Panel inferior + panel de filtros si `show_ui`.
    if state.reader.show_ui {
        layers.push(
            container(bottom_panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Bottom)
                .padding(18)
                .into(),
        );
        if state.reader.show_filters {
            layers.push(
                container(filter_panel)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 0.0,
                        bottom: 100.0,
                        left: 0.0,
                    })
                    .into(),
            );
        }
    }

    container(stack(layers))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_t| container::Style {
            background: Some(Color::BLACK.into()),
            ..Default::default()
        })
        .into()
}