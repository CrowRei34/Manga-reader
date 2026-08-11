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

use crate::app::{AppState, Message as AppMessage};
use crate::core::daemon::api::MangaSourceApi;
use crate::core::db;
use crate::core::db::dao::manga_dao;
use crate::core::error::{DaemonError, DbError};
use crate::core::models::{Chapter, Manga, MangaRef};
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
}

#[derive(Debug, Clone)]
pub enum Message {
    Load(MangaRef),
    Fetched(Result<Manga, DaemonError>),
    ChapterSelected(Chapter),
    AddToLibrary,
    ReadNow,
    DownloadChapter(Chapter),
    DownloadAll,
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
            let idx = state.details.chapters.iter().position(|x| x.url == c.url).unwrap_or(0);
            state.reader.chapters = state.details.chapters.clone();
            state.reader.current_chapter = idx;
            // Modo de lectura por defecto desde settings (TODO: mapear).
            Task::batch([
                Task::done(AppMessage::Reader(reader::Message::Load(c))),
                Task::done(AppMessage::NavigateTo(Screen::Reader)),
            ])
        }
        Message::ReadNow => {
            // Abre el primer capítulo (menor número).
            let first = state
                .details
                .chapters
                .iter()
                .min_by(|a, b| a.number.partial_cmp(&b.number).unwrap_or(std::cmp::Ordering::Equal))
                .cloned();
            match first {
                Some(c) => update(state, Message::ChapterSelected(c)),
                None => Task::none(),
            }
        }
        Message::AddToLibrary => {
            let m_opt = state.details.manga.clone();
            let dbh = state.db.clone();
            if let (Some(m), Some(db)) = (m_opt, dbh) {
                return Task::perform(
                    db::db_blocking(db, move |conn| {
                        let id = manga_dao::upsert(conn, &m, 0)?;
                        manga_dao::set_library_flag(conn, id, true)?;
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
        Message::Back => {
            let target = state.details.back_target.clone().unwrap_or(Screen::Home);
            Task::done(AppMessage::NavigateTo(target))
        }
    }
}

/// Vista del feature (réplica del diseño).
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
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

    // Cover grande (2× la card).
    let cover: Element<'_, AppMessage> = match m
        .cover_url
        .as_ref()
        .and_then(|u| state.covers.get(u))
    {
        Some(path) => image(image::Handle::from_path(path.clone()))
            .width(Length::Fixed(COVER_W * 1.4))
            .height(Length::Fixed(COVER_H * 1.4))
            .content_fit(ContentFit::Cover)
            .into(),
        None => container(icon::glyph(icon::IMAGE, 56, palette::TEXT_DIM))
            .width(Length::Fixed(COVER_W * 1.4))
            .height(Length::Fixed(COVER_H * 1.4))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(crate::theme::card_container)
            .into(),
    };

    let title_block = column![
        text(m.title.clone()).size(26).color(palette::TEXT),
        text(m.authors.first().cloned().unwrap_or_default())
            .size(14)
            .color(palette::TEXT_MUTED),
        text("Sinopsis").size(15).color(palette::ACCENT),
        scrollable(
            text(m.description.clone().unwrap_or_else(|| "Sin descripción".into()))
                .size(13)
                .color(palette::TEXT_MUTED),
        )
        .height(Length::Fixed(140.0)),
    ]
    .spacing(8);

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
                icon::glyph(icon::BOOKMARK, 16, palette::ACCENT),
                text("En Biblioteca").size(14).color(palette::ACCENT),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(AppMessage::Details(Message::AddToLibrary))
        .style(crate::theme::ghost_button)
        .padding([10, 18]),
    ]
    .spacing(12);

    let header_row = row![cover, column![title_block, buttons_row].spacing(12)].spacing(20);

    // Lista de capítulos ordenada ascendente.
    let mut chapters = state.details.chapters.clone();
    chapters.sort_by(|a, b| a.number.partial_cmp(&b.number).unwrap_or(std::cmp::Ordering::Equal));
    let chapter_rows: Vec<Element<'_, AppMessage>> = chapters
        .iter()
        .map(|c| {
            let status_icon = if c.read {
                icon::glyph(icon::CHECK, 18, palette::SUCCESS)
            } else {
                icon::glyph(icon::DOWNLOAD_FOR_OFFLINE, 18, palette::TEXT_MUTED)
            };
            row![
                text(c.title.clone()).size(14).color(palette::TEXT).width(Length::Fill),
                button(status_icon)
                    .on_press(AppMessage::Details(Message::DownloadChapter(c.clone())))
                    .style(crate::theme::link_button)
                    .padding(4),
                button(text("Ver").size(13).color(palette::TEXT_MUTED))
                    .on_press(AppMessage::Details(Message::ChapterSelected(c.clone())))
                    .style(crate::theme::link_button)
                    .padding(4),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
        })
        .collect();

    let chapters_header = row![
        text(format!("Capítulos ({})", chapters.len()))
            .size(18)
            .color(palette::TEXT),
        iced::widget::horizontal_space(),
        button(
            row![
                icon::glyph(icon::DOWNLOAD, 16, palette::ACCENT),
                text("Descargar Todo").size(13).color(palette::ACCENT),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center),
        )
        .on_press(AppMessage::Details(Message::DownloadAll))
        .style(crate::theme::link_button_accent)
        .padding(4),
    ]
    .align_y(iced::Alignment::Center);

    column![
        back,
        header_row,
        chapters_header,
        scrollable(Column::with_children(chapter_rows).spacing(2)),
    ]
    .spacing(16)
    .into()
}
