mod commands;
mod db;
mod deepseek;
mod models;
mod research;
mod search;
mod secrets;
mod security;
mod tools;

use commands::{
    create_conversation, delete_conversation, delete_secret, get_conversations, get_messages,
    get_settings, has_secret, save_message, save_settings, send_message, set_secret,
    stop_generation, update_conversation,
};
use db::Database;
use research::{
    cancel_research_task, export_research_task, get_research_task, get_research_tasks,
    pause_research_task, prepare_research_task, resume_research_task, start_research_task,
    ResearchRunControl,
};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::Manager;
use tokio_util::sync::CancellationToken;

pub struct AppState {
    db: Database,
    cancellations: Mutex<HashMap<String, CancellationToken>>,
    research_runs: Mutex<HashMap<String, ResearchRunControl>>,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("无法创建应用数据目录: {error}"))?;
            std::fs::create_dir_all(&app_dir)
                .map_err(|error| format!("无法创建应用数据目录: {error}"))?;
            let db = Database::new(app_dir.join("deepseek-desktop.sqlite3"))
                .map_err(|error| error.to_string())?;
            app.manage(AppState {
                db,
                cancellations: Mutex::new(HashMap::new()),
                research_runs: Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_conversations,
            create_conversation,
            update_conversation,
            delete_conversation,
            get_messages,
            save_message,
            get_settings,
            save_settings,
            set_secret,
            has_secret,
            delete_secret,
            send_message,
            stop_generation,
            prepare_research_task,
            start_research_task,
            pause_research_task,
            resume_research_task,
            cancel_research_task,
            get_research_task,
            get_research_tasks,
            export_research_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
