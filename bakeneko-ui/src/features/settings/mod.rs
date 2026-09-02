//! Pantalla de Ajustes (Settings) — réplica del diseño original: dos
//! columnas (sub-nav "Apariencia"/"Lector" a la izquierda; panel derecho
//! con Previsualización + dropdowns de Tema, Color de Acento y Densidad
//! de Portadas). Cada cambio persiste vía `settings::save` (write-through).
use iced::widget::{column, container, pick_list, row, scrollable, text, toggler};
use iced::{Border, Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::settings::save;
use crate::theme::palette;
use crate::widgets::icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    Appearance,
    Reader,
}

#[derive(Debug, Default)]
pub struct State {
    pub section: Section,
}

#[derive(Debug, Clone)]
pub enum Message {
    SectionChanged(Section),
    AccentChanged(String),
    DensityChanged(String),
    ReaderModeChanged(String),
    ReaderFilterChanged(String),
    DiscordPresenceChanged(bool),
    DiscordAdultChanged(bool),
}

/// Reducer del feature Settings. Muta `state.settings` y persiste de
/// inmediato (fire-and-forget: si falla el guardado, se reporta en
/// `state.error` sin romper el event loop).
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::SectionChanged(section) => {
            state.settings_state.section = section;
            return Task::none();
        }
        Message::AccentChanged(a) => state.settings.accent = a,
        Message::DensityChanged(d) => state.settings.library_view = d,
        Message::ReaderModeChanged(mode) => state.settings.reader_mode = mode,
        Message::ReaderFilterChanged(filter) => state.settings.reader_filter = filter,
        Message::DiscordPresenceChanged(enabled) => {
            state.settings.discord_presence_enabled = enabled;
            state.discord_presence = enabled
                .then(|| crate::discord_presence::DiscordPresence::start(
                    bakeneko_core::settings::DISCORD_APPLICATION_ID,
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
    let accent = crate::theme::accent(&state.settings);
    // Navegación real entre las dos secciones de ajustes.
    let apariencia = iced::widget::button(text("Apariencia").size(14))
        .on_press(AppMessage::Settings(Message::SectionChanged(Section::Appearance)))
        .style(crate::theme::nav_button(state.settings_state.section == Section::Appearance))
        .padding([9, 12])
        .width(Length::Fill);
    let lector = iced::widget::button(text("Lector y actividad").size(14))
        .on_press(AppMessage::Settings(Message::SectionChanged(Section::Reader)))
        .style(crate::theme::nav_button(state.settings_state.section == Section::Reader))
        .padding([9, 12])
        .width(Length::Fill);
    let subnav = container(column![apariencia, lector].spacing(4))
        .style(crate::theme::card_container)
        .padding(6)
        .width(Length::Fixed(170.0));

    // Previsualización viva: muestra las mismas superficies y contraste que
    // usan las tarjetas reales, actualizada con cada cambio de esquema.
    let (preview_bg, preview_text, preview_accent) =
        crate::theme::preview_colors(&state.settings);
    let tile_height = if state.settings.library_view.eq_ignore_ascii_case("compact") {
        78.0
    } else {
        96.0
    };
    let preview_covers = (0..4)
        .map(|_| {
            let accent = preview_accent;
            container(
                column![
                    container(icon::glyph(icon::IMAGE, 22, preview_text))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                        .height(Length::Fill),
                    container(text(""))
                        .height(Length::Fixed(3.0))
                        .width(Length::Fill)
                        .style(move |_| iced::widget::container::Style {
                            background: Some(accent.into()),
                            ..Default::default()
                        }),
                ]
                .spacing(0),
            )
            .width(Length::Fixed(58.0))
            .height(Length::Fixed(tile_height))
            .style(move |_| iced::widget::container::Style {
                background: Some(preview_bg.into()),
                border: Border {
                    color: accent,
                    width: 1.0,
                    radius: crate::theme::radius(0.0),
                },
                ..Default::default()
            })
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
    .style(crate::theme::panel_container)
    .width(Length::Fill);

    let acento = pick_list(
        crate::theme::COLOR_SCHEMES
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>(),
        Some(crate::theme::scheme_label(&state.settings.accent).to_owned()),
        |label| AppMessage::Settings(Message::AccentChanged(
            crate::theme::scheme_id(&label).to_owned(),
        )),
    )
    .style(crate::theme::dropdown)
    .menu_style(crate::theme::dropdown_menu)
    .width(Length::Fill);

    let densidad = pick_list(
        vec!["COMFORTABLE".to_string(), "COMPACT".to_string()],
        Some(if state.settings.library_view.eq_ignore_ascii_case("compact") {
            "COMPACT"
        } else {
            "COMFORTABLE"
        }.to_string()),
        |d| AppMessage::Settings(Message::DensityChanged(d.to_ascii_lowercase())),
    )
    .style(crate::theme::dropdown)
    .menu_style(crate::theme::dropdown_menu)
    .width(Length::Fill);

    let reader_mode = pick_list(
        vec!["WEBTOON".to_string(), "PAGINADO".to_string()],
        Some(if state.settings.reader_mode == "paginated" { "PAGINADO" } else { "WEBTOON" }.to_string()),
        |mode| AppMessage::Settings(Message::ReaderModeChanged(
            if mode == "PAGINADO" { "paginated" } else { "webtoon" }.to_string(),
        )),
    )
    .style(crate::theme::dropdown)
    .menu_style(crate::theme::dropdown_menu)
    .width(Length::Fill);

    let reader_filter = pick_list(
        vec!["NINGUNO", "INVERTIDO", "ESCALA DE GRISES", "SEPIA", "ANTI LUZ AZUL"]
            .into_iter().map(str::to_string).collect::<Vec<_>>(),
        Some(match state.settings.reader_filter.as_str() {
            "inverted" => "INVERTIDO", "grayscale" => "ESCALA DE GRISES",
            "sepia" => "SEPIA", "bluelight" => "ANTI LUZ AZUL", _ => "NINGUNO",
        }.to_string()),
        |filter| AppMessage::Settings(Message::ReaderFilterChanged(match filter.as_str() {
            "INVERTIDO" => "inverted", "ESCALA DE GRISES" => "grayscale",
            "SEPIA" => "sepia", "ANTI LUZ AZUL" => "bluelight", _ => "none",
        }.to_string())),
    )
    .style(crate::theme::dropdown)
    .menu_style(crate::theme::dropdown_menu)
    .width(Length::Fill);

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
        "No se pudo iniciar la conexión oficial con Discord."
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

    let contents = match state.settings_state.section {
        Section::Appearance => column![
            text("Apariencia").size(18).color(accent),
            preview,
            text("Esquema de color").size(14).color(palette::TEXT),
            acento,
            text("Densidad de portadas").size(14).color(palette::TEXT),
            densidad,
        ].spacing(12),
        Section::Reader => column![
            text("Lector").size(18).color(accent),
            text("Modo de lectura predeterminado").size(14).color(palette::TEXT),
            reader_mode,
            text("Filtro visual predeterminado").size(14).color(palette::TEXT),
            reader_filter,
            text("Actividad de Discord").size(18).color(accent),
            text("Usa exclusivamente la aplicación oficial de Bakeneko.")
                .size(12).color(palette::TEXT_MUTED),
            discord_enabled,
            discord_adult,
            text(discord_status).size(12).color(palette::TEXT_DIM),
        ].spacing(12),
    };

    let panel = container(
        scrollable(contents)
            .style(crate::theme::scrollable_style)
            .height(Length::Fill),
    )
    .padding(iced::Padding::new(16.0))
    .style(crate::theme::panel_container)
    .width(Length::Fill)
    .height(Length::Fill);

    let settings_body: Element<'_, AppMessage> = if state.window_size.0 < 760.0 {
        column![subnav, panel].spacing(14).width(Length::Fill).into()
    } else {
        row![subnav, panel].spacing(24).width(Length::Fill).into()
    };

    column![
        text("Ajustes").size(28).color(palette::TEXT),
        settings_body,
    ]
    .spacing(16)
    .height(Length::Fill)
    .into()
}
