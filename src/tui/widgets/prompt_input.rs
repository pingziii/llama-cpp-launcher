use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[allow(dead_code)]
#[derive(Default)]
pub struct PromptInputWidget {
    pub buffer: String,
    pub cursor_position: usize,
}

#[allow(dead_code)]
impl PromptInputWidget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor_position = 0;
    }

    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 && !self.buffer.is_empty() {
            self.cursor_position -= 1;
            self.buffer.remove(self.cursor_position);
        }
    }

    pub fn cursor_left(&mut self) {
        self.cursor_position = self.cursor_position.saturating_sub(1);
    }

    pub fn cursor_right(&mut self) {
        if self.cursor_position < self.buffer.len() {
            self.cursor_position += 1;
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let display_text = if self.buffer.is_empty() {
            " Type a message and press Enter... ".to_string()
        } else {
            self.buffer.clone()
        };

        let paragraph = Paragraph::new(display_text)
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title(" Prompt "));

        f.render_widget(paragraph, area);

        // Set cursor position
        f.set_cursor_position((area.x + 1 + self.cursor_position as u16, area.y + 1));
    }
}
