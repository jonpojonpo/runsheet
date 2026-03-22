use crate::duckdb_engine::DuckDbEngine;
use crate::error::AppError;
use crate::llm::{LlmConfig, SettingsPayload};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct AppState {
    pub db: Mutex<DuckDbEngine>,
    pub llm_config: Mutex<LlmConfig>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self, AppError> {
        let db = DuckDbEngine::new()?;
        let mut llm_config = LlmConfig::from_env();

        // Load persisted settings if present — overrides env vars
        if let Ok(dir) = app.path().app_config_dir() {
            let path = dir.join("settings.json");
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(saved) = serde_json::from_str::<SettingsPayload>(&json) {
                    llm_config.apply_settings(saved);
                }
            }
        }

        Ok(Self {
            db: Mutex::new(db),
            llm_config: Mutex::new(llm_config),
        })
    }
}
