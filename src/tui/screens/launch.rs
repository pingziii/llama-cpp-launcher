use std::path::Path;

use super::{App, AppScreen, Screen, ScreenAction};
use crate::config::Preset;
use crate::hardware::HardwareProfile;
use crate::launcher::build_server_args;
use crate::params::{
    calculate_optimal_params, calculate_optimal_with_preset, calculate_params_with_preset,
    estimate_model_memory, LaunchParams, ModelMetadata,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

// ── Adjustable parameter system ─────────────────────────────────────────

enum ParamAdjust {
    /// Integer value with min, max, step.
    Int(u64, u64, u64, u64),
    /// Choice with current index and option list.
    Choice(usize, &'static [&'static str]),
    /// Boolean toggle.
    Toggle(bool),
}

impl ParamAdjust {
    fn display(&self) -> String {
        match self {
            Self::Int(v, _, _, _) => v.to_string(),
            Self::Choice(idx, opts) => opts[*idx].to_string(),
            Self::Toggle(v) => {
                if *v {
                    "on".to_string()
                } else {
                    "off".to_string()
                }
            }
        }
    }

    fn dec(&mut self) {
        match self {
            Self::Int(v, min, _, step) => {
                *v = v.saturating_sub(*step).max(*min);
            }
            Self::Choice(idx, opts) => {
                *idx = if *idx == 0 { opts.len() - 1 } else { *idx - 1 };
            }
            Self::Toggle(v) => *v = !*v,
        }
    }

    fn inc(&mut self) {
        match self {
            Self::Int(v, _, max, step) => {
                *v = (*v + *step).min(*max);
            }
            Self::Choice(idx, opts) => {
                *idx = (*idx + 1) % opts.len();
            }
            Self::Toggle(v) => *v = !*v,
        }
    }

    fn step_dec(&mut self) {
        match self {
            Self::Int(v, min, _, step) => {
                *v = v.saturating_sub(*step * 10).max(*min);
            }
            other => {
                // For non-Int types, step = same as single press
                other.dec();
            }
        }
    }

    fn step_inc(&mut self) {
        match self {
            Self::Int(v, _, max, step) => {
                *v = (*v + *step * 10).min(*max);
            }
            other => other.inc(),
        }
    }
}

struct AdjustableParam {
    label: &'static str,
    value: ParamAdjust,
}

/// Apply all adjustable param values back into a LaunchParams.
fn apply_adjustable_params(params: &mut LaunchParams, adjustables: &[AdjustableParam]) {
    for a in adjustables {
        match a.label {
            "GPU Layers" => {
                if let ParamAdjust::Int(v, _, _, _) = a.value {
                    params.n_gpu_layers = v as u32;
                }
            }
            "Context Size" => {
                if let ParamAdjust::Int(v, _, _, _) = a.value {
                    params.ctx_size = v as u32;
                }
            }
            "Batch Size" => {
                if let ParamAdjust::Int(v, _, _, _) = a.value {
                    params.batch_size = v as u32;
                }
            }
            "UBatch Size" => {
                if let ParamAdjust::Int(v, _, _, _) = a.value {
                    params.ubatch_size = v as u32;
                }
            }
            "Flash Attn" => {
                if let ParamAdjust::Choice(idx, opts) = a.value {
                    params.flash_attn = opts[idx].to_string();
                }
            }
            "Threads" => {
                if let ParamAdjust::Int(v, _, _, _) = a.value {
                    params.thread_count = v as u32;
                }
            }
            "Parallel" => {
                if let ParamAdjust::Int(v, _, _, _) = a.value {
                    params.parallel_slots = v as u32;
                }
            }
            "KV Cache K" => {
                if let ParamAdjust::Choice(idx, opts) = a.value {
                    params.cache_type_k = opts[idx].to_string();
                }
            }
            "KV Cache V" => {
                if let ParamAdjust::Choice(idx, opts) = a.value {
                    params.cache_type_v = opts[idx].to_string();
                }
            }
            "Mlock" => {
                if let ParamAdjust::Toggle(v) = a.value {
                    params.mlock = v;
                }
            }
            "Host" => {
                if let ParamAdjust::Choice(idx, opts) = a.value {
                    params.host = opts[idx].to_string();
                }
            }
            "NoKVOffload" => {
                if let ParamAdjust::Toggle(v) = a.value {
                    params.no_kv_offload = v;
                }
            }
            "Jinja" => {
                if let ParamAdjust::Toggle(v) = a.value {
                    params.jinja = v;
                }
            }
            "Port" => {
                if let ParamAdjust::Int(v, _, _, _) = a.value {
                    params.port = v as u16;
                }
            }
            _ => {}
        }
    }
}

// ── Screen ──────────────────────────────────────────────────────────────

pub struct LaunchScreen {
    hw: HardwareProfile,
    model_path: String,
    host: String,
    port: u16,
    params: Option<LaunchParams>,
    selected_preset_index: usize,
    presets: Vec<Preset>,
    llama_bin_path: Option<String>,
    metadata: Option<ModelMetadata>,
    metadata_error: Option<String>,
    launch_error: Option<String>,
    adjust_mode: bool,
    adjust_index: usize,
    adjustable_params: Vec<AdjustableParam>,
}

impl LaunchScreen {
    pub fn new(app: &mut App) -> Self {
        let hw = app.hardware.clone().unwrap();
        let config = &app.config;
        let model_path = config
            .last_model
            .clone()
            .unwrap_or_else(|| config.model_dir.clone());

        let host = config.host.clone();
        let port = config.port;

        // Read GGUF metadata from the selected model
        let (metadata, metadata_error) =
            match crate::params::read_gguf_metadata(Path::new(&model_path)) {
                Ok(m) => (Some(m), None),
                Err(e) => (None, Some(format!("{e}"))),
            };

        let presets = config.presets.clone();
        let default_preset_idx = config
            .last_preset
            .as_deref()
            .and_then(|last| config.presets.iter().position(|p| p.name == last))
            .unwrap_or(0);
        let preset_idx = if presets.is_empty() {
            0
        } else {
            default_preset_idx.min(presets.len().saturating_sub(1))
        };

        let params = Self::compute_params(
            &hw,
            &model_path,
            &host,
            port,
            &presets,
            preset_idx,
            &metadata,
        );

        let adjustable_params = build_adjustable_params(&params);

        Self {
            hw,
            model_path,
            host,
            port,
            params: Some(params),
            selected_preset_index: preset_idx,
            presets,
            llama_bin_path: config.llama_bin_path.clone(),
            metadata,
            metadata_error,
            launch_error: app.launch_error.take(),
            adjust_mode: false,
            adjust_index: 0,
            adjustable_params,
        }
    }

    fn compute_params(
        hw: &HardwareProfile,
        model_path: &str,
        host: &str,
        port: u16,
        presets: &[Preset],
        preset_idx: usize,
        metadata: &Option<ModelMetadata>,
    ) -> LaunchParams {
        let preset = presets.get(preset_idx);
        match (preset, metadata) {
            (Some(p), Some(m)) => calculate_optimal_with_preset(hw, m, model_path, host, port, p),
            (None, Some(m)) => calculate_optimal_params(hw, m, model_path, host, port),
            (Some(p), None) => calculate_params_with_preset(hw, model_path, host, port, p),
            (None, None) => crate::params::calculate_params(hw, model_path, host, port),
        }
    }

    /// Sync self.host and self.port from self.params after adjust.
    fn sync_host_port(&mut self) {
        if let Some(ref p) = self.params {
            self.host.clone_from(&p.host);
            self.port = p.port;
        }
    }

    fn render_params(&self) -> Vec<Line<'static>> {
        let params = self.params.as_ref().unwrap();

        // Use metadata parameter count for memory estimation if available
        let file_size = std::fs::metadata(&self.model_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let param_count_for_memory = self.metadata.as_ref().map(|m| m.parameter_count as u64);
        let estimated_mb = estimate_model_memory(file_size, param_count_for_memory) / (1024 * 1024);

        // Format parameter count from metadata
        let param_str = self
            .metadata
            .as_ref()
            .map(|m| {
                if m.parameter_count >= 1_000_000_000.0 {
                    format!("{:.1}B", m.parameter_count / 1e9)
                } else if m.parameter_count >= 1_000_000.0 {
                    format!("{:.0}M", m.parameter_count / 1e6)
                } else {
                    format!("{}", m.parameter_count as u64)
                }
            })
            .unwrap_or_else(|| "unknown".to_string());

        let mem_gb = if self.hw.total_ram_bytes > 0 {
            format!("{:.1} GB", self.hw.total_ram_bytes as f64 / 1e9)
        } else {
            "unknown".to_string()
        };

        let preset_name = self
            .presets
            .get(self.selected_preset_index)
            .map(|p| p.name.as_str())
            .unwrap_or("none");

        // Build the command-line args for display
        let args = build_server_args(params);
        let bin_path = self
            .llama_bin_path
            .as_deref()
            .unwrap_or("llama-server (from PATH)");

        let mut lines = vec![
            Line::from(Span::styled(
                " Launch Configuration",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(" Model: {}", self.model_path)),
        ];

        // Show model metadata when available
        if let Some(ref meta) = self.metadata {
            if !meta.architecture.is_empty() {
                lines.push(Line::from(format!(" Architecture: {}", meta.architecture)));
            }
            lines.push(Line::from(format!(" Parameters: {param_str}")));
            if meta.context_length > 0 {
                lines.push(Line::from(format!(
                    " Model context: {} tokens",
                    meta.context_length
                )));
            }
        } else if let Some(ref err) = self.metadata_error {
            lines.push(Line::from(Span::styled(
                format!(" Metadata: {err}"),
                Style::default().fg(Color::Red),
            )));
        }

        // ── Launch error ─────────────────────────────────────────────
        if let Some(ref err) = self.launch_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" Launch failed: {err}"),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                " Check binary path, port availability, and model compatibility.",
                Style::default().fg(Color::DarkGray),
            )));
        }

        // ── Adjustable params section ───────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            " Adjustable Parameters:",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        for (i, ap) in self.adjustable_params.iter().enumerate() {
            let selected = self.adjust_mode && i == self.adjust_index;
            let prefix = if selected { " >" } else { "  " };
            let style = if selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let hint = match &ap.value {
                ParamAdjust::Int(..) => "  [-+]",
                ParamAdjust::Choice(..) => "  [< >]",
                ParamAdjust::Toggle(_) => "",
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("{prefix} {:15} {}", ap.label, ap.value.display()),
                    style,
                ),
                Span::styled(hint, Style::default().fg(Color::DarkGray)),
            ]));
        }

        // ── Static info ─────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(" Estimated memory: ~{} MB", estimated_mb),
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(Span::styled(
            format!(" Available RAM: {mem_gb}"),
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                " Preset [{}]: {}",
                self.selected_preset_index + 1,
                preset_name,
            ),
            Style::default().fg(Color::Cyan),
        )));

        if !self.adjust_mode {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " Command:",
                Style::default().fg(Color::Magenta),
            )));
            lines.push(Line::from(Span::styled(
                format!("   {bin_path}"),
                Style::default().fg(Color::White),
            )));

            for chunk in args.chunks(4) {
                let arg_line: Vec<String> = chunk.iter().map(|a| shell_escape(a)).collect();
                lines.push(Line::from(Span::styled(
                    format!("   {}", arg_line.join(" ")),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                " [ Adjust mode: ↑↓ select | ←→ adjust | 'a'/Esc done ]",
                Style::default().fg(Color::Green),
            )));
        }

        lines
    }
}

/// Build the adjustable params list from initial LaunchParams.
fn build_adjustable_params(params: &LaunchParams) -> Vec<AdjustableParam> {
    vec![
        AdjustableParam {
            label: "Host",
            value: ParamAdjust::Choice(
                ["127.0.0.1", "0.0.0.0"]
                    .iter()
                    .position(|&s| s == params.host)
                    .unwrap_or(0),
                &["127.0.0.1", "0.0.0.0"],
            ),
        },
        AdjustableParam {
            label: "Port",
            value: ParamAdjust::Int(params.port as u64, 1024, 65535, 1),
        },
        AdjustableParam {
            label: "GPU Layers",
            value: ParamAdjust::Int(params.n_gpu_layers as u64, 0, 99, 1),
        },
        AdjustableParam {
            label: "Context Size",
            value: ParamAdjust::Int(params.ctx_size as u64, 512, 131_072, 512),
        },
        AdjustableParam {
            label: "Batch Size",
            value: ParamAdjust::Int(params.batch_size as u64, 64, 8_192, 64),
        },
        AdjustableParam {
            label: "UBatch Size",
            value: ParamAdjust::Int(params.ubatch_size as u64, 64, 2_048, 64),
        },
        AdjustableParam {
            label: "Flash Attn",
            value: ParamAdjust::Choice(
                ["auto", "on", "off"]
                    .iter()
                    .position(|&s| s == params.flash_attn)
                    .unwrap_or(0),
                &["auto", "on", "off"],
            ),
        },
        AdjustableParam {
            label: "Threads",
            value: ParamAdjust::Int(params.thread_count as u64, 1, 64, 1),
        },
        AdjustableParam {
            label: "Parallel",
            value: ParamAdjust::Int(params.parallel_slots as u64, 1, 32, 1),
        },
        AdjustableParam {
            label: "KV Cache K",
            value: ParamAdjust::Choice(
                ["f16", "q8_0", "q4_0"]
                    .iter()
                    .position(|&s| s == params.cache_type_k)
                    .unwrap_or(0),
                &["f16", "q8_0", "q4_0"],
            ),
        },
        AdjustableParam {
            label: "KV Cache V",
            value: ParamAdjust::Choice(
                ["f16", "q8_0", "q4_0"]
                    .iter()
                    .position(|&s| s == params.cache_type_v)
                    .unwrap_or(0),
                &["f16", "q8_0", "q4_0"],
            ),
        },
        AdjustableParam {
            label: "Mlock",
            value: ParamAdjust::Toggle(params.mlock),
        },
        AdjustableParam {
            label: "NoKVOffload",
            value: ParamAdjust::Toggle(params.no_kv_offload),
        },
        AdjustableParam {
            label: "Jinja",
            value: ParamAdjust::Toggle(params.jinja),
        },
    ]
}

/// Simple shell quoting for display: wrap in quotes if contains spaces.
fn shell_escape(s: &str) -> String {
    if s.contains(' ') || s.contains('\\') {
        format!("\"{s}\"")
    } else {
        s.to_string()
    }
}

impl Screen for LaunchScreen {
    fn render(&mut self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);

        // Title
        let title_style = if self.adjust_mode {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Cyan)
        };
        let title = Block::default()
            .title(if self.adjust_mode {
                " Launch Model — Adjust Mode "
            } else {
                " Launch Model "
            })
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(title_style);
        f.render_widget(title, chunks[0]);

        // Params display
        let lines = self.render_params();
        let text = Text::from(lines);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Configuration ");
        let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
        f.render_widget(paragraph, chunks[1]);

        // Footer
        let footer_text = if self.adjust_mode {
            " ↑↓ select | ←→ adjust (×10 with Shift) | 'a'/Esc done | Enter launch "
        } else {
            " Enter: launch | p: cycle preset | a: adjust | Esc: back | q: quit "
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
        // ── Adjust mode ───────────────────────────────────────────────
        if self.adjust_mode {
            let shift = key
                .modifiers
                .contains(crossterm::event::KeyModifiers::SHIFT);
            match key.code {
                crossterm::event::KeyCode::Up if self.adjust_index > 0 => {
                    self.adjust_index -= 1;
                }
                crossterm::event::KeyCode::Down
                    if self.adjust_index + 1 < self.adjustable_params.len() =>
                {
                    self.adjust_index += 1;
                }
                crossterm::event::KeyCode::Left => {
                    if let Some(ap) = self.adjustable_params.get_mut(self.adjust_index) {
                        if shift {
                            ap.value.step_dec();
                        } else {
                            ap.value.dec();
                        }
                        if let Some(ref mut p) = self.params {
                            apply_adjustable_params(p, &self.adjustable_params);
                        }
                        self.sync_host_port();
                    }
                }
                crossterm::event::KeyCode::Right => {
                    if let Some(ap) = self.adjustable_params.get_mut(self.adjust_index) {
                        if shift {
                            ap.value.step_inc();
                        } else {
                            ap.value.inc();
                        }
                        if let Some(ref mut p) = self.params {
                            apply_adjustable_params(p, &self.adjustable_params);
                        }
                        self.sync_host_port();
                    }
                }
                crossterm::event::KeyCode::Char('a') => {
                    self.adjust_mode = false;
                }
                crossterm::event::KeyCode::Esc => {
                    self.adjust_mode = false;
                }
                crossterm::event::KeyCode::Enter => {
                    self.adjust_mode = false;
                    if let Some(ref p) = self.params {
                        return ScreenAction::Launch(p.clone());
                    }
                }
                _ => {}
            }
            return ScreenAction::None;
        }

        // ── Normal mode ───────────────────────────────────────────────
        match key.code {
            crossterm::event::KeyCode::Enter => {
                if let Some(ref p) = self.params {
                    return ScreenAction::Launch(p.clone());
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Char('p') => {
                if !self.presets.is_empty() {
                    self.selected_preset_index =
                        (self.selected_preset_index + 1) % self.presets.len();
                    self.params = Some(Self::compute_params(
                        &self.hw,
                        &self.model_path,
                        &self.host,
                        self.port,
                        &self.presets,
                        self.selected_preset_index,
                        &self.metadata,
                    ));
                    // Rebuild adjustable params to match new values
                    self.adjustable_params = build_adjustable_params(self.params.as_ref().unwrap());
                }
                ScreenAction::None
            }
            crossterm::event::KeyCode::Char('a') => {
                self.adjust_mode = true;
                self.adjust_index = 0;
                ScreenAction::None
            }
            crossterm::event::KeyCode::Esc => ScreenAction::Navigate(AppScreen::ModelSelect),
            crossterm::event::KeyCode::Char('q') => ScreenAction::Exit,
            _ => ScreenAction::None,
        }
    }
}
