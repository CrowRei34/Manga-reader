//! Pantalla de Extensiones (Extensions). Lista estática de `state.sources`
//! (las fuentes del daemon, pobladas en `Message::SourcesListed` al arrancar).
//! Sin gestión real: sólo muestra id + nombre de cada fuente.
use iced::widget::{column, row, scrollable, text};
use iced::Element;

use crate::app::{AppState, Message as AppMessage};

/// Vista del feature: lista scrollable de fuentes del daemon. Sin `update` —
/// es de solo lectura.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    if state.sources.is_empty() {
        return column![
            text("Extensiones").size(24),
            text("Sin extensiones").size(16),
        ]
        .spacing(8)
        .into();
    }

    let list_col = column(state.sources.iter().map(|s| {
        row![text(&s.name), text(&s.id).size(12)].spacing(8).into()
    }))
    .spacing(4);

    column![
        text("Extensiones").size(24),
        scrollable(list_col),
    ]
    .spacing(8)
    .into()
}
