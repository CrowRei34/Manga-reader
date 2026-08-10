mod app;
mod core;
mod features;
mod theme;
mod widgets;

use std::sync::Arc;
use std::time::Duration;

use iced::Task;

use app::{AppState, Message};
use core::daemon::api::MangaSourceApi;
use core::daemon::client::DaemonClient;
use core::settings;
use core::xdg::Xdg;

/// Fuente de íconos Material (sidebar, botones, estados).
pub static ICON_FONT_BYTES: &[u8] = include_bytes!("../assets/MaterialIcons.ttf");

/// Binario bakeneko. iced 0.13 cambió `Application` (trait, estilo 0.12) por
/// un builder deApplication> con `iced::application(title, update, view)`.
/// El reducer y la vista viven en `app::update` / `app::view` (standalone
/// functions sobre la `AppState` compartida). La subscription se conecta vía
/// `.subscription(...)`. La inicialización (daemon + DB) vive en `init_state`,
/// pasada a `.run_with(...)` como sustituto del `Application::new` de 0.12.
fn main() -> iced::Result {
    if let Err(e) = Xdg::ensure_dirs() {
        eprintln!("XDG dirs: {e:?}");
    }

    iced::application("bakeneko", app::update, app::view)
        .theme(|state| theme::iced_theme(&state.settings))
        .subscription(app::subscription)
        .font(ICON_FONT_BYTES)
        .run_with(init_state)
}

/// Inicialización de la app: abre (y migra) la base SQLite, spawnea el daemon
/// en un tokio task desacoplado (via `DaemonClient::spawn_arc`) y devuelve el
/// estado inicial junto con un `Task` que polla `ping` hasta que el daemon
/// responda o expire el timeout de 20 s — el resultado aterriza en el reducer
/// como `Message::DaemonStarted`, que a su vez dispara `sources.list`.
///
/// El flujo completo descrito en el brief:
///   App launches → `init_state` crea `DaemonClient` + DB →
///   spawn_arc arranca el daemon en el fondo →
///   `Task::perform` polla `ping` y emite `Message::DaemonStarted` →
///   `update` setea `daemon_ready = true` y emite el `Task` para `list_sources` →
///   `Message::SourcesListed` puebla `state.sources` →
///   home/browse reciben datos reales.
fn init_state() -> (AppState, Task<Message>) {
    // 1) Abrir + migrar la base SQLite en $XDG_DATA_HOME/bakeneko/bakeneko.sqlite.
    //    El `AppState.db` contiene `Arc<Mutex<Connection>>` para que los
    //    features (`home`/`library`) lancen consultas vía `db_blocking`.
    let db = {
        let path = Xdg::data_root().join("bakeneko.sqlite");
        match core::db::Database::open(Some(&path)) {
            Ok(d) => {
                if let Err(e) = d.migrate() {
                    eprintln!("[db] migrate falló: {e}");
                }
                Some(d.connection())
            }
            Err(e) => {
                eprintln!("[db] open falló: {e}");
                None
            }
        }
    };

    // 2) Spawnear el daemon en background. `spawn_arc` devuelve un `Arc` que el
    //    UI guarda en `state.daemon`; el fondo corre `start()` (Java + socket
    //    Unix) y el UI prosigue sin bloquear el event loop.
    let jar = DaemonClient::default_jar_path();
    let jar_str = jar.to_string_lossy().into_owned();
    let daemon = DaemonClient::spawn_arc(&jar_str);

    // 3) El `Task` inicial polla `ping` hasta que el daemon responda. Cada
    //    intento reintentable duerma 200 ms; el deadline de 20 s cubre el
    //    arranque típico del JVM. El `Result` alimentado a `Message::DaemonStarted`
    //    refleja éxito (ping válido) o fallo (timeout/spawn).
    let wait = Arc::clone(&daemon);
    let task = Task::perform(
        async move {
            let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
            loop {
                match wait.ping().await {
                    Ok(reply) => return Ok(reply),
                    Err(e) => {
                        if tokio::time::Instant::now() >= deadline {
                            return Err(e);
                        }
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }
            }
        },
        Message::DaemonStarted,
    );

    // 4) Estado inicial: daemon (Some) + db (option Some) + daemon_ready=false.
    //    `..AppState::default()` inicializa el resto (cache, home, browse,
    //    library) con sus defaults ya probados por `tests/app_test`. La
    //    settings se carga desde disco para que la pantalla de Ajustes
    //    persista al reiniciar.
    let state = AppState {
        daemon: Some(daemon),
        daemon_ready: false,
        db,
        settings: settings::load(),
        ..AppState::default()
    };

    (state, task)
}