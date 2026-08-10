mod app;
mod core;
mod features;
mod theme;

use iced::Task;

use app::{AppState, Message};

/// Binario bakeneko. iced 0.13 cambió `Application` (trait, estilo 0.12) por
/// un builder deApplication> con `iced::application(title, update, view)`.
/// El reducer y la vista viven en `app::update` / `app::view` (standalone
/// functions sobre la `AppState` compartida). La subscription se conecta vía
/// `.subscription(...)`.
fn main() -> iced::Result {
    if let Err(e) = core::xdg::Xdg::ensure_dirs() {
        eprintln!("XDG dirs: {e:?}");
    }

    iced::application("Bakeneko Reader", app::update, app::view)
        .subscription(app::subscription)
        .run_with(|| (AppState::default(), Task::<Message>::none()))
}