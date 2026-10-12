//! Capa de entrada: la interfaz de terminal.

mod action;
mod app;
pub mod appearance;
pub mod glyphs;
mod list;
mod prompt;
mod state;
mod views;

pub use app::{App, Backend, Services};
