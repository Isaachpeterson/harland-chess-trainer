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
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

/// Result returned to the frontend after syncing games.
#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub fetched: u32,
    pub new: u32,
    pub updated: u32,
}

/// Result returned after analyzing a single game.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeGameResult {
    pub game_id: String,
    pub evals_stored: u32,
    pub source: String,
}

/// Result returned after analyzing a batch of pending games.
#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeBatchResult {
    pub games_analyzed: u32,
    pub games_skipped: u32,
    pub total_evals: u32,
    pub errors: Vec<String>,
}

/// Result of detecting mistakes in a single game.
#[derive(Debug, Clone, Serialize)]
pub struct DetectMistakesResult {
    pub game_id: String,
    pub inaccuracies: u32,
    pub mistakes: u32,
    pub blunders: u32,
}

/// Result of detecting mistakes across all analyzed games.
#[derive(Debug, Clone, Serialize)]
pub struct DetectAllMistakesResult {
    pub games_processed: u32,
    pub total_inaccuracies: u32,
    pub total_mistakes: u32,
    pub total_blunders: u32,
    pub errors: Vec<String>,
}

/// Progress event payload emitted during batch analysis.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisProgress {
    pub game_id: String,
    pub games_done: u32,
    pub games_total: u32,
    pub status: String,
}

/// Managed state: the storage handle and optional engine, initialized at startup.
struct AppState {
    storage: storage::Storage,
    engine: Option<engine::Engine>,
}

impl AppState {
    /// Lazily initializes the Stockfish engine if not already running.
    async fn ensure_engine(&mut self) -> Result<&mut engine::Engine, String> {
        if self.engine.is_none() {
            let path = resolve_stockfish_path();
            let eng = engine::Engine::new(&path)
                .await
                .map_err(|e| format!("failed to start Stockfish at '{path}': {e}"))?;
            self.engine = Some(eng);
        }
        Ok(self.engine.as_mut().unwrap())
    }
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

/// Resolves the Stockfish binary path from env var or falls back to "stockfish" on PATH.
fn resolve_stockfish_path() -> String {
    std::env::var("STOCKFISH_PATH").unwrap_or_else(|_| "stockfish".to_owned())
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

/// Analyzes a single game, extracting Lichess evals from PGN or falling back to Stockfish.
#[tauri::command]
async fn analyze_game(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    game_id: String,
    force_stockfish: bool,
) -> Result<AnalyzeGameResult, String> {
    let mut state_guard = state.lock().await;

    // Load game
    let game = state_guard
        .storage
        .get_game(&game_id)
        .await
        .map_err(|e| format!("failed to load game: {e}"))?
        .ok_or_else(|| format!("game not found: {game_id}"))?;

    // Check if already analyzed
    if !force_stockfish && game.analysis_completed_at.is_some() {
        return Ok(AnalyzeGameResult {
            game_id,
            evals_stored: 0,
            source: "already_analyzed".to_owned(),
        });
    }

    // Parse PGN
    let parsed = chess_core::parse_pgn(&game.pgn, &game.user_color)
        .map_err(|e| format!("PGN parse error for game {game_id}: {e}"))?;

    // Collect Lichess evals from PGN, track missing plies
    let mut evals: Vec<storage::MoveEvaluation> = Vec::new();
    let mut needs_engine: Vec<&chess_core::ParsedMove> = Vec::new();

    if !force_stockfish {
        for mv in &parsed {
            if let Some(ref eval) = mv.lichess_eval {
                evals.push(storage::MoveEvaluation {
                    game_id: game_id.clone(),
                    ply: mv.ply as i32,
                    eval_cp: eval.eval_cp,
                    eval_mate: eval.eval_mate,
                    source: "lichess".to_owned(),
                });
            } else {
                needs_engine.push(mv);
            }
        }
    } else {
        needs_engine = parsed.iter().collect();
    }

    // Run Stockfish for plies without Lichess evals
    if !needs_engine.is_empty() {
        let eng = state_guard.ensure_engine().await?;
        let config = engine::AnalyzeConfig {
            depth: Some(20),
            movetime_ms: None,
            multipv: 1,
        };

        for mv in &needs_engine {
            let result = eng
                .analyze(&mv.fen_after, &config)
                .await
                .map_err(|e| format!("engine error at ply {}: {e}", mv.ply))?;

            // Engine scores are from the side-to-move's perspective.
            // After an even ply (White just moved), it's Black to move → negate.
            // After an odd ply (Black just moved), it's White to move → keep.
            let negate = mv.ply % 2 == 0;
            let eval_cp = result.score_cp.map(|cp| if negate { -cp } else { cp });
            let eval_mate = result.mate_in.map(|m| if negate { -m } else { m });

            evals.push(storage::MoveEvaluation {
                game_id: game_id.clone(),
                ply: mv.ply as i32,
                eval_cp,
                eval_mate,
                source: "stockfish".to_owned(),
            });
        }
    }

    let evals_stored = evals.len() as u32;

    // Persist evaluations
    state_guard
        .storage
        .insert_evaluations(&game_id, &evals)
        .await
        .map_err(|e| format!("failed to store evaluations: {e}"))?;

    // Determine the primary source
    let source = if needs_engine.is_empty() {
        "lichess"
    } else {
        "stockfish"
    };

    // Update analysis status
    state_guard
        .storage
        .update_analysis_status(&game_id, source)
        .await
        .map_err(|e| format!("failed to update analysis status: {e}"))?;

    Ok(AnalyzeGameResult {
        game_id,
        evals_stored,
        source: source.to_owned(),
    })
}

/// Analyzes all stored games that lack evaluations. Emits `analysis-progress` events.
#[tauri::command]
async fn analyze_pending_games(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    force_stockfish: bool,
) -> Result<AnalyzeBatchResult, String> {
    // Snapshot the list of unanalyzed games, then release the lock so other
    // commands (like sync_games) are not blocked for the entire batch.
    let games = {
        let guard = state.lock().await;
        guard
            .storage
            .list_unanalyzed_games()
            .await
            .map_err(|e| format!("failed to list unanalyzed games: {e}"))?
    };

    let games_total = games.len() as u32;
    let mut games_analyzed = 0u32;
    let mut games_skipped = 0u32;
    let mut total_evals = 0u32;
    let mut errors = Vec::new();

    for (idx, game) in games.iter().enumerate() {
        // Emit progress
        let _ = app.emit(
            "analysis-progress",
            AnalysisProgress {
                game_id: game.id.clone(),
                games_done: idx as u32,
                games_total,
                status: "analyzing".to_owned(),
            },
        );

        // Parse PGN (pure computation, no lock needed)
        let parsed = match chess_core::parse_pgn(&game.pgn, &game.user_color) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("game {}: PGN parse error: {e}", game.id));
                games_skipped += 1;
                continue;
            }
        };

        if parsed.is_empty() {
            games_skipped += 1;
            continue;
        }

        // Collect Lichess evals from PGN, track missing plies
        let mut evals: Vec<storage::MoveEvaluation> = Vec::new();
        let mut needs_engine_plies: Vec<(u32, String)> = Vec::new();

        if !force_stockfish {
            for mv in &parsed {
                if let Some(ref eval) = mv.lichess_eval {
                    evals.push(storage::MoveEvaluation {
                        game_id: game.id.clone(),
                        ply: mv.ply as i32,
                        eval_cp: eval.eval_cp,
                        eval_mate: eval.eval_mate,
                        source: "lichess".to_owned(),
                    });
                } else {
                    needs_engine_plies.push((mv.ply, mv.fen_after.clone()));
                }
            }
        } else {
            for mv in &parsed {
                needs_engine_plies.push((mv.ply, mv.fen_after.clone()));
            }
        }

        // Run Stockfish for missing plies (lock needed for engine access)
        if !needs_engine_plies.is_empty() {
            let mut state_guard = state.lock().await;
            let eng = match state_guard.ensure_engine().await {
                Ok(e) => e,
                Err(e) => {
                    errors.push(format!("game {}: {e}", game.id));
                    games_skipped += 1;
                    continue;
                }
            };

            let config = engine::AnalyzeConfig {
                depth: Some(20),
                movetime_ms: None,
                multipv: 1,
            };

            let mut engine_failed = false;
            for (ply, fen) in &needs_engine_plies {
                match eng.analyze(fen, &config).await {
                    Ok(result) => {
                        let negate = ply % 2 == 0;
                        let eval_cp = result.score_cp.map(|cp| if negate { -cp } else { cp });
                        let eval_mate = result.mate_in.map(|m| if negate { -m } else { m });

                        evals.push(storage::MoveEvaluation {
                            game_id: game.id.clone(),
                            ply: *ply as i32,
                            eval_cp,
                            eval_mate,
                            source: "stockfish".to_owned(),
                        });
                    }
                    Err(e) => {
                        errors.push(format!("game {} ply {}: engine error: {e}", game.id, ply));
                        engine_failed = true;
                        break;
                    }
                }
            }

            if engine_failed {
                games_skipped += 1;
                continue;
            }
            // state_guard dropped here, releasing the lock before DB writes
        }

        // Persist evaluations (short lock)
        {
            let guard = state.lock().await;
            if let Err(e) = guard.storage.insert_evaluations(&game.id, &evals).await {
                errors.push(format!("game {}: failed to store evals: {e}", game.id));
                games_skipped += 1;
                continue;
            }

            let source = if needs_engine_plies.is_empty() {
                "lichess"
            } else {
                "stockfish"
            };
            let _ = guard.storage.update_analysis_status(&game.id, source).await;
        }

        total_evals += evals.len() as u32;
        games_analyzed += 1;
    }

    // Emit final progress
    let _ = app.emit(
        "analysis-progress",
        AnalysisProgress {
            game_id: String::new(),
            games_done: games_total,
            games_total,
            status: "complete".to_owned(),
        },
    );

    Ok(AnalyzeBatchResult {
        games_analyzed,
        games_skipped,
        total_evals,
        errors,
    })
}

/// Runs blunder detection on stored evaluations for a single game.
///
/// Reads per-move evaluations, classifies each user move using `chess_core::classify_mistake`,
/// and persists detected mistakes.
#[tauri::command]
async fn detect_mistakes(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    game_id: String,
) -> Result<DetectMistakesResult, String> {
    let state_guard = state.lock().await;

    // Load game
    let game = state_guard
        .storage
        .get_game(&game_id)
        .await
        .map_err(|e| format!("failed to load game: {e}"))?
        .ok_or_else(|| format!("game not found: {game_id}"))?;

    // Load evaluations
    let evals = state_guard
        .storage
        .get_evaluations(&game_id)
        .await
        .map_err(|e| format!("failed to load evaluations: {e}"))?;

    if evals.is_empty() {
        return Ok(DetectMistakesResult {
            game_id,
            inaccuracies: 0,
            mistakes: 0,
            blunders: 0,
        });
    }

    // Parse PGN to get move data (fen_before, move_uci, is_user_move)
    let parsed = chess_core::parse_pgn(&game.pgn, &game.user_color)
        .map_err(|e| format!("PGN parse error: {e}"))?;

    // Build an eval lookup by ply
    let eval_by_ply: std::collections::HashMap<i32, &storage::MoveEvaluation> =
        evals.iter().map(|e| (e.ply, e)).collect();

    let user_is_white = game.user_color == "white";
    let thresholds = chess_core::MistakeThresholds::default();

    let mut mistake_inserts = Vec::new();
    let mut inaccuracies = 0u32;
    let mut mistake_count = 0u32;
    let mut blunder_count = 0u32;

    for mv in &parsed {
        if !mv.is_user_move {
            continue;
        }

        let ply = mv.ply as i32;

        // eval_before: evaluation of the position before the user's move.
        // This is the eval after the *previous* ply (opponent's move), stored at ply-1.
        // For the very first move (ply 0 or ply 1), if no prior eval exists, skip.
        let eval_before = if ply > 0 {
            eval_by_ply.get(&(ply - 1))
        } else {
            // Ply 0 = White's first move. No prior eval to compare against.
            None
        };

        let eval_after = eval_by_ply.get(&ply);

        // Need both evals to classify
        let (before, after) = match (eval_before, eval_after) {
            (Some(b), Some(a)) => (b, a),
            _ => continue,
        };

        let classification = chess_core::classify_mistake(
            before.eval_cp,
            before.eval_mate,
            after.eval_cp,
            after.eval_mate,
            user_is_white,
            &thresholds,
        );

        if let Some(class) = classification {
            let class_str = match class {
                chess_core::MistakeClassification::Inaccuracy => {
                    inaccuracies += 1;
                    "inaccuracy"
                }
                chess_core::MistakeClassification::Mistake => {
                    mistake_count += 1;
                    "mistake"
                }
                chess_core::MistakeClassification::Blunder => {
                    blunder_count += 1;
                    "blunder"
                }
            };

            mistake_inserts.push(storage::MistakeInsert {
                game_id: game_id.clone(),
                ply,
                fen_before: mv.fen_before.clone(),
                user_move: mv.move_uci.clone(),
                best_move: String::new(), // Populated in Slice 5 during puzzle generation
                eval_before_cp: before.eval_cp,
                eval_before_mate: before.eval_mate,
                eval_after_cp: after.eval_cp,
                eval_after_mate: after.eval_mate,
                classification: class_str.to_owned(),
            });
        }
    }

    // Persist mistakes
    state_guard
        .storage
        .insert_mistakes(&game_id, &mistake_inserts)
        .await
        .map_err(|e| format!("failed to store mistakes: {e}"))?;

    Ok(DetectMistakesResult {
        game_id,
        inaccuracies,
        mistakes: mistake_count,
        blunders: blunder_count,
    })
}

/// Runs blunder detection on all analyzed games.
#[tauri::command]
async fn detect_all_mistakes(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<DetectAllMistakesResult, String> {
    let game_ids: Vec<String> = {
        let guard = state.lock().await;
        guard
            .storage
            .list_analyzed_games()
            .await
            .map_err(|e| format!("failed to list analyzed games: {e}"))?
            .into_iter()
            .map(|g| g.id)
            .collect()
    };

    let mut games_processed = 0u32;
    let mut total_inaccuracies = 0u32;
    let mut total_mistakes = 0u32;
    let mut total_blunders = 0u32;
    let mut errors = Vec::new();

    for game_id in &game_ids {
        // Re-acquire lock for each game to avoid holding it across long operations
        let result = {
            let state_guard = state.lock().await;

            let game = match state_guard.storage.get_game(game_id).await {
                Ok(Some(g)) => g,
                Ok(None) => {
                    errors.push(format!("game {game_id} not found"));
                    continue;
                }
                Err(e) => {
                    errors.push(format!("game {game_id}: {e}"));
                    continue;
                }
            };

            let evals = match state_guard.storage.get_evaluations(game_id).await {
                Ok(e) => e,
                Err(e) => {
                    errors.push(format!("game {game_id}: failed to load evals: {e}"));
                    continue;
                }
            };

            if evals.is_empty() {
                continue;
            }

            let parsed = match chess_core::parse_pgn(&game.pgn, &game.user_color) {
                Ok(p) => p,
                Err(e) => {
                    errors.push(format!("game {game_id}: PGN parse error: {e}"));
                    continue;
                }
            };

            let eval_by_ply: std::collections::HashMap<i32, &storage::MoveEvaluation> =
                evals.iter().map(|e| (e.ply, e)).collect();

            let user_is_white = game.user_color == "white";
            let thresholds = chess_core::MistakeThresholds::default();

            let mut mistake_inserts = Vec::new();
            let mut inacc = 0u32;
            let mut mist = 0u32;
            let mut blund = 0u32;

            for mv in &parsed {
                if !mv.is_user_move {
                    continue;
                }

                let ply = mv.ply as i32;

                let eval_before = if ply > 0 {
                    eval_by_ply.get(&(ply - 1))
                } else {
                    None
                };
                let eval_after = eval_by_ply.get(&ply);

                let (before, after) = match (eval_before, eval_after) {
                    (Some(b), Some(a)) => (b, a),
                    _ => continue,
                };

                let classification = chess_core::classify_mistake(
                    before.eval_cp,
                    before.eval_mate,
                    after.eval_cp,
                    after.eval_mate,
                    user_is_white,
                    &thresholds,
                );

                if let Some(class) = classification {
                    let class_str = match class {
                        chess_core::MistakeClassification::Inaccuracy => {
                            inacc += 1;
                            "inaccuracy"
                        }
                        chess_core::MistakeClassification::Mistake => {
                            mist += 1;
                            "mistake"
                        }
                        chess_core::MistakeClassification::Blunder => {
                            blund += 1;
                            "blunder"
                        }
                    };

                    mistake_inserts.push(storage::MistakeInsert {
                        game_id: game_id.clone(),
                        ply,
                        fen_before: mv.fen_before.clone(),
                        user_move: mv.move_uci.clone(),
                        best_move: String::new(),
                        eval_before_cp: before.eval_cp,
                        eval_before_mate: before.eval_mate,
                        eval_after_cp: after.eval_cp,
                        eval_after_mate: after.eval_mate,
                        classification: class_str.to_owned(),
                    });
                }
            }

            if let Err(e) = state_guard
                .storage
                .insert_mistakes(game_id, &mistake_inserts)
                .await
            {
                errors.push(format!("game {game_id}: failed to store mistakes: {e}"));
                continue;
            }

            (inacc, mist, blund)
        };

        total_inaccuracies += result.0;
        total_mistakes += result.1;
        total_blunders += result.2;
        games_processed += 1;
    }

    Ok(DetectAllMistakesResult {
        games_processed,
        total_inaccuracies,
        total_mistakes,
        total_blunders,
        errors,
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

            app.manage(Arc::new(Mutex::new(AppState {
                storage,
                engine: None,
            })));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            sync_games,
            analyze_game,
            analyze_pending_games,
            detect_mistakes,
            detect_all_mistakes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
