//! Pantalla de Biblioteca (Library). Réplica del diseño original:
//! - Título "Biblioteca" + ícono de filtro
//! - Fila de chips de categorías ("Todas" activa + categorías + "+ Nueva")
//! - Grid scrollable de cover cards
use iced::widget::{button, column, container, row, scrollable, text, text_input, Row};
use iced::{Element, Length, Task};

use crate::app::{AppState, Message as AppMessage};
use bakeneko_core::db;
use bakeneko_core::db::dao::{category_dao, manga_dao};
use bakeneko_core::error::DbError;
use bakeneko_core::models::{Category, Manga};
use crate::theme::palette;

#[derive(Debug, Default)]
pub struct State {
    /// `None` = "Todas".
    pub category_filter: Option<i64>,
    pub categories: Vec<Category>,
    pub query: String,
    pub adult_filter: AdultFilter,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AdultFilter {
    #[default]
    All,
    Safe,
    Adult,
}

#[derive(Debug, Clone)]
pub enum Message {
    Load,
    CategoryFilter(Option<i64>),
    QueryChanged(String),
    AdultFilterChanged(AdultFilter),
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
                            match category {
                                Some(cat) => manga_dao::list_library_by_category(conn, cat),
                                None => manga_dao::list_library(conn),
                            }
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
        Message::QueryChanged(query) => {
            state.library_state.query = query;
            Task::none()
        }
        Message::AdultFilterChanged(filter) => {
            state.library_state.adult_filter = filter;
            Task::none()
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
    let search = text_input("Buscar en tu biblioteca…", &state.library_state.query)
        .on_input(|query| AppMessage::Library(Message::QueryChanged(query)))
        .style(crate::theme::search_input)
        .padding([8, 12])
        .width(if state.window_size.0 < 720.0 { Length::Fill } else { Length::Fixed(320.0) });
    let title_row: Element<'_, AppMessage> = if state.window_size.0 < 720.0 {
        column![text("Biblioteca").size(28).color(palette::TEXT), search]
            .spacing(10)
            .width(Length::Fill)
            .into()
    } else {
        row![
            text("Biblioteca").size(28).color(palette::TEXT),
            iced::widget::horizontal_space(),
            search,
        ]
        .align_y(iced::Alignment::Center)
        .into()
    };

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
    for (label, filter) in [
        ("Todo el contenido", AdultFilter::All),
        ("Apto", AdultFilter::Safe),
        ("+18", AdultFilter::Adult),
    ] {
        chips.push(
            button(text(label).size(13))
                .on_press(AppMessage::Library(Message::AdultFilterChanged(filter)))
                .style(crate::theme::chip_button(state.library_state.adult_filter == filter))
                .padding([6, 14])
                .into(),
        );
    }
    let chips_row = container(
        scrollable(
            Row::with_children(chips)
                .spacing(8)
                .padding([4, 0])
                .align_y(iced::Alignment::Center),
        )
            .style(crate::theme::scrollable_style)
            .direction(scrollable::Direction::Horizontal(Default::default()))
            .width(Length::Fill),
    )
    .style(crate::theme::panel_container)
    .padding([8, 10])
    .width(Length::Fill);

    let query = state.library_state.query.trim().to_lowercase();
    let filtered: Vec<Manga> = state.library.iter().filter(|manga| {
        let matches_query = query.is_empty()
            || manga.title.to_lowercase().contains(&query)
            || manga.authors.iter().any(|author| author.to_lowercase().contains(&query));
        let matches_rating = match state.library_state.adult_filter {
            AdultFilter::All => true,
            AdultFilter::Safe => !manga.is_nsfw,
            AdultFilter::Adult => manga.is_nsfw,
        };
        matches_query && matches_rating
    }).cloned().collect();

    let grid = if state.library.is_empty() {
        column![
            text("Tu biblioteca está vacía").size(16).color(palette::TEXT_MUTED),
            text("Agrega mangas desde Explorar para verlos aquí.")
                .size(13)
                .color(palette::TEXT_DIM),
        ]
        .spacing(8)
    } else if filtered.is_empty() {
        column![text("No hay obras que coincidan con estos filtros.")
            .size(14).color(palette::TEXT_MUTED)]
    } else {
        let (columns, cover_width) = crate::widgets::cover::grid_metrics(
            state.window_size.0, &state.settings.library_view,
        );
        column![crate::widgets::cover::cover_grid_sized(
            &filtered,
            &state.covers,
            columns,
            cover_width,
            crate::widgets::cover::details_msg,
        )]
    };

    let header = container(column![title_row, chips_row].spacing(16))
        .style(crate::theme::content_container)
        .padding(iced::Padding { top: 0.0, bottom: 8.0, left: 0.0, right: 0.0 });

    let grid = container(grid)
        .style(crate::theme::empty_state)
        .padding(16)
        .width(Length::Fill);

    column![
        header,
        container(scrollable(grid).style(crate::theme::scrollable_style).width(Length::Fill).height(Length::Fill)).clip(true).height(Length::Fill)
    ]
    .spacing(8)
    .into()
}
