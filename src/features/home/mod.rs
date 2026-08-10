//! Pantalla de Inicio (Home). Stub mínimo del Task 11; las siguientes
//! tareas rellenan catálogo reciente, últimos leídos, etc.
use iced::widget::{center, text};
use iced::{Element, Task};

use crate::app::{AppState, Message as AppMessage};

#[derive(Debug, Default)]
pub struct State;

#[derive(Debug, Clone)]
pub enum Message {}

pub fn update(_state: &mut AppState, _msg: Message) -> Task<AppMessage> {
    Task::none()
}

pub fn view(_state: &AppState) -> Element<'_, AppMessage> {
    center(text("Inicio")).into()
}