//! Cover card: portada (imagen cacheada o placeholder con ícono) + título
//! + subtítulo. Compartida por Home, Explorar y Biblioteca.
//!
//! La portada se carga asíncrona: el caller dispara `fetch_covers` tras
//! poblar su lista (browse/catalog, library, home recent) y las imágenes
//! aterrizan como `Message::CoverLoaded(url, path)` en el reducer global,
//! que las guarda en `state.covers: HashMap<String, PathBuf>`.
use std::collections::HashMap;
use std::path::PathBuf;

use iced::widget::{button, column, container, image, text};
use iced::{ContentFit, Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::models::{Manga, MangaRef};
use crate::features::details;
use crate::theme::palette;
use crate::widgets::icon;

/// Mensaje estándar al tocar una portada: abre Details para ese manga.
/// (fn-pointer para `responsive_cover_grid`, que no acepta closures con captures.)
pub fn details_msg(m: &Manga) -> AppMessage {
    AppMessage::Details(details::Message::Load(MangaRef {
        source: m.source.clone(),
        url: m.url.clone(),
        title: m.title.clone(),
    }))
}

/// Ancho/alto de portada (aspect ~0.70 como en el diseño original).
pub const COVER_W: f32 = 168.0;
pub const COVER_H: f32 = 240.0;

/// Dispara la descarga de portadas para una lista de mangas. Cada URL
/// sin entrada en `state.covers` genera un `Task` que baja la imagen vía
/// `ImageCache` y aterriza en `Message::CoverLoaded`.
pub fn fetch_covers(state: &AppState, mangas: &[Manga]) -> Task<AppMessage> {
    let mut tasks = Vec::new();
    for m in mangas {
        if let Some(url) = &m.cover_url {
            if state.covers.contains_key(url) {
                continue;
            }
            let url = url.clone();
            let cache = state.cache.clone();
            let headers = state.cover_headers.clone();
            tasks.push(Task::perform(
                async move {
                    let path = cache.get(&url, &headers).await.ok();
                    (url, path)
                },
                |(url, path)| AppMessage::CoverLoaded(url, path),
            ));
        }
    }
    if tasks.is_empty() {
        Task::none()
    } else {
        Task::batch(tasks)
    }
}

/// Cover card vertical: imagen (o placeholder) + título + subtítulo.
/// `on_press` se encadena fuera (el card es un `button` fantasma).
pub fn cover_card<'a>(
    m: &Manga,
    covers: &HashMap<String, PathBuf>,
    subtitle: Option<String>,
    msg: AppMessage,
) -> Element<'a, AppMessage> {
    let img: Element<'a, AppMessage> = match m
        .cover_url
        .as_ref()
        .and_then(|u| covers.get(u))
    {
        Some(path) => image(image::Handle::from_path(path.clone()))
            .width(Length::Fixed(COVER_W))
            .height(Length::Fixed(COVER_H))
            .content_fit(ContentFit::Cover)
            .into(),
        // OJO: `center_x(Length)` SOBREESCRIBE el width/height — pasarle
        // `Fill` aquí infla el placeholder dentro de scrollables (layout
        // infinito que tapa el header/búsqueda). Centrar con el tamaño fijo.
        None => container(icon::glyph(icon::IMAGE, 40, palette::TEXT_DIM))
            .center_x(Length::Fixed(COVER_W))
            .center_y(Length::Fixed(COVER_H))
            .style(crate::theme::card_container)
            .into(),
    };

    let title = text(m.title.clone())
        .size(14)
        .color(palette::TEXT)
        .width(Length::Fixed(COVER_W));
    let sub = text(subtitle.unwrap_or_else(|| m.authors.first().cloned().unwrap_or_default()))
        .size(12)
        .color(palette::TEXT_MUTED)
        .width(Length::Fixed(COVER_W));

    button(
        column![img, title, sub].spacing(4).width(Length::Fixed(COVER_W)),
    )
    .style(crate::theme::link_button)
    .padding(0)
    .on_press(msg)
    .into()
}

/// Grid de cover cards: chunked en filas de `per_row` (el diseño original
/// usa ~5-6 por fila en 1100px de contenido).
pub fn cover_grid<'a>(
    mangas: &[Manga],
    covers: &HashMap<String, PathBuf>,
    per_row: usize,
    msg: impl Fn(&Manga) -> AppMessage,
) -> Element<'a, AppMessage> {
    let rows = mangas
        .chunks(per_row.max(1))
        .map(|chunk| {
            let mut r = iced::widget::Row::new().spacing(16);
            for m in chunk {
                r = r.push(cover_card(m, covers, None, msg(m)));
            }
            r.into()
        })
        .collect::<Vec<Element<'a, AppMessage>>>();
    iced::widget::Column::with_children(rows).spacing(16).into()
}

/// Grid responsivo determinista: el caller pasa `per_row` calculado del
/// ancho de ventana (ver `per_row_for`). No usamos `iced::widget::responsive`
/// porque dentro de `scrollable` recibe altura infinita y solapa el header.
pub fn per_row_for(window_width: f32) -> usize {
    // Ancho de contenido ≈ ventana - sidebar(185) - padding(40).
    let content = (window_width - 225.0).max(200.0);
    ((content / (COVER_W + 16.0)).floor() as usize).max(1)
}
