use crate::error::AppError;
use crate::llm::SettingsPayload;
use crate::state::AppState;
use tauri::{AppHandle, Manager, State};

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| AppError::io(format!("Cannot resolve config dir: {e}")))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| AppError::io(format!("Cannot create config dir: {e}")))?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<SettingsPayload, AppError> {
    let config = state.llm_config.lock().unwrap_or_else(|e| e.into_inner());
    Ok(SettingsPayload::from(&*config))
}

#[tauri::command]
pub async fn save_settings(
    settings: SettingsPayload,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Update in-memory state (lock dropped before I/O)
    {
        let mut config = state.llm_config.lock().unwrap_or_else(|e| e.into_inner());
        config.apply_settings(settings.clone());
    }
    // Persist to disk
    let path = settings_path(&app)?;
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| AppError::io(format!("Serialization error: {e}")))?;
    std::fs::write(&path, json)
        .map_err(|e| AppError::io(format!("Failed to write settings: {e}")))?;
    Ok(())
}
