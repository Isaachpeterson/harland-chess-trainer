// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

/// Result returned to the frontend after syncing games.
#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub fetched: u32,
    pub new: u32,
    pub updated: u32,
}

/// Managed state: the storage handle, initialized once at startup.
struct AppState {
    storage: storage::Storage,
}

/// Resolves the path to the SQLite database in the Tauri app data directory.
fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("failed to create app data dir: {e}"))?;
    Ok(data_dir.join("harland.db"))
}

/// Fetches games from Lichess for the given username and stores them in the local database.
#[tauri::command]
async fn sync_games(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    username: String,
    max_games: u32,
) -> Result<SyncResult, String> {
    let client = lichess_client::LichessClient::new()
        .map_err(|e| format!("failed to create client: {e}"))?;

    let games = client
        .fetch_user_games(&username, max_games)
        .await
        .map_err(|e| format!("failed to fetch games: {e}"))?;

    let fetched = games.len() as u32;
    let mut new_count = 0u32;
    let mut updated_count = 0u32;

    let state = state.lock().await;

    for game in &games {
        let user_color = game
            .user_color(&username)
            .unwrap_or_else(|| "white".to_owned());
        let user_result = game
            .user_result(&username)
            .unwrap_or_else(|| "draw".to_owned());

        let time_control = game.clock.as_ref().map(|c| {
            format!(
                "{}+{}",
                c.initial.unwrap_or(0) / 1000,
                c.increment.unwrap_or(0)
            )
        });

        let analysis_source = if game.has_analysis() {
            Some("lichess".to_owned())
        } else {
            None
        };

        let insert = storage::GameInsert {
            id: game.id.clone(),
            pgn: game.pgn.clone().unwrap_or_default(),
            user_color,
            user_result,
            time_control,
            rated: game.rated,
            created_at: game.created_at / 1000, // Convert ms → seconds
            analysis_source,
        };

        let outcome = state
            .storage
            .insert_game(&insert)
            .await
            .map_err(|e| format!("failed to store game {}: {e}", game.id))?;

        if outcome.was_new {
            new_count += 1;
        } else {
            updated_count += 1;
        }
    }

    Ok(SyncResult {
        fetched,
        new: new_count,
        updated: updated_count,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let path = db_path(app.handle())?;
            let storage = tauri::async_runtime::block_on(storage::Storage::new(&path))
                .map_err(|e| format!("failed to initialize database: {e}"))?;

            app.manage(Arc::new(Mutex::new(AppState { storage })));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![sync_games])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
