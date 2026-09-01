//! Íconos Material (codepoints de MaterialIcons-Regular.ttf).
//!
//! Uso: `icon::home()` devuelve un `Text` con la fuente de íconos aplicada.
//! Codepoints: https://fonts.google.com/icons (Material Icons, ligaduras off).
use iced::widget::text;
use iced::{Color, Font};


/// Fuente de íconos Material. iced la registra en `main` vía
/// `.font(include_bytes!("MaterialIcons.ttf"))`; el nombre resuelve por el
/// family name embebido en el TTF ("Material Icons").
pub const ICON_FONT: Font = Font::with_name("Material Icons");

// Codepoints (Material Icons, versión ligada por nombre). Cada constante es
// el `char` Unicode Private-Use-Area correspondiente al ícono.
pub const HOME: char = '\u{e88a}';
pub const LIBRARY: char = '\u{e02f}';       // import_contacts (libro abierto)
pub const EXPLORE: char = '\u{e87a}';
pub const DOWNLOAD: char = '\u{e2c4}';
pub const EXTENSIONS: char = '\u{e87b}';
pub const SETTINGS: char = '\u{e8b8}';
pub const SEARCH: char = '\u{e8b6}';
pub const BACK: char = '\u{e5c4}';
pub const PLAY: char = '\u{e037}';
pub const BOOKMARK: char = '\u{e866}';
pub const CLOSE: char = '\u{e5cd}';
pub const PAUSE: char = '\u{e034}';
pub const CHECK: char = '\u{e5ca}';
pub const ERROR: char = '\u{e001}';          // error_outline (círculo con !)
pub const DOWNLOAD_FOR_OFFLINE: char = '\u{e171}';
pub const IMAGE: char = '\u{e3f4}';          // image (placeholder de portada)

/// Texto de ícono con tamaño y color. El caller encadena `.into()`.
pub fn glyph(code: char, size: u16, color: Color) -> text::Text<'static> {
    text(code.to_string()).font(ICON_FONT).size(size).color(color)
}
