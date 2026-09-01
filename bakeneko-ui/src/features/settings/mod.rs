//! Pantalla de Ajustes (Settings) — réplica del diseño original: dos
//! columnas (sub-nav "Apariencia"/"Lector" a la izquierda; panel derecho
//! con Previsualización + dropdowns de Tema, Color de Acento y Densidad
//! de Portadas). Cada cambio persiste vía `settings::save` (write-through).
use iced::widget::{column, container, pick_list, row, text, text_input, toggler};
use iced::{Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::settings::save;
use crate::theme::palette;
use crate::widgets::icon;

#[derive(Debug, Clone)]
pub enum Message {
    ThemeChanged(String),
    AccentChanged(String),
    DensityChanged(String),
    DiscordClientIdChanged(String),
    DiscordPresenceChanged(bool),
    DiscordAdultChanged(bool),
}

/// Reducer del feature Settings. Muta `state.settings` y persiste de
/// inmediato (fire-and-forget: si falla el guardado, se reporta en
/// `state.error` sin romper el event loop).
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::ThemeChanged(t) => state.settings.theme = t,
        Message::AccentChanged(a) => state.settings.accent = a,
        Message::DensityChanged(d) => state.settings.library_view = d,
        Message::DiscordClientIdChanged(id) => {
            state.settings.discord_client_id = id;
            if state.settings.discord_presence_enabled {
                state.discord_presence = crate::discord_presence::DiscordPresence::start(
                    &state.settings.discord_client_id,
                );
            }
        }
        Message::DiscordPresenceChanged(enabled) => {
            state.settings.discord_presence_enabled = enabled;
            state.discord_presence = enabled
                .then(|| crate::discord_presence::DiscordPresence::start(
                    &state.settings.discord_client_id,
                ))
                .flatten();
        }
        Message::DiscordAdultChanged(show) => state.settings.discord_show_adult = show,
    }

    if let Err(e) = save(&state.settings) {
        state.error = Some(format!("No se pudo guardar la configuración: {e}"));
    }
    Task::none()
}

/// Vista: row de dos columnas — sub-nav + panel.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    // Sub-nav izquierda.
    let apariencia = text("Apariencia").size(14).color(palette::ACCENT);
    let lector = text("Lector").size(14).color(palette::TEXT_MUTED);
    let subnav = column![apariencia, lector].spacing(16).padding(iced::Padding::new(8.0));

    // Previsualización: card con placeholders de portada.
    let preview_covers = (0..4)
        .map(|_| {
            container(icon::glyph(icon::IMAGE, 28, palette::TEXT_DIM))
                .center_x(Length::Fixed(60.0))
                .center_y(Length::Fixed(90.0))
                .style(crate::theme::card_container)
                .into()
        })
        .collect::<Vec<Element<'_, AppMessage>>>();
    let preview = container(
        column![
            text("Previsualización").size(14).color(palette::TEXT),
            iced::widget::Row::with_children(preview_covers).spacing(12),
        ]
        .spacing(12)
        .padding(16),
    )
    .style(crate::theme::card_container)
    .width(Length::Fill);

    let tema = pick_list(
        vec!["SYSTEM".to_string(), "DARK".to_string(), "LIGHT".to_string()],
        Some(state.settings.theme.to_uppercase()),
        |t| AppMessage::Settings(Message::ThemeChanged(t.to_lowercase())),
    )
    .style(crate::theme::dropdown)
    .menu_style(crate::theme::dropdown_menu)
    .width(Length::Fill);

    let acento = pick_list(
        vec!["TERRACOTTA".to_string()],
        Some("TERRACOTTA".to_string()),
        |a| AppMessage::Settings(Message::AccentChanged(a)),
    )
    .style(crate::theme::dropdown)
    .menu_style(crate::theme::dropdown_menu)
    .width(Length::Fill);

    let densidad = pick_list(
        vec!["COMFORTABLE".to_string(), "COMPACT".to_string()],
        Some("COMFORTABLE".to_string()),
        |d| AppMessage::Settings(Message::DensityChanged(d)),
    )
    .style(crate::theme::dropdown)
    .menu_style(crate::theme::dropdown_menu)
    .width(Length::Fill);

    let discord_id = text_input(
        "Application ID de Discord",
        &state.settings.discord_client_id,
    )
    .on_input(|id| AppMessage::Settings(Message::DiscordClientIdChanged(id)))
    .style(crate::theme::search_input)
    .padding([8, 12]);

    let discord_enabled = toggler(state.settings.discord_presence_enabled)
        .label("Mostrar lo que estoy leyendo")
        .on_toggle(|enabled| AppMessage::Settings(Message::DiscordPresenceChanged(enabled)))
        .style(crate::theme::toggle);
    let discord_adult = toggler(state.settings.discord_show_adult)
        .label("Mostrar títulos y portadas +18")
        .on_toggle(|show| AppMessage::Settings(Message::DiscordAdultChanged(show)))
        .style(crate::theme::toggle);
    let discord_status = if state.settings.discord_presence_enabled
        && state.discord_presence.is_none()
    {
        "Introduce un Application ID numérico válido."
    } else if let Some(discord) = &state.discord_presence {
        match discord.status() {
            crate::discord_presence::ConnectionStatus::Connected => "Conectado a Discord",
            crate::discord_presence::ConnectionStatus::Connecting => "Esperando actividad para conectar…",
            crate::discord_presence::ConnectionStatus::Disconnected => {
                "Discord desconectado; se reintentará automáticamente."
            }
        }
    } else {
        "Desactivado"
    };

    let panel = column![
        preview,
        text("Tema").size(14).color(palette::TEXT),
        tema,
        text("Color de Acento").size(14).color(palette::TEXT),
        acento,
        text("Densidad de Portadas").size(14).color(palette::TEXT),
        densidad,
        text("Discord Rich Presence").size(16).color(palette::ACCENT),
        text("Bakeneko incluye su Application ID oficial; puedes sustituirlo para desarrollo.")
            .size(12).color(palette::TEXT_MUTED),
        discord_id,
        discord_enabled,
        discord_adult,
        text(discord_status).size(12).color(palette::TEXT_DIM),
    ]
    .spacing(10)
    .padding(iced::Padding::new(16.0))
    .width(Length::Fill);

    column![
        text("Ajustes").size(22).color(palette::TEXT),
        row![subnav, panel].spacing(24),
    ]
    .spacing(16)
    .into()
}
