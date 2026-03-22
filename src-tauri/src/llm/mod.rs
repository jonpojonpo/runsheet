pub mod anthropic;
pub mod openai;
pub mod mistral;

use crate::error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Settings payload — serialized to disk and exchanged with the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsPayload {
    pub provider: ProviderKind,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
    pub mistral_api_key: Option<String>,
    pub mistral_model: String,
}

/// Optional streaming callback — called once per text token as it arrives.
pub type TokenCallback = Option<Arc<dyn Fn(String) + Send + Sync>>;

/// A simple text message — used for the public-facing chat command input/output
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,   // "user" | "assistant"
    pub content: String,
}

/// A tool call the LLM wants to make
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// The final response from the LLM after the tool loop completes
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LlmResponse {
    pub text: String,
    pub tool_calls_made: Vec<ToolCallRecord>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub input: Value,
    pub result: Value,
}

/// The pluggable LLM provider trait.
/// messages is a Vec<Value> using a provider-agnostic convention:
///   - Text message:       {"role": "user"|"assistant", "content": "string"}
///   - Tool use (asst):    {"role": "assistant", "tool_use": [{"id":..,"name":..,"input":..}]}
///   - Tool result (user): {"role": "user", "tool_results": [{"tool_use_id":..,"content":..,"is_error":bool}]}
#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(
        &self,
        system: &str,
        messages: Vec<Value>,
        tools: Vec<ToolDefinition>,
        on_token: TokenCallback,
    ) -> Result<LlmTurn, AppError>;

    fn name(&self) -> &str;
}

/// A single turn result — either text or a tool call request
#[derive(Debug)]
pub enum LlmTurn {
    Text(String),
    ToolUse(Vec<ToolCall>),
    TextAndToolUse { text: String, tool_calls: Vec<ToolCall> },
}

/// Tool definition sent to the LLM
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Which provider is active + config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: ProviderKind,
    pub anthropic_api_key: Option<String>,
    pub anthropic_model: String,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
    pub mistral_api_key: Option<String>,
    pub mistral_model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Anthropic,
    OpenAi,
    Mistral,
}

impl LlmConfig {
    pub fn from_env() -> Self {
        Self {
            provider: ProviderKind::Anthropic,
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            anthropic_model: "claude-haiku-4-5-20251001".to_string(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            openai_model: "gpt-4o-mini".to_string(),
            mistral_api_key: std::env::var("MISTRAL_API_KEY").ok(),
            mistral_model: "mistral-small-latest".to_string(),
        }
    }

    pub fn apply_settings(&mut self, s: SettingsPayload) {
        self.provider = s.provider;
        if s.anthropic_api_key.is_some() { self.anthropic_api_key = s.anthropic_api_key; }
        self.anthropic_model = s.anthropic_model;
        if s.openai_api_key.is_some() { self.openai_api_key = s.openai_api_key; }
        self.openai_model = s.openai_model;
        if s.mistral_api_key.is_some() { self.mistral_api_key = s.mistral_api_key; }
        self.mistral_model = s.mistral_model;
    }

    pub fn build_provider(&self) -> Result<Box<dyn LlmProvider>, AppError> {
        match self.provider {
            ProviderKind::Anthropic => {
                let key = self.anthropic_api_key.clone()
                    .ok_or_else(|| AppError::config("ANTHROPIC_API_KEY not set"))?;
                Ok(Box::new(anthropic::AnthropicProvider::new(key, self.anthropic_model.clone())))
            }
            ProviderKind::OpenAi => {
                let key = self.openai_api_key.clone()
                    .ok_or_else(|| AppError::config("OPENAI_API_KEY not set"))?;
                Ok(Box::new(openai::OpenAiProvider::new(key, self.openai_model.clone())))
            }
            ProviderKind::Mistral => {
                let key = self.mistral_api_key.clone()
                    .ok_or_else(|| AppError::config("MISTRAL_API_KEY not set"))?;
                Ok(Box::new(mistral::MistralProvider::new(key, self.mistral_model.clone())))
            }
        }
    }
}

impl From<&LlmConfig> for SettingsPayload {
    fn from(c: &LlmConfig) -> Self {
        Self {
            provider: c.provider.clone(),
            anthropic_api_key: c.anthropic_api_key.clone(),
            anthropic_model: c.anthropic_model.clone(),
            openai_api_key: c.openai_api_key.clone(),
            openai_model: c.openai_model.clone(),
            mistral_api_key: c.mistral_api_key.clone(),
            mistral_model: c.mistral_model.clone(),
        }
    }
}
