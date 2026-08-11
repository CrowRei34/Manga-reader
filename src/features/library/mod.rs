//! Pantalla de Biblioteca (Library). Réplica del diseño original:
//! - Título "Biblioteca" + ícono de filtro
//! - Fila de chips de categorías ("Todas" activa + categorías + "+ Nueva")
//! - Grid scrollable de cover cards
use iced::widget::{button, column, row, scrollable, text, Row};
use iced::{Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::db;
use crate::core::db::dao::{category_dao, manga_dao};
use crate::core::error::DbError;
use crate::core::models::Category;
use crate::theme::palette;
use crate::widgets::icon;

#[derive(Debug, Default)]
pub struct State {
    /// `None` = "Todas".
    pub category_filter: Option<i64>,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Load,
    CategoryFilter(Option<i64>),
    CategoriesLoaded(Result<Vec<Category>, DbError>),
}

pub fn update(state: &mut AppState, msg: Message) -> Task<AppMessage> {
    match msg {
        Message::Load => {
            let dbh = state.db.clone();
            if let Some(db) = dbh {
                let category = state.library_state.category_filter;
                Task::batch([
                    Task::perform(
                        db::db_blocking(db.clone(), move |conn| {
                            let mut all = manga_dao::list_library(conn)?;
                            if let Some(cat) = category {
                                all.retain(|m| {
                                    m.blob.get("category").and_then(|v| v.as_i64()) == Some(cat)
                                });
                            }
                            Ok(all)
                        }),
                        AppMessage::LibraryLoaded,
                    ),
                    Task::perform(
                        db::db_blocking(db, |conn| category_dao::list(conn)),
                        |r| AppMessage::Library(Message::CategoriesLoaded(r)),
                    ),
                ])
            } else {
                Task::none()
            }
        }
        Message::CategoryFilter(c) => {
            state.library_state.category_filter = c;
            update(state, Message::Load)
        }
        Message::CategoriesLoaded(Ok(cats)) => {
            state.library_state.categories = cats;
            Task::none()
        }
        Message::CategoriesLoaded(Err(e)) => {
            state.error = Some(e.to_string());
            Task::none()
        }
    }
}

/// Vista: título + chips + grid de portadas.
pub fn view(state: &AppState) -> Element<'_, AppMessage> {
    let title_row = row![
        text("Biblioteca").size(22).color(palette::TEXT),
        iced::widget::horizontal_space(),
        button(icon::glyph(icon::FILTER, 18, palette::TEXT_MUTED))
            .style(crate::theme::link_button)
            .padding(6),
    ]
    .align_y(iced::Alignment::Center);

    // Chips: "Todas" + categorías existentes + "+ Nueva".
    let mut chips: Vec<Element<'_, AppMessage>> = vec![
        button(text("Todas").size(13))
            .on_press(AppMessage::Library(Message::CategoryFilter(None)))
            .style(crate::theme::chip_button(state.library_state.category_filter.is_none()))
            .padding([6, 14])
            .into(),
    ];
    for cat in &state.library_state.categories {
        let selected = state.library_state.category_filter == cat.id;
        chips.push(
            button(text(cat.name.clone()).size(13))
                .on_press(AppMessage::Library(Message::CategoryFilter(cat.id)))
                .style(crate::theme::chip_button(selected))
                .padding([6, 14])
                .into(),
        );
    }
    chips.push(
        button(text("+ Nueva").size(13))
            .style(crate::theme::ghost_button)
            .padding([6, 14])
            .into(),
    );
    let chips_row = Row::with_children(chips).spacing(8);

    let grid = if state.library.is_empty() {
        column![
            text("Tu biblioteca está vacía").size(16).color(palette::TEXT_MUTED),
            text("Agrega mangas desde Explorar para verlos aquí.")
                .size(13)
                .color(palette::TEXT_DIM),
        ]
        .spacing(8)
    } else {
        column![crate::widgets::cover::cover_grid(
            &state.library,
            &state.covers,
            crate::widgets::cover::per_row_for(state.window_size.0),
            crate::widgets::cover::details_msg,
        )]
    };

    column![title_row, chips_row, scrollable(grid).height(Length::Fill)]
        .spacing(16)
        .into()
}
