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

/// Result of generating puzzles from blunders.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratePuzzlesResult {
    pub puzzles_created: u32,
    pub puzzles_rejected: u32,
    pub puzzles_skipped: u32,
    pub errors: Vec<String>,
}

/// A puzzle returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct PuzzleResponse {
    pub id: i64,
    pub fen: String,
    pub solution_moves: Vec<String>,
}

/// Result of submitting a puzzle attempt.
#[derive(Debug, Clone, Serialize)]
pub struct SubmitAttemptResult {
    pub attempt_id: i64,
}

/// Aggregate puzzle attempt statistics.
#[derive(Debug, Clone, Serialize)]
pub struct AttemptsSummaryResponse {
    pub total_attempts: i64,
    pub total_successes: i64,
    pub success_rate: f64,
    pub puzzles_attempted: i64,
    pub puzzles_attempted_today: i64,
}

/// Progress event payload emitted during batch analysis.
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisProgress {
    pub game_id: String,
    pub games_done: u32,
    pub games_total: u32,
    pub status: String,
}

/// Structured progress event emitted by `full_sync` at each pipeline stage.
#[derive(Debug, Clone, Serialize)]
pub struct SyncProgress {
    /// Stage name: "fetching" | "analyzing" | "detecting" | "generating" | "complete" | "error"
    pub stage: String,
    /// Human-readable status message for the UI.
    pub message: String,
    /// Fraction complete: 0.0–1.0.
    pub fraction: f64,
}

/// Combined result returned by the `full_sync` command.
#[derive(Debug, Clone, Serialize)]
pub struct FullSyncResult {
    pub fetched: u32,
    pub new_games: u32,
    pub games_analyzed: u32,
    pub total_blunders: u32,
    pub puzzles_created: u32,
    pub errors: Vec<String>,
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

/// Generates training puzzles from all detected blunders that don't already have puzzles.
///
/// For each blunder, re-analyzes the pre-blunder position with multi-PV, applies quality
/// filters, and stores accepted puzzles. Also backfills the `best_move` column on mistakes.
#[tauri::command]
async fn generate_puzzles(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<GeneratePuzzlesResult, String> {
    // Load all blunders
    let blunders = {
        let guard = state.lock().await;
        guard
            .storage
            .list_blunders(10_000, 0)
            .await
            .map_err(|e| format!("failed to list blunders: {e}"))?
    };

    // Filter out blunders that already have puzzles
    let mut inputs = Vec::new();
    let mut skipped = 0u32;
    {
        let guard = state.lock().await;
        for blunder in &blunders {
            let exists = guard
                .storage
                .puzzle_exists_for_mistake(blunder.id)
                .await
                .map_err(|e| format!("failed to check puzzle existence: {e}"))?;
            if exists {
                skipped += 1;
                continue;
            }

            // To detect recaptures, we need the previous move. Parse PGN to find it.
            let previous_move_uci = {
                let game = guard
                    .storage
                    .get_game(&blunder.game_id)
                    .await
                    .map_err(|e| format!("failed to load game: {e}"))?;
                if let Some(game) = game {
                    chess_core::parse_pgn(&game.pgn, &game.user_color)
                        .ok()
                        .and_then(|parsed| {
                            let prev_ply = (blunder.ply - 1) as u32;
                            parsed
                                .iter()
                                .find(|m| m.ply == prev_ply)
                                .map(|m| m.move_uci.clone())
                        })
                } else {
                    None
                }
            };

            inputs.push(puzzle_gen::BlunderInput {
                mistake_id: blunder.id,
                game_id: blunder.game_id.clone(),
                ply: blunder.ply,
                fen_before: blunder.fen_before.clone(),
                user_move: blunder.user_move.clone(),
                previous_move_uci,
            });
        }
    }

    if inputs.is_empty() {
        return Ok(GeneratePuzzlesResult {
            puzzles_created: 0,
            puzzles_rejected: 0,
            puzzles_skipped: skipped,
            errors: Vec::new(),
        });
    }

    // Run puzzle generation (needs mutable engine access)
    let results = {
        let mut guard = state.lock().await;
        let eng = guard.ensure_engine().await?;
        let config = puzzle_gen::PuzzleGenConfig::default();
        puzzle_gen::generate_puzzles(&inputs, eng, &config).await
    };

    // Store accepted puzzles and update best_move on mistakes
    let mut created = 0u32;
    let mut rejected = 0u32;
    let mut errors = Vec::new();

    let guard = state.lock().await;
    for result in results {
        match result {
            puzzle_gen::PuzzleGenResult::Accepted(candidate) => {
                let insert = storage::PuzzleInsert {
                    mistake_id: candidate.mistake_id,
                    fen: candidate.fen,
                    solution_moves: serde_json::to_string(&candidate.solution_uci_moves)
                        .unwrap_or_default(),
                    themes: if candidate.themes.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(&candidate.themes).unwrap_or_default())
                    },
                };

                match guard.storage.insert_puzzle(&insert).await {
                    Ok(_) => {
                        // Backfill best_move on the mistake
                        if let Err(e) = guard
                            .storage
                            .update_mistake_best_move(
                                candidate.mistake_id,
                                &candidate.best_move_uci,
                            )
                            .await
                        {
                            errors.push(format!(
                                "mistake {}: failed to update best_move: {e}",
                                candidate.mistake_id
                            ));
                        }
                        created += 1;
                    }
                    Err(e) => {
                        errors.push(format!(
                            "mistake {}: failed to store puzzle: {e}",
                            candidate.mistake_id
                        ));
                    }
                }
            }
            puzzle_gen::PuzzleGenResult::Rejected { .. } => {
                rejected += 1;
            }
        }
    }

    Ok(GeneratePuzzlesResult {
        puzzles_created: created,
        puzzles_rejected: rejected,
        puzzles_skipped: skipped,
        errors,
    })
}

/// Returns the next puzzle to solve. Prefers unattempted puzzles; falls back to
/// already-attempted ones if all have been seen.
#[tauri::command]
async fn get_next_puzzle(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<PuzzleResponse>, String> {
    let guard = state.lock().await;
    let puzzle = guard
        .storage
        .get_next_puzzle()
        .await
        .map_err(|e| format!("failed to get next puzzle: {e}"))?;

    Ok(puzzle.map(|p| {
        let solution_moves: Vec<String> =
            serde_json::from_str(&p.solution_moves).unwrap_or_default();
        PuzzleResponse {
            id: p.id,
            fen: p.fen,
            solution_moves,
        }
    }))
}

/// Records a puzzle attempt.
#[tauri::command]
async fn submit_puzzle_attempt(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    puzzle_id: i64,
    success: bool,
    time_taken_ms: i64,
    move_played: String,
) -> Result<SubmitAttemptResult, String> {
    let guard = state.lock().await;
    let attempt_id = guard
        .storage
        .record_attempt(puzzle_id, success, time_taken_ms, &move_played)
        .await
        .map_err(|e| format!("failed to record attempt: {e}"))?;

    Ok(SubmitAttemptResult { attempt_id })
}

/// Returns aggregate puzzle attempt statistics.
#[tauri::command]
async fn get_attempts_summary(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<AttemptsSummaryResponse, String> {
    let guard = state.lock().await;
    let summary = guard
        .storage
        .get_attempts_summary()
        .await
        .map_err(|e| format!("failed to get attempts summary: {e}"))?;

    Ok(AttemptsSummaryResponse {
        total_attempts: summary.total_attempts,
        total_successes: summary.total_successes,
        success_rate: summary.success_rate,
        puzzles_attempted: summary.puzzles_attempted,
        puzzles_attempted_today: summary.puzzles_attempted_today,
    })
}

/// Returns the user's settings from the database.
#[tauri::command]
async fn get_settings(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<storage::UserSettings, String> {
    let guard = state.lock().await;
    guard
        .storage
        .get_settings()
        .await
        .map_err(|e| format!("failed to load settings: {e}"))
}

/// Persists the user's settings to the database.
#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    settings: storage::UserSettings,
) -> Result<(), String> {
    let guard = state.lock().await;
    guard
        .storage
        .save_settings(&settings)
        .await
        .map_err(|e| format!("failed to save settings: {e}"))
}

/// Runs the full sync pipeline: fetch → analyze → detect → generate.
///
/// Reads username and preferences from the stored settings. Emits `"sync-progress"` events
/// at each stage so the frontend can show a progress bar.
#[tauri::command]
async fn full_sync(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
) -> Result<FullSyncResult, String> {
    // --- Load settings ---
    let settings = {
        let guard = state.lock().await;
        guard
            .storage
            .get_settings()
            .await
            .map_err(|e| format!("failed to load settings: {e}"))?
    };

    if settings.lichess_username.trim().is_empty() {
        return Err(
            "No Lichess username configured. Please set your username in Settings.".to_owned(),
        );
    }

    let username = settings.lichess_username.trim().to_owned();
    let max_games = settings.max_games as u32;
    let force_stockfish = settings.use_stockfish;
    let mut all_errors: Vec<String> = Vec::new();

    // --- Stage 1: Fetch games ---
    let _ = app.emit(
        "sync-progress",
        SyncProgress {
            stage: "fetching".to_owned(),
            message: format!("Fetching up to {max_games} games for {username}…"),
            fraction: 0.0,
        },
    );

    let client = lichess_client::LichessClient::new()
        .map_err(|e| format!("failed to create Lichess client: {e}"))?;

    let games = client
        .fetch_user_games(&username, max_games)
        .await
        .map_err(|e| format!("failed to fetch games: {e}"))?;

    let fetched = games.len() as u32;
    let mut new_count = 0u32;

    {
        let guard = state.lock().await;
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
                created_at: game.created_at / 1000,
                analysis_source,
            };

            match guard.storage.insert_game(&insert).await {
                Ok(outcome) => {
                    if outcome.was_new {
                        new_count += 1;
                    }
                }
                Err(e) => {
                    all_errors.push(format!("failed to store game {}: {e}", game.id));
                }
            }
        }
    }

    // --- Stage 2: Analyze games ---
    let _ = app.emit(
        "sync-progress",
        SyncProgress {
            stage: "analyzing".to_owned(),
            message: format!("Analyzing {fetched} games…"),
            fraction: 0.25,
        },
    );

    let unanalyzed = {
        let guard = state.lock().await;
        guard
            .storage
            .list_unanalyzed_games()
            .await
            .map_err(|e| format!("failed to list unanalyzed games: {e}"))?
    };

    let games_total = unanalyzed.len() as u32;
    let mut games_analyzed = 0u32;

    for (idx, game) in unanalyzed.iter().enumerate() {
        let _ = app.emit(
            "sync-progress",
            SyncProgress {
                stage: "analyzing".to_owned(),
                message: format!("Analyzing game {} of {}…", idx + 1, games_total),
                fraction: 0.25 + 0.25 * (idx as f64 / games_total.max(1) as f64),
            },
        );

        let parsed = match chess_core::parse_pgn(&game.pgn, &game.user_color) {
            Ok(p) if !p.is_empty() => p,
            Ok(_) => continue,
            Err(e) => {
                all_errors.push(format!("game {}: PGN parse error: {e}", game.id));
                continue;
            }
        };

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

        if !needs_engine_plies.is_empty() {
            let mut guard = state.lock().await;
            let eng = match guard.ensure_engine().await {
                Ok(e) => e,
                Err(e) => {
                    all_errors.push(format!("game {}: {e}", game.id));
                    continue;
                }
            };

            let config = engine::AnalyzeConfig {
                depth: Some(20),
                movetime_ms: None,
                multipv: 1,
            };

            let mut failed = false;
            for (ply, fen) in &needs_engine_plies {
                match eng.analyze(fen, &config).await {
                    Ok(result) => {
                        let negate = ply % 2 == 0;
                        evals.push(storage::MoveEvaluation {
                            game_id: game.id.clone(),
                            ply: *ply as i32,
                            eval_cp: result.score_cp.map(|cp| if negate { -cp } else { cp }),
                            eval_mate: result.mate_in.map(|m| if negate { -m } else { m }),
                            source: "stockfish".to_owned(),
                        });
                    }
                    Err(e) => {
                        all_errors.push(format!("game {} ply {ply}: engine error: {e}", game.id));
                        failed = true;
                        break;
                    }
                }
            }
            if failed {
                continue;
            }
        }

        {
            let guard = state.lock().await;
            let source = if needs_engine_plies.is_empty() {
                "lichess"
            } else {
                "stockfish"
            };
            if let Err(e) = guard.storage.insert_evaluations(&game.id, &evals).await {
                all_errors.push(format!("game {}: failed to store evals: {e}", game.id));
                continue;
            }
            let _ = guard.storage.update_analysis_status(&game.id, source).await;
        }

        games_analyzed += 1;
    }

    // --- Stage 3: Detect blunders ---
    let _ = app.emit(
        "sync-progress",
        SyncProgress {
            stage: "detecting".to_owned(),
            message: "Detecting blunders…".to_owned(),
            fraction: 0.5,
        },
    );

    let analyzed_games = {
        let guard = state.lock().await;
        guard
            .storage
            .list_analyzed_games()
            .await
            .map_err(|e| format!("failed to list analyzed games: {e}"))?
    };

    let mut total_blunders = 0u32;

    for game in &analyzed_games {
        let guard = state.lock().await;

        let evals = match guard.storage.get_evaluations(&game.id).await {
            Ok(e) if !e.is_empty() => e,
            _ => continue,
        };

        let parsed = match chess_core::parse_pgn(&game.pgn, &game.user_color) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let eval_by_ply: std::collections::HashMap<i32, &storage::MoveEvaluation> =
            evals.iter().map(|e| (e.ply, e)).collect();

        let user_is_white = game.user_color == "white";
        let thresholds = chess_core::MistakeThresholds::default();
        let mut mistake_inserts = Vec::new();

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

            if let Some(class) = chess_core::classify_mistake(
                before.eval_cp,
                before.eval_mate,
                after.eval_cp,
                after.eval_mate,
                user_is_white,
                &thresholds,
            ) {
                let class_str = match class {
                    chess_core::MistakeClassification::Inaccuracy => "inaccuracy",
                    chess_core::MistakeClassification::Mistake => "mistake",
                    chess_core::MistakeClassification::Blunder => {
                        total_blunders += 1;
                        "blunder"
                    }
                };
                mistake_inserts.push(storage::MistakeInsert {
                    game_id: game.id.clone(),
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

        if let Err(e) = guard
            .storage
            .insert_mistakes(&game.id, &mistake_inserts)
            .await
        {
            all_errors.push(format!("game {}: failed to store mistakes: {e}", game.id));
        }
    }

    // --- Stage 4: Generate puzzles ---
    let _ = app.emit(
        "sync-progress",
        SyncProgress {
            stage: "generating".to_owned(),
            message: "Generating puzzles from blunders…".to_owned(),
            fraction: 0.75,
        },
    );

    let blunders = {
        let guard = state.lock().await;
        guard
            .storage
            .list_blunders(10_000, 0)
            .await
            .map_err(|e| format!("failed to list blunders: {e}"))?
    };

    let mut puzzle_inputs = Vec::new();
    {
        let guard = state.lock().await;
        for blunder in &blunders {
            let exists = guard
                .storage
                .puzzle_exists_for_mistake(blunder.id)
                .await
                .unwrap_or(true); // skip on error
            if exists {
                continue;
            }

            let previous_move_uci = guard
                .storage
                .get_game(&blunder.game_id)
                .await
                .ok()
                .flatten()
                .and_then(|game| {
                    chess_core::parse_pgn(&game.pgn, &game.user_color)
                        .ok()
                        .and_then(|parsed| {
                            let prev_ply = (blunder.ply - 1) as u32;
                            parsed
                                .iter()
                                .find(|m| m.ply == prev_ply)
                                .map(|m| m.move_uci.clone())
                        })
                });

            puzzle_inputs.push(puzzle_gen::BlunderInput {
                mistake_id: blunder.id,
                game_id: blunder.game_id.clone(),
                ply: blunder.ply,
                fen_before: blunder.fen_before.clone(),
                user_move: blunder.user_move.clone(),
                previous_move_uci,
            });
        }
    }

    let mut puzzles_created = 0u32;

    if !puzzle_inputs.is_empty() {
        let results = {
            let mut guard = state.lock().await;
            let eng = guard.ensure_engine().await?;
            let config = puzzle_gen::PuzzleGenConfig::default();
            puzzle_gen::generate_puzzles(&puzzle_inputs, eng, &config).await
        };

        let guard = state.lock().await;
        for result in results {
            if let puzzle_gen::PuzzleGenResult::Accepted(candidate) = result {
                let insert = storage::PuzzleInsert {
                    mistake_id: candidate.mistake_id,
                    fen: candidate.fen,
                    solution_moves: serde_json::to_string(&candidate.solution_uci_moves)
                        .unwrap_or_default(),
                    themes: if candidate.themes.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(&candidate.themes).unwrap_or_default())
                    },
                };
                if guard.storage.insert_puzzle(&insert).await.is_ok() {
                    let _ = guard
                        .storage
                        .update_mistake_best_move(candidate.mistake_id, &candidate.best_move_uci)
                        .await;
                    puzzles_created += 1;
                }
            }
        }
    }

    // --- Complete ---
    let _ = app.emit(
        "sync-progress",
        SyncProgress {
            stage: "complete".to_owned(),
            message: format!(
                "Done! Fetched {fetched} games, generated {puzzles_created} new puzzles."
            ),
            fraction: 1.0,
        },
    );

    Ok(FullSyncResult {
        fetched,
        new_games: new_count,
        games_analyzed,
        total_blunders,
        puzzles_created,
        errors: all_errors,
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
            detect_all_mistakes,
            generate_puzzles,
            get_next_puzzle,
            submit_puzzle_attempt,
            get_attempts_summary,
            get_settings,
            save_settings,
            full_sync
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
