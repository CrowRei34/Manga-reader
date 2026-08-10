//! Tema de la ventana. Mapea la preferencia de la settings (`theme`:
//! "light"/"dark"/"system") al `iced::Theme` correspondiente y aplica el
//! accent configurado (`#rrggbb`) como color primario vía `Theme::custom`.
use iced::Color;

use crate::core::settings::Settings;

/// Tema de la ventana derivado de `settings.theme` + `settings.accent`.
pub fn iced_theme(settings: &Settings) -> iced::Theme {
    let base = match settings.theme.as_str() {
        "light" => iced::Theme::Light,
        // "system" → placeholder: detección real del tema del sistema
        // (dark) pendiente.
        "system" => iced::Theme::Dark,
        _ => iced::Theme::Dark,
    };
    theme_with_accent(base, &settings.accent)
}

/// Aplica el accent (`#rrggbb`) al tema base como color primario. Si el hex
/// no es válido, devuelve el tema base sin cambios.
fn theme_with_accent(base: iced::Theme, accent: &str) -> iced::Theme {
    let Some(color) = parse_hex(accent) else {
        return base;
    };
    let mut palette = base.palette();
    palette.primary = color;
    iced::Theme::custom("bakeneko".to_string(), palette)
}

/// Parsea `#rrggbb` (tolera omitir `#` y espacios) a un `iced::Color`.
fn parse_hex(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}
