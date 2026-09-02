//! Cover card: portada (imagen cacheada o placeholder con ícono) + título
//! + subtítulo. Compartida por Home, Explorar y Biblioteca.
//!
//! La portada se carga asíncrona: el caller dispara `fetch_covers` tras
//! poblar su lista (browse/catalog, library, home recent) y las imágenes
//! aterrizan como `Message::CoverLoaded(url, path)` en el reducer global,
//! que las guarda en `state.covers: HashMap<String, PathBuf>`.
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use iced::widget::{button, column, container, image, text, Space};
use iced::{ContentFit, Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::models::{Manga, MangaRef, Source};
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
    let headers = state.cover_headers.clone();
    for m in mangas {
        if let Some(url) = &m.cover_url {
            if state.covers.contains_key(url) {
                continue;
            }
            let url = url.clone();
            let cache = state.cache.clone();
            let headers = headers.clone();
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

    let display_title = if m.is_nsfw { format!("+18 · {}", m.title) } else { m.title.clone() };
    let title = text(ellipsize(&display_title, 48))
        .size(14)
        .color(if m.is_nsfw { palette::ACCENT } else { palette::TEXT })
        .width(Length::Fixed(COVER_W))
        .height(Length::Fixed(38.0));
    let subtitle = subtitle.unwrap_or_else(|| m.authors.first().cloned().unwrap_or_default());
    let sub = text(ellipsize(&subtitle, 32))
        .size(12)
        .color(palette::TEXT_MUTED)
        .width(Length::Fixed(COVER_W))
        .height(Length::Fixed(18.0));

    button(
        column![img, title, sub].spacing(4).width(Length::Fixed(COVER_W)),
    )
    .style(crate::theme::link_button)
    .padding(0)
    .on_press(msg)
    .into()
}

fn ellipsize(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let mut shortened: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    shortened.push('…');
    shortened
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
            let mut r = iced::widget::Row::new().spacing(16).width(Length::Fill);
            for (index, m) in chunk.iter().enumerate() {
                if chunk.len() == 1 || index > 0 {
                    r = r.push(Space::new(Length::Fill, Length::Shrink));
                }
                r = r.push(cover_card(m, covers, None, msg(m)));
            }
            if chunk.len() == 1 {
                r = r.push(Space::new(Length::Fill, Length::Shrink));
            }
            r.into()
        })
        .collect::<Vec<Element<'a, AppMessage>>>();
    iced::widget::Column::with_children(rows).spacing(16).width(Length::Fill).into()
}

/// Grid para resultados federados. Muestra la fuente en cada tarjeta para
/// distinguir mangas con el mismo título provenientes de parsers distintos.
pub fn search_result_grid<'a>(
    mangas: &[Manga],
    covers: &HashMap<String, PathBuf>,
    per_row: usize,
    sources: &[Source],
    grouped: bool,
    visible_per_source: &HashMap<String, usize>,
    exhausted_sources: &HashSet<String>,
) -> Element<'a, AppMessage> {
    if !grouped {
        return cover_grid(mangas, covers, per_row, details_msg);
    }
    let mut groups: std::collections::BTreeMap<String, Vec<&Manga>> = std::collections::BTreeMap::new();
    for manga in mangas { groups.entry(manga.source.clone()).or_default().push(manga); }
    let mut sections: Vec<Element<'a, AppMessage>> = Vec::new();
    for (source_id, group) in groups {
        let source = sources.iter().find(|s| s.id == source_id);
        let source_name = source.map(|s| s.name.clone()).unwrap_or_else(|| source_id.clone());
        let language = source.and_then(|s| s.language.as_deref()).map(language_label).unwrap_or("Idioma mixto");
        let visible_limit = visible_per_source.get(&source_id).copied().unwrap_or(10);
        let visible = group.iter().take(visible_limit).copied().collect::<Vec<_>>();
        let more = group.len() > visible_limit || !exhausted_sources.contains(&source_id);
        let heading = iced::widget::row![
            iced::widget::text(source_name).size(18).color(palette::TEXT),
            iced::widget::text(format!("{} · {} mangas encontrados", language, group.len()))
                .size(12).color(palette::TEXT_MUTED),
        ].spacing(10).align_y(iced::Alignment::Center);
        let rows = visible.chunks(per_row.max(1)).map(|chunk| {
            let mut row = iced::widget::Row::new().spacing(16).width(Length::Fill);
            for (index, manga) in chunk.iter().enumerate() {
                if chunk.len() == 1 || index > 0 {
                    row = row.push(Space::new(Length::Fill, Length::Shrink));
                }
                let author = manga.authors.first().cloned().unwrap_or_default();
                let subtitle = if author.is_empty() { language.to_owned() } else { format!("{} · {}", language, author) };
                row = row.push(cover_card(manga, covers, Some(subtitle), details_msg(manga)));
            }
            if chunk.len() == 1 {
                row = row.push(Space::new(Length::Fill, Length::Shrink));
            }
            row.into()
        }).collect::<Vec<Element<'a, AppMessage>>>();
        sections.push(heading.into());
        sections.push(iced::widget::Column::with_children(rows).spacing(16).width(Length::Fill).into());
        if more {
            sections.push(
                iced::widget::container(
                    iced::widget::button(iced::widget::text("Ver más").size(13))
                        .on_press(AppMessage::Browse(
                            crate::features::browse::Message::MoreSource(source_id.clone())
                        ))
                        .style(crate::theme::ghost_button)
                        .padding([8, 20]),
                )
                .width(Length::Fill)
                .center_x(Length::Fill)
                .into(),
            );
        }
        sections.push(iced::widget::Space::new(Length::Fill, Length::Fixed(12.0)).into());
    }
    iced::widget::Column::with_children(sections).spacing(10).width(Length::Fill).into()
}

fn language_label(locale: &str) -> &'static str {
    crate::language::label(Some(locale))
}

/// Grid responsivo determinista: el caller pasa `per_row` calculado del
/// ancho de ventana (ver `per_row_for`). No usamos `iced::widget::responsive`
/// porque dentro de `scrollable` recibe altura infinita y solapa el header.
pub fn per_row_for(window_width: f32) -> usize {
    // Ancho de contenido ≈ ventana - sidebar(185) - padding(40).
    let content = (window_width - 225.0).max(200.0);
    ((content / (COVER_W + 16.0)).floor() as usize).max(1)
}
