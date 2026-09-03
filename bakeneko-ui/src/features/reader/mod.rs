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
use bakeneko_core::daemon::api::MangaSourceApi;
use bakeneko_core::db::dao::{history_dao, manga_dao};
use bakeneko_core::models::Chapter;
use bakeneko_core::util::now_millis;
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
    Inverted,
    Grayscale,
    Sepia,
    Bluelight,
}

impl ColorFilter {
    pub fn all() -> &'static [ColorFilter] {
        &[
            ColorFilter::None,
            ColorFilter::Inverted,
            ColorFilter::Grayscale,
            ColorFilter::Sepia,
            ColorFilter::Bluelight,
        ]
    }
    pub fn label(&self) -> &'static str {
        match self {
            ColorFilter::None => "Ninguno",
            ColorFilter::Inverted => "Invertir (Modo noche)",
            ColorFilter::Grayscale => "Blanco y negro",
            ColorFilter::Sepia => "Sepia (Papel)",
            ColorFilter::Bluelight => "Anti luz azul",
        }
    }

    pub fn from_setting(value: &str) -> Self {
        match value {
            "inverted" => Self::Inverted,
            "grayscale" => Self::Grayscale,
            "sepia" => Self::Sepia,
            "bluelight" => Self::Bluelight,
            _ => Self::None,
        }
    }

    pub fn setting(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Inverted => "inverted",
            Self::Grayscale => "grayscale",
            Self::Sepia => "sepia",
            Self::Bluelight => "bluelight",
        }
    }
}

impl ReadMode {
    pub fn from_setting(value: &str) -> Self {
        if value == "paginated" { Self::Paginated } else { Self::Webtoon }
    }

    pub fn setting(self) -> &'static str {
        match self { Self::Webtoon => "webtoon", Self::Paginated => "paginated" }
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
    /// Handles de imágenes precargados en memoria (evita I/O síncrono de disco durante render).
    pub page_handles: Vec<Option<iced::widget::image::Handle>>,
    /// Dimensiones (w, h) por página, paralelo a `page_paths`. (0,0) = aún

    /// desconocidas. Se usan para el render ventaneado del webtoon.
    pub page_dims: Vec<(u32, u32)>,
    /// Offset relativo (0..1) del scroll webtoon, para el ventaneo.
    pub scroll_y: f32,
    /// Índice de página actual (modo `Paginated`).
    pub current_page: usize,
    /// `true` mientras se cargan/resuelven/descargan las páginas.
    pub loading: bool,
    /// Identifica la carga de capítulo activa. Las respuestas de cargas
    /// anteriores se ignoran para evitar mezclar páginas al cambiar rápido.
    pub load_generation: u64,
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
    /// Generación del filtro activo; invalida trabajos anteriores.
    pub filter_generation: u64,
    pub filter_pending: usize,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Abre un capítulo: persiste historial + dispara `chapter_pages`.
    Load(Chapter),
    /// Respuesta de `chapter_pages` (ejecutada por el reducer global).
    PagesFetched {
        generation: u64,
        result: Result<Vec<bakeneko_core::models::Page>, bakeneko_core::error::DaemonError>,
    },
    /// Respuesta de `source_headers`.
    HeadersFetched {
        generation: u64,
        result: Result<HashMap<String, String>, bakeneko_core::error::DaemonError>,
    },
    /// Una página se descargó (path en disco + dimensiones, o `None` si falló).
    PageDownloaded {
        generation: u64,
        index: usize,
        entry: Option<(PathBuf, (u32, u32))>,
    },
    /// Resultado de aplicar un filtro sin bloquear el hilo de UI.
    FilteredPage {
        generation: u64,
        index: usize,
        entry: Option<(u32, u32, Vec<u8>)>,
    },

    /// Scroll del webtoon (offset relativo 0..1) para el render ventaneado.

    Scrolled(f32),
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
            state.reader.load_generation = state.reader.load_generation.wrapping_add(1);
            let generation = state.reader.load_generation;
            if let (Some(discord), Some(manga)) = (&state.discord_presence, &state.details.manga) {
                discord.set_reading(crate::discord_presence::ReadingActivity {
                    title: manga.title.clone(),
                    chapter: chapter_label_for_presence(&ch),
                    cover_url: crate::discord_presence::compatible_cover_url([
                        manga.large_cover_url.as_deref(),
                        manga.cover_url.as_deref(),
                    ]),
                    is_adult: manga.is_nsfw,
                    show_adult: state.settings.discord_show_adult,
                });
            }
            state.reader.read_mode = ReadMode::from_setting(&state.settings.reader_mode);
            state.reader.color_filter = ColorFilter::from_setting(&state.settings.reader_filter);
            state.reader.chapter = Some(ch.clone());
            state.reader.page_paths.clear();
            state.reader.page_handles.clear();
            state.reader.page_dims.clear();
            state.reader.scroll_y = 0.0;
            // Recupera la última página únicamente si corresponde al mismo capítulo.
            let manga = state.details.manga.clone();
            let saved_page = match (&state.db, &manga) {
                (Some(db), Some(manga)) => {
                    let conn = db.lock().unwrap();
                    manga_dao::upsert(&conn, manga, now_millis()).ok()
                        .and_then(|mid| history_dao::get(&conn, mid).ok().flatten())
                        .filter(|(saved_chapter, _, _)| *saved_chapter == chapter_idx(&ch))
                        .map(|(_, page, _)| page.max(0) as usize)
                        .unwrap_or(0)
                }
                _ => 0,
            };
            state.reader.current_page = saved_page;
            state.reader.loading = true;
            state.reader.error = None;
            // La barra se muestra al entrar (descubrible); tap la esconde.
            state.reader.show_ui = true;
            state.reader.show_filters = false;

            // Persiste historial en background.
            let dbh = state.db.clone();
            let cidx = chapter_idx(&ch);
            std::thread::spawn(move || {
                if let Some(db) = dbh {
                    let conn = db.lock().unwrap();
                    // El usuario puede abrir un capítulo antes de que termine
                    // el guardado asíncrono de Details. Asegura el manga aquí
                    // para que la entrada de historial no se pierda por una
                    // carrera entre ambos hilos.
                    if let Some(manga) = manga.as_ref() {
                        if let Ok(mid) = manga_dao::upsert(&conn, manga, now_millis()) {
                            let _ = history_dao::upsert(
                                &conn, mid, cidx, saved_page as i32, now_millis(),
                            );
                        }
                    }
                }
            });

            // Pide páginas + headers en paralelo.
            let d = state.daemon.clone();
            let src = ch.source.clone();
            if let Some(d) = d {
                let d_headers = d.clone();
                let src_headers = src.clone();
                eprintln!("[reader] Load: pidiendo pages+headers src={src}");
                let pages_task = Task::perform(
                    async move { d.chapter_pages(&src, &ch).await },
                    move |result| AppMessage::ReaderPagesFetched { generation, result },
                );
                let headers_task = Task::perform(
                    async move { d_headers.source_headers(&src_headers).await },
                    move |result| AppMessage::Reader(Message::HeadersFetched { generation, result }),
                );
                Task::batch([pages_task, headers_task])
            } else {
                state.reader.loading = false;
                Task::none()
            }
        }
        Message::PagesFetched { generation, result } => {
            if generation != state.reader.load_generation {
                return Task::none();
            }
            let pages = match result {
                Ok(pages) => pages,
                Err(e) => {
                    eprintln!("[reader] PagesFetched ERR: {e}");
                    state.reader.loading = false;
                    state.reader.error = Some(e.to_string());
                    return Task::none();
                }
            };
            eprintln!("[reader] PagesFetched OK: {} páginas (headers={})", pages.len(), state.reader.headers.len());
            if pages.is_empty() {
                state.reader.loading = false;
                state.reader.error = Some("No se encontraron páginas.".into());
                return Task::none();
            }
            let page_count = pages.len();
            state.reader.current_page = state.reader.current_page.min(page_count - 1);
            let restored_offset = state.reader.current_page as f32 / page_count as f32;
            // Descarga concurrente controlada (buffer_unordered de 4 páginas a la vez)
            // para no saturar Tokio, la red ni la I/O de disco.
            let daemon = state.daemon.clone();
            let cache = state.cache.clone();
            let headers = state.reader.headers.clone();
            let tasks_stream = iced::stream::channel(100, move |mut tx| async move {
                use iced::futures::SinkExt;
                use iced::futures::StreamExt;

                let stream = iced::futures::stream::iter(pages.into_iter().enumerate()).map(|(i, page)| {
                    let daemon = daemon.clone();
                    let cache = cache.clone();
                    let headers = headers.clone();
                    async move {
                        if let Some(d) = daemon {
                            let final_url = match d.page_url(&page.source, &page).await {
                                Ok(u) => u,
                                Err(e) => {
                                    eprintln!("[reader] page {i}: page_url ERR {e}");
                                    page.url.clone()
                                }
                            };
                            let mut path = None;
                            for attempt in 1..=3 {
                                match cache.get(&final_url, &headers).await {
                                    Ok(p) => {
                                        path = Some(p);
                                        break;
                                    }
                                    Err(e) => {
                                        if attempt == 3 {
                                            eprintln!("[reader] page {i}: descarga ERR final {e}");
                                        } else {
                                            tokio::time::sleep(tokio::time::Duration::from_millis(250 * attempt as u64)).await;
                                        }
                                    }
                                }
                            }
                            let entry = match path {
                                Some(p) => tokio::task::spawn_blocking(move || {
                                    fit_page_to_texture_limits(&p).map(|dims| (p, dims))
                                })
                                .await
                                .ok()
                                .flatten(),
                                None => None,
                            };
                            (i, entry)
                        } else {
                            (i, None)
                        }
                    }
                }).buffer_unordered(4);

                let mut buffered = stream;
                while let Some((index, entry)) = buffered.next().await {
                    let _ = tx.send(AppMessage::Reader(Message::PageDownloaded {
                        generation,
                        index,
                        entry,
                    })).await;
                }
            });
            Task::batch([
                Task::run(tasks_stream, |msg| msg),
                scrollable::snap_to(
                    scrollable::Id::new("reader-pages"),
                    scrollable::RelativeOffset { x: 0.0, y: restored_offset },
                ),
            ])
        }
        Message::HeadersFetched { generation, result } => {
            if generation != state.reader.load_generation {
                return Task::none();
            }
            match result {
                Ok(headers) => state.reader.headers = headers,
                Err(e) => state.reader.error = Some(e.to_string()),
            }
            Task::none()
        }
        Message::PageDownloaded { generation, index, entry } => {
            if generation != state.reader.load_generation {
                return Task::none();
            }
            eprintln!("[reader] PageDownloaded idx={index} ok={}", entry.is_some());
            while state.reader.page_paths.len() <= index {
                state.reader.page_paths.push(PathBuf::new());
                state.reader.page_handles.push(None);
                state.reader.page_dims.push((0, 0));
            }
            if let Some((p, dims)) = entry {
                let filter = state.reader.color_filter;
                let handle = load_page_handle(&p, filter)
                    .unwrap_or_else(|| iced::widget::image::Handle::from_path(p.clone()));
                state.reader.page_paths[index] = p;
                state.reader.page_handles[index] = Some(handle);
                state.reader.page_dims[index] = dims;
            }
            // Quitar pantalla de carga únicamente cuando haya al menos 1 imagen lista.
            if state.reader.page_handles.iter().any(|h| h.is_some()) {
                state.reader.loading = false;
            }
            Task::none()
        }
        Message::FilteredPage { generation, index, entry } => {
            if generation != state.reader.filter_generation {
                return Task::none();
            }
            if let Some((width, height, rgba)) = entry {
                state.reader.page_handles[index] = Some(
                    iced::widget::image::Handle::from_rgba(width, height, rgba),
                );
            }
            state.reader.filter_pending = state.reader.filter_pending.saturating_sub(1);
            Task::none()
        }





        Message::Scrolled(y) => {
            let y = if y.is_finite() { y.clamp(0.0, 1.0) } else { 0.0 };
            let total = state.reader.page_paths.len();
            if total > 0 {
                let page_idx = ((y * (total as f32)).floor() as usize).min(total - 1);
                if page_idx != state.reader.current_page {
                    state.reader.current_page = page_idx;
                    // Solo actualizamos scroll_y cuando cambia la página activa para recargar el buffer
                    state.reader.scroll_y = y;
                    update_history(state);
                }
            }
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
            state.settings.reader_mode = state.reader.read_mode.setting().into();
            let _ = bakeneko_core::settings::save(&state.settings);
            Task::none()
        }
        Message::ToggleFilterPanel => {
            state.reader.show_filters = !state.reader.show_filters;
            Task::none()
        }
        Message::SetFilter(f) => {
            state.reader.color_filter = f;
            state.settings.reader_filter = f.setting().into();
            let _ = bakeneko_core::settings::save(&state.settings);
            state.reader.filter_generation = state.reader.filter_generation.wrapping_add(1);
            let generation = state.reader.filter_generation;
            let paths: Vec<(usize, PathBuf)> = state.reader.page_paths.iter().enumerate()
                .filter(|(_, path)| !path.as_os_str().is_empty())
                .map(|(index, path)| (index, path.clone()))
                .collect();
            state.reader.filter_pending = paths.len();
            if f == ColorFilter::None {
                for (index, path) in paths {
                    state.reader.page_handles[index] =
                        Some(iced::widget::image::Handle::from_path(path));
                }
                state.reader.filter_pending = 0;
                return Task::none();
            }
            let tasks_stream = iced::stream::channel(32, move |mut tx| async move {
                use iced::futures::{SinkExt, StreamExt};
                let stream = iced::futures::stream::iter(paths.into_iter().map(|(index, path)| {
                    async move {
                        let entry = tokio::task::spawn_blocking(move || {
                            load_filtered_rgba(&path, f)
                        }).await.ok().flatten();
                        (generation, index, entry)
                    }
                })).buffer_unordered(2);
                let mut stream = stream;
                while let Some((generation, index, entry)) = stream.next().await {
                    let _ = tx.send(AppMessage::Reader(Message::FilteredPage {
                        generation, index, entry,
                    })).await;
                }
            });
            Task::run(tasks_stream, |msg| msg)
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
            update_history(state);
            if let Some(discord) = &state.discord_presence {
                discord.clear();
            }
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

fn chapter_label_for_presence(chapter: &Chapter) -> String {
    if chapter.number > 0.0 {
        format!("Capítulo {}", chapter.number)
    } else if chapter.title.trim().is_empty() {
        "Leyendo".into()
    } else {
        chapter.title.trim().to_owned()
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
    if let (Some(ch), Some(manga), Some(db)) = (
        state.reader.chapter.clone(), state.details.manga.clone(), dbh,
    ) {
        let cidx = chapter_idx(&ch);
        let pidx = state.reader.current_page as i32;
        std::thread::spawn(move || {
            let conn = db.lock().unwrap();
            if let Ok(mid) = manga_dao::upsert(&conn, &manga, now_millis()) {
                let _ = history_dao::upsert(&conn, mid, cidx, pidx, now_millis());
            }
        });
    }
}



pub fn apply_color_filter(img: &mut ::image::RgbaImage, filter: ColorFilter) {
    match filter {
        ColorFilter::None => {}
        ColorFilter::Inverted => {
            for pixel in img.pixels_mut() {
                pixel[0] = 255 - pixel[0];
                pixel[1] = 255 - pixel[1];
                pixel[2] = 255 - pixel[2];
            }
        }
        ColorFilter::Grayscale => {
            for pixel in img.pixels_mut() {
                let gray = (0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32) as u8;
                pixel[0] = gray;
                pixel[1] = gray;
                pixel[2] = gray;
            }
        }
        ColorFilter::Sepia => {
            for pixel in img.pixels_mut() {
                let r = pixel[0] as f32;
                let g = pixel[1] as f32;
                let b = pixel[2] as f32;
                pixel[0] = ((r * 0.393) + (g * 0.769) + (b * 0.189)).min(255.0) as u8;
                pixel[1] = ((r * 0.349) + (g * 0.686) + (b * 0.168)).min(255.0) as u8;
                pixel[2] = ((r * 0.272) + (g * 0.534) + (b * 0.131)).min(255.0) as u8;
            }
        }
        ColorFilter::Bluelight => {
            for pixel in img.pixels_mut() {
                pixel[0] = (pixel[0] as f32 * 1.05).min(255.0) as u8;
                pixel[1] = (pixel[1] as f32 * 0.95) as u8;
                pixel[2] = (pixel[2] as f32 * 0.70) as u8;
            }
        }
    }
}

/// Límite optimizado de textura para cualquier hardware (CPU/GPU integrado/dedicado).


/// Cap a 1400px de ancho max (ancho de pantalla de lectura es <= 900px).
const MAX_TEX_W: u32 = 1200;
const MAX_TEX_H: u32 = 3000;


/// Lee las dimensiones de la página (header, sin decode); si exceden el
/// límite de textura, re-escala el archivo en sitio. Devuelve (w, h) finales,
/// o `None` si el archivo no es una imagen decodificable.
fn fit_page_to_texture_limits(path: &std::path::Path) -> Option<(u32, u32)> {
    let mut dims_res = ::image::image_dimensions(path);
    if dims_res.is_err() {
        std::thread::sleep(std::time::Duration::from_millis(150));
        dims_res = ::image::image_dimensions(path);
    }
    let (w, h) = dims_res.unwrap_or((800, 1200));
    if w <= MAX_TEX_W && h <= MAX_TEX_H {
        return Some((w, h));
    }
    let img = match ::image::open(path) {
        Ok(i) => i,
        Err(_) => return Some((w, h)),
    };
    let scale = (MAX_TEX_W as f32 / w as f32).min(MAX_TEX_H as f32 / h as f32);
    let nw = ((w as f32 * scale) as u32).max(1);
    let nh = ((h as f32 * scale) as u32).max(1);
    let resized = img.resize(nw, nh, ::image::imageops::FilterType::Nearest);
    let tmp_path = path.with_extension("tmp_resize");
    if resized.save(&tmp_path).is_ok() {
        let _ = std::fs::rename(&tmp_path, path);
    }
    Some((nw, nh))
}

fn load_page_handle(path: &std::path::Path, filter: ColorFilter) -> Option<iced::widget::image::Handle> {
    if filter == ColorFilter::None {
        return Some(iced::widget::image::Handle::from_path(path.to_path_buf()));
    }
    let (nw, nh) = fit_page_to_texture_limits(path)?;
    let img = ::image::open(path).ok()?;
    let resized = img.resize(nw, nh, ::image::imageops::FilterType::Nearest);
    let mut rgba = resized.to_rgba8();
    apply_color_filter(&mut rgba, filter);
    let (width, height) = rgba.dimensions();
    Some(iced::widget::image::Handle::from_rgba(width, height, rgba.into_raw()))
}

fn load_filtered_rgba(
    path: &std::path::Path,
    filter: ColorFilter,
) -> Option<(u32, u32, Vec<u8>)> {
    let (width, height) = fit_page_to_texture_limits(path)?;
    let img = ::image::open(path).ok()?;
    let resized = img.resize(width, height, ::image::imageops::FilterType::Nearest);
    let mut rgba = resized.to_rgba8();
    apply_color_filter(&mut rgba, filter);
    Some((width, height, rgba.into_raw()))
}







/// Vista del lector: webtoon (scroll vertical) o paginated (una página),
/// con overlays flotantes: X para salir, chip contador de páginas, panel
/// inferior translúcido y panel de filtros — espejo del panel del original.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    use iced::widget::{mouse_area, stack};
    let accent = crate::theme::accent(&state.settings);

    // --- Contenido de páginas (área central, fondo negro puro) ---
    let page_element = |handle: &iced::widget::image::Handle, fit: ContentFit| -> Element<'_, AppMessage> {
        image(handle.clone())
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
    } else if state.reader.loading || !state.reader.page_handles.iter().any(|h| h.is_some()) {
        // Cargando páginas

        container(text("Cargando páginas…").size(16).color(palette::TEXT_MUTED))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    } else {
        match state.reader.read_mode {
            ReadMode::Webtoon => {
                let display_w = state.window_size.0.min(900.0).max(300.0);
                let viewport_h = state.window_size.1.max(400.0);

                let mut col = Column::new().spacing(4).max_width(900);
                for idx in 0..state.reader.page_paths.len() {
                    // Mantener solo cinco páginas dentro del árbol gráfico.
                    // Dibujar todos los handles a la vez llena el atlas de
                    // texturas de wgpu en capítulos largos y termina en
                    // `Not enough memory left`. Las páginas lejanas conservan
                    // su altura para que el scroll no salte.
                    let inside_render_window = idx.abs_diff(state.reader.current_page) <= 2;
                    if inside_render_window {
                        if let Some(Some(handle)) = state.reader.page_handles.get(idx) {
                            col = col.push(page_element(handle, ContentFit::Contain));
                            continue;
                        }
                    }

                    {
                        // Placeholder con la altura real escalada.
                        // También cubre páginas aún no descargadas.
                        let (w, h) = state.reader.page_dims.get(idx).copied().unwrap_or((0, 0));
                        let scaled_h = if w > 0 {
                            h as f32 * display_w / w as f32
                        } else {
                            viewport_h
                        };
                        col = col.push(iced::widget::Space::new(
                            Length::Fill,
                            Length::Fixed(scaled_h),
                        ));
                    }
                }

                scrollable(container(col).center_x(Length::Fill))
                    .id(scrollable::Id::new("reader-pages"))
                    .style(crate::theme::scrollable_style)
                    .width(Length::Fill)
                    .on_scroll(|vp| {
                        AppMessage::Reader(Message::Scrolled(vp.relative_offset().y))
                    })
                    .height(Length::Fill)
                    .into()
            }






            ReadMode::Paginated => {
                let current = state
                    .reader
                    .page_handles
                    .get(state.reader.current_page)
                    .and_then(|opt| opt.as_ref())
                    .map(|h| page_element(h, ContentFit::Contain))
                    .unwrap_or_else(|| text("Cargando página…").into());
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

    // --- Botón X (cerrar) top-left ---
    let close_btn = container(
        button(icon::glyph(icon::CLOSE, 18, palette::ON_ACCENT))
            .on_press(AppMessage::Reader(Message::Back))
            .style(crate::theme::primary_button)
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
                .color(if active { accent } else { palette::TEXT_MUTED }),
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
                            if active { accent } else { Color::TRANSPARENT },
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
