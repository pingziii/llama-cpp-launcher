use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Renders a scrollable chat conversation history.
#[allow(dead_code)]
#[derive(Default)]
pub struct ChatViewWidget {
    pub messages: Vec<String>,
    pub scroll_offset: usize,
}

#[allow(dead_code)]
impl ChatViewWidget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        let formatted = format!("{}: {}", role, content);
        self.messages.push(formatted);
        self.scroll_to_bottom();
    }

    pub fn render(&self) -> Paragraph<'_> {
        let lines: Vec<Line> = self
            .messages
            .iter()
            .map(|msg| {
                if msg.starts_with(">") {
                    Line::from(Span::styled(msg, Style::default().fg(Color::Cyan)))
                } else {
                    Line::from(Span::styled(msg, Style::default().fg(Color::White)))
                }
            })
            .collect();

        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Conversation "),
            )
            .wrap(Wrap { trim: false })
    }

    fn scroll_to_bottom(&mut self) {
        if !self.messages.is_empty() {
            self.scroll_offset = self.messages.len().saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_view_new_empty() {
        let view = ChatViewWidget::new();
        assert!(view.messages.is_empty());
        assert_eq!(view.scroll_offset, 0);
    }

    #[test]
    fn test_chat_view_add_message() {
        let mut view = ChatViewWidget::new();
        view.add_message("user", "Hello");
        assert_eq!(view.messages.len(), 1);
        assert_eq!(view.messages[0], "user: Hello");
    }

    #[test]
    fn test_chat_view_auto_scroll_on_add() {
        let mut view = ChatViewWidget::new();
        // Empty: scroll_offset should be 0
        assert_eq!(view.scroll_offset, 0);

        view.add_message("user", "msg 1");
        assert_eq!(view.scroll_offset, 0);

        view.add_message("assistant", "msg 2");
        assert_eq!(view.scroll_offset, 1);

        view.add_message("user", "msg 3");
        assert_eq!(view.scroll_offset, 2);
    }

    #[test]
    fn test_chat_view_multiple_messages() {
        let mut view = ChatViewWidget::new();
        view.add_message("user", "Hello");
        view.add_message("assistant", "Hi there!");
        view.add_message("user", "How are you?");
        view.add_message("assistant", "I'm doing great, thanks!");

        assert_eq!(view.messages.len(), 4);
        assert_eq!(view.messages[0], "user: Hello");
        assert_eq!(view.messages[1], "assistant: Hi there!");
        assert_eq!(view.messages[2], "user: How are you?");
        assert_eq!(view.messages[3], "assistant: I'm doing great, thanks!");
    }

    #[test]
    fn test_chat_view_render_does_not_panic() {
        let mut view = ChatViewWidget::new();
        view.add_message("user", "Hello");
        // Render should complete without panic
        let _ = view.render();
    }

    #[test]
    fn test_chat_view_styles_user_messages() {
        let mut view = ChatViewWidget::new();
        view.add_message(">", "Hello");
        // The formatted message starts with ">:"
        assert!(view.messages[0].starts_with(">:"));
    }
}
