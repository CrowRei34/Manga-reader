//! Cover card: portada (imagen cacheada o placeholder con ícono) + título
//! + subtítulo. Compartida por Home, Explorar y Biblioteca.
use std::collections::{HashMap, HashSet};

use iced::widget::{button, column, container, image, text};
use iced::{ContentFit, Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::models::{Manga, MangaRef, Source};
use crate::features::details;
use crate::theme::palette;
use crate::widgets::icon;

/// Mensaje estándar al tocar una portada: abre Details para ese manga.
pub fn details_msg(m: &Manga) -> AppMessage {
    AppMessage::Details(details::Message::Load(MangaRef {
        source: m.source.clone(),
        url: m.url.clone(),
        title: m.title.clone(),
        cover_url: m.cover_url.clone().or_else(|| m.large_cover_url.clone()),
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
        let primary = m.cover_url.as_deref().filter(|s| !s.trim().is_empty());
        let fallback = m.large_cover_url.as_deref().filter(|s| !s.trim().is_empty());

        let target_url = match (primary, fallback) {
            (Some(p), _) => p,
            (None, Some(f)) => f,
            (None, None) => continue,
        };

        if let Some(Some(_)) = state.covers.get(target_url) {
            continue;
        }

        let primary_url = primary.map(|s| s.to_string());
        let fallback_url = fallback.map(|s| s.to_string());
        let key_url = target_url.to_string();
        let cache = state.cache.clone();
        let headers = headers.clone();

        tasks.push(Task::perform(
            async move {
                let mut path = None;
                if let Some(pu) = &primary_url {
                    path = tokio::time::timeout(
                        std::time::Duration::from_secs(15),
                        cache.get(pu, &headers),
                    )
                    .await
                    .ok()
                    .and_then(|res| res.ok());
                }
                if path.is_none() {
                    if let Some(fu) = &fallback_url {
                        path = tokio::time::timeout(
                            std::time::Duration::from_secs(15),
                            cache.get(fu, &headers),
                        )
                        .await
                        .ok()
                        .and_then(|res| res.ok());
                    }
                }
                let handle = if let Some(p) = &path {
                    tokio::task::spawn_blocking({
                        let p = p.clone();
                        move || {
                            if let Ok(bytes) = std::fs::read(&p) {
                                if let Ok(img) = ::image::load_from_memory(&bytes) {
                                    let img = img.resize_to_fill(
                                        COVER_W as u32,
                                        COVER_H as u32,
                                        ::image::imageops::FilterType::Triangle,
                                    );
                                    let rgba = img.into_rgba8();
                                    let (w, h) = rgba.dimensions();
                                    return Some(iced::widget::image::Handle::from_rgba(
                                        w,
                                        h,
                                        rgba.into_raw(),
                                    ));
                                }
                            }
                            None
                        }
                    })
                    .await
                    .ok()
                    .flatten()
                } else {
                    None
                };
                (key_url, handle)
            },
            |(url, handle)| AppMessage::CoverLoaded(url, handle),
        ));
    }

    if tasks.is_empty() {
        Task::none()
    } else {
        Task::batch(tasks)
    }
}

/// Cover card vertical: imagen (o placeholder) + título + subtítulo.
pub fn cover_card<'a>(
    m: &Manga,
    covers: &HashMap<String, Option<iced::widget::image::Handle>>,
    subtitle: Option<String>,
    msg: AppMessage,
) -> Element<'a, AppMessage> {
    let handle_opt = m
        .cover_url
        .as_deref()
        .and_then(|u| covers.get(u).and_then(|opt| opt.as_ref()))
        .or_else(|| {
            m.large_cover_url
                .as_deref()
                .and_then(|u| covers.get(u).and_then(|opt| opt.as_ref()))
        });

    let img: Element<'a, AppMessage> = match handle_opt {
        Some(handle) => image(handle.clone())
            .width(Length::Fixed(COVER_W))
            .height(Length::Fixed(COVER_H))
            .content_fit(ContentFit::Cover)
            .into(),
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

/// Grid de cover cards con espaciado dinámico.
pub fn cover_grid<'a>(
    mangas: &[Manga],
    covers: &HashMap<String, Option<iced::widget::image::Handle>>,
    window_width: f32,
    msg: impl Fn(&Manga) -> AppMessage,
) -> Element<'a, AppMessage> {
    let (per_row, spacing) = grid_layout_for(window_width);
    let rows = mangas
        .chunks(per_row.max(1))
        .map(|chunk| {
            let mut r = iced::widget::Row::new().spacing(spacing);
            for m in chunk {
                r = r.push(cover_card(m, covers, None, msg(m)));
            }
            r.into()
        })
        .collect::<Vec<Element<'a, AppMessage>>>();
    iced::widget::Column::with_children(rows).spacing(16).into()
}

/// Grid para resultados federados.
pub fn search_result_grid<'a>(
    mangas: &[Manga],
    covers: &HashMap<String, Option<iced::widget::image::Handle>>,
    window_width: f32,
    sources: &[Source],
    grouped: bool,
    visible_per_source: &HashMap<String, usize>,
    exhausted_sources: &HashSet<String>,
) -> Element<'a, AppMessage> {
    if !grouped {
        return cover_grid(mangas, covers, window_width, details_msg);
    }
    let (per_row, spacing) = grid_layout_for(window_width);
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
            let mut row = iced::widget::Row::new().spacing(spacing);
            for manga in chunk {
                let author = manga.authors.first().cloned().unwrap_or_default();
                let subtitle = if author.is_empty() { language.to_owned() } else { format!("{} · {}", language, author) };
                row = row.push(cover_card(manga, covers, Some(subtitle), details_msg(manga)));
            }
            row.into()
        }).collect::<Vec<Element<'a, AppMessage>>>();
        sections.push(heading.into());
        sections.push(iced::widget::Column::with_children(rows).spacing(16).into());
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
    iced::widget::Column::with_children(sections).spacing(10).into()
}

fn language_label(locale: &str) -> &'static str {
    let locale = locale.to_ascii_lowercase();
    if locale.starts_with("es") { "Español" } else if locale.starts_with("en") { "Inglés" }
    else if locale.starts_with("pt") { "Portugués" } else if locale.starts_with("fr") { "Francés" }
    else if locale.starts_with("ja") { "Japonés" } else if locale.starts_with("zh") { "Chino" }
    else { "Otro idioma" }
}

/// Calcula el número de columnas y el espaciado dinámico exacto para llenar el ancho.
pub fn grid_layout_for(window_width: f32) -> (usize, f32) {
    let content_w = (window_width - 241.0).max(COVER_W);
    let per_row = ((content_w + 16.0) / (COVER_W + 16.0)).floor() as usize;
    let per_row = per_row.max(1);
    
    let spacing = if per_row > 1 {
        let remaining_space = content_w - (per_row as f32 * COVER_W);
        (remaining_space / (per_row - 1) as f32).max(12.0)
    } else {
        16.0
    };
    (per_row, spacing)
}

#[allow(dead_code)]
pub fn per_row_for(window_width: f32) -> usize {
    grid_layout_for(window_width).0
}
