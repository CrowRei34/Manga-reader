//! AppState + Message global + reducer `update` + `view` + `subscription`.
//!
//! El `App` (struct que implementa `iced::Application`) vive en `main.rs`
//! y delega aquí en `app::update` / `app::view` / `app::subscription`.
use iced::{Element, Subscription, Task};
use std::sync::{Arc, Mutex};

use crate::core::daemon::client::DaemonClient;
use crate::core::daemon::api::MangaSourceApi;
use crate::core::downloads::{DownloadEvent, DownloadManager};
use crate::core::error::{DaemonError, DbError};
use crate::core::models::{Manga, PingReply, Source};
use crate::core::net::ImageCache;
use crate::core::settings::Settings;
use crate::features::shell::NavMsg;
use crate::features::{browse, details, downloads, extensions, home, library, reader, settings, Screen};

/// Estado raíz de la app. Una sola `AppState` mutable a través de todos
/// los features; los sub-estados viven embebidos (`home`, `browse`, ...).
pub struct AppState {
    pub screen: Screen,
    pub error: Option<String>,
    pub settings: Settings,
    pub sources: Vec<Source>,
    pub daemon_ready: bool,
    pub daemon: Option<Arc<DaemonClient>>,
    pub db: Option<Arc<Mutex<rusqlite::Connection>>>,
    pub cache: Arc<ImageCache>,
    pub home: home::State,
    pub browse: browse::State,
    pub library_state: library::State,
    pub details: details::State,
    pub reader: reader::State,
    pub library: Vec<Manga>,
    /// Manager de descargas, creado de forma perezosa (una vez que daemon +
    /// DB están listos). `None` = aún no creado.
    pub downloads: Option<DownloadManager>,
    /// Estado de la pantalla de descargas (cola en memoria).
    pub downloads_state: downloads::State,
    /// Estado de extensiones (query + toggles).
    pub extensions: extensions::State,
    /// Portadas cacheadas: `cover_url` → path en disco (vía `ImageCache`).
    pub covers: std::collections::HashMap<String, std::path::PathBuf>,
    /// Headers HTTP para bajar portadas (Referer etc.). Por ahora vacío;
    /// se puede poblar con `source_headers` por fuente si hace falta.
    pub cover_headers: std::collections::HashMap<String, String>,
    /// Tamaño de la ventana (para grids responsivos).
    pub window_size: (f32, f32),
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            screen: Screen::default(),
            error: None,
            settings: Settings::default(),
            sources: Vec::new(),
            daemon_ready: false,
            daemon: None,
            db: None,
            cache: Arc::new(ImageCache::new()),
            home: home::State::default(),
            browse: browse::State::default(),
            library_state: library::State::default(),
            details: details::State::default(),
            reader: reader::State::default(),
            library: Vec::new(),
            downloads: None,
            downloads_state: downloads::State::default(),
            extensions: extensions::State::default(),
            covers: std::collections::HashMap::new(),
            cover_headers: std::collections::HashMap::new(),
            window_size: (1280.0, 800.0),
        }
    }
}

/// Mensaje global. Cada feature expone su sub-`Message` y se envuelve aquí.
#[derive(Debug, Clone)]
pub enum Message {
    DaemonStarted(Result<PingReply, DaemonError>),
    SourcesListed(Result<Vec<Source>, DaemonError>),
    CatalogListed(Result<Vec<Manga>, DaemonError>),
    /// Paginación: agrega a `browse.list` (no reemplaza).
    CatalogMoreListed(Result<Vec<Manga>, DaemonError>),
    DaemonDied,
    NavigateTo(Screen),
    ErrorDismissed,
    Home(home::Message),
    Browse(browse::Message),
    Library(library::Message),
    LibraryLoaded(Result<Vec<Manga>, DbError>),
    Details(details::Message),
    DetailsFetched(Result<Manga, DaemonError>),
    Reader(reader::Message),
    ReaderPagesFetched(Result<Vec<crate::core::models::Page>, DaemonError>),
    DownloadsLoaded(downloads::Loaded),
    DownloadEvent(DownloadEvent),
    Download(downloads::Message),
    Settings(settings::Message),
    Extensions(extensions::Message),
    /// Resultado de la descarga de una portada (`cover_url` → path en disco).
    CoverLoaded(String, Option<std::path::PathBuf>),
    /// Resize de la ventana (para recalcular `per_row` de los grids).
    WindowResized(f32, f32),
}

impl From<NavMsg> for Message {
    fn from(n: NavMsg) -> Self {
        match n {
            NavMsg::Navigate(s) => Message::NavigateTo(s),
        }
    }
}

/// Reducer global. Mutar `state` y devolver el `Task` que dispara
/// los siguientes efectos (carga de fuentes tras ping rpc, etc.).
pub fn update(state: &mut AppState, msg: Message) -> Task<Message> {
    match msg {
        Message::DaemonStarted(Ok(_)) => {
            state.daemon_ready = true;
            state.error = None;
            // Crea el DownloadManager de forma perezosa en cuanto daemon + DB
            // están disponibles (la subscription de eventos lo recoge en el
            // siguiente frame). Concurrencia tomada de la settings actual.
            if state.downloads.is_none() {
                if let (Some(db), Some(d)) = (state.db.clone(), state.daemon.clone()) {
                    state.downloads = Some(DownloadManager::new(
                        db,
                        d,
                        state.cache.clone(),
                        state.settings.download_concurrency as usize,
                    ));
                }
            }
            // dispara la carga inicial de fuentes + biblioteca + historial
            if let Some(d) = state.daemon.clone() {
                return Task::batch([
                    Task::perform(
                        async move { d.list_sources().await },
                        Message::SourcesListed,
                    ),
                    library::update(state, library::Message::Load),
                    home::update(state, home::Message::LoadRecent),
                ]);
            }
            Task::none()
        }
        Message::DaemonStarted(Err(e)) => {
            state.daemon_ready = false;
            state.error = Some(e.to_string());
            Task::none()
        }
        Message::SourcesListed(Ok(s)) => {
            state.sources = s;
            // Auto-selecciona la fuente default y carga su catálogo (como el
            // app original, que arranca mostrando MangaDex). Si está
            // deshabilitada en Extensiones, usa la primera disponible.
            let default_id = if state.sources.iter().any(|x| x.id == "MANGADEX")
                && !state.extensions.disabled.contains("MANGADEX")
            {
                Some("MANGADEX".to_string())
            } else {
                state
                    .sources
                    .iter()
                    .find(|x| !state.extensions.disabled.contains(&x.id))
                    .map(|x| x.id.clone())
            };
            match default_id {
                Some(id) => browse::update(state, browse::Message::SourceSelected(id)),
                None => Task::none(),
            }
        }
        Message::SourcesListed(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
        Message::CatalogListed(Ok(m)) => {
            state.browse.list = m;
            state.browse.loading = false;
            // Descarga las portadas del catálogo (async, una por URL nueva).
            let list = state.browse.list.clone();
            crate::widgets::cover::fetch_covers(state, &list)
        }
        Message::CatalogListed(Err(e)) => {
            state.browse.loading = false;
            state.error = Some(e.to_string());
            Task::none()
        }
        Message::CatalogMoreListed(Ok(more)) => {
            state.browse.loading_more = false;
            if more.is_empty() {
                state.browse.has_more = false;
                Task::none()
            } else {
                state.browse.list.extend(more.clone());
                let list = state.browse.list.clone();
                crate::widgets::cover::fetch_covers(state, &list)
            }
        }
        Message::CatalogMoreListed(Err(e)) => {
            state.browse.loading_more = false;
            state.browse.has_more = false;
            state.error = Some(e.to_string());
            Task::none()
        }
        Message::DaemonDied => {
            state.daemon_ready = false;
            state.error = Some("Daemon cerró el socket".into());
            Task::none()
        }
        Message::NavigateTo(s) => {
            state.screen = s.clone();
            // Cargas perezosas al entrar a cada pantalla.
            match s {
                Screen::Downloads => downloads::update(state, downloads::Message::Load),
                Screen::Library => library::update(state, library::Message::Load),
                Screen::Home => home::update(state, home::Message::LoadRecent),
                _ => Task::none(),
            }
        }
        Message::ErrorDismissed => {
            state.error = None;
            Task::none()
        }
        Message::Home(m) => home::update(state, m),
        Message::Browse(m) => browse::update(state, m),
        Message::Library(m) => library::update(state, m),
        Message::LibraryLoaded(Ok(list)) => {
            state.library = list;
            let list = state.library.clone();
            crate::widgets::cover::fetch_covers(state, &list)
        }
        Message::LibraryLoaded(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
        Message::Details(m) => details::update(state, m),
        Message::DetailsFetched(r) => details::update(state, details::Message::Fetched(r)),
        Message::Reader(m) => reader::update(state, m),
        Message::ReaderPagesFetched(r) => {
            reader::update(state, reader::Message::PagesFetched(r))
        }
        Message::DownloadsLoaded(Ok((entries, titles))) => {
            state.downloads_state.entries = entries;
            state.downloads_state.titles = titles;
            Task::none()
        }
        Message::DownloadsLoaded(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
        Message::DownloadEvent(ev) => {
            downloads::apply_event(state, ev);
            Task::none()
        }
        Message::Download(m) => downloads::update(state, m),
        Message::Settings(m) => settings::update(state, m),
        Message::Extensions(m) => extensions::update(state, m),
        Message::CoverLoaded(url, path) => {
            if let Some(p) = path {
                state.covers.insert(url, p);
            }
            Task::none()
        }
        Message::WindowResized(w, h) => {
            state.window_size = (w, h);
            Task::none()
        }
    }
}

/// Emite `Message::DaemonDied` si el socket del daemon se cierra. La
/// subscription lanza un stream que polla `DaemonClient::is_alive` cada 2 s
/// mientras el Arc del daemon viva en `state.daemon`. Al romperse el socket
/// (daemon muerto o `stop()`), `is_alive` devuelve `false` y se envía
/// `Message::DaemonDied` (luego el stream termina -> iced detiene la
/// subscription hasta que se reconstruya con un `Some(daemon)`).
pub fn subscription(state: &AppState) -> Subscription<Message> {
    use iced::futures::sink::SinkExt;
    use std::time::Duration;

    let mut subs: Vec<Subscription<Message>> = Vec::new();

    // Watch del socket del daemon (Task 13): emite `Message::DaemonDied` si
    // `is_alive` deja de responder.
    if let Some(d) = &state.daemon {
        let d = d.clone();
        subs.push(Subscription::run_with_id(
            "daemon-socket",
            iced::stream::channel(16, move |mut tx| async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    if !d.is_alive() {
                        let _ = tx.send(Message::DaemonDied).await;
                        break;
                    }
                }
            }),
        ));
    }

    // Stream de eventos del DownloadManager (Task 18). Se omite mientras no
    // exista el manager (se crea perezoso tras `DaemonStarted`); al crearlo,
    // el `run_with_id("downloads")` arranca el stream y lo mantiene vivo.
    if let Some(mgr) = &state.downloads {
        let mut rx = mgr.subscribe();
        subs.push(Subscription::run_with_id(
            "downloads",
            iced::stream::channel(64, move |mut tx| async move {
                while let Ok(ev) = rx.recv().await {
                    if tx.send(Message::DownloadEvent(ev)).await.is_err() {
                        break;
                    }
                }
            }),
        ));
    }

    // Resize de ventana → recalcula per_row de los grids.
    subs.push(
        iced::window::resize_events()
            .map(|(_id, size)| Message::WindowResized(size.width, size.height)),
    );

    if subs.is_empty() {
        Subscription::none()
    } else {
        Subscription::batch(subs)
    }
}

/// Vista raíz: elige el contenido según `screen` y lo envuelve con el
/// nav rail del shell.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let content: Element<Message> = match state.screen {
        Screen::Home => home::view(state),
        Screen::Browse => browse::view(state),
        Screen::Library => library::view(state),
        Screen::Details => details::view(state),
        Screen::Reader => reader::view(state),
        Screen::Downloads => downloads::view(state),
        Screen::Settings => settings::view(state),
        Screen::Extensions => extensions::view(state),
    };
    crate::features::shell::view(&state.screen, content)
}