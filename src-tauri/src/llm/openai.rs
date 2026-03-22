//! OpenAI provider stub — implements the LlmProvider trait using the OpenAI Chat Completions API.
use super::{LlmProvider, LlmTurn, ToolCall, ToolDefinition, TokenCallback};
use crate::error::AppError;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    client: Client,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: Client::new(),
            base_url: "https://api.openai.com".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self { api_key, model, client: Client::new(), base_url }
    }

    /// Convert provider-agnostic message to OpenAI format.
    fn to_openai_message(msg: &Value) -> Value {
        let role = msg["role"].as_str().unwrap_or("user");

        if let Some(tool_use_arr) = msg.get("tool_use").and_then(|v| v.as_array()) {
            // Assistant with tool calls
            let tool_calls: Vec<Value> = tool_use_arr.iter().map(|tc| json!({
                "id": tc["id"],
                "type": "function",
                "function": {
                    "name": tc["name"],
                    "arguments": serde_json::to_string(&tc["input"]).unwrap_or_default(),
                }
            })).collect();
            json!({ "role": "assistant", "content": null, "tool_calls": tool_calls })
        } else if let Some(results_arr) = msg.get("tool_results").and_then(|v| v.as_array()) {
            // Tool results — one message per result in OpenAI
            // Return the first one; caller should iterate if multiple
            // For simplicity, we serialize all into one representation
            if let Some(first) = results_arr.first() {
                json!({
                    "role": "tool",
                    "tool_call_id": first["tool_use_id"],
                    "content": first["content"].as_str().unwrap_or(""),
                })
            } else {
                json!({ "role": "user", "content": "" })
            }
        } else {
            let content = msg["content"].as_str().unwrap_or("");
            json!({ "role": role, "content": content })
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(
        &self,
        system: &str,
        messages: Vec<Value>,
        tools: Vec<ToolDefinition>,
        _on_token: TokenCallback,
    ) -> Result<LlmTurn, AppError> {
        let mut oai_messages: Vec<Value> = vec![
            json!({ "role": "system", "content": system }),
        ];
        for m in &messages {
            oai_messages.push(Self::to_openai_message(m));
        }

        let oai_tools: Vec<Value> = tools.iter().map(|t| json!({
            "type": "function",
            "function": {
                "name": t.name,
                "description": t.description,
                "parameters": t.input_schema,
            }
        })).collect();

        let body = json!({
            "model": self.model,
            "messages": oai_messages,
            "tools": oai_tools,
            "max_tokens": 4096,
        });

        let resp = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::llm(format!("OpenAI API error {}: {}", status, body)));
        }

        let parsed: Value = resp.json().await?;
        let choice = &parsed["choices"][0]["message"];
        let finish_reason = parsed["choices"][0]["finish_reason"].as_str().unwrap_or("");

        if finish_reason == "tool_calls" {
            let tool_calls: Vec<ToolCall> = choice["tool_calls"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .map(|tc| {
                    let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                    let input: Value = serde_json::from_str(args_str).unwrap_or(Value::Null);
                    ToolCall {
                        id: tc["id"].as_str().unwrap_or("").to_string(),
                        name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                        input,
                    }
                })
                .collect();
            Ok(LlmTurn::ToolUse(tool_calls))
        } else {
            let text = choice["content"].as_str().unwrap_or("").to_string();
            Ok(LlmTurn::Text(text))
        }
    }
}
