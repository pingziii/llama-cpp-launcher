use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

#[allow(dead_code)]
pub struct ModelListWidget {
    pub items: Vec<String>,
    pub state: ListState,
}

#[allow(dead_code)]
impl ModelListWidget {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }

    pub fn render(&mut self) -> List<'_> {
        let list_items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| ListItem::new(item.clone()))
            .collect();

        List::new(list_items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ")
    }

    pub fn next(&mut self) {
        let i = self.state.selected().unwrap_or(0);
        if i + 1 < self.items.len() {
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
