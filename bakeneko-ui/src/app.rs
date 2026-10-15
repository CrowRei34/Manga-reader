//! AppState + Message global + reducer `update` + `view` + `subscription`.
//!
//! El `App` (struct que implementa `iced::Application`) vive en `main.rs`
//! y delega aquí en `app::update` / `app::view` / `app::subscription`.
use iced::{Element, Subscription, Task};
use std::sync::{Arc, Mutex};

use bakeneko_core::daemon::client::DaemonClient;
use bakeneko_core::daemon::api::MangaSourceApi;
use bakeneko_core::downloads::{DownloadEvent, DownloadManager};
use bakeneko_core::error::{DaemonError, DbError};
use bakeneko_core::models::{Manga, PingReply, Source};
use bakeneko_core::net::ImageCache;
use bakeneko_core::settings::Settings;
use crate::features::shell::NavMsg;
use crate::features::{browse, details, downloads, extensions, home, library, reader, settings, Screen};

/// Estado raíz de la app. Una sola `AppState` mutable a través de todos
/// los features; los sub-estados viven embebidos (`home`, `browse`, ...).
pub struct AppState {
    pub screen: Screen,
    pub error: Option<String>,
    pub settings: Settings,
    pub sources: Vec<Source>,
    pub source_load_attempts: u8,
    pub daemon_ready: bool,
    pub daemon: Option<Arc<DaemonClient>>,
    pub db: Option<Arc<Mutex<rusqlite::Connection>>>,
    pub cache: Arc<ImageCache>,
    pub home: home::State,
    pub browse: browse::State,
    pub library_state: library::State,
    pub details: details::State,
    pub reader: reader::State,
    pub settings_state: settings::State,
    pub discord_presence: Option<crate::discord_presence::DiscordPresence>,
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
            source_load_attempts: 0,
            daemon_ready: false,
            daemon: None,
            db: None,
            cache: Arc::new(ImageCache::new()),
            home: home::State::default(),
            browse: browse::State::default(),
            library_state: library::State::default(),
            details: details::State::default(),
            reader: reader::State::default(),
            settings_state: settings::State::default(),
            discord_presence: None,
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
    SearchResult {
        generation: u64,
        source: String,
        result: Result<Vec<Manga>, DaemonError>,
    },
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
    ReaderPagesFetched {
        generation: u64,
        result: Result<Vec<bakeneko_core::models::Page>, DaemonError>,
        headers: std::collections::HashMap<String, String>,
    },
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
                    let mgr = DownloadManager::new(
                        db,
                        d,
                        state.cache.clone(),
                        state.settings.download_concurrency as usize,
                    );
                    mgr.start_worker();
                    state.downloads = Some(mgr);
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
            if s.is_empty() {
                state.error = Some("El daemon respondió sin fuentes; reintentando…".into());
                return retry_source_list(state);
            }
            eprintln!("[sources] {} fuentes cargadas", s.len());
            state.source_load_attempts = 0;
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
            retry_source_list(state)
        }
        Message::SearchResult { generation, source, result } => {
            browse::apply_search_result(state, generation, source, result)
        }
        Message::DaemonDied => {
            state.daemon_ready = false;
            state.error = Some("Daemon cerró el socket".into());
            Task::none()
        }
        Message::NavigateTo(s) => {
            if s != Screen::Reader {
                if let Some(discord) = &state.discord_presence {
                    discord.clear();
                }
            }
            state.screen = s.clone();
            // Cargas perezosas al entrar a cada pantalla.
            match s {
                Screen::Downloads => downloads::update(state, downloads::Message::Load),
                Screen::Library => library::update(state, library::Message::Load),
                Screen::Home => Task::batch([
                    home::update(state, home::Message::LoadRecent),
                    library::update(state, library::Message::Load),
                ]),
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
            crate::widgets::cover::fetch_covers(state, &state.library)
        }

        Message::LibraryLoaded(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
        Message::Details(m) => details::update(state, m),
        Message::DetailsFetched(r) => details::update(state, details::Message::Fetched(r)),
        Message::Reader(m) => reader::update(state, m),
        Message::ReaderPagesFetched { generation, result, headers } => {
            reader::update(state, reader::Message::PagesFetched { generation, result, headers })
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

fn retry_source_list(state: &mut AppState) -> Task<Message> {
    state.source_load_attempts = state.source_load_attempts.saturating_add(1);
    if state.source_load_attempts > 3 {
        state.error = Some(
            "No se pudieron cargar las fuentes después de 3 intentos. Reinicia el daemon desde la aplicación."
                .into(),
        );
        return Task::none();
    }
    let Some(daemon) = state.daemon.clone() else { return Task::none() };
    Task::perform(
        async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            daemon.list_sources().await
        },
        Message::SourcesListed,
    )
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

    // Atajos de teclado del lector: ←/→ páginas, Esc salir, F fullscreen.
    if state.screen == Screen::Reader {
        subs.push(iced::keyboard::on_key_press(|key, _mods| {
            use iced::keyboard::key::Named;
            use iced::keyboard::Key;
            match key.as_ref() {
                Key::Named(Named::ArrowLeft) => {
                    Some(Message::Reader(reader::Message::PrevPage))
                }
                Key::Named(Named::ArrowRight) => {
                    Some(Message::Reader(reader::Message::NextPage))
                }
                Key::Named(Named::Escape) => Some(Message::Reader(reader::Message::Back)),
                Key::Character("f") => {
                    Some(Message::Reader(reader::Message::ToggleFullscreen))
                }
                _ => None,
            }
        }));
    }

    if subs.is_empty() {
        Subscription::none()
    } else {
        Subscription::batch(subs)
    }
}

/// Vista raíz: elige el contenido según `screen` y lo envuelve con el
/// nav rail del shell. EXCEPCIÓN: el Reader va fullscreen (sin sidebar,
/// fondo negro puro) — espejo del lector del app original.
pub fn view(state: &AppState) -> Element<'_, Message> {
    // Reader: fullscreen negro, sin nav rail.
    if state.screen == Screen::Reader {
        return reader::view(state);
    }

    let content: Element<Message> = match state.screen {
        Screen::Home => home::view(state),
        Screen::Browse => browse::view(state),
        Screen::Library => library::view(state),
        Screen::Details => details::view(state),
        Screen::Reader => unreachable!(), // handled above
        Screen::Downloads => downloads::view(state),
        Screen::Settings => settings::view(state),
        Screen::Extensions => extensions::view(state),
    };
    crate::features::shell::view(
        &state.screen,
        crate::theme::accent(&state.settings),
        content,
    )
}
