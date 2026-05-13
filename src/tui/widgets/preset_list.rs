use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::config::Preset;

/// A selectable list of saved presets for the launch screen.
#[allow(dead_code)]
pub struct PresetListWidget {
    presets: Vec<Preset>,
    pub state: ListState,
}

#[allow(dead_code)]
impl PresetListWidget {
    pub fn new(presets: Vec<Preset>) -> Self {
        let mut state = ListState::default();
        if !presets.is_empty() {
            state.select(Some(0));
        }
        Self { presets, state }
    }

    pub fn render(&mut self) -> List<'_> {
        let items: Vec<ListItem> = self
            .presets
            .iter()
            .map(|p| {
                let desc = p.description.as_deref().unwrap_or("");
                ListItem::new(format!(" {}  -  {}", p.name, desc))
            })
            .collect();

        List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Presets "))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ")
    }

    pub fn selected_preset(&self) -> Option<&Preset> {
        self.state.selected().and_then(|i| self.presets.get(i))
    }

    pub fn next(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        if i + 1 < self.presets.len() {
            self.state.select(Some(i + 1));
        }
    }

    pub fn previous(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        if i > 0 {
            self.state.select(Some(i - 1));
        }
    }
}
