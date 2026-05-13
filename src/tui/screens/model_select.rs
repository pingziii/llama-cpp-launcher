use std::path::Path;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{App, AppScreen, Screen, ScreenAction};

struct ModelEntry {
    path: String,
    name: String, // filename without .gguf (for display)
    size_mb: u64,
    parameter_count: Option<u64>,
}

struct ModelGroup {
    provider: String, // first path component (e.g., "Qwen", "Meta")
    models: Vec<ModelEntry>,
}

pub struct ModelSelectScreen {
    groups: Vec<ModelGroup>,
    group_index: usize,
    item_list_state: ListState, // index within current group
}

/// Recursively scan a directory for .gguf files.
fn scan_gguf_files(dir: &Path) -> Vec<ModelEntry> {
    let mut models = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            models.extend(scan_gguf_files(&path));
        } else if path.extension().map(|e| e == "gguf").unwrap_or(false) {
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Skip mmproj files (vision projection models, not standalone LLMs)
            if file_name.to_lowercase().starts_with("mmproj-") {
                continue;
            }

            let display_name = file_name
                .strip_suffix(".gguf")
                .unwrap_or(&file_name)
                .to_string();

            let size_mb = std::fs::metadata(&path)
                .map(|m| m.len() / (1024 * 1024))
                .unwrap_or(0);
            let params = parse_parameters(&file_name);

            models.push(ModelEntry {
                path: path.to_string_lossy().to_string(),
                name: display_name,
                size_mb,
                parameter_count: params,
            });
        }
    }
    models
}

/// Build grouped model list from a flat scan result.
fn build_groups(models: Vec<ModelEntry>, base_dir: &Path) -> Vec<ModelGroup> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<String, Vec<ModelEntry>> = BTreeMap::new();
    for m in models {
        // Compute relative path from base_dir to determine provider
        let relative = Path::new(&m.path)
            .strip_prefix(base_dir)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("unknown");

        let provider = if let Some(idx) = relative.find(['/', '\\']) {
            relative[..idx].to_string()
        } else {
            "root".to_string()
        };

        grouped.entry(provider).or_default().push(m);
    }

    grouped
        .into_iter()
        .map(|(provider, models)| ModelGroup {
            provider: if provider == "root" {
                "Uncategorized".to_string()
            } else {
                provider
            },
            models,
        })
        .collect()
}

/// Parse model parameters from filename (e.g., "7B", "13B", "70B").
fn parse_parameters(name: &str) -> Option<u64> {
    // Look for patterns like 7B, 13B, 70B
    let upper = name.to_uppercase();
    // Try to find "NB" where N is digits
    for (i, c) in upper.char_indices() {
        if c == 'B' && i > 0 {
            let start = i.saturating_sub(3);
            let before = &upper[start..i];
            let num_part = before
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>();
            if let Ok(num) = num_part.parse::<u64>() {
                if num > 0 && num < 1000 {
                    return Some(num * 1_000_000_000);
                }
            }
        }
    }
    // Try "Xb" (lowercase)
    for (i, c) in name.char_indices() {
        if c == 'b' && i > 0 {
            let start = i.saturating_sub(3);
            let before = &name[start..i];
            let num_part: String = before.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(num) = num_part.parse::<u64>() {
                if num > 0 && num < 1000 {
                    return Some(num * 1_000_000_000);
                }
            }
        }
    }
    None
}

impl ModelSelectScreen {
    pub fn new(app: &mut App) -> Self {
        let config = &app.config;
        let model_dir = Path::new(&config.model_dir);
        let mut groups = Vec::new();

        if model_dir.is_dir() {
            let models = scan_gguf_files(model_dir);
            groups = build_groups(models, model_dir);
        }

        // Sort models within each group by name
        for group in &mut groups {
            group.models.sort_by(|a, b| a.name.cmp(&b.name));
        }

        let group_index = 0;
        let mut item_list_state = ListState::default();
        if !groups.is_empty() && !groups[0].models.is_empty() {
            item_list_state.select(Some(0));
        }

        Self {
            groups,
            group_index,
            item_list_state,
        }
    }

    /// Total model count across all groups.
    fn total_models(&self) -> usize {
        self.groups.iter().map(|g| g.models.len()).sum()
    }

    /// Current group's model count, or 0.
    fn current_group_size(&self) -> usize {
        self.groups
            .get(self.group_index)
            .map(|g| g.models.len())
            .unwrap_or(0)
    }
}

impl Screen for ModelSelectScreen {
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
            .title(" Select Model ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        f.render_widget(title, chunks[0]);

        // Build flat item list with group headers
        if self.groups.is_empty() {
            let text = Text::from(vec![
                Line::from(Span::styled(
                    " No models found!",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(format!(" Model directory: {}", "see config (press 'c')")),
                Line::from(""),
                Line::from(" Place .gguf files in your model directory."),
            ]);
            let block = Block::default().borders(Borders::ALL).title(" Models ");
            let paragraph = Paragraph::new(text)
                .block(block)
                .alignment(Alignment::Center);
            f.render_widget(paragraph, chunks[1]);
        } else {
            // Find the longest model name for column alignment
            let max_name_len = self
                .groups
                .iter()
                .flat_map(|g| g.models.iter())
                .map(|m| m.name.len())
                .max()
                .unwrap_or(0)
                .min(50); // cap to prevent excessive padding

            let mut flat_items: Vec<ListItem> = Vec::new();
            let mut selected_flat_index: Option<usize> = None;
            let mut idx: usize = 0;

            for (g_idx, group) in self.groups.iter().enumerate() {
                // Group header row
                let header_style = if g_idx == self.group_index {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD)
                };
                let header_text = format!(" {} ─ {} models", group.provider, group.models.len());
                flat_items.push(ListItem::new(Line::from(Span::styled(
                    header_text,
                    header_style,
                ))));
                idx += 1;

                for (m_idx, m) in group.models.iter().enumerate() {
                    let params = m
                        .parameter_count
                        .map(|p| format!("{:.0}B", p as f64 / 1e9))
                        .unwrap_or_else(|| "?".to_string());
                    let content = format!(
                        "    {:max_name_len$}  │  {} params  │  {} MB",
                        m.name,
                        params,
                        m.size_mb,
                        max_name_len = max_name_len,
                    );
                    flat_items.push(ListItem::new(content));

                    if g_idx == self.group_index {
                        if let Some(sel) = self.item_list_state.selected() {
                            if m_idx == sel {
                                selected_flat_index = Some(idx);
                            }
                        }
                    }
                    idx += 1;
                }
            }

            let total = self.total_models();
            let group_name = &self.groups[self.group_index].provider;
            let list_title = format!(" Models ({total} total · {group_name}) ");
            let list = List::new(flat_items)
                .block(Block::default().borders(Borders::ALL).title(list_title))
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            let mut flat_state = ListState::default();
            flat_state.select(selected_flat_index);
            f.render_stateful_widget(list, chunks[1], &mut flat_state);
        }

        // Footer
        let footer_text = if self.groups.is_empty() {
            " Press 'c' to open config | 'q' to quit "
        } else {
            " ↑↓ navigate  |  ←→ switch group  |  Enter select  |  'c' config  |  Esc back  |  'q' quit "
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
            crossterm::event::KeyCode::Up => {
                let sel = self.item_list_state.selected().unwrap_or(0);
                if sel > 0 {
                    self.item_list_state.select(Some(sel - 1));
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Down => {
                let size = self.current_group_size();
                if size > 0 {
                    let sel = self.item_list_state.selected().unwrap_or(0);
                    if sel + 1 < size {
                        self.item_list_state.select(Some(sel + 1));
                    }
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Left => {
                if self.group_index > 0 {
                    self.group_index -= 1;
                    self.item_list_state.select(Some(0));
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Right => {
                if self.group_index + 1 < self.groups.len() {
                    self.group_index += 1;
                    self.item_list_state.select(Some(0));
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Enter => {
                if let Some(i) = self.item_list_state.selected() {
                    if let Some(group) = self.groups.get(self.group_index) {
                        if let Some(model) = group.models.get(i) {
                            return ScreenAction::SelectModel(model.path.clone());
                        }
                    }
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Char('c') => ScreenAction::Navigate(AppScreen::Config),
            crossterm::event::KeyCode::Esc => ScreenAction::Navigate(AppScreen::Hardware),
            crossterm::event::KeyCode::Char('q') => ScreenAction::Exit,
            _ => ScreenAction::None,
        }
    }
}
