use std::sync::{Arc, Mutex};
use std::time::Duration;

use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tracing::{error, info, warn};

use crate::error::{AppError, Result};
use crate::launcher::{build_server_args, find_llama_binary};
use crate::params::LaunchParams;

/// Represents a running or stopped llama.cpp server process.
pub struct LLamaProcess {
    child: Option<Child>,
    pub port: u16,
    pub host: String,
    pub log_buffer: Arc<Mutex<Vec<String>>>,
}

impl LLamaProcess {
    /// Spawn a llama.cpp server with the given params and wait for it to be ready.
    /// Optionally provide a configured binary path (from config.llama_bin_path).
    #[allow(dead_code)]
    pub async fn start(params: &LaunchParams, llama_bin: Option<&str>) -> Result<Self> {
        let proc = Self::spawn(params, llama_bin).await?;
        proc.wait_for_ready_with_cancel(None).await?;
        Ok(proc)
    }

    /// Spawn the server process without waiting for health check.
    pub async fn spawn(params: &LaunchParams, llama_bin: Option<&str>) -> Result<Self> {
        let cmd_result = find_llama_binary(llama_bin)?;
        let args = build_server_args(params);

        info!(
            port = params.port,
            model = %params.model_path,
            threads = params.thread_count,
            gpu_layers = params.n_gpu_layers,
            ctx_size = params.ctx_size,
            "Starting llama.cpp server"
        );

        let program_path = cmd_result.get_program().to_string_lossy().to_string();
        let mut tokio_cmd = Command::from(cmd_result);
        let mut child = tokio_cmd
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::ProcessStart(e.to_string()))?;

        let log_buffer = Arc::new(Mutex::new(Vec::new()));
        let buf_clone = log_buffer.clone();

        // Push the full command line as the first line
        let cmd_line = {
            let mut parts = vec![program_path];
            parts.extend(args.iter().map(|a| {
                if a.contains(' ') || a.contains('\\') {
                    format!("\"{a}\"")
                } else {
                    a.clone()
                }
            }));
            parts.join(" ")
        };
        log_buffer.lock().unwrap().push(cmd_line);

        // Spawn background task to read stderr into the log buffer
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut buf = buf_clone.lock().unwrap();
                    buf.push(line);
                    if buf.len() > 1000 {
                        buf.remove(0);
                    }
                }
            });
        }

        Ok(LLamaProcess {
            child: Some(child),
            port: params.port,
            host: params.host.clone(),
            log_buffer,
        })
    }

    /// Retry health check every 3s until success or cancelled.
    /// Connects via TCP and sends `GET /health`, verifies HTTP 200 + body contains `"ok"`.
    /// If host is `0.0.0.0`, connects to `127.0.0.1` instead.
    /// Pass `None` for no cancellation (keeps retrying until success).
    pub async fn wait_for_ready_with_cancel(
        &self,
        mut cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<()> {
        let connect_host = if self.host == "0.0.0.0" {
            "127.0.0.1"
        } else {
            &self.host
        };
        let addr = format!("{connect_host}:{}", self.port);

        loop {
            // Check cancellation
            if let Some(ref rx) = cancel_rx {
                if *rx.borrow() {
                    return Err(AppError::Cancelled);
                }
            }

            match Self::http_health_check(&addr, connect_host, self.port).await {
                Ok(true) => {
                    info!("llama.cpp server is ready via /health");
                    return Ok(());
                }
                _ => {
                    if let Some(ref mut rx) = cancel_rx {
                        tokio::select! {
                            _ = rx.changed() => {
                                if *rx.borrow() {
                                    return Err(AppError::Cancelled);
                                }
                            }
                            _ = tokio::time::sleep(Duration::from_secs(3)) => {}
                        }
                    } else {
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                }
            }
        }
    }

    /// Send raw HTTP GET /health and verify 200 + body contains "ok".
    async fn http_health_check(
        addr: &str,
        host: &str,
        port: u16,
    ) -> std::result::Result<bool, String> {
        let timeout = Duration::from_secs(5);
        let mut stream = tokio::time::timeout(timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| "connect timeout".to_string())?
            .map_err(|e| format!("connect failed: {e}"))?;

        let request =
            format!("GET /health HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");
        tokio::time::timeout(timeout, stream.write_all(request.as_bytes()))
            .await
            .map_err(|_| "write timeout".to_string())?
            .map_err(|e| format!("write failed: {e}"))?;

        let mut reader = BufReader::new(&mut stream);
        let mut response = String::new();
        tokio::time::timeout(timeout, reader.read_line(&mut response))
            .await
            .map_err(|_| "read timeout".to_string())?
            .map_err(|e| format!("read failed: {e}"))?;

        // Check HTTP status line
        if !response.starts_with("HTTP/1.") || !response.contains("200") {
            return Err(format!("bad status: {response}"));
        }

        // Read headers until empty line
        loop {
            let mut line = String::new();
            let n = reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("read line: {e}"))?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break; // empty line = end of headers
            }
        }
        // Read body (up to 1KB)
        let mut body = vec![0u8; 1024];
        let n = reader
            .read(&mut body)
            .await
            .map_err(|e| format!("read body: {e}"))?;
        let body_str = String::from_utf8_lossy(&body[..n]);

        if body_str.contains("\"ok\"") {
            Ok(true)
        } else {
            Err(format!("body does not indicate ok: {body_str}"))
        }
    }

    /// Non-blocking check if the underlying process is still running.
    pub fn is_running(&mut self) -> bool {
        match self.child.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => false, // exited
                Ok(None) => true,     // still running
                Err(_) => false,      // error checking
            },
            None => false,
        }
    }

    /// Send SIGTERM and wait for graceful shutdown.
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            info!("Stopping llama.cpp process");
            let _ = child.kill().await;
            let timeout = Duration::from_secs(30);
            match tokio::time::timeout(timeout, child.wait()).await {
                Ok(Ok(status)) => {
                    info!(code = %status, "Process stopped");
                    Ok(())
                }
                Ok(Err(e)) => {
                    error!("Error waiting for process: {e}");
                    Err(AppError::ProcessCrashed(e.to_string()))
                }
                Err(_) => {
                    warn!("Process did not exit within 30s");
                    child
                        .kill()
                        .await
                        .map_err(|e| AppError::ProcessStart(e.to_string()))?;
                    child.wait().await.ok();
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }
}
