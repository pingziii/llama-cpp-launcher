use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use tracing::info;

use crate::config::Config;
use crate::error::Result;
use crate::hardware::HardwareProfile;
use crate::launcher::LLamaClient;
use crate::launcher::LLamaProcess;
use crate::params::LaunchParams;

use super::screens::{
    ChatScreen, ConfigScreen, HardwareScreen, LaunchScreen, ModelSelectScreen, Screen,
};

/// Top-level application state and event loop.
pub struct App {
    pub config: Config,
    pub hardware: Option<HardwareProfile>,
    pub selected_model: Option<String>,
    pub launch_params: Option<LaunchParams>,
    pub launch_error: Option<String>,
    pub process: Option<LLamaProcess>,
    pub client: Option<LLamaClient>,
    pub current_screen: AppScreen,
    /// Shared health status for ChatScreen: None=unknown, Some(true)=ok, Some(false)=dead
    pub server_healthy: Arc<Mutex<Option<bool>>>,
    health_check_counter: u32,
    consecutive_failures: u32,
}

pub enum AppScreen {
    Hardware,
    ModelSelect,
    Launch,
    Chat,
    Config,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            hardware: None,
            selected_model: None,
            launch_params: None,
            launch_error: None,
            process: None,
            client: None,
            current_screen: AppScreen::Hardware,
            server_healthy: Arc::new(Mutex::new(None)),
            health_check_counter: 0,
            consecutive_failures: 0,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = init_terminal()?;

        let result = self.run_loop(&mut terminal).await;

        // Clean up: kill any running process
        if let Some(mut proc) = self.process.take() {
            let _ = proc.stop().await;
        }

        restore_terminal(&mut terminal)?;

        result
    }

    async fn run_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        let mut current_screen: Option<Box<dyn Screen>> = None;

        loop {
            // Check if screen needs to be recreated due to navigation
            let needs_new = current_screen.is_none();
            if needs_new {
                terminal.clear()?;
                current_screen = Some(self.create_screen());
            }

            terminal.draw(|f| {
                if let Some(screen) = current_screen.as_mut() {
                    screen.render(f);
                }
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        // Global Ctrl+C to quit even if screen doesn't handle it
                        if key.code == KeyCode::Char('c')
                            && key.modifiers == crossterm::event::KeyModifiers::CONTROL
                        {
                            info!("Ctrl+C: user requested exit");
                            break;
                        }

                        let screen = current_screen.as_mut().unwrap();
                        match screen.handle_key(key) {
                            crate::tui::screens::ScreenAction::Exit => {
                                info!("User requested exit");
                                break;
                            }
                            crate::tui::screens::ScreenAction::Navigate(next) => {
                                self.current_screen = next;
                                current_screen = None; // recreate on next iteration
                            }
                            crate::tui::screens::ScreenAction::None => {}
                            crate::tui::screens::ScreenAction::Launch(p) => {
                                self.launch_params = Some(p.clone());
                                // Spawn process first (no health check)
                                match LLamaProcess::spawn(&p, self.config.llama_bin_path.as_deref())
                                    .await
                                {
                                    Ok(proc) => {
                                        self.process = Some(proc);
                                        let (cancel_tx, cancel_rx) =
                                            tokio::sync::watch::channel(false);

                                        // Loading sub-loop: retry health check every 3s, cancel with Esc
                                        'loading: loop {
                                            terminal.clear()?;
                                            terminal.draw(|f| {
                                                let text = format!(
                                                    " Starting llama.cpp server...\n\n model: {}\n host: {}:{}\n\n Press Esc to cancel",
                                                    p.model_path, p.host, p.port
                                                );
                                                let paragraph = ratatui::widgets::Paragraph::new(text)
                                                    .block(ratatui::widgets::Block::default()
                                                        .title(" Launching ")
                                                        .borders(ratatui::widgets::Borders::ALL))
                                                    .alignment(ratatui::layout::Alignment::Center);
                                                f.render_widget(paragraph, f.area());
                                            })?;

                                            // Poll for events for ~3s (30 × 100ms) checking cancel
                                            let mut cancelled = false;
                                            for _ in 0..30 {
                                                if event::poll(Duration::from_millis(100))? {
                                                    if let Event::Key(key) = event::read()? {
                                                        if key.kind
                                                            == crossterm::event::KeyEventKind::Press
                                                        {
                                                            match key.code {
                                                                crossterm::event::KeyCode::Esc => {
                                                                cancelled = true;
                                                                }
                                                                crossterm::event::KeyCode::Char('c')
                                                                    if key.modifiers == crossterm::event::KeyModifiers::CONTROL =>
                                                                {
                                                                    cancelled = true;
                                                                    // Will break outer event loop below
                                                                }
                                                                _ => {}
                                                            }
                                                        }
                                                    }
                                                }
                                                if cancelled {
                                                    break;
                                                }
                                            }

                                            if cancelled {
                                                let _ = cancel_tx.send(true);
                                                if let Some(mut p) = self.process.take() {
                                                    let _ = p.stop().await;
                                                }
                                                self.launch_error =
                                                    Some("Loading cancelled by user".to_string());
                                                self.current_screen = AppScreen::Launch;
                                                current_screen = None;
                                                break 'loading;
                                            }

                                            // Try health check
                                            let proc = self.process.as_ref().unwrap();
                                            match proc
                                                .wait_for_ready_with_cancel(Some(cancel_rx.clone()))
                                                .await
                                            {
                                                Ok(()) => {
                                                    info!("llama.cpp server started successfully");
                                                    self.launch_error = None;
                                                    let client_host = if p.host == "0.0.0.0" {
                                                        "127.0.0.1"
                                                    } else {
                                                        &p.host
                                                    };
                                                    self.client =
                                                        Some(LLamaClient::new(client_host, p.port));
                                                    // Reset health monitoring state
                                                    *self.server_healthy.lock().unwrap() = Some(true);
                                                    self.consecutive_failures = 0;
                                                    self.health_check_counter = 0;
                                                    self.current_screen = AppScreen::Chat;
                                                    current_screen = None;
                                                    break 'loading;
                                                }
                                                Err(crate::error::AppError::Cancelled) => {
                                                    // Cancelled during health check call
                                                    if let Some(mut p) = self.process.take() {
                                                        let _ = p.stop().await;
                                                    }
                                                    self.launch_error = Some(
                                                        "Loading cancelled by user".to_string(),
                                                    );
                                                    self.current_screen = AppScreen::Launch;
                                                    current_screen = None;
                                                    break 'loading;
                                                }
                                                Err(_) => {
                                                    // Not ready yet, loop back and retry
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to spawn llama.cpp: {e}");
                                        self.launch_error = Some(format!("{e}"));
                                        self.current_screen = AppScreen::Launch;
                                        current_screen = None;
                                    }
                                }
                            }
                            crate::tui::screens::ScreenAction::StopServer => {
                                info!("Stopping server by user request");
                                if let Some(mut proc) = self.process.take() {
                                    let _ = proc.stop().await;
                                }
                                self.client = None;
                                self.launch_params = None;
                                *self.server_healthy.lock().unwrap() = None;
                                self.consecutive_failures = 0;
                                self.health_check_counter = 0;
                                self.current_screen = AppScreen::ModelSelect;
                                current_screen = None;
                            }
                            crate::tui::screens::ScreenAction::SelectModel(path) => {
                                self.selected_model = Some(path);
                                self.launch_error = None;
                                self.current_screen = AppScreen::Launch;
                                current_screen = None;
                            }
                        }
                    }
                }
            }

            // ── Periodic server health check (only on ChatScreen) ──
            if matches!(self.current_screen, AppScreen::Chat) && self.client.is_some() {
                self.health_check_counter += 1;
                // Every ~3 seconds (30 × 100ms poll)
                if self.health_check_counter >= 30 {
                    self.health_check_counter = 0;

                    // Check if process exited first
                    let process_alive = self
                        .process
                        .as_mut()
                        .map(|p| p.is_running())
                        .unwrap_or(false);

                    if !process_alive {
                        info!("Detected llama.cpp process exited");
                        *self.server_healthy.lock().unwrap() = Some(false);
                        self.consecutive_failures += 1;
                    } else if let Some(ref client) = self.client {
                        let healthy = client.health_check().await.unwrap_or(false);
                        let mut status = self.server_healthy.lock().unwrap();
                        *status = Some(healthy);
                        if healthy {
                            self.consecutive_failures = 0;
                        } else {
                            self.consecutive_failures += 1;
                        }
                    }

                    // After 5 consecutive failures (~15s), auto-navigate back to launch
                    if self.consecutive_failures >= 5 {
                        info!(
                            failures = self.consecutive_failures,
                            "Server unreachable, navigating back"
                        );
                        if let Some(mut proc) = self.process.take() {
                            let _ = proc.stop().await;
                        }
                        self.client = None;
                        self.launch_params = None;
                        self.launch_error =
                            Some("Server stopped responding after startup".to_string());
                        self.current_screen = AppScreen::Launch;
                        current_screen = None;
                        self.consecutive_failures = 0;
                    }
                }
            }
        }

        Ok(())
    }

    fn create_screen(&mut self) -> Box<dyn Screen> {
        match &self.current_screen {
            AppScreen::Hardware => Box::new(HardwareScreen::new(self)),
            AppScreen::ModelSelect => Box::new(ModelSelectScreen::new(self)),
            AppScreen::Launch => Box::new(LaunchScreen::new(self)),
            AppScreen::Chat => Box::new(ChatScreen::new(self, self.server_healthy.clone())),
            AppScreen::Config => Box::new(ConfigScreen::new(self)),
        }
    }
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>> {
    use ratatui::backend::CrosstermBackend;

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;

    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
    terminal.show_cursor()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen
    )?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}
