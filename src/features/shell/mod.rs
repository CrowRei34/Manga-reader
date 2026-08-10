//! Shell con el nav rail lateral (Task 11). Tooltips en español neutro
//! para coincidir con el resto de la app.
use iced::widget::{button, text, Column, Row};
use iced::Element;

use super::Screen;

/// Mensajes de navegación que el shell emite hacia el app-message global.
#[derive(Debug, Clone)]
pub enum NavMsg {
    Navigate(Screen),
}

/// Monta el shell: nav rail a la izquierda + contenido a la derecha.
///
/// `M: From<NavMsg>` deja que cada botón emita el `M` concreto del app
/// (resuelto por el `impl From<NavMsg> for Message` de `app.rs`).
pub fn view<'a, M: 'a + From<NavMsg> + Clone>(
    screen: &Screen,
    content: Element<'a, M>,
) -> Element<'a, M> {
    let items: [(&'static str, Screen); 7] = [
        ("Inicio", Screen::Home),
        ("Explorar", Screen::Browse),
        ("Biblioteca", Screen::Library),
        ("Lector", Screen::Reader),
        ("Descargas", Screen::Downloads),
        ("Ajustes", Screen::Settings),
        ("Extensiones", Screen::Extensions),
    ];

    let buttons: Vec<Element<'a, M>> = items
        .into_iter()
        .map(|(label, target)| {
            let btn = button(text(label))
                .on_press(M::from(NavMsg::Navigate(target.clone())));
            // Resaltado del botón activo — Task 19 (theme) añadirá estilo.
            let _ = screen;
            btn.into()
        })
        .collect();

    let rail = Column::with_children(buttons).spacing(4);
    Row::with_children(vec![rail.into(), content]).into()
}