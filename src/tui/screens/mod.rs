mod chat;
mod config;
mod hardware;
mod launch;
mod model_select;

pub use chat::ChatScreen;
pub use config::ConfigScreen;
pub use hardware::HardwareScreen;
pub use launch::LaunchScreen;
pub use model_select::ModelSelectScreen;

use ratatui::Frame;

use crate::params::LaunchParams;

use super::app::{App, AppScreen};

/// Actions returned from screen key handlers.
pub enum ScreenAction {
    None,
    Exit,
    Navigate(AppScreen),
    Launch(LaunchParams),
    SelectModel(String),
    StopServer,
}

/// Trait implemented by each screen.
pub trait Screen {
    fn render(&mut self, f: &mut Frame);
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenAction;
}
