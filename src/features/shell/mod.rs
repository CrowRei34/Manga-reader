//! Shell: sidebar fijo (logo BAKENEKO + nav con íconos) + área de contenido.
//!
//! Réplica visual del app Flutter original: sidebar casi negro sepia (~185px),
//! logo en terracota mayúsculas espaciadas, items de nav con ícono Material +
//! label, activo = tinte acento + texto blanco, hover = superficie cálida.
use iced::widget::{button, column, container, row, text, Column};
use iced::{Element, Length};

use crate::theme::palette;
use crate::widgets::icon;
use super::Screen;

#[derive(Debug, Clone)]
pub enum NavMsg {
    Navigate(Screen),
}

const NAV_ITEMS: [(&str, Screen, char); 6] = [
    ("Home", Screen::Home, icon::HOME),
    ("Biblioteca", Screen::Library, icon::LIBRARY),
    ("Explorar", Screen::Browse, icon::EXPLORE),
    ("Descargas", Screen::Downloads, icon::DOWNLOAD),
    ("Extensiones", Screen::Extensions, icon::EXTENSIONS),
    ("Ajustes", Screen::Settings, icon::SETTINGS),
];

/// Vista del shell: `row![ sidebar, content ]`. El sidebar es una columna
/// con logo + items; el contenido ocupa el resto con padding.
pub fn view<'a, M: 'a + From<NavMsg> + Clone>(
    screen: &Screen,
    content: Element<'a, M>,
) -> Element<'a, M> {
    // Logo BAKENEKO (terracota, bold, letter-spaced aprox. con mayúsculas).
    let logo = text("BAKENEKO")
        .size(20)
        .color(palette::ACCENT);

    let buttons: Vec<Element<'a, M>> = NAV_ITEMS
        .iter()
        .map(|(label, target, ic)| {
            let selected = *target == *screen;
            let fg = if selected { palette::TEXT } else { palette::TEXT_MUTED };
            let content = row![
                icon::glyph(*ic, 18, fg),
                text(*label).size(14).color(fg),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center);
            button(content)
                .on_press(M::from(NavMsg::Navigate(target.clone())))
                .style(crate::theme::nav_button(selected))
                .width(Length::Fill)
                .padding([8, 12])
                .into()
        })
        .collect();

    let rail = Column::with_children(buttons).spacing(2);

    let sidebar = container(
        column![logo, rail]
            .spacing(24)
            .padding(iced::Padding::new(16.0)),
    )
    .style(crate::theme::sidebar_container)
    .width(Length::Fixed(185.0))
    .height(Length::Fill);

    let body = container(content)
        .style(crate::theme::content_container)
        .padding(iced::Padding::new(20.0))
        .width(Length::Fill)
        .height(Length::Fill);

    row![sidebar, body].into()
}
