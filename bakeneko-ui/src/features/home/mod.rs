//! Pantalla de Inicio (Home). Réplica del diseño original:
//! - Título "Inicio"
//! - Sección "Continuar leyendo" (historial) con scroll horizontal de covers
//!   + subtítulo "Cap. N"
//! - Sección "Añadidos recientemente" con grid de covers de la biblioteca
use iced::widget::{column, container, row, scrollable, text, Row};
use iced::{Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::db;
use bakeneko_core::db::dao::{history_dao, manga_dao};
use bakeneko_core::error::DbError;
use bakeneko_core::models::Manga;
use crate::theme::palette;
use crate::widgets::cover::cover_card;

#[derive(Debug, Default)]
pub struct State {
    /// (manga, chapter_index) del historial, ordenado por `updated_at DESC`.
    pub recent: Vec<(Manga, i32)>,
    /// Mangas añadidos recientemente a la biblioteca (`added_at DESC`).
    pub recently_added: Vec<Manga>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadRecent,
    RecentLoaded(Result<(Vec<(Manga, i32)>, Vec<Manga>), DbError>),
}

pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::LoadRecent => {
            let dbh = state.db.clone();
            if let Some(db) = dbh {
                Task::perform(
                    db::db_blocking(db, |conn| {
                        let ids = history_dao::recent(conn, 10)?;
                        let mut recent = Vec::with_capacity(ids.len());
                        for (id, chapter_index, _, _) in ids {
                            if let Some(m) = manga_dao::get_by_id(conn, id)? {
                                recent.push((m, chapter_index));
                            }
                        }
                        let recently_added = manga_dao::list_recently_added(conn, 12)?;
                        Ok((recent, recently_added))
                    }),
                    |r| AppMessage::Home(Message::RecentLoaded(r)),
                )
            } else {
                Task::none()
            }
        }
        Message::RecentLoaded(Ok((recent, recently_added))) => {
            state.home.recent = recent;
            state.home.recently_added = recently_added;
            // Trae las portadas del historial y de los añadidos recientemente.
            let mut mangas: Vec<Manga> = state.home.recent.iter().map(|(m, _)| m.clone()).collect();
            mangas.extend(state.home.recently_added.iter().cloned());
            crate::widgets::cover::fetch_covers(state, &mangas)
        }
        Message::RecentLoaded(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
    }
}

/// Cabecera de sección: label muted + línea divisoria que ocupa el resto.
fn section_header<'a>(label: &str) -> Element<'a, AppMessage> {
    row![
        container(text(""))
            .style(crate::theme::accent_rule)
            .width(Length::Fixed(22.0))
            .height(Length::Fixed(3.0)),
        text(label.to_string()).size(14).color(palette::TEXT_MUTED),
        container(text(""))
            .style(crate::theme::divider)
            .height(Length::Fixed(1.0))
            .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(iced::Alignment::Center)
    .into()
}

pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let title = container(
        row![
            container(text(""))
                .style(crate::theme::accent_rule)
                .width(Length::Fixed(4.0))
                .height(Length::Fixed(28.0)),
            text("Inicio").size(28).color(palette::TEXT),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .style(crate::theme::panel_container)
    .padding([16, 18])
    .width(Length::Fill);

    // Continuar leyendo: scroll horizontal de covers con "Cap. N".
    let recent_cards: Vec<Element<'_, AppMessage>> = state
        .home
        .recent
        .iter()
        .map(|(m, cidx)| {
            cover_card(
                m,
                &state.covers,
                Some(format!("Cap. {cidx}")),
                crate::widgets::cover::details_msg(m),
            )
        })
        .collect();

    let recent_content: Element<'_, AppMessage> = if recent_cards.is_empty() {
        container(
            text("No hay lecturas recientes en el historial.")
                .size(13)
                .color(palette::TEXT_MUTED),
        )
        .padding([8, 10])
        .into()
    } else {
        scrollable(
            Row::with_children(recent_cards)
                .spacing(16)
                .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 7.0, left: 0.0 }),
        )
        .style(crate::theme::scrollable_style)
        .width(Length::Fill)
        .height(Length::Shrink)
        .direction(scrollable::Direction::Horizontal(Default::default()))
        .into()
    };

    let recent_row = container(recent_content)
        .style(crate::theme::panel_container)
        .padding([12, 14])
        .width(Length::Fill)
        .clip(true);

    // Añadidos recientemente: grid responsivo de la biblioteca (hasta 12 obras ordenadas por fecha).
    let (columns, cover_width) = crate::widgets::cover::grid_metrics(
        state.window_size.0, &state.settings.library_view,
    );
    let grid_content: Element<'_, AppMessage> = if state.home.recently_added.is_empty() {
        container(
            text("No hay mangas en tu biblioteca todavía.")
                .size(13)
                .color(palette::TEXT_MUTED),
        )
        .padding([8, 10])
        .into()
    } else {
        crate::widgets::cover::cover_grid_sized(
            &state.home.recently_added,
            &state.covers,
            columns,
            cover_width,
            crate::widgets::cover::details_msg,
        )
    };

    let body = column![
        title,
        section_header("Continuar leyendo"),
        recent_row,
        section_header("Añadidos recientemente"),
        grid_content,
    ]
    .spacing(20)
    .width(Length::Fill);

    scrollable(body).style(crate::theme::scrollable_style).width(Length::Fill).height(Length::Fill).into()
}
