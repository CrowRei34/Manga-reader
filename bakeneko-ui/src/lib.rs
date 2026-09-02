// Librería bakeneko: expone los módulos públicos para tests de integración
// (tests/*.rs) y para el binario.
pub use bakeneko_core as core;

pub mod app;
pub mod discord_presence;
pub mod features;
pub mod language;
pub mod theme;
pub mod widgets;
