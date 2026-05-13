use crate::hardware::HardwareProfile;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::{App, AppScreen, Screen, ScreenAction};

pub struct HardwareScreen {
    hardware: Option<HardwareProfile>,
    model_dir: String,
    config_path: String,
    done: bool,
}

impl HardwareScreen {
    pub fn new(app: &mut App) -> Self {
        let hardware = crate::hardware::detect_hardware();
        app.hardware = Some(hardware.clone());

        let config_path = crate::config::config_file_path().display().to_string();
        let model_dir = app.config.model_dir.clone();

        HardwareScreen {
            hardware: Some(hardware),
            model_dir,
            config_path,
            done: false,
        }
    }
}

impl Screen for HardwareScreen {
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

        // Title
        let title = Block::default()
            .title(" TUI LLM Launcher ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        f.render_widget(title, chunks[0]);

        // Hardware info
        if let Some(hw) = &self.hardware {
            let gpu_info = match &hw.gpu_name {
                Some(name) => {
                    let vram = hw
                        .gpu_vram_bytes
                        .map(|b| format!("{:.1} GB", b as f64 / 1e9))
                        .unwrap_or_else(|| "N/A".to_string());
                    format!("GPU: {name} ({vram} VRAM)")
                }
                None => "GPU: None detected (CPU-only mode)".to_string(),
            };

            let lines = vec![
                Line::from(Span::styled(
                    " Hardware Detection Complete",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(format!(" CPU Cores: {}", hw.cpu_cores)),
                Line::from(format!(
                    " RAM: {:.1} GB total",
                    hw.total_ram_bytes as f64 / 1e9
                )),
                Line::from(gpu_info),
                Line::from(""),
                Line::from(Span::styled(
                    " Configuration",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(format!(" Config path: {}", self.config_path)),
                Line::from(format!(" Model directory: {}", self.model_dir)),
            ];

            let text = Text::from(lines);
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" System Info ");
            let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
            f.render_widget(paragraph, chunks[1]);
        } else {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Detecting Hardware... ");
            f.render_widget(block, chunks[1]);
        }

        // Footer
        let footer_text = if self.done {
            " Press any key to continue... "
        } else {
            " Detecting hardware... "
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
            crossterm::event::KeyCode::Char('q') => ScreenAction::Exit,
            _ => {
                self.done = true;
                ScreenAction::Navigate(AppScreen::ModelSelect)
            }
        }
    }
}
