//! Pantalla de Ajustes (Settings). Pickers sobre `state.settings`:
//! tema (dark/light/system), acento (hex), concurrencia de descargas
//! (numérico) y vista de biblioteca (grid/list). Cada cambio muta
//! `state.settings` y persiste vía `settings::save` (write-through, sin
//! botón "Guardar").
use iced::widget::{column, pick_list, row, text, text_input};
use iced::{Element, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::settings::save;

#[derive(Debug, Clone)]
pub enum Message {
    ThemeChanged(String),
    AccentChanged(String),
    ConcurrencyChanged(u32),
    LibraryViewChanged(String),
}

/// Reducer del feature Settings. Muta `state.settings` y persiste de
/// inmediato (fire-and-forget: si falla el guardado, se reporta en
/// `state.error` sin romper el event loop).
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::ThemeChanged(t) => state.settings.theme = t,
        Message::AccentChanged(a) => state.settings.accent = a,
        Message::ConcurrencyChanged(n) => state.settings.download_concurrency = n,
        Message::LibraryViewChanged(v) => state.settings.library_view = v,
    }
    if let Err(e) = save(&state.settings) {
        state.error = Some(format!("No se pudo guardar la configuración: {e}"));
    }
    Task::none()
}

/// Vista del feature: filas etiqueta + control para cada ajuste.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let theme = pick_list(
        vec![
            "dark".to_string(),
            "light".to_string(),
            "system".to_string(),
        ],
        Some(state.settings.theme.clone()),
        |t| AppMessage::Settings(Message::ThemeChanged(t)),
    );

    let accent = text_input("#7c5cbf", &state.settings.accent)
        .on_input(|a| AppMessage::Settings(Message::AccentChanged(a)));

    // Entrada numérica: el parse fallido (p.ej. campo vacío a medio editar)
    // no muta nada — se ignora con `ErrorDismissed` como no-op.
    let concurrency = text_input("2", &state.settings.download_concurrency.to_string())
        .on_input(|s| match s.parse::<u32>() {
            Ok(n) => AppMessage::Settings(Message::ConcurrencyChanged(n)),
            Err(_) => AppMessage::ErrorDismissed,
        });

    let library_view = pick_list(
        vec!["grid".to_string(), "list".to_string()],
        Some(state.settings.library_view.clone()),
        |v| AppMessage::Settings(Message::LibraryViewChanged(v)),
    );

    column![
        text("Ajustes").size(24),
        row![text("Tema").width(180), theme].spacing(8),
        row![text("Color de acento (hex)").width(180), accent].spacing(8),
        row![text("Descargas concurrentes").width(180), concurrency].spacing(8),
        row![text("Vista de biblioteca").width(180), library_view].spacing(8),
    ]
    .spacing(8)
    .into()
}
