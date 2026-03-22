use super::{LlmProvider, LlmTurn, ToolCall, ToolDefinition, TokenCallback};
use crate::error::AppError;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: Client::new(),
        }
    }

    /// Convert our provider-agnostic message Value to Anthropic wire format.
    fn to_anthropic_message(msg: &Value) -> Value {
        let role = msg["role"].as_str().unwrap_or("user");

        if let Some(tool_use_arr) = msg.get("tool_use").and_then(|v| v.as_array()) {
            let blocks: Vec<Value> = tool_use_arr.iter().map(|tc| json!({
                "type": "tool_use",
                "id": tc["id"],
                "name": tc["name"],
                "input": tc["input"],
            })).collect();
            json!({ "role": role, "content": blocks })
        } else if let Some(results_arr) = msg.get("tool_results").and_then(|v| v.as_array()) {
            let blocks: Vec<Value> = results_arr.iter().map(|tr| {
                let mut block = json!({
                    "type": "tool_result",
                    "tool_use_id": tr["tool_use_id"],
                    "content": tr["content"],
                });
                if let Some(is_err) = tr.get("is_error") {
                    block["is_error"] = is_err.clone();
                }
                block
            }).collect();
            json!({ "role": "user", "content": blocks })
        } else {
            let content = msg["content"].as_str().unwrap_or("");
            json!({ "role": role, "content": content })
        }
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<Value>,
    tools: Vec<Value>,
    stream: bool,
}

/// Accumulator for a streaming tool_use block
struct ToolAccum {
    id: String,
    name: String,
    input_json: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(
        &self,
        system: &str,
        messages: Vec<Value>,
        tools: Vec<ToolDefinition>,
        on_token: TokenCallback,
    ) -> Result<LlmTurn, AppError> {
        let anthropic_messages: Vec<Value> = messages
            .iter()
            .map(Self::to_anthropic_message)
            .collect();

        let anthropic_tools: Vec<Value> = tools
            .iter()
            .map(|t| json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            }))
            .collect();

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            system: system.to_string(),
            messages: anthropic_messages,
            tools: anthropic_tools,
            stream: true,
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::llm(format!("Anthropic API error {}: {}", status, body)));
        }

        let mut text_buf = String::new();
        let mut tool_accums: BTreeMap<usize, ToolAccum> = BTreeMap::new();
        let mut byte_buf = Vec::<u8>::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AppError::llm(format!("Stream error: {}", e)))?;
            byte_buf.extend_from_slice(&chunk);

            // Process complete lines
            while let Some(nl) = byte_buf.iter().position(|&b| b == b'\n') {
                let line_bytes = byte_buf.drain(..=nl).collect::<Vec<u8>>();
                let line = String::from_utf8_lossy(&line_bytes);
                let line = line.trim();

                if let Some(data) = line.strip_prefix("data: ") {
                    let Ok(event): Result<Value, _> = serde_json::from_str(data) else {
                        continue;
                    };

                    match event["type"].as_str() {
                        Some("content_block_start") => {
                            let idx = event["index"].as_u64().unwrap_or(0) as usize;
                            if event["content_block"]["type"].as_str() == Some("tool_use") {
                                let id = event["content_block"]["id"].as_str().unwrap_or("").to_string();
                                let name = event["content_block"]["name"].as_str().unwrap_or("").to_string();
                                tool_accums.insert(idx, ToolAccum { id, name, input_json: String::new() });
                            }
                        }
                        Some("content_block_delta") => {
                            let idx = event["index"].as_u64().unwrap_or(0) as usize;
                            let delta = &event["delta"];
                            match delta["type"].as_str() {
                                Some("text_delta") => {
                                    if let Some(text) = delta["text"].as_str() {
                                        text_buf.push_str(text);
                                        if let Some(cb) = &on_token {
                                            cb(text.to_string());
                                        }
                                    }
                                }
                                Some("input_json_delta") => {
                                    if let Some(partial) = delta["partial_json"].as_str() {
                                        if let Some(accum) = tool_accums.get_mut(&idx) {
                                            accum.input_json.push_str(partial);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        Some("message_stop") => break,
                        _ => {}
                    }
                }
            }
        }

        let combined_text = text_buf.trim().to_string();

        let tool_calls: Vec<ToolCall> = tool_accums
            .into_values()
            .map(|accum| {
                let input: Value = serde_json::from_str(&accum.input_json).unwrap_or(Value::Object(Default::default()));
                ToolCall { id: accum.id, name: accum.name, input }
            })
            .collect();

        Ok(match (combined_text.is_empty(), tool_calls.is_empty()) {
            (true, false) => LlmTurn::ToolUse(tool_calls),
            (false, true) => LlmTurn::Text(combined_text),
            (false, false) => LlmTurn::TextAndToolUse { text: combined_text, tool_calls },
            (true, true) => LlmTurn::Text(String::new()),
        })
    }
}
