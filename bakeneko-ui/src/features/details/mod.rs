//! Pantalla de Detalle (Details) — réplica visual del diseño original:
//!
//! ```text
//! ‹ Atrás
//! [cover]  Título (grande) | autor | SINOPSIS | descripción…
//!          [▶ Leer Ahora] [🔖 En Biblioteca]
//! Capítulos (N)                                  ⭳ Descargar Todo
//!   Capítulo 1                              ✓    Ver
//!   …
//! ```
use iced::widget::{button, column, container, image, row, scrollable, text, Column};
use iced::{ContentFit, Element, Length, Task};
use std::collections::BTreeMap;

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::daemon::api::MangaSourceApi;
use bakeneko_core::db;
use bakeneko_core::db::dao::{history_dao, manga_dao};
use bakeneko_core::error::{DaemonError, DbError};
use bakeneko_core::models::{Chapter, Manga, MangaRef};
use crate::features::library;
use crate::features::reader;
use crate::features::Screen;
use crate::theme::palette;
use crate::widgets::cover::{COVER_H, COVER_W};
use crate::widgets::icon;

#[derive(Debug, Default)]
pub struct State {
    pub manga: Option<Manga>,
    pub chapters: Vec<Chapter>,
    pub loading: bool,
    /// Pantalla desde la que se abrió el detalle (para el botón Atrás).
    pub back_target: Option<Screen>,
    pub in_library: bool,
    /// Filtro de idioma de capítulos; `None` muestra todos.
    pub language_filter: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Load(MangaRef),
    Fetched(Result<Manga, DaemonError>),
    ChapterSelected(Chapter),
    ToggleLibrary,
    ReadNow,
    DownloadChapter(Chapter),
    DownloadAll,
    SetLanguage(Option<String>),
    Back,
}

/// Reducer del feature Details. Muta `state.details` y devuelve
/// `Task<AppMessage>` (los efectos async resuelven contra el reducer
/// global: `DetailsFetched`, `Library(Load)`, etc.).
pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::Load(mref) => {
            state.details.loading = true;
            state.details.manga = None;
            state.details.chapters.clear();
            state.details.language_filter = None;
            state.details.in_library = state.library.iter().any(|manga| {
                manga.source == mref.source && manga.url == mref.url
            });
            // Guarda la pantalla origen para "Atrás".
            if state.screen != Screen::Details {
                state.details.back_target = Some(state.screen.clone());
            }
            state.screen = Screen::Details;
            let d = state.daemon.clone();
            let src = mref.source.clone();
            let manga = Manga {
                source: mref.source,
                url: mref.url,
                title: mref.title,
                ..Default::default()
            };
            if let Some(d) = d {
                Task::perform(
                    async move { d.manga_details(&src, &manga).await },
                    AppMessage::DetailsFetched,
                )
            } else {
                Task::none()
            }
        }
        Message::Fetched(Ok(manga)) => {
            state.details.chapters = manga.chapters.clone();
            state.details.manga = Some(manga.clone());
            state.details.loading = false;
            // Trae la portada grande si falta.
            let cover_task = {
                let list = vec![manga.clone()];
                crate::widgets::cover::fetch_covers(state, &list)
            };
            // Persiste en DB (background) — `upsert` con `library=0` para
            // no pisar el flag si ya estaba en biblioteca.
            let dbh = state.db.clone();
            let persist = if let Some(db) = dbh {
                Task::perform(
                    db::db_blocking(db, move |conn| {
                        manga_dao::upsert(conn, &manga, 0)?;
                        Ok::<(), DbError>(())
                    }),
                    |r| match r {
                        Ok(_) => AppMessage::ErrorDismissed,
                        Err(e) => AppMessage::LibraryLoaded(Err(e)),
                    },
                )
            } else {
                Task::none()
            };
            Task::batch([cover_task, persist])
        }
        Message::Fetched(Err(e)) => {
            state.error = Some(e.to_string());
            state.details.loading = false;
            Task::none()
        }
        Message::ChapterSelected(c) => {
            // Pasa la lista de capítulos + índice de este capítulo al reader
            // (para navegar ‹ › entre capítulos).
            let chapters: Vec<Chapter> = state.details.chapters.iter()
                .filter(|chapter| state.details.language_filter.as_deref()
                    .map(|filter| chapter_language_key(chapter) == filter)
                    .unwrap_or(true))
                .cloned()
                .collect();
            let idx = chapters.iter().position(|x| x.url == c.url).unwrap_or(0);
            state.reader.chapters = chapters;
            state.reader.current_chapter = idx;
            // Modo de lectura por defecto desde settings (TODO: mapear).
            Task::batch([
                Task::done(AppMessage::Reader(reader::Message::Load(c))),
                Task::done(AppMessage::NavigateTo(Screen::Reader)),
            ])
        }
        Message::ReadNow => {
            // Continúa el último capítulo leído; si no hay historial, comienza
            // por el primero disponible del idioma seleccionado.
            let saved_chapter = match (&state.db, &state.details.manga) {
                (Some(db), Some(manga)) => {
                    let conn = db.lock().unwrap();
                    manga_dao::get_id_by_key(&conn, &manga.source, &manga.url).ok()
                        .and_then(|mid| history_dao::get(&conn, mid).ok().flatten())
                        .map(|(chapter, _, _)| chapter)
                }
                _ => None,
            };
            let available: Vec<Chapter> = state
                .details
                .chapters
                .iter()
                .filter(|chapter| state.details.language_filter.as_deref()
                    .map(|filter| chapter_language_key(chapter) == filter)
                    .unwrap_or(true))
                .cloned()
                .collect();
            let target = saved_chapter
                .and_then(|saved| available.iter().find(|chapter| chapter.number as i32 == saved).cloned())
                .or_else(|| available.into_iter().min_by(|a, b| {
                    a.number.partial_cmp(&b.number).unwrap_or(std::cmp::Ordering::Equal)
                }));
            match target {
                Some(c) => update(state, Message::ChapterSelected(c)),
                None => Task::none(),
            }
        }
        Message::ToggleLibrary => {
            let m_opt = state.details.manga.clone();
            let dbh = state.db.clone();
            if let (Some(m), Some(db)) = (m_opt, dbh) {
                let in_library = !state.details.in_library;
                state.details.in_library = in_library;
                return Task::perform(
                    db::db_blocking(db, move |conn| {
                        let id = manga_dao::upsert(conn, &m, 0)?;
                        manga_dao::set_library_flag(conn, id, in_library)?;
                        Ok::<(), DbError>(())
                    }),
                    |r| match r {
                        Ok(_) => AppMessage::Library(library::Message::Load),
                        Err(e) => AppMessage::LibraryLoaded(Err(e)),
                    },
                );
            }
            Task::none()
        }
        Message::DownloadChapter(c) => {
            // Encola un capítulo suelto en el DownloadManager.
            if let (Some(mgr), Some(m)) = (state.downloads.clone(), state.details.manga.clone()) {
                let _ = mgr.enqueue(&m, &c);
            }
            Task::none()
        }
        Message::DownloadAll => {
            if let (Some(mgr), Some(m)) = (state.downloads.clone(), state.details.manga.clone()) {
                for c in &state.details.chapters {
                    let _ = mgr.enqueue(&m, c);
                }
            }
            Task::none()
        }
        Message::SetLanguage(language) => {
            state.details.language_filter = language;
            Task::none()
        }
        Message::Back => {
            let target = state.details.back_target.clone().unwrap_or(Screen::Home);
            Task::done(AppMessage::NavigateTo(target))
        }
    }
}

/// Vista del feature (réplica del diseño).
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let accent = crate::theme::accent(&state.settings);
    let back = button(
        row![
            icon::glyph(icon::BACK, 16, palette::TEXT_MUTED),
            text("Atrás").size(14).color(palette::TEXT_MUTED),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center),
    )
    .on_press(AppMessage::Details(Message::Back))
    .style(crate::theme::link_button)
    .padding(4);

    if state.details.loading && state.details.manga.is_none() {
        return column![back, text("Cargando…").size(16).color(palette::TEXT_MUTED)]
            .spacing(16)
            .into();
    }
    let Some(m) = &state.details.manga else {
        return column![back, text("Sin datos").size(16).color(palette::TEXT_MUTED)]
            .spacing(16)
            .into();
    };

    let make_cover = |w: f32, h: f32| -> Element<'_, AppMessage> {
        match m
            .cover_url
            .as_ref()
            .and_then(|u| state.covers.get(u))
        {
            Some(path) => image(image::Handle::from_path(path.clone()))
                .width(Length::Fixed(w))
                .height(Length::Fixed(h))
                .content_fit(ContentFit::Cover)
                .into(),
            None => container(icon::glyph(icon::IMAGE, 48, palette::TEXT_DIM))
                .center_x(Length::Fixed(w))
                .center_y(Length::Fixed(h))
                .style(crate::theme::card_container)
                .into(),
        }
    };

    // Filtro rápido por idioma; después agrupa y ordena numéricamente.
    let mut language_options: Vec<(String, &'static str)> = state.details.chapters.iter()
        .map(|chapter| {
            let key = chapter_language_key(chapter).to_owned();
            (key.clone(), crate::language::label_for_key(&key))
        })
        .collect();
    language_options.sort_by(|a, b| a.1.cmp(b.1));
    language_options.dedup_by(|a, b| a.0 == b.0);
    let filtered_chapters: Vec<Chapter> = state.details.chapters.iter()
        .filter(|chapter| state.details.language_filter.as_deref()
            .map(|filter| chapter_language_key(chapter) == filter)
            .unwrap_or(true))
        .cloned()
        .collect();
    let chapter_groups = organize_chapters(&filtered_chapters);
    let mut chapter_rows: Vec<Element<'_, AppMessage>> = Vec::new();
    for (language, chapters) in &chapter_groups {
        chapter_rows.push(
            container(text(format!("{} ({})", language, chapters.len())).size(14).color(accent))
                .padding([8, 4])
                .width(Length::Fill)
                .into(),
        );

        for c in chapters {
            let status_icon = if c.read {
                icon::glyph(icon::CHECK, 18, palette::SUCCESS)
            } else {
                icon::glyph(icon::DOWNLOAD_FOR_OFFLINE, 18, palette::TEXT_MUTED)
            };
            let title: Element<'_, AppMessage> = match chapter_subtitle(c) {
                Some(subtitle) => column![
                    text(chapter_label(c)).size(14).color(palette::TEXT),
                    text(subtitle).size(12).color(palette::TEXT_MUTED),
                ].spacing(2).width(Length::Fill).into(),
                None => text(chapter_label(c)).size(14).color(palette::TEXT).width(Length::Fill).into(),
            };

            let chapter_btn = button(
                row![
                    title,
                    iced::widget::horizontal_space(),
                ]
                .align_y(iced::Alignment::Center)
                .width(Length::Fill)
            )
            .on_press(AppMessage::Details(Message::ChapterSelected(c.clone())))
            .style(crate::theme::chapter_card_button)
            .padding([9, 12])
            .width(Length::Fill);

            let download_btn = button(status_icon)
                .on_press(AppMessage::Details(Message::DownloadChapter(c.clone())))
                .style(crate::theme::link_button)
                .padding([6, 8]);

            chapter_rows.push(
                row![
                    chapter_btn,
                    download_btn,
                    iced::widget::Space::with_width(Length::Fixed(8.0)),
                ]
                .spacing(2)
                .align_y(iced::Alignment::Center)
                .width(Length::Fill)
                .into(),
            );
        }
    }

    let chapters_header = container(row![
        text(format!("Capítulos ({}/{})", filtered_chapters.len(), state.details.chapters.len()))
            .size(18)
            .color(palette::TEXT),
        iced::widget::horizontal_space(),
        button(
            row![
                icon::glyph(icon::DOWNLOAD, 16, accent),
                text("Descargar Todo").size(13).color(accent),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(AppMessage::Details(Message::DownloadAll))
        .style(crate::theme::link_button_accent)
        .padding(4),
    ]
    .align_y(iced::Alignment::Center))
    .style(crate::theme::card_container)
    .padding([10, 12])
    .width(Length::Fill);

    let mut language_buttons = row![button(text("Todos").size(12))
        .on_press(AppMessage::Details(Message::SetLanguage(None)))
        .style(if state.details.language_filter.is_none() { crate::theme::primary_button } else { crate::theme::link_button })
        .padding([5, 9])]
        .spacing(6)
        .padding([4, 0])
        .align_y(iced::Alignment::Center);
    for (language, label) in language_options {
        let active = state.details.language_filter.as_deref() == Some(language.as_str());
        language_buttons = language_buttons.push(
            button(text(label).size(12))
                .on_press(AppMessage::Details(Message::SetLanguage(Some(language))))
                .style(if active { crate::theme::primary_button } else { crate::theme::link_button })
                .padding([5, 9]),
        );
    }

    let clean_description = format_synopsis(m.description.as_deref());

    if state.window_size.0 >= 960.0 {
        let left_width = (state.window_size.0 * 0.36).clamp(380.0, 460.0);
        let cover_w: f32 = if left_width < 420.0 { 125.0 } else { 140.0 };
        let cover_h = (cover_w / 0.69).round();

        let left_buttons = column![
            button(
                row![
                    icon::glyph(icon::PLAY, 16, palette::ON_ACCENT),
                    text("Leer Ahora").size(13).color(palette::ON_ACCENT),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .on_press(AppMessage::Details(Message::ReadNow))
            .style(crate::theme::primary_button)
            .padding([9, 14])
            .width(Length::Fill),
            button(
                row![
                    icon::glyph(icon::BOOKMARK, 16, accent),
                    text(if state.details.in_library { "Quitar de Biblioteca" } else { "Agregar a Biblioteca" })
                        .size(13).color(accent),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .on_press(AppMessage::Details(Message::ToggleLibrary))
            .style(crate::theme::ghost_button)
            .padding([9, 14])
            .width(Length::Fill),
        ]
        .spacing(8)
        .width(Length::Fill);

        let cover_card = container(make_cover(cover_w, cover_h))
            .style(crate::theme::card_container)
            .padding(4);

        let top_meta = column![
            text(m.title.clone()).size(17).color(palette::TEXT),
            text(m.authors.first().cloned().unwrap_or_default())
                .size(12)
                .color(palette::TEXT_MUTED),
            iced::widget::Space::with_height(Length::Fixed(4.0)),
            left_buttons,
        ]
        .spacing(6)
        .width(Length::Fill);

        let top_header = row![
            cover_card,
            top_meta,
        ]
        .spacing(12)
        .align_y(iced::Alignment::Start)
        .width(Length::Fill);

        let synopsis_box = column![
            text("Sinopsis").size(14).color(accent),
            scrollable(
                container(
                    text(clean_description.clone())
                        .size(13)
                        .line_height(iced::widget::text::LineHeight::Relative(1.45))
                        .color(palette::TEXT)
                )
                .padding(iced::Padding {
                    top: 2.0,
                    right: 12.0,
                    bottom: 8.0,
                    left: 2.0,
                })
            )
            .style(crate::theme::scrollable_style)
            .width(Length::Fill)
            .height(Length::Fill),
        ]
        .spacing(6)
        .width(Length::Fill)
        .height(Length::Fill);

        let left_info = container(
            column![
                top_header,
                synopsis_box,
            ]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .style(crate::theme::panel_container)
        .padding(14)
        .width(Length::Fixed(left_width))
        .height(Length::Fill)
        .clip(true);

        let left_col = column![
            back,
            left_info,
        ]
        .spacing(12)
        .width(Length::Fixed(left_width))
        .height(Length::Fill);

        let right_col = column![
            chapters_header,
            container(scrollable(language_buttons)
                .style(crate::theme::scrollable_style)
                .width(Length::Fill)
                .direction(scrollable::Direction::Horizontal(Default::default())))
                .style(crate::theme::panel_container)
                .padding([6, 8])
                .width(Length::Fill),
            container(scrollable(
                Column::with_children(chapter_rows)
                    .spacing(4)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 14.0,
                        bottom: 0.0,
                        left: 0.0,
                    })
                    .width(Length::Fill)
            )
            .style(crate::theme::scrollable_style)
            .width(Length::Fill)
            .height(Length::Fill))
            .style(crate::theme::panel_container)
            .padding(iced::Padding {
                top: 8.0,
                right: 4.0,
                bottom: 8.0,
                left: 8.0,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true),
        ]
        .spacing(12)
        .width(Length::Fill)
        .height(Length::Fill);

        row![left_col, right_col]
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        let title_block = column![
            text(m.title.clone()).size(24).color(palette::TEXT),
            text(m.authors.first().cloned().unwrap_or_default())
                .size(13)
                .color(palette::TEXT_MUTED),
            text("Sinopsis").size(14).color(accent),
            scrollable(
                container(
                    text(clean_description)
                        .size(13)
                        .line_height(iced::widget::text::LineHeight::Relative(1.45))
                        .color(palette::TEXT)
                )
                .padding(iced::Padding {
                    top: 2.0,
                    right: 12.0,
                    bottom: 6.0,
                    left: 2.0,
                })
            )
            .style(crate::theme::scrollable_style)
            .width(Length::Fill)
            .height(Length::Fixed(140.0)),
        ]
        .spacing(8)
        .width(Length::Fill);

        let buttons_row = row![
            button(
                row![
                    icon::glyph(icon::PLAY, 16, palette::ON_ACCENT),
                    text("Leer Ahora").size(14).color(palette::ON_ACCENT),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .on_press(AppMessage::Details(Message::ReadNow))
            .style(crate::theme::primary_button)
            .padding([10, 18]),
            button(
                row![
                    icon::glyph(icon::BOOKMARK, 16, accent),
                    text(if state.details.in_library { "Quitar de Biblioteca" } else { "Agregar a Biblioteca" })
                        .size(14).color(accent),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .on_press(AppMessage::Details(Message::ToggleLibrary))
            .style(crate::theme::ghost_button)
            .padding([10, 18]),
        ]
        .spacing(12);

        let header_row: Element<'_, AppMessage> = if state.window_size.0 < 760.0 {
            container(
                column![
                    container(make_cover(COVER_W * 1.3, COVER_H * 1.3))
                        .style(crate::theme::card_container)
                        .padding(6),
                    column![title_block, buttons_row].spacing(14),
                ]
                .spacing(18)
                .align_x(iced::Alignment::Center),
            )
            .style(crate::theme::panel_container)
            .padding(14)
            .width(Length::Fill)
            .clip(true)
            .into()
        } else {
            container(
                row![
                    container(make_cover(COVER_W * 1.3, COVER_H * 1.3))
                        .style(crate::theme::card_container)
                        .padding(6),
                    column![title_block, buttons_row].spacing(14),
                ]
                .spacing(22)
                .align_y(iced::Alignment::Start),
            )
            .style(crate::theme::panel_container)
            .padding(18)
            .width(Length::Fill)
            .clip(true)
            .into()
        };

        column![
            back,
            header_row,
            container(scrollable(language_buttons)
                .style(crate::theme::scrollable_style)
                .width(Length::Fill)
                .direction(scrollable::Direction::Horizontal(Default::default())))
                .style(crate::theme::panel_container)
                .padding([6, 8])
                .width(Length::Fill),
            chapters_header,
            container(scrollable(
                Column::with_children(chapter_rows)
                    .spacing(4)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 14.0,
                        bottom: 0.0,
                        left: 0.0,
                    })
                    .width(Length::Fill)
            )
            .style(crate::theme::scrollable_style)
            .width(Length::Fill)
            .height(Length::Fill))
            .style(crate::theme::panel_container)
            .padding(iced::Padding {
                top: 8.0,
                right: 4.0,
                bottom: 8.0,
                left: 8.0,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true),
        ]
        .spacing(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

fn organize_chapters(chapters: &[Chapter]) -> BTreeMap<String, Vec<Chapter>> {
    let mut groups: BTreeMap<String, Vec<Chapter>> = BTreeMap::new();
    for chapter in chapters {
        groups.entry(language_label(chapter).to_string()).or_default().push(chapter.clone());
    }
    for items in groups.values_mut() {
        items.sort_by(|a, b| {
            a.number.partial_cmp(&b.number).unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.upload_date.cmp(&b.upload_date))
                .then_with(|| a.title.cmp(&b.title))
        });
    }
    groups
}

fn language_label(chapter: &Chapter) -> &'static str {
    crate::language::label_for_key(chapter_language_key(chapter))
}

fn chapter_language_key(chapter: &Chapter) -> &'static str {
    if let Some(locale) = chapter.language.as_deref().filter(|value| !value.trim().is_empty()) {
        let key = crate::language::key(Some(locale));
        if key != "other" && key != "mixed" { return key; }
    }
    // Las fuentes multilingües de Futon (especialmente MangaDex) guardan el
    // display name del locale en branch: "English", "Español", "Українська"…
    if let Some(branch) = chapter.branch.as_deref().filter(|value| !value.trim().is_empty()) {
        let key = crate::language::key(Some(branch));
        if key != "other" && key != "mixed" { return key; }
    }
    // Compatibilidad con parsers antiguos que incluían el locale en el ID.
    crate::language::key(chapter.source.rsplit('_').next())
}

fn chapter_label(chapter: &Chapter) -> String {
    if chapter.number > 0.0 && chapter.number.is_finite() {
        if chapter.number.fract() == 0.0 {
            format!("Capítulo {}", chapter.number as i64)
        } else {
            let number = format!("{:.2}", chapter.number).trim_end_matches('0').to_string();
            format!("Capítulo {number}")
        }
    } else if chapter.title.trim().is_empty() {
        "Capítulo sin número".to_string()
    } else {
        chapter.title.trim().to_string()
    }
}

fn chapter_subtitle(chapter: &Chapter) -> Option<String> {
    let title = chapter.title.trim();
    if title.is_empty() || title.eq_ignore_ascii_case(&chapter_label(chapter)) {
        return None;
    }
    let lower = title.to_lowercase();
    let is_generic = ["capítulo", "capitulo", "chapter", "cap.", "ch."]
        .iter()
        .any(|prefix| lower.starts_with(prefix));
    (!is_generic).then(|| title.to_string())
}

fn format_synopsis(raw: Option<&str>) -> String {
    let Some(text) = raw else {
        return "Sin descripción".to_string();
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return "Sin descripción".to_string();
    }
    let cleaned = trimmed
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n\n")
        .replace("<p>", "")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">");

    let mut result = String::with_capacity(cleaned.len());
    let mut consecutive_newlines = 0;
    for ch in cleaned.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                result.push(ch);
            }
        } else {
            consecutive_newlines = 0;
            result.push(ch);
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chapter(number: f32, title: &str, language: Option<&str>) -> Chapter {
        Chapter {
            source: "MANGADEX".into(), url: format!("/{number}/{title}"), title: title.into(), number,
            volume: 0, language: language.map(str::to_string), scanlator: None, upload_date: 0,
            branch: None, blob: Default::default(), read: false,
        }
    }

    #[test]
    fn groups_by_language_and_sorts_by_number() {
        let chapters = vec![
            chapter(2.0, "Segundo", Some("es")),
            chapter(1.0, "First", Some("en")),
            chapter(1.0, "Primero", Some("es")),
        ];
        let groups = organize_chapters(&chapters);
        assert_eq!(groups["Español"][0].number, 1.0);
        assert_eq!(groups["Español"][1].number, 2.0);
        assert_eq!(groups["Inglés"].len(), 1);
    }

    #[test]
    fn normalizes_number_and_preserves_descriptive_title() {
        let c = chapter(5.5, "Reunión lasciva", Some("es"));
        assert_eq!(chapter_label(&c), "Capítulo 5.5");
        assert_eq!(chapter_subtitle(&c).as_deref(), Some("Reunión lasciva"));

        let generic = chapter(5.0, "Capítulo 5", Some("es"));
        assert_eq!(chapter_label(&generic), "Capítulo 5");
        assert!(chapter_subtitle(&generic).is_none());
    }

    #[test]
    fn groups_futon_bcp47_locales_by_base_language() {
        assert_eq!(language_label(&chapter(1.0, "", Some("pt-BR"))), "Portugués");
        assert_eq!(language_label(&chapter(1.0, "", Some("es-419"))), "Español");
        assert_eq!(language_label(&chapter(1.0, "", Some("zh-Hans"))), "Chino");
        assert_eq!(language_label(&chapter(1.0, "", Some("sl"))), "Esloveno");
    }

    #[test]
    fn uses_futon_multilingual_branch_when_locale_is_missing() {
        let mut english = chapter(1.0, "First Mineral Collection", None);
        english.branch = Some("English".into());
        let mut ukrainian = chapter(1.0, "Перша колекція мінералів", None);
        ukrainian.branch = Some("Українська".into());

        assert_eq!(language_label(&english), "Inglés");
        assert_eq!(language_label(&ukrainian), "Ucraniano");
    }
}
