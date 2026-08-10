//! Visor de páginas (Reader). Recibe un `Chapter` (normalmente desde
//! `details::view` al tocar un capítulo) y pide al daemon las páginas
//! (`MangaSourceApi::chapter_pages`) que aterrizan como
//! `Message::PagesFetched(Result<Vec<Page>, DaemonError>)`. La vista muestra
//! la página actual como imagen real (descargada/cacheada vía `ImageCache`,
//! Task 16) junto con navegación `‹`/`›` y un contador `página / total`.
//!
//! Cada apertura de capítulo persiste el historial en la DB
//! (`history_dao::upsert` con `chapter_index`, `page_index=0`) en background
//! vía `std::thread::spawn` (fire-and-forget: el reader no bloquea su flow
//! de carga por una escritura de historial), y cada cambio de página
//! actualiza el `page_index`. Los DAOs viven en `crate::core::db::dao`;
//! `manga_dao::get_id_by_key` resuelve el `manga_id` a partir de
//! `source`+`url` del capítulo.
//!
//! Las imágenes se cargan bajo demanda: al llegar `PagesFetched` (y en cada
//! `Prev`/`Next`) se dispara un `Task::perform` que descarga la página actual
//! a través de `ImageCache::get` (la primera vez baja; después sale del disco)
//! y un prefetch fire-and-forget de las vecinas (`tokio::spawn`) para que ya
//! estén cacheadas cuando el usuario navegue. El resultado aterriza como
//! `Message::ImageReady { url, path }`, que guarda el path en
//! `state.reader.image_path` y la vista lo pinta con `Handle::from_path`.
use iced::widget::{button, column, horizontal_space, image, row, scrollable, text};
use iced::{Element, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::daemon::api::MangaSourceApi;
use crate::core::db::dao::{history_dao, manga_dao};
use crate::core::models::Chapter;
use crate::core::util::now_millis;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct State {
    /// Capítulo abierto (presente desde `Load` hasta el siguiente `Load`).
    pub chapter: Option<Chapter>,
    /// Páginas devueltas por `chapter_pages`.
    pub pages: Vec<crate::core::models::Page>,
    /// Índice de la página actual (base 0).
    pub current: usize,
    /// `true` entre `Load` y la llegada de `PagesFetched`.
    pub loading: bool,
    /// Path en disco de la imagen cacheada de la página actual (`None` =
    /// aún no descargada). Se pinta con `Handle::from_path`.
    pub image_path: Option<PathBuf>,
    /// URL de la página a la que corresponde `image_path` (para invalidar al
    /// navegar y evitar pintar una imagen de otra página).
    pub image_url: Option<String>,
    /// Headers HTTP de la fuente activa (`source_headers`), necesarios para
    /// descargar las imágenes (auth/cloudflare, etc.).
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Abre un capítulo: persiste historial + dispara `chapter_pages`.
    Load(Chapter),
    /// Respuesta de `chapter_pages` (ejecutada por el reducer global).
    PagesFetched(Result<Vec<crate::core::models::Page>, crate::core::error::DaemonError>),
    /// Página anterior (saturando en 0); dispara `PageChanged` después.
    Prev,
    /// Página siguiente (limitada por el total); dispara `PageChanged` después.
    Next,
    /// Actualiza `history.page_index` tras `Prev`/`Next` (DB en background).
    PageChanged,
    /// Resultado de `ImageCache::get` para la página `url`: `path` en disco
    /// (`None` = fallo). Guarda el path si sigue siendo la página actual.
    ImageReady { url: String, path: Option<PathBuf> },
    /// Respuesta de `source_headers` (ejecutada en `Load`, en paralelo con
    /// `chapter_pages`); alimenta las peticiones de imagen con los headers
    /// de la fuente.
    HeadersFetched(Result<HashMap<String, String>, crate::core::error::DaemonError>),
}

/// `chapter_index` proxy para el historial. No existe un campo `index`
/// estable en `Chapter`, así que usamos `number` truncado a `i32`.
fn chapter_idx(ch: &Chapter) -> i32 {
    ch.number as i32
}

/// Construye el `Task` que descarga (o reusa de caché) la imagen de la página
/// `index` vía `ImageCache::get` y aterriza en `Message::ImageReady`.
fn current_image_task(state: &AppState) -> Option<Task<AppMessage>> {
    let page = state.reader.pages.get(state.reader.current)?;
    let url = page.url.clone();
    let cache = state.cache.clone();
    let headers = state.reader.headers.clone();
    Some(Task::perform(
        async move {
            let path = cache.get(&url, &headers).await.ok();
            (url, path)
        },
        |(url, path)| AppMessage::Reader(Message::ImageReady { url, path }),
    ))
}

/// Prefetch fire-and-forget de las páginas vecinas (`current±1`) para que ya
/// estén en disco cuando el usuario navegue. Se corre en un `tokio::spawn`
/// desacoplado (no bloquea el reducer ni produce mensajes).
fn prefetch_pages(state: &AppState) {
    let c = state.reader.current;
    let n = state.reader.pages.len();
    for idx in [c.saturating_sub(1), c.saturating_add(1)] {
        if idx >= n {
            continue;
        }
        let Some(page) = state.reader.pages.get(idx) else { continue };
        let url = page.url.clone();
        let cache = state.cache.clone();
        let headers = state.reader.headers.clone();
        tokio::spawn(async move {
            let _ = cache.get(&url, &headers).await;
        });
    }
}

/// Reducer del feature Reader. Muta `state.reader` y devuelve
/// `Task<AppMessage>` (el wrapper global es `AppMessage::Reader(…)`).
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::Load(ch) => {
            state.reader.chapter = Some(ch.clone());
            state.reader.pages.clear();
            state.reader.current = 0;
            state.reader.loading = true;
            state.reader.image_path = None;
            state.reader.image_url = None;

            // Persiste historial en background: `chapter_index` desde
            // `ch.number`, `page_index=0` (abrimos en la primera página).
            // `manga_dao::get_id_by_key` resuelve el `manga_id` por
            // `source`+`url` — el capítulo comparte esas claves con el manga.
            // Fire-and-forget (`std::thread::spawn`): el reader no bloquea
            // su flow de carga por una escritura de historial.
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

            // Pide las páginas al daemon y, en paralelo, los headers de la
            // fuente (para las peticiones de imagen). Se recogen con `batch`
            // para correr ambas RPCs sin bloquear el event loop.
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
            state.reader.pages = pages;
            state.reader.current = 0;
            state.reader.loading = false;
            // Descarga la imagen de la primera página + prefetch de la siguiente.
            if let Some(t) = current_image_task(state) {
                prefetch_pages(state);
                t
            } else {
                Task::none()
            }
        }
        Message::PagesFetched(Err(e)) => {
            state.error = Some(e.to_string());
            state.reader.loading = false;
            Task::none()
        }
        Message::HeadersFetched(Ok(headers)) => {
            state.reader.headers = headers;
            // Si las páginas ya llegaron pero la imagen de la página actual
            // aún no se descargó (depende de los headers), la pedimos ahora.
            if state.reader.image_path.is_none() {
                if let Some(t) = current_image_task(state) {
                    prefetch_pages(state);
                    return t;
                }
            }
            Task::none()
        }
        Message::HeadersFetched(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
        Message::Prev => {
            state.reader.current = state.reader.current.saturating_sub(1);
            let mut tasks = vec![Task::done(AppMessage::Reader(Message::PageChanged))];
            if let Some(t) = current_image_task(state) {
                prefetch_pages(state);
                tasks.push(t);
            }
            Task::batch(tasks)
        }
        Message::Next => {
            if state.reader.current + 1 < state.reader.pages.len() {
                state.reader.current += 1;
            }
            let mut tasks = vec![Task::done(AppMessage::Reader(Message::PageChanged))];
            if let Some(t) = current_image_task(state) {
                prefetch_pages(state);
                tasks.push(t);
            }
            Task::batch(tasks)
        }
        Message::ImageReady { url, path } => {
            // Solo pinta si la imagen corresponde a la página actual (evita
            // carreras con el prefetch de vecinas).
            if state.reader.pages.get(state.reader.current).map(|p| &p.url) == Some(&url) {
                state.reader.image_path = path;
            }
            Task::none()
        }
        Message::PageChanged => {
            // Solo actualiza `page_index`; conserva el `chapter_index`
            // derivado del capítulo abierto. Mismo patrón fire-and-forget
            // que en `Load` (background thread).
            let dbh = state.db.clone();
            if let Some(ch) = state.reader.chapter.clone() {
                let src = ch.source.clone();
                let url = ch.url.clone();
                let cidx = chapter_idx(&ch);
                let pidx = state.reader.current as i32;
                std::thread::spawn(move || {
                    if let Some(db) = dbh {
                        let conn = db.lock().unwrap();
                        if let Ok(mid) = manga_dao::get_id_by_key(&conn, &src, &url) {
                            let _ = history_dao::upsert(&conn, mid, cidx, pidx, now_millis());
                        }
                    }
                });
            }
            Task::none()
        }
    }
}

/// Vista del feature: imagen real de la página actual (cacheada por
/// `ImageCache`), placeholder mientras descarga, contador `actual / total`
/// y botones de navegación `‹`/`›`.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    if state.reader.loading || state.reader.pages.is_empty() {
        return column![text("Cargando…").size(16)].spacing(8).into();
    }

    // Imagen real: si `ImageCache` ya la dejó en disco, la pintamos con
    // `Handle::from_path`; si no, placeholder hasta que aterrice `ImageReady`.
    let page_view: Element<'_, AppMessage> = match &state.reader.image_path {
        Some(path) => image(iced::widget::image::Handle::from_path(path.clone()))
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into(),
        None => scrollable(text("Descargando imagen…").size(14))
            .height(iced::Length::Fill)
            .into(),
    };

    let counter = text(format!(
        "{} / {}",
        state.reader.current + 1,
        state.reader.pages.len()
    ));

    let nav = row![
        button(text("‹")).on_press(AppMessage::Reader(Message::Prev)),
        horizontal_space(),
        counter,
        horizontal_space(),
        button(text("›")).on_press(AppMessage::Reader(Message::Next)),
    ]
    .spacing(8);

    column![page_view, nav].spacing(8).into()
}