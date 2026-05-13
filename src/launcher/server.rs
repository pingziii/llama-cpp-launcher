use std::time::Duration;

use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::error::Result;

/// A single token yielded from a streaming response.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StreamToken {
    pub token: String,
    pub is_done: bool,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct ChatCompletionRequest {
    messages: Vec<ChatMessage>,
    stream: bool,
    temperature: f64,
    n_predict: i32,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct ChatMessage {
    role: String,
    content: String,
}

/// HTTP client for communicating with the llama.cpp server API.
#[allow(dead_code)]
pub struct LLamaClient {
    client: Client,
    base_url: String,
}

#[allow(dead_code)]
impl LLamaClient {
    pub fn new(host: &str, port: u16) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: format!("http://{host}:{port}"),
        }
    }

    /// Check if the server health endpoint responds.
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Send a prompt and receive streaming tokens via an mpsc channel.
    pub async fn send_prompt(&self, prompt: &str) -> Result<mpsc::Receiver<StreamToken>> {
        let (tx, rx) = mpsc::channel::<StreamToken>(128);

        let url = format!("{}/v1/chat/completions", self.base_url);
        let body = ChatCompletionRequest {
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            stream: true,
            temperature: 0.7,
            n_predict: -1,
        };

        let client = self.client.clone();

        tokio::spawn(async move {
            match client.post(&url).json(&body).send().await {
                Ok(response) => {
                    let mut stream = response.bytes_stream();
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                let text = String::from_utf8_lossy(&bytes);
                                for line in text.lines() {
                                    if let Some(token) = parse_sse_line(line) {
                                        if tx.send(token).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Stream error: {e}");
                                let _ = tx
                                    .send(StreamToken {
                                        token: format!("\n[Error: {e}]"),
                                        is_done: true,
                                    })
                                    .await;
                                return;
                            }
                        }
                    }
                    let _ = tx
                        .send(StreamToken {
                            token: String::new(),
                            is_done: true,
                        })
                        .await;
                }
                Err(e) => {
                    tracing::error!("Request failed: {e}");
                    let _ = tx
                        .send(StreamToken {
                            token: format!("\n[Connection error: {e}]"),
                            is_done: true,
                        })
                        .await;
                }
            }
        });

        Ok(rx)
    }

    /// Abort the current generation.
    pub async fn abort_generation(&self) -> Result<()> {
        let url = format!("{}/v1/chat/completions/abort", self.base_url);
        let _ = self.client.post(&url).send().await;
        Ok(())
    }
}

/// Parse an SSE data line from llama.cpp's streaming response.
#[allow(dead_code)]
pub(crate) fn parse_sse_line(line: &str) -> Option<StreamToken> {
    let line = line.trim();
    if !line.starts_with("data: ") {
        return None;
    }

    let data = line.strip_prefix("data: ")?;

    if data == "[DONE]" {
        return Some(StreamToken {
            token: String::new(),
            is_done: true,
        });
    }

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
        if let Some(choices) = val.get("choices").and_then(|c| c.as_array()) {
            if let Some(choice) = choices.first() {
                if let Some(delta) = choice.get("delta") {
                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        if !content.is_empty() {
                            return Some(StreamToken {
                                token: content.to_string(),
                                is_done: false,
                            });
                        }
                    }
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_line_regular_token() {
        let line = r#"data: {"choices":[{"delta":{"content":"Hello"},"index":0}]}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        let token = result.unwrap();
        assert_eq!(token.token, "Hello");
        assert!(!token.is_done);
    }

    #[test]
    fn test_parse_sse_line_done_signal() {
        let line = "data: [DONE]";
        let result = parse_sse_line(line);
        assert!(result.is_some());
        let token = result.unwrap();
        assert!(token.token.is_empty());
        assert!(token.is_done);
    }

    #[test]
    fn test_parse_sse_line_non_data_line() {
        let line = "not a data line";
        let result = parse_sse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_empty_data() {
        let line = "data: ";
        let result = parse_sse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_malformed_json() {
        let line = "data: {bad json}";
        let result = parse_sse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_empty_content() {
        let line = r#"data: {"choices":[{"delta":{"content":""},"index":0}]}"#;
        let result = parse_sse_line(line);
        assert!(result.is_none(), "empty content should return None");
    }

    #[test]
    fn test_parse_sse_line_leading_whitespace() {
        let line = r#"  data: {"choices":[{"delta":{"content":"Hi"},"index":0}]}"#;
        let result = parse_sse_line(line);
        assert!(result.is_some());
        assert_eq!(result.unwrap().token, "Hi");
    }

    #[test]
    fn test_parse_sse_line_no_choices() {
        let line = r#"data: {"model":"test","usage":{}}"#;
        let result = parse_sse_line(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_sse_line_no_delta() {
        let line = r#"data: {"choices":[{"index":0,"finish_reason":"stop"}]}"#;
        let result = parse_sse_line(line);
        assert!(result.is_none());
    }
}
