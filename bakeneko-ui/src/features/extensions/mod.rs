//! Pantalla de Extensiones (Extensions) — réplica del diseño original:
//! título, barra de búsqueda y lista de fuentes con toggle a la derecha.
//! Las fuentes vienen del daemon (`sources.list`, poblado al arrancar).
//! Los toggles son locales (filtran qué fuentes se ofrecen en Explorar).
use iced::widget::{column, container, row, scrollable, text, text_input, toggler};
use iced::{Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::theme::palette;


#[derive(Debug, Default)]
pub struct State {
    pub query: String,
    /// Fuentes deshabilitadas por el usuario (por id).
    pub disabled: std::collections::HashSet<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    QueryChanged(String),
    ToggleSource(String, bool),
}

pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::QueryChanged(q) => {
            state.extensions.query = q;
            Task::none()
        }
        Message::ToggleSource(id, on) => {
            if on {
                state.extensions.disabled.remove(&id);
            } else {
                state.extensions.disabled.insert(id);
            }
            Task::none()
        }
    }
}

pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let header = container(
        row![
            container(text(""))
                .style(crate::theme::accent_rule)
                .width(Length::Fixed(4.0))
                .height(Length::Fixed(26.0)),
            text("Extensiones").size(28).color(palette::TEXT),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    )
    .style(crate::theme::panel_container)
    .padding([14, 16])
    .width(Length::Fill);

    let search = text_input("Buscar extensiones…", &state.extensions.query)
        .on_input(|q| AppMessage::Extensions(Message::QueryChanged(q)))
        .style(crate::theme::search_input)
        .padding([8, 12]);

    let q = state.extensions.query.to_lowercase();
    let rows: Vec<Element<'_, AppMessage>> = state
        .sources
        .iter()
        .filter(|s| q.is_empty() || s.name.to_lowercase().contains(&q))
        .map(|s| {
            let on = !state.extensions.disabled.contains(&s.id);
            container(row![
                column![
                    text(s.name.clone()).size(15).color(palette::TEXT),
                    text(format!("Idioma: {}", crate::language::label(s.language.as_deref())))
                        .size(12).color(palette::TEXT_MUTED),
                ]
                .spacing(2)
                .width(Length::Fill),
                toggler(on)
                    .on_toggle(move |v| {
                        AppMessage::Extensions(Message::ToggleSource(s.id.clone(), v))
                    })
                    .style(crate::theme::toggle),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .padding([10, 12])
            .width(Length::Fill))
            .style(crate::theme::card_container)
            .into()
        })
        .collect();

    column![header, search, scrollable(column(rows).spacing(8)).style(crate::theme::scrollable_style).width(Length::Fill).height(Length::Fill)]
        .spacing(16)
        .into()
}
