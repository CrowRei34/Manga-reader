//! Pantalla de Exploración (Browse). Stub mínimo del Task 11: el campo
//! `list` ya existe para recibir `Message::CatalogListed(Ok(...))` del
//! reducer global y que el Task 16 rellene la grilla.
use iced::widget::{center, text};
use iced::{Element, Task};

use crate::app::{AppState, Message as AppMessage};
use crate::core::models::Manga;

#[derive(Debug, Default)]
pub struct State {
    pub list: Vec<Manga>,
}

#[derive(Debug, Clone)]
pub enum Message {}

pub fn update(_state: &mut AppState, _msg: Message) -> Task<AppMessage> {
    Task::none()
}

pub fn view(_state: &AppState) -> Element<'_, AppMessage> {
    center(text("Explorar")).into()
}