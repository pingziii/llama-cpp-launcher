use ratatui::{
    layout::Alignment,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Real-time performance metrics display widget.
#[allow(dead_code)]
pub struct MetricsWidget {
    pub tokens_per_sec: f64,
    pub context_usage: u32,
    pub uptime_secs: u64,
}

impl Default for MetricsWidget {
    fn default() -> Self {
        Self {
            tokens_per_sec: 0.0,
            context_usage: 0,
            uptime_secs: 0,
        }
    }
}

#[allow(dead_code)]
impl MetricsWidget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&self) -> Paragraph<'_> {
        let text = Line::from(vec![
            Span::styled(
                format!(" {:.1} t/s ", self.tokens_per_sec),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" ctx: {} ", self.context_usage),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(" uptime: {}s ", self.uptime_secs),
                Style::default().fg(Color::Green),
            ),
        ]);

        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Metrics "))
    }
}
