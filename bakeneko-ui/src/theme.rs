//! Tema Bakeneko: paleta sepia/marrón oscuro con acento terracota,
//! réplica visual del app Flutter original.
//!
//! Incluye: `iced_theme` (Theme global custom) + helpers de estilo para
//! botones (nav, primary, ghost, chip), contenedores (sidebar, card, barra
//! superior), inputs (text_input, pick_list), toggler y reglas de color
//! de texto.
use iced::border::Radius;
use iced::widget::{button, container, pick_list, scrollable, text_input, toggler};
use iced::{Border, Color, Theme};

use bakeneko_core::settings::Settings;

/// Paleta extraída de las capturas del app original (Flutter).
/// `Color::from_rgb8` no es const en iced 0.13 → usamos el literal del struct.
pub mod palette {
    use iced::Color;

    const fn c(r: u8, g: u8, b: u8) -> Color {
        Color { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a: 1.0 }
    }

    /// Fondo raíz de la ventana / contenido.
    pub const BG: Color = c(0x18, 0x13, 0x10);
    /// Acento terracota (botones primarios, logo, links).
    pub const ACCENT: Color = c(0xE0, 0x78, 0x56);
    /// Texto sobre acento (oscuro, contraste).
    pub const ON_ACCENT: Color = c(0x1A, 0x15, 0x10);
    /// Texto principal (blanco cálido).
    pub const TEXT: Color = c(0xF7, 0xF0, 0xE8);
    /// Texto secundario (gris cálido).
    pub const TEXT_MUTED: Color = c(0xB8, 0xA7, 0x97);
    /// Texto deshabilitado / placeholder.
    pub const TEXT_DIM: Color = c(0x7D, 0x6D, 0x60);
    /// Verde de éxito (checks de capítulos descargados).
    pub const SUCCESS: Color = c(0x57, 0xC2, 0x4E);
    /// Rojo de error (círculo "!" de descargas fallidas).
    pub const DANGER: Color = c(0xE5, 0x53, 0x4B);
}

pub const COLOR_SCHEMES: [&str; 15] = [
    "Matecito",
    "Cacao",
    "Salvia",
    "Arcilla",
    "Tinta",
    "Terracotta",
    "Gruvbox",
    "Catppuccin Mocha",
    "Nord",
    "Tokyo Night",
    "Dracula",
    "Everforest",
    "Rosé Pine",
    "Monokai",
    "Kanagawa",
];

fn scheme_key(value: &str) -> &'static str {
    let value = value.trim().to_ascii_lowercase();
    if value.contains("matecito") || value.contains("mate") {
        "matecito"
    } else if value.contains("cacao") {
        "cacao"
    } else if value.contains("salvia") {
        "salvia"
    } else if value.contains("arcilla") {
        "arcilla"
    } else if value.contains("tinta") {
        "tinta"
    } else if value.contains("gruvbox") {
        "gruvbox"
    } else if value.contains("catppuccin") {
        "catppuccin"
    } else if value.contains("nord") {
        "nord"
    } else if value.contains("tokyo") {
        "tokyo-night"
    } else if value.contains("dracula") {
        "dracula"
    } else if value.contains("everforest") {
        "everforest"
    } else if value.contains("rose") || value.contains("rosé") {
        "rose-pine"
    } else if value.contains("monokai") {
        "monokai"
    } else if value.contains("kanagawa") {
        "kanagawa"
    } else {
        "terracotta"
    }
}

pub fn scheme_label(value: &str) -> &'static str {
    match scheme_key(value) {
        "matecito" => "Matecito",
        "cacao" => "Cacao",
        "salvia" => "Salvia",
        "arcilla" => "Arcilla",
        "tinta" => "Tinta",
        "gruvbox" => "Gruvbox",
        "catppuccin" => "Catppuccin Mocha",
        "nord" => "Nord",
        "tokyo-night" => "Tokyo Night",
        "dracula" => "Dracula",
        "everforest" => "Everforest",
        "rose-pine" => "Rosé Pine",
        "monokai" => "Monokai",
        "kanagawa" => "Kanagawa",
        _ => "Terracotta",
    }
}

pub fn scheme_id(label: &str) -> &'static str {
    scheme_key(label)
}

fn scheme_palette(value: &str) -> (Color, Color, Color) {
    let rgb = |r, g, b| Color::from_rgb8(r, g, b);
    match scheme_key(value) {
        // Paletas propias: fondos profundos, texto crema y acentos de baja saturación.
        "matecito" => (rgb(0x1D, 0x21, 0x1A), rgb(0xE2, 0xDD, 0xC7), rgb(0x9E, 0xA6, 0x72)),
        "cacao" => (rgb(0x20, 0x19, 0x17), rgb(0xE7, 0xD8, 0xCA), rgb(0xB2, 0x82, 0x68)),
        "salvia" => (rgb(0x1C, 0x21, 0x20), rgb(0xD9, 0xDE, 0xD4), rgb(0x8F, 0xA5, 0x94)),
        "arcilla" => (rgb(0x22, 0x1A, 0x17), rgb(0xE8, 0xD8, 0xCD), rgb(0xC0, 0x78, 0x5E)),
        "tinta" => (rgb(0x18, 0x1B, 0x1E), rgb(0xD8, 0xDC, 0xDF), rgb(0x7F, 0x96, 0xA3)),
        "gruvbox" => (rgb(0x28, 0x28, 0x28), rgb(0xEB, 0xDB, 0xB2), rgb(0xD7, 0x99, 0x21)),
        "catppuccin" => (rgb(0x1E, 0x1E, 0x2E), rgb(0xCD, 0xD6, 0xF4), rgb(0xCB, 0xA6, 0xF7)),
        "nord" => (rgb(0x2E, 0x34, 0x40), rgb(0xEC, 0xEF, 0xF4), rgb(0x88, 0xC0, 0xD0)),
        "tokyo-night" => (rgb(0x1A, 0x1B, 0x26), rgb(0xC0, 0xCA, 0xF5), rgb(0x7A, 0xA2, 0xF7)),
        "dracula" => (rgb(0x28, 0x2A, 0x36), rgb(0xF8, 0xF8, 0xF2), rgb(0xBD, 0x93, 0xF9)),
        "everforest" => (rgb(0x2D, 0x35, 0x3B), rgb(0xD3, 0xC6, 0xAA), rgb(0xA7, 0xC0, 0x80)),
        "rose-pine" => (rgb(0x19, 0x17, 0x24), rgb(0xE0, 0xDE, 0xF4), rgb(0xC4, 0xA7, 0xE7)),
        "monokai" => (rgb(0x27, 0x28, 0x22), rgb(0xF8, 0xF8, 0xF2), rgb(0xA6, 0xE2, 0x2E)),
        "kanagawa" => (rgb(0x1F, 0x1F, 0x28), rgb(0xDC, 0xD7, 0xBA), rgb(0xE6, 0xC3, 0x84)),
        _ => (palette::BG, palette::TEXT, palette::ACCENT),
    }
}

pub fn accent(settings: &Settings) -> Color {
    scheme_palette(&settings.accent).2
}

/// Colores base expuestos para la muestra visual de Ajustes.
pub fn preview_colors(settings: &Settings) -> (Color, Color, Color) {
    scheme_palette(&settings.accent)
}

#[derive(Clone, Copy)]
struct UiColors {
    background: Color,
    sidebar: Color,
    elevated: Color,
    input: Color,
    hover: Color,
    border: Color,
    accent: Color,
    accent_hover: Color,
    accent_pressed: Color,
    on_accent: Color,
    text: Color,
    muted: Color,
    dim: Color,
    accent_tint: Color,
}

fn mix(a: Color, b: Color, amount: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * amount,
        g: a.g + (b.g - a.g) * amount,
        b: a.b + (b.b - a.b) * amount,
        a: 1.0,
    }
}

fn ui_colors(theme: &Theme) -> UiColors {
    let palette = theme.palette();
    UiColors {
        background: palette.background,
        sidebar: mix(palette.background, Color::BLACK, 0.28),
        elevated: mix(palette.background, palette.text, 0.065),
        input: mix(palette.background, palette.text, 0.035),
        hover: mix(palette.background, palette.primary, 0.16),
        border: mix(palette.background, palette.text, 0.22),
        accent: palette.primary,
        accent_hover: mix(palette.primary, Color::WHITE, 0.16),
        accent_pressed: mix(palette.primary, palette.background, 0.2),
        on_accent: palette.background,
        text: palette.text,
        muted: mix(palette.background, palette.text, 0.66),
        dim: mix(palette.background, palette.text, 0.42),
        accent_tint: mix(palette.background, palette.primary, 0.22),
    }
}

/// Tema global de la app. Siempre construimos la paleta custom Bakeneko
/// (el modo claro no existe en el diseño original; `theme: "light"` se
/// mapea al mismo tema oscuro por ahora).
pub fn iced_theme(settings: &Settings) -> Theme {
    let mut palette = Theme::Dark.palette();
    let (background, text, accent) = scheme_palette(&settings.accent);
    palette.background = background;
    palette.text = text;
    palette.primary = accent;
    palette.success = palette::SUCCESS;
    palette.danger = palette::DANGER;
    Theme::custom(format!("bakeneko-{}", scheme_key(&settings.accent)), palette)
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
    move |theme, status| {
        let colors = ui_colors(theme);
        let (bg, fg) = match (selected, status) {
            (true, _) => (Some(colors.accent_tint.into()), colors.text),
            (false, button::Status::Hovered) => (Some(colors.hover.into()), colors.text),
            (false, button::Status::Pressed) => (Some(colors.hover.into()), colors.text),
            (false, _) => (None, colors.muted),
        };
        button::Style {
            background: bg,
            text_color: fg,
            border: Border {
                color: if selected { colors.accent } else { Color::TRANSPARENT },
                width: if selected { 1.0 } else { 0.0 },
                radius: radius(0.0),
            },
            ..Default::default()
        }
    }
}

/// Botón primario (terracota sólido, texto oscuro). Ej: "Buscar", "Leer Ahora".
pub fn primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let colors = ui_colors(theme);
    let bg = match status {
        button::Status::Hovered => colors.accent_hover,
        button::Status::Pressed => colors.accent_pressed,
        button::Status::Disabled => colors.accent_pressed,
        _ => colors.accent,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: colors.on_accent,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Botón ghost: transparente con borde sepia. Ej: "En Biblioteca", "+ Nueva".
pub fn ghost_button(theme: &Theme, status: button::Status) -> button::Style {
    let colors = ui_colors(theme);
    let (bg, border_color) = match status {
        button::Status::Hovered => (Some(colors.hover.into()), colors.accent),
        button::Status::Pressed => (Some(colors.hover.into()), colors.accent),
        _ => (None, colors.border),
    };
    button::Style {
        background: bg,
        text_color: colors.accent,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Chip de categoría (Biblioteca). Activo = relleno acento, texto oscuro;
/// inactivo = borde sepia, texto muted.
pub fn chip_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let colors = ui_colors(theme);
        let style = match (selected, status) {
            (true, _) => button::Style {
                background: Some(colors.accent.into()),
                text_color: colors.on_accent,
                ..Default::default()
            },
            (false, button::Status::Hovered) => button::Style {
                background: Some(Color { a: 0.55, ..colors.hover }.into()),
                text_color: colors.text,
                ..Default::default()
            },
            (false, _) => button::Style {
                background: None,
                text_color: colors.muted,
                ..Default::default()
            },
        };
        button::Style {
            border: Border {
                color: if selected { colors.accent } else { colors.border },
                width: 1.0,
                radius: radius(0.0),
            },
            ..style
        }
    }
}

/// Botón de texto plano (links como "Ver", "Descargar Todo", "Atrás").
pub fn link_button(theme: &Theme, status: button::Status) -> button::Style {
    let colors = ui_colors(theme);
    button::Style {
        background: match status {
            button::Status::Hovered | button::Status::Pressed => Some(colors.hover.into()),
            _ => None,
        },
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => colors.accent_hover,
            _ => colors.muted,
        },
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Botón de texto acentuado (ej. "Descargar Todo" en terracota).
pub fn link_button_accent(theme: &Theme, status: button::Status) -> button::Style {
    let colors = ui_colors(theme);
    button::Style {
        background: match status {
            button::Status::Hovered | button::Status::Pressed => Some(colors.hover.into()),
            _ => None,
        },
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => colors.accent_hover,
            _ => colors.accent,
        },
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Botón estilo card para capítulos: fondo superficie elevada, borde sepia, hover acentuado.
pub fn chapter_card_button(theme: &Theme, status: button::Status) -> button::Style {
    let colors = ui_colors(theme);
    let (bg, border_color) = match status {
        button::Status::Hovered => (Some(colors.hover.into()), colors.accent),
        button::Status::Pressed => (Some(colors.hover.into()), colors.accent),
        _ => (Some(Color { a: 0.86, ..colors.elevated }.into()), Color { a: 0.72, ..colors.border }),
    };
    button::Style {
        background: bg,
        text_color: colors.text,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Contenedores
// ---------------------------------------------------------------------------

/// Sidebar: fondo casi negro sepia, ancho fijo gestionado por el caller.
pub fn sidebar_container(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        background: Some(colors.sidebar.into()),
        text_color: Some(colors.text),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Card elevada (Previsualización, filas, cover cards).
pub fn card_container(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        background: Some(Color { a: 0.86, ..colors.elevated }.into()),
        text_color: Some(colors.text),
        border: Border {
            color: Color { a: 0.72, ..colors.border },
            width: 1.0,
            // Listas y bloques editoriales usan un marco más afilado.
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Panel suave para agrupar controles o metadatos sin perder la jerarquía.
pub fn panel_container(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        // Casi opaco para que las capas de portadas nunca se filtren por
        // debajo de un panel abierto, incluso durante un frame de scroll.
        background: Some(Color { a: 0.98, ..colors.elevated }.into()),
        text_color: Some(colors.text),
        border: Border {
            color: Color { a: 0.62, ..colors.border },
            width: 1.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Regla de acento que aporta ritmo editorial sin crear contenido nuevo.
pub fn accent_rule(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        background: Some(colors.accent.into()),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Card de portada: conserva la acción de abrir detalles, pero hace visible
/// el marco, el foco y la transición de estado de cada obra.
pub fn cover_card_button(theme: &Theme, status: button::Status) -> button::Style {
    let colors = ui_colors(theme);
    let background = match status {
        button::Status::Hovered => Some(Color { a: 0.28, ..colors.hover }.into()),
        button::Status::Pressed => Some(Color { a: 0.42, ..colors.accent_tint }.into()),
        _ => None,
    };
    button::Style {
        background,
        text_color: colors.text,
        border: Border {
            // El foco se expresa con fondo, no con un marco pegado al título.
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Estado vacío o de espera, con una superficie contenida y legible.
pub fn empty_state(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        background: Some(Color { a: 0.52, ..colors.elevated }.into()),
        text_color: Some(colors.muted),
        border: Border {
            color: Color { a: 0.65, ..colors.border },
            width: 1.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Panel flotante del lector (barra inferior / panel de filtros): superficie
/// elevada translúcida con radio grande — espejo del panel del original
/// (surface al 92% + rounded 12).
pub fn reader_panel(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        background: Some(Color { a: 0.95, ..colors.elevated }.into()),
        text_color: Some(colors.text),
        border: Border {
            color: Color { a: 0.85, ..colors.border },
            width: 1.0,
            radius: radius(2.0),
        },
        ..Default::default()
    }
}

/// Chip translúcido del lector (contador de páginas flotante).
pub fn reader_chip(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        background: Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.55 }.into()),
        text_color: Some(colors.text),
        border: Border {
            color: Color { a: 0.35, ..colors.border },
            width: 1.0,
            radius: radius(0.0),
        },
        ..Default::default()
    }
}

/// Opción del panel del lector (filtros/modo): activa = tinte acento.
pub fn panel_option(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let colors = ui_colors(theme);
        let (bg, fg) = match (active, status) {
            (true, _) => (Some(colors.accent_tint.into()), colors.text),
            (false, button::Status::Hovered) => (Some(colors.hover.into()), colors.text),
            (false, _) => (None, colors.muted),
        };
        button::Style {
            background: bg,
            text_color: fg,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: radius(0.0),
            },
            ..Default::default()
        }
    }
}

/// Contenedor del área de contenido (fondo BG, padding por el caller).
pub fn content_container(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        background: Some(colors.background.into()),
        text_color: Some(colors.text),
        ..Default::default()
    }
}

/// Divisor horizontal fino (línea sepia).
pub fn divider(theme: &Theme) -> container::Style {
    let colors = ui_colors(theme);
    container::Style {
        background: Some(colors.border.into()),
        ..Default::default()
    }
}

/// Barra de desplazamiento común: discreta en reposo, visible al interactuar
/// y con el mismo lenguaje visual sepia que el resto de la aplicación.
pub fn scrollable_style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let colors = ui_colors(theme);
    let active = matches!(
        status,
        scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. }
    );
    let rail = scrollable::Rail {
        // La barra se mantiene invisible en reposo; sólo aparece al pasar el
        // cursor o arrastrarla, evitando que tape chips y portadas.
        background: Some(Color { a: 0.0, ..colors.sidebar }.into()),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: radius(0.0),
        },
        scroller: scrollable::Scroller {
            color: if active {
                Color { a: 0.78, ..colors.accent }
            } else {
                Color { a: 0.0, ..colors.dim }
            },
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: radius(0.0),
            },
        },
    };
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: Some(Color { a: 0.0, ..colors.background }.into()),
    }
}

// ---------------------------------------------------------------------------
// Inputs
// ---------------------------------------------------------------------------

/// Text field (búsqueda): fondo INPUT, borde sepia; foco = borde acento.
pub fn search_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let colors = ui_colors(theme);
    let border_color = match status {
        text_input::Status::Focused { .. } => colors.accent,
        text_input::Status::Hovered => colors.border,
        _ => colors.border,
    };
    text_input::Style {
        background: colors.input.into(),
        border: Border {
            color: border_color,
            width: 1.2,
            radius: radius(0.0),
        },
        icon: colors.muted,
        placeholder: colors.dim,
        value: colors.text,
        selection: colors.accent_tint,
    }
}

/// Dropdown (pick_list): fondo INPUT, borde sepia, texto blanco.
pub fn dropdown(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let colors = ui_colors(theme);
    let border_color = match status {
        pick_list::Status::Hovered => colors.accent,
        pick_list::Status::Opened { .. } => colors.accent,
        _ => colors.border,
    };
    pick_list::Style {
        text_color: colors.text,
        placeholder_color: colors.dim,
        handle_color: colors.muted,
        background: colors.input.into(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius(0.0),
        },
    }
}

/// Menú desplegable abierto (overlay del pick_list).
pub fn dropdown_menu(theme: &Theme) -> iced::overlay::menu::Style {
    let colors = ui_colors(theme);
    iced::overlay::menu::Style {
        background: colors.elevated.into(),
        border: Border {
            color: colors.border,
            width: 1.0,
            radius: radius(0.0),
        },
        text_color: colors.text,
        selected_text_color: colors.on_accent,
        selected_background: colors.accent.into(),
    }
}

/// Toggle (Extensiones): off = fondo elevado con knob sepia; on = acento.
pub fn toggle(theme: &Theme, status: toggler::Status) -> toggler::Style {
    let colors = ui_colors(theme);
    let active = matches!(status, toggler::Status::Active { .. } | toggler::Status::Hovered { .. } if is_on(&status));
    toggler::Style {
        background: if active { colors.accent } else { colors.elevated },
        background_border_width: 1.0,
        background_border_color: if active { colors.accent } else { colors.border },
        foreground: if active { colors.on_accent } else { colors.muted },
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
