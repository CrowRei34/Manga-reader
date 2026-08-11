//! Tema Bakeneko: paleta sepia/marrón oscuro con acento terracota,
//! réplica visual del app Flutter original.
//!
//! Incluye: `iced_theme` (Theme global custom) + helpers de estilo para
//! botones (nav, primary, ghost, chip), contenedores (sidebar, card, barra
//! superior), inputs (text_input, pick_list), toggler y reglas de color
//! de texto.
use iced::border::Radius;
use iced::widget::{button, container, pick_list, text_input, toggler};
use iced::{Border, Color, Theme};

use crate::core::settings::Settings;

/// Paleta extraída de las capturas del app original (Flutter).
/// `Color::from_rgb8` no es const en iced 0.13 → usamos el literal del struct.
pub mod palette {
    use iced::Color;

    const fn c(r: u8, g: u8, b: u8) -> Color {
        Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
    }

    /// Fondo raíz de la ventana / contenido.
    pub const BG: Color = c(0x1A, 0x15, 0x10);
    /// Fondo del sidebar (un punto más oscuro que el contenido).
    pub const SIDEBAR: Color = c(0x16, 0x12, 0x0D);
    /// Superficie elevada (cards, preview, rows de listas).
    pub const ELEVATED: Color = c(0x27, 0x1F, 0x17);
    /// Fondo de inputs (text field, dropdowns).
    pub const INPUT: Color = c(0x24, 0x1C, 0x14);
    /// Hover de superficies (filas, nav items).
    pub const HOVER: Color = c(0x32, 0x28, 0x1E);
    /// Borde sutil sepia.
    pub const BORDER: Color = c(0x40, 0x32, 0x26);
    /// Acento terracota (botones primarios, logo, links).
    pub const ACCENT: Color = c(0xE0, 0x78, 0x56);
    /// Acento hover (un poco más claro).
    pub const ACCENT_HOVER: Color = c(0xEA, 0x8A, 0x69);
    /// Acento presionado (más oscuro).
    pub const ACCENT_PRESSED: Color = c(0xC4, 0x62, 0x42);
    /// Texto sobre acento (oscuro, contraste).
    pub const ON_ACCENT: Color = c(0x1A, 0x15, 0x10);
    /// Texto principal (blanco cálido).
    pub const TEXT: Color = c(0xF2, 0xEC, 0xE6);
    /// Texto secundario (gris cálido).
    pub const TEXT_MUTED: Color = c(0xA8, 0x9A, 0x8B);
    /// Texto deshabilitado / placeholder.
    pub const TEXT_DIM: Color = c(0x6E, 0x62, 0x56);
    /// Verde de éxito (checks de capítulos descargados).
    pub const SUCCESS: Color = c(0x57, 0xC2, 0x4E);
    /// Rojo de error (círculo "!" de descargas fallidas).
    pub const DANGER: Color = c(0xE5, 0x53, 0x4B);
    /// Tinte acento suave para fondos activos (nav seleccionado).
    pub const ACCENT_TINT: Color = c(0x3A, 0x2A, 0x20);
}

/// Tema global de la app. Siempre construimos la paleta custom Bakeneko
/// (el modo claro no existe en el diseño original; `theme: "light"` se
/// mapea al mismo tema oscuro por ahora).
pub fn iced_theme(_settings: &Settings) -> Theme {
    let mut palette = Theme::Dark.palette();
    palette.background = palette::BG;
    palette.text = palette::TEXT;
    palette.primary = palette::ACCENT;
    palette.success = palette::SUCCESS;
    palette.danger = palette::DANGER;
    Theme::custom("bakeneko".to_string(), palette)
}

/// Radio de borde estándar (botones, inputs, cards).
pub fn radius(r: f32) -> Radius {
    Radius::from(r)
}

// ---------------------------------------------------------------------------
// Botones
// ---------------------------------------------------------------------------

/// Botón de navegación del sidebar. `selected` resalta con tinte acento +
/// texto blanco; inactivo = transparente con texto muted.
pub fn nav_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, fg) = match (selected, status) {
            (true, _) => (Some(palette::ACCENT_TINT.into()), palette::TEXT),
            (false, button::Status::Hovered) => (Some(palette::HOVER.into()), palette::TEXT),
            (false, button::Status::Pressed) => (Some(palette::HOVER.into()), palette::TEXT),
            (false, _) => (None, palette::TEXT_MUTED),
        };
        button::Style {
            background: bg,
            text_color: fg,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: radius(8.0),
            },
            ..Default::default()
        }
    }
}

/// Botón primario (terracota sólido, texto oscuro). Ej: "Buscar", "Leer Ahora".
pub fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => palette::ACCENT_HOVER,
        button::Status::Pressed => palette::ACCENT_PRESSED,
        button::Status::Disabled => palette::ACCENT_PRESSED,
        _ => palette::ACCENT,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: palette::ON_ACCENT,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(8.0),
        },
        ..Default::default()
    }
}

/// Botón ghost: transparente con borde sepia. Ej: "En Biblioteca", "+ Nueva".
pub fn ghost_button(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, border_color) = match status {
        button::Status::Hovered => (Some(palette::HOVER.into()), palette::ACCENT),
        button::Status::Pressed => (Some(palette::HOVER.into()), palette::ACCENT),
        _ => (None, palette::BORDER),
    };
    button::Style {
        background: bg,
        text_color: palette::ACCENT,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius(8.0),
        },
        ..Default::default()
    }
}

/// Chip de categoría (Biblioteca). Activo = relleno acento, texto oscuro;
/// inactivo = borde sepia, texto muted.
pub fn chip_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let style = match (selected, status) {
            (true, _) => button::Style {
                background: Some(palette::ACCENT.into()),
                text_color: palette::ON_ACCENT,
                ..Default::default()
            },
            (false, button::Status::Hovered) => button::Style {
                background: Some(palette::HOVER.into()),
                text_color: palette::TEXT,
                ..Default::default()
            },
            (false, _) => button::Style {
                background: Some(palette::ELEVATED.into()),
                text_color: palette::TEXT_MUTED,
                ..Default::default()
            },
        };
        button::Style {
            border: Border {
                color: if selected { Color::TRANSPARENT } else { palette::BORDER },
                width: if selected { 0.0 } else { 1.0 },
                radius: radius(8.0),
            },
            ..style
        }
    }
}

/// Botón de texto plano (links como "Ver", "Descargar Todo", "Atrás").
pub fn link_button(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: match status {
            button::Status::Hovered | button::Status::Pressed => Some(palette::HOVER.into()),
            _ => None,
        },
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => palette::ACCENT_HOVER,
            _ => palette::TEXT_MUTED,
        },
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(6.0),
        },
        ..Default::default()
    }
}

/// Botón de texto acentuado (ej. "Descargar Todo" en terracota).
pub fn link_button_accent(_theme: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: match status {
            button::Status::Hovered | button::Status::Pressed => Some(palette::HOVER.into()),
            _ => None,
        },
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => palette::ACCENT_HOVER,
            _ => palette::ACCENT,
        },
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(6.0),
        },
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Contenedores
// ---------------------------------------------------------------------------

/// Sidebar: fondo casi negro sepia, ancho fijo gestionado por el caller.
pub fn sidebar_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(palette::SIDEBAR.into()),
        text_color: Some(palette::TEXT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Card elevada (Previsualización, filas, cover cards).
pub fn card_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(palette::ELEVATED.into()),
        text_color: Some(palette::TEXT),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: radius(10.0),
        },
        ..Default::default()
    }
}

/// Panel flotante del lector (barra inferior / panel de filtros): superficie
/// elevada translúcida con radio grande — espejo del panel del original
/// (surface al 92% + rounded 12).
pub fn reader_panel(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color { a: 0.95, ..palette::ELEVATED }.into()),
        text_color: Some(palette::TEXT),
        border: Border {
            color: Color { a: 0.85, ..palette::BORDER },
            width: 1.0,
            radius: radius(14.0),
        },
        ..Default::default()
    }
}

/// Chip translúcido del lector (contador de páginas flotante).
pub fn reader_chip(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.55 }.into()),
        text_color: Some(palette::TEXT),
        border: Border {
            color: Color { a: 0.35, ..palette::BORDER },
            width: 1.0,
            radius: radius(999.0),
        },
        ..Default::default()
    }
}

/// Opción del panel del lector (filtros/modo): activa = tinte acento.
pub fn panel_option(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, fg) = match (active, status) {
            (true, _) => (Some(palette::ACCENT_TINT.into()), palette::TEXT),
            (false, button::Status::Hovered) => (Some(palette::HOVER.into()), palette::TEXT),
            (false, _) => (None, palette::TEXT_MUTED),
        };
        button::Style {
            background: bg,
            text_color: fg,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: radius(8.0),
            },
            ..Default::default()
        }
    }
}

/// Contenedor del área de contenido (fondo BG, padding por el caller).
pub fn content_container(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(palette::BG.into()),
        text_color: Some(palette::TEXT),
        ..Default::default()
    }
}

/// Divisor horizontal fino (línea sepia).
pub fn divider(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(palette::BORDER.into()),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

/// Text field (búsqueda): fondo INPUT, borde sepia; foco = borde acento.
pub fn search_input(_theme: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => palette::ACCENT,
        text_input::Status::Hovered => palette::BORDER,
        _ => palette::BORDER,
    };
    text_input::Style {
        background: palette::INPUT.into(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius(8.0),
        },
        icon: palette::TEXT_MUTED,
        placeholder: palette::TEXT_DIM,
        value: palette::TEXT,
        selection: palette::ACCENT_TINT,
    }
}

/// Dropdown (pick_list): fondo INPUT, borde sepia, texto blanco.
pub fn dropdown(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let border_color = match status {
        pick_list::Status::Hovered => palette::ACCENT,
        pick_list::Status::Opened { .. } => palette::ACCENT,
        _ => palette::BORDER,
    };
    pick_list::Style {
        text_color: palette::TEXT,
        placeholder_color: palette::TEXT_DIM,
        handle_color: palette::TEXT_MUTED,
        background: palette::INPUT.into(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius(8.0),
        },
    }
}

/// Menú desplegable abierto (overlay del pick_list).
pub fn dropdown_menu(_theme: &Theme) -> iced::overlay::menu::Style {
    iced::overlay::menu::Style {
        background: palette::ELEVATED.into(),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: radius(8.0),
        },
        text_color: palette::TEXT,
        selected_text_color: palette::ON_ACCENT,
        selected_background: palette::ACCENT.into(),
    }
}

/// Toggle (Extensiones): off = fondo elevado con knob sepia; on = acento.
pub fn toggle(_theme: &Theme, status: toggler::Status) -> toggler::Style {
    let active = matches!(status, toggler::Status::Active { .. } | toggler::Status::Hovered { .. } if is_on(&status));
    toggler::Style {
        background: if active { palette::ACCENT } else { palette::ELEVATED },
        background_border_width: 1.0,
        background_border_color: if active { palette::ACCENT } else { palette::BORDER },
        foreground: if active { palette::ON_ACCENT } else { palette::TEXT_MUTED },
        foreground_border_width: 0.0,
        foreground_border_color: Color::TRANSPARENT,
    }
}

fn is_on(status: &toggler::Status) -> bool {
    match status {
        toggler::Status::Active { is_toggled } => *is_toggled,
        toggler::Status::Hovered { is_toggled } => *is_toggled,
        toggler::Status::Disabled => false,
    }
}
