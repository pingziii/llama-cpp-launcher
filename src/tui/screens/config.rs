use super::{App, AppScreen, Screen, ScreenAction};
use crate::config::Config;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub struct ConfigScreen {
    config: Config,
    selected_field: usize,
    edit_mode: bool,
    edit_buffer: String,
    saved: bool,
    message: Option<String>,
}

impl ConfigScreen {
    pub fn new(app: &mut App) -> Self {
        let config = app.config.clone();

        ConfigScreen {
            config,
            selected_field: 0,
            edit_mode: false,
            edit_buffer: String::new(),
            saved: false,
            message: None,
        }
    }

    fn toggle_field(&mut self) {
        const FIELD_COUNT: usize = 3;
        self.selected_field = (self.selected_field + 1) % FIELD_COUNT;
    }

    fn start_edit(&mut self) {
        self.edit_buffer = match self.selected_field {
            0 => self.config.model_dir.clone(),
            1 => self.config.llama_bin_path.clone().unwrap_or_default(),
            2 => String::new(), // debug toggle handled directly
            _ => return,
        };
        self.edit_mode = true;
    }

    fn save_field(&mut self) {
        match self.selected_field {
            0 => {
                let val = self.edit_buffer.trim().to_string();
                if !val.is_empty() {
                    self.config.model_dir = val;
                }
            }
            1 => {
                let val = self.edit_buffer.trim().to_string();
                self.config.llama_bin_path = if val.is_empty() { None } else { Some(val) };
            }
            _ => {}
        }
        self.edit_mode = false;
        self.edit_buffer.clear();
    }

    fn save_config_to_disk(&mut self) -> bool {
        match crate::config::save_config(&self.config) {
            Ok(()) => {
                self.message = Some("Configuration saved successfully".to_string());
                self.saved = true;
                true
            }
            Err(e) => {
                self.message = Some(format!("Save failed: {e}"));
                false
            }
        }
    }

    fn toggle_debug(&mut self) {
        self.config.debug_logging = !self.config.debug_logging;
    }
}

impl Screen for ConfigScreen {
    fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(5),
            ])
            .split(area);

        // Title
        let title = Block::default()
            .title(" Configuration Editor ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        f.render_widget(title, chunks[0]);

        // Field definitions
        let fields: Vec<(&str, String)> = vec![
            ("Model Directory", self.config.model_dir.clone()),
            (
                "llama.cpp Binary",
                self.config
                    .llama_bin_path
                    .clone()
                    .unwrap_or_else(|| "(search PATH)".to_string()),
            ),
            (
                "Debug Logging",
                if self.config.debug_logging {
                    "enabled"
                } else {
                    "disabled"
                }
                .to_string(),
            ),
        ];

        let mut lines: Vec<Line> = Vec::new();

        if self.edit_mode {
            let label = &fields[self.selected_field].0;
            lines.push(Line::from(Span::styled(
                format!(" Editing: {label}"),
                Style::default().fg(Color::Yellow),
            )));
            lines.push(Line::from(""));
            let display = if self.edit_buffer.is_empty() {
                "(empty - will clear)".to_string()
            } else {
                self.edit_buffer.clone()
            };
            lines.push(Line::from(Span::styled(
                format!(" > {display}"),
                Style::default().fg(Color::Green),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Press Enter to confirm, Esc to cancel",
                Style::default().fg(Color::DarkGray),
            )));
        } else {
            for (i, (label, value)) in fields.iter().enumerate() {
                let is_selected = i == self.selected_field;

                if let Some(msg) = &self.message {
                    lines.push(Line::from(Span::styled(
                        format!(" {msg}"),
                        Style::default().fg(Color::Green),
                    )));
                    lines.push(Line::from(""));
                }

                let prefix = if is_selected { " > " } else { "   " };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                lines.push(Line::from(vec![
                    Span::styled(format!("{prefix}{label}: "), style),
                    Span::styled(value, Style::default().fg(Color::Cyan)),
                ]));
            }

            lines.push(Line::from(""));
            let help = Line::from(Span::styled(
                " Tab: next field | Enter: edit | s: save | Esc: back (auto-save) | q: quit",
                Style::default().fg(Color::DarkGray),
            ));
            lines.push(help);
        }

        let block = Block::default().borders(Borders::ALL).title(" Settings ");
        let paragraph = Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, chunks[1]);

        // Footer
        let footer_text = " Use Tab/Enter to edit fields | Ctrl+S to save to disk ";
        let footer = Paragraph::new(Line::from(Span::styled(
            footer_text,
            Style::default().fg(Color::Green),
        )))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }

    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenAction {
        if self.edit_mode {
            match key.code {
                crossterm::event::KeyCode::Enter => {
                    self.save_field();
                }
                crossterm::event::KeyCode::Esc => {
                    self.edit_mode = false;
                    self.edit_buffer.clear();
                }
                crossterm::event::KeyCode::Backspace => {
                    self.edit_buffer.pop();
                }
                crossterm::event::KeyCode::Char(c) => {
                    self.edit_buffer.push(c);
                }
                _ => {}
            }
            return ScreenAction::None;
        }

        match key.code {
            crossterm::event::KeyCode::Tab => {
                self.toggle_field();
            }
            crossterm::event::KeyCode::Char('s') => {
                self.save_config_to_disk();
            }
            crossterm::event::KeyCode::Enter => {
                if self.selected_field == 2 {
                    self.toggle_debug();
                } else {
                    self.start_edit();
                }
            }
            crossterm::event::KeyCode::Char('r') => {
                self.save_config_to_disk();
            }
            crossterm::event::KeyCode::Esc => {
                return ScreenAction::Navigate(AppScreen::ModelSelect);
            }
            crossterm::event::KeyCode::Char('q') => {
                return ScreenAction::Exit;
            }
            _ => {}
        }
        ScreenAction::None
    }
}
