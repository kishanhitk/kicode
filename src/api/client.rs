use crate::api::streaming::{StreamAccumulator, parse_sse_line};
use crate::api::types::{ChatRequest, ChatResponse, Message, ProviderPreferences, ToolSchema};
use crate::config::Config;
use crate::error::{KicodeError, Result};
use futures::StreamExt;
use reqwest::Client;

const API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const MODELS_URL: &str = "https://openrouter.ai/api/v1/models";

fn debug_enabled() -> bool {
    std::env::var("KICODE_DEBUG").is_ok()
}

macro_rules! debug_log {
    ($($arg:tt)*) => {
        if debug_enabled() {
            eprintln!("[DEBUG] {}", format!($($arg)*));
        }
    };
}

pub struct OpenRouterClient {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenRouterClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            api_key: config.api_key.clone(),
            model: config.model.clone(),
        }
    }

    /// Validates an API key by making a request to the OpenRouter models endpoint.
    /// Returns Ok(true) if valid, Ok(false) if invalid, or Err on network errors.
    pub async fn validate_key(api_key: &str) -> Result<bool> {
        let client = Client::new();
        let response = client
            .get(MODELS_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    pub async fn chat_stream<F>(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        mut on_chunk: F,
    ) -> Result<Message>
    where
        F: FnMut(String),
    {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.clone(),
            tools: if tools.is_empty() { None } else { Some(tools) },
            provider: Some(ProviderPreferences {
                order: Some(vec![
                    "DeepInfra".to_string(),
                    "Together".to_string(),
                    "Fireworks".to_string(),
                ]),
                ignore: Some(vec!["Novita".to_string()]),
            }),
            stream: true,
        };

        debug_log!("Sending request with {} messages", messages.len());
        for (i, msg) in messages.iter().enumerate() {
            debug_log!(
                "  [{}] role={:?}, content_len={:?}, tool_calls={:?}, tool_call_id={:?}",
                i,
                msg.role,
                msg.content.as_ref().map(|s| s.len()),
                msg.tool_calls.as_ref().map(|t| t.len()),
                msg.tool_call_id
            );
        }

        if debug_enabled() {
            if let Ok(json) = serde_json::to_string_pretty(&request) {
                // Show provider preferences even if truncating the rest
                if json.contains("provider") {
                    eprintln!("[DEBUG] Provider preferences included in request");
                }
            }
        }

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/kicode")
            .header("X-Title", "Kicode")
            .json(&request)
            .send()
            .await?;

        debug_log!("Response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(KicodeError::Api(format!(
                "API request failed with status {}: {}",
                status, body
            )));
        }

        let mut stream = response.bytes_stream();
        let mut accumulator = StreamAccumulator::new();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(data) = parse_sse_line(&line) {
                    if debug_enabled()
                        && data.contains("\"content\":\"")
                        && !data.contains("\"content\":\"\"")
                    {
                        eprintln!(
                            "[DEBUG] Raw SSE with content: {}",
                            &data[..data.len().min(500)]
                        );
                    }
                    match serde_json::from_str::<ChatResponse>(&data) {
                        Ok(response) => {
                            if let Some(choice) = response.choices.first() {
                                if choice.finish_reason.is_some() {
                                    debug_log!("Choice finish_reason: {:?}", choice.finish_reason);
                                }
                                if let Some(ref delta) = choice.delta {
                                    if delta.content.is_some() || delta.tool_calls.is_some() {
                                        debug_log!(
                                            "Delta: content={:?}, tool_calls={:?}",
                                            delta.content.as_ref().map(|s| s.len()),
                                            delta.tool_calls.as_ref().map(|t| t.len())
                                        );
                                    }
                                    if let Some(content) = accumulator.accumulate(delta) {
                                        on_chunk(content);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Only warn on actual parse errors, not [DONE] markers
                            if !data.contains("[DONE]") && !data.trim().is_empty() {
                                eprintln!(
                                    "Parse warning: {} for data: {}",
                                    e,
                                    &data[..data.len().min(100)]
                                );
                            }
                        }
                    }
                }
            }
        }

        // Process any remaining data in the buffer
        if !buffer.trim().is_empty() {
            if let Some(data) = parse_sse_line(&buffer) {
                if let Ok(response) = serde_json::from_str::<ChatResponse>(&data) {
                    if let Some(choice) = response.choices.first() {
                        if let Some(ref delta) = choice.delta {
                            if let Some(content) = accumulator.accumulate(delta) {
                                on_chunk(content);
                            }
                        }
                    }
                }
            }
        }

        let result = accumulator.into_message();
        debug_log!(
            "Response complete - content: {:?}, tool_calls: {:?}",
            result.content.as_ref().map(|s| s.len()),
            result.tool_calls.as_ref().map(|t| t.len())
        );
        Ok(result)
    }
}
