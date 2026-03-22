mod commands;
mod duckdb_engine;
mod error;
mod llm;
mod state;

use commands::{
    chat::chat,
    data::{drop_table, get_schema, ingest_file, run_sql},
    models::list_models,
    settings::{get_settings, save_settings},
};
use state::AppState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // System tray
            tauri::tray::TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Runsheet")
                .build(app)?;

            let state = AppState::new(app.handle()).expect("Failed to initialise app state");
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ingest_file,
            run_sql,
            get_schema,
            drop_table,
            chat,
            get_settings,
            save_settings,
            list_models,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
