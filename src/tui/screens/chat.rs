use std::sync::{Arc, Mutex};

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::{App, Screen, ScreenAction};

pub struct ChatScreen {
    host: String,
    port: u16,
    model_name: String,
    model_path: String,
    show_logs: bool,
    log_buffer: Arc<Mutex<Vec<String>>>,
    log_scroll: usize,
    server_healthy: Arc<Mutex<Option<bool>>>,
}

impl ChatScreen {
    pub fn new(app: &mut App, server_healthy: Arc<Mutex<Option<bool>>>) -> Self {
        // Use the actual launch params (user-adjusted), fall back to config
        let (host, port, model_name) = app
            .launch_params
            .as_ref()
            .map(|p| (p.host.clone(), p.port, p.model_name.clone()))
            .unwrap_or_else(|| {
                (
                    app.config.host.clone(),
                    app.config.port,
                    "unknown".to_string(),
                )
            });
        let model_path = app
            .launch_params
            .as_ref()
            .map(|p| p.model_path.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let log_buffer = app
            .process
            .as_ref()
            .map(|p| p.log_buffer.clone())
            .unwrap_or_default();

        ChatScreen {
            host,
            port,
            model_name,
            model_path,
            show_logs: false,
            log_buffer,
            log_scroll: 0,
            server_healthy,
        }
    }
}

impl Screen for ChatScreen {
    fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area);

        // Determine health status for title bar
        let health_status = *self.server_healthy.lock().unwrap();
        let (title_text, title_color) = if self.show_logs {
            (" Server Logs ", Color::Yellow)
        } else {
            match health_status {
                Some(true) => (" Model Server Running ● ", Color::Green),
                Some(false) => (" Server Unresponsive ✕ ", Color::Red),
                None => (" Model Server Running ", Color::Green),
            }
        };
        let title = Block::default()
            .title(title_text)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(title_color));
        f.render_widget(title, chunks[0]);

        if self.show_logs {
            // ── Log view with scroll ──
            let lines: Vec<String> = self.log_buffer.lock().unwrap().clone();
            let max_lines = (chunks[1].height as usize).saturating_sub(2);
            let total_lines = lines.len();

            // Clamp scroll so we can't scroll past the beginning
            let max_scroll = total_lines.saturating_sub(max_lines);
            if self.log_scroll > max_scroll {
                self.log_scroll = max_scroll;
            }

            // Slice from (total - max_lines - scroll) to (total - scroll)
            let start = total_lines.saturating_sub(max_lines + self.log_scroll);
            let end = total_lines.saturating_sub(self.log_scroll);
            let visible: Vec<&str> = lines[start..end].iter().map(|s| s.as_str()).collect();
            let log_text = visible.join("\n");

            let scroll_hint = if self.log_scroll > 0 {
                format!(" — ↑{} lines", self.log_scroll)
            } else {
                String::new()
            };
            let block = Block::default().borders(Borders::ALL).title(format!(
                " Console Output ({} lines{}) ",
                total_lines, scroll_hint
            ));
            let paragraph = Paragraph::new(log_text)
                .block(block)
                .style(Style::default().fg(Color::White))
                .wrap(Wrap { trim: false });
            f.render_widget(paragraph, chunks[1]);
        } else {
            // ── Info view ──
            let api_url = format!("http://{}:{}/v1/chat/completions", self.host, self.port);
            let health_url = format!("http://{}:{}/health", self.host, self.port);

            let health_indicator = match health_status {
                Some(true) => Span::styled(" ● Healthy ", Style::default().fg(Color::Green)),
                Some(false) => Span::styled(" ✕ Unreachable ", Style::default().fg(Color::Red)),
                None => Span::styled(" ? Checking... ", Style::default().fg(Color::Yellow)),
            };

            let mut lines = vec![
                Line::from(Span::styled(
                    " Server Status:",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(health_indicator),
                Line::from(""),
                Line::from(Span::styled(
                    " Model Name:",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(format!("   {}", self.model_name)),
                Line::from(""),
                Line::from(Span::styled(
                    " Model File:",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(format!("   {}", self.model_path)),
                Line::from(""),
                Line::from(Span::styled(
                    " API Endpoint:",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(Span::styled(
                    format!("   {}", api_url),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    " Health Check:",
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(Span::styled(
                    format!("   {}", health_url),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("   Host: {}    Port: {}", self.host, self.port),
                    Style::default().fg(Color::White),
                )),
            ];

            // Show crash warning when server is unreachable
            if health_status == Some(false) {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    " ⚠ Server has stopped responding. Press Esc to return to launch screen. ",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                )));
            }

            let text = Text::from(lines);
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Server Info ");
            let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
            f.render_widget(paragraph, chunks[1]);
        }

        // Footer
        let footer_text = if self.show_logs {
            " ↑↓ scroll  |  'l': info view  |  's': stop server  |  'q': quit "
        } else {
            " 'l': console logs | 's': stop server | 'q': quit | Esc: go back "
        };
        let footer = Paragraph::new(Line::from(Span::styled(
            footer_text,
            Style::default().fg(Color::Green),
        )))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenAction {
        match key.code {
            crossterm::event::KeyCode::Char('l') => {
                self.show_logs = !self.show_logs;
                self.log_scroll = 0; // reset to bottom when toggling
                ScreenAction::None
            }
            crossterm::event::KeyCode::Up => {
                if self.show_logs {
                    self.log_scroll += 1;
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Down => {
                if self.show_logs && self.log_scroll > 0 {
                    self.log_scroll -= 1;
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::PageUp => {
                if self.show_logs {
                    self.log_scroll += 10;
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::PageDown => {
                if self.show_logs {
                    let pagedown = self.log_scroll.saturating_sub(10);
                    // Reduce by at most 10, but never go below 0 (handled by clamp in render)
                    self.log_scroll = pagedown;
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Char('s') => ScreenAction::StopServer,
            crossterm::event::KeyCode::Char('q') => ScreenAction::Exit,
            crossterm::event::KeyCode::Esc => ScreenAction::StopServer,
            _ => ScreenAction::None,
        }
    }
}
