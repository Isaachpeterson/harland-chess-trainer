// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! # storage
//!
//! SQLite persistence layer for Harland Chess Trainer. Owns the database schema
//! and migrations. Exports typed query functions for games, puzzles, mistakes,
//! and puzzle attempts. No raw SQL leaks into other crates.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

/// A game record as stored in the database.
#[derive(Debug, Clone)]
pub struct StoredGame {
    pub id: String,
    pub pgn: String,
    pub user_color: String,
    pub user_result: String,
    pub time_control: Option<String>,
    pub rated: bool,
    pub created_at: i64,
    pub analysis_source: Option<String>,
    pub analysis_completed_at: Option<i64>,
}

/// Input for inserting/upserting a game.
#[derive(Debug, Clone)]
pub struct GameInsert {
    pub id: String,
    pub pgn: String,
    pub user_color: String,
    pub user_result: String,
    pub time_control: Option<String>,
    pub rated: bool,
    pub created_at: i64,
    pub analysis_source: Option<String>,
}

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct UpsertOutcome {
    pub was_new: bool,
}

/// A per-move evaluation record.
#[derive(Debug, Clone)]
pub struct MoveEvaluation {
    pub game_id: String,
    pub ply: i32,
    /// Centipawn evaluation from White's perspective, or `None` for mate scores.
    pub eval_cp: Option<i32>,
    /// Mate-in-N from White's perspective (positive = White mates), or `None` for cp scores.
    pub eval_mate: Option<i32>,
    /// Source of the evaluation: `"lichess"` or `"stockfish"`.
    pub source: String,
}

/// A detected mistake stored in the database.
#[derive(Debug, Clone)]
pub struct StoredMistake {
    pub id: i64,
    pub game_id: String,
    pub ply: i32,
    pub fen_before: String,
    pub user_move: String,
    pub best_move: String,
    pub eval_before_cp: Option<i32>,
    pub eval_before_mate: Option<i32>,
    pub eval_after_cp: Option<i32>,
    pub eval_after_mate: Option<i32>,
    pub classification: String,
}

/// Input for inserting a mistake.
#[derive(Debug, Clone)]
pub struct MistakeInsert {
    pub game_id: String,
    pub ply: i32,
    pub fen_before: String,
    pub user_move: String,
    pub best_move: String,
    pub eval_before_cp: Option<i32>,
    pub eval_before_mate: Option<i32>,
    pub eval_after_cp: Option<i32>,
    pub eval_after_mate: Option<i32>,
    pub classification: String,
}

/// A stored puzzle.
#[derive(Debug, Clone)]
pub struct StoredPuzzle {
    pub id: i64,
    pub mistake_id: i64,
    pub fen: String,
    pub solution_moves: String,
    pub themes: Option<String>,
    pub created_at: i64,
}

/// Input for inserting a puzzle.
#[derive(Debug, Clone)]
pub struct PuzzleInsert {
    pub mistake_id: i64,
    pub fen: String,
    pub solution_moves: String,
    pub themes: Option<String>,
}

/// A stored puzzle attempt.
#[derive(Debug, Clone)]
pub struct StoredAttempt {
    pub id: i64,
    pub puzzle_id: i64,
    pub attempted_at: i64,
    pub success: bool,
    pub time_taken_ms: i64,
    pub move_played: String,
}

/// User preferences persisted in the database.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSettings {
    pub lichess_username: String,
    pub max_games: i64,
    /// When `true`, always use local Stockfish instead of Lichess embedded evals.
    pub use_stockfish: bool,
    pub inaccuracy_threshold_cp: i64,
    pub mistake_threshold_cp: i64,
    pub blunder_threshold_cp: i64,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            lichess_username: String::new(),
            max_games: 50,
            use_stockfish: false,
            inaccuracy_threshold_cp: 50,
            mistake_threshold_cp: 100,
            blunder_threshold_cp: 200,
        }
    }
}

/// Aggregate statistics for puzzle attempts.
#[derive(Debug, Clone)]
pub struct AttemptsSummary {
    pub total_attempts: i64,
    pub total_successes: i64,
    pub success_rate: f64,
    pub puzzles_attempted: i64,
    pub puzzles_attempted_today: i64,
}

/// Errors from the storage layer.
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

/// SQLite-backed persistence layer.
pub struct Storage {
    pool: SqlitePool,
}

impl Storage {
    /// Opens (or creates) a SQLite database at the given path and runs migrations.
    pub async fn new(db_path: &Path) -> Result<Self, StorageError> {
        let options =
            SqliteConnectOptions::from_str(&format!("sqlite:{}?mode=rwc", db_path.display()))
                .map_err(|e| StorageError::Database(sqlx::Error::Configuration(Box::new(e))))?
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Run embedded migrations
        sqlx::query(include_str!("../migrations/0001_initial.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0002_evaluations.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0003_mistakes.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0004_puzzles.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0005_attempts.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0006_settings.sql"))
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Opens an in-memory SQLite database (for testing).
    pub async fn new_in_memory() -> Result<Self, StorageError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;

        sqlx::query(include_str!("../migrations/0001_initial.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0002_evaluations.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0003_mistakes.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0004_puzzles.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0005_attempts.sql"))
            .execute(&pool)
            .await?;
        sqlx::query(include_str!("../migrations/0006_settings.sql"))
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    /// Upserts a game. If the game already exists, updates PGN and analysis fields
    /// (newer analysis wins). Returns whether the row was newly created.
    pub async fn insert_game(&self, game: &GameInsert) -> Result<UpsertOutcome, StorageError> {
        // Check if it already exists
        let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM games WHERE id = ?1")
            .bind(&game.id)
            .fetch_optional(&self.pool)
            .await?;

        if existing.is_some() {
            // Update: preserve existing analysis_source unless the new one is non-null
            sqlx::query(
                "UPDATE games SET
                    pgn = ?1,
                    user_color = ?2,
                    user_result = ?3,
                    time_control = ?4,
                    rated = ?5,
                    created_at = ?6,
                    analysis_source = COALESCE(?7, analysis_source)
                 WHERE id = ?8",
            )
            .bind(&game.pgn)
            .bind(&game.user_color)
            .bind(&game.user_result)
            .bind(&game.time_control)
            .bind(game.rated)
            .bind(game.created_at)
            .bind(&game.analysis_source)
            .bind(&game.id)
            .execute(&self.pool)
            .await?;

            Ok(UpsertOutcome { was_new: false })
        } else {
            sqlx::query(
                "INSERT INTO games (id, pgn, user_color, user_result, time_control, rated, created_at, analysis_source)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )
            .bind(&game.id)
            .bind(&game.pgn)
            .bind(&game.user_color)
            .bind(&game.user_result)
            .bind(&game.time_control)
            .bind(game.rated)
            .bind(game.created_at)
            .bind(&game.analysis_source)
            .execute(&self.pool)
            .await?;

            Ok(UpsertOutcome { was_new: true })
        }
    }

    /// Retrieves a game by its Lichess ID.
    pub async fn get_game(&self, id: &str) -> Result<Option<StoredGame>, StorageError> {
        let row = sqlx::query(
            "SELECT id, pgn, user_color, user_result, time_control, rated, created_at, analysis_source, analysis_completed_at
             FROM games WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| StoredGame {
            id: r.get("id"),
            pgn: r.get("pgn"),
            user_color: r.get("user_color"),
            user_result: r.get("user_result"),
            time_control: r.get("time_control"),
            rated: r.get::<i32, _>("rated") != 0,
            created_at: r.get("created_at"),
            analysis_source: r.get("analysis_source"),
            analysis_completed_at: r.get("analysis_completed_at"),
        }))
    }

    /// Returns a reference to the underlying connection pool (for advanced queries in tests).
    pub fn pool_ref(&self) -> &SqlitePool {
        &self.pool
    }

    /// Returns the count of games in the database.
    pub async fn game_count(&self) -> Result<i64, StorageError> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM games")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("cnt"))
    }

    // -------------------------------------------------------------------
    // Evaluation methods (Slice 3)
    // -------------------------------------------------------------------

    /// Batch-inserts move evaluations for a game, replacing any existing evals.
    pub async fn insert_evaluations(
        &self,
        game_id: &str,
        evals: &[MoveEvaluation],
    ) -> Result<(), StorageError> {
        // Delete existing evals for this game so a re-analysis is clean
        sqlx::query("DELETE FROM move_evaluations WHERE game_id = ?1")
            .bind(game_id)
            .execute(&self.pool)
            .await?;

        for eval in evals {
            sqlx::query(
                "INSERT INTO move_evaluations (game_id, ply, eval_cp, eval_mate, source)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(&eval.game_id)
            .bind(eval.ply)
            .bind(eval.eval_cp)
            .bind(eval.eval_mate)
            .bind(&eval.source)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Retrieves all move evaluations for a game, ordered by ply.
    pub async fn get_evaluations(
        &self,
        game_id: &str,
    ) -> Result<Vec<MoveEvaluation>, StorageError> {
        let rows = sqlx::query(
            "SELECT game_id, ply, eval_cp, eval_mate, source
             FROM move_evaluations WHERE game_id = ?1 ORDER BY ply",
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| MoveEvaluation {
                game_id: r.get("game_id"),
                ply: r.get("ply"),
                eval_cp: r.get("eval_cp"),
                eval_mate: r.get("eval_mate"),
                source: r.get("source"),
            })
            .collect())
    }

    /// Returns the count of evaluations stored for a game.
    pub async fn evaluation_count(&self, game_id: &str) -> Result<i64, StorageError> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM move_evaluations WHERE game_id = ?1")
            .bind(game_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("cnt"))
    }

    /// Updates a game's analysis status after evaluation is complete.
    pub async fn update_analysis_status(
        &self,
        game_id: &str,
        source: &str,
    ) -> Result<(), StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        sqlx::query(
            "UPDATE games SET analysis_source = ?1, analysis_completed_at = ?2 WHERE id = ?3",
        )
        .bind(source)
        .bind(now)
        .bind(game_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // -------------------------------------------------------------------
    // Mistake methods (Slice 4)
    // -------------------------------------------------------------------

    /// Batch-inserts mistakes for a game, replacing any existing mistakes.
    pub async fn insert_mistakes(
        &self,
        game_id: &str,
        mistakes: &[MistakeInsert],
    ) -> Result<(), StorageError> {
        // Delete existing mistakes for this game so re-detection is idempotent
        sqlx::query("DELETE FROM mistakes WHERE game_id = ?1")
            .bind(game_id)
            .execute(&self.pool)
            .await?;

        for m in mistakes {
            sqlx::query(
                "INSERT INTO mistakes (game_id, ply, fen_before, user_move, best_move, eval_before_cp, eval_before_mate, eval_after_cp, eval_after_mate, classification)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )
            .bind(&m.game_id)
            .bind(m.ply)
            .bind(&m.fen_before)
            .bind(&m.user_move)
            .bind(&m.best_move)
            .bind(m.eval_before_cp)
            .bind(m.eval_before_mate)
            .bind(m.eval_after_cp)
            .bind(m.eval_after_mate)
            .bind(&m.classification)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Retrieves all mistakes for a specific game, ordered by ply.
    pub async fn get_mistakes_for_game(
        &self,
        game_id: &str,
    ) -> Result<Vec<StoredMistake>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, game_id, ply, fen_before, user_move, best_move, eval_before_cp, eval_before_mate, eval_after_cp, eval_after_mate, classification
             FROM mistakes WHERE game_id = ?1 ORDER BY ply",
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredMistake {
                id: r.get("id"),
                game_id: r.get("game_id"),
                ply: r.get("ply"),
                fen_before: r.get("fen_before"),
                user_move: r.get("user_move"),
                best_move: r.get("best_move"),
                eval_before_cp: r.get("eval_before_cp"),
                eval_before_mate: r.get("eval_before_mate"),
                eval_after_cp: r.get("eval_after_cp"),
                eval_after_mate: r.get("eval_after_mate"),
                classification: r.get("classification"),
            })
            .collect())
    }

    /// Lists blunders across all games, ordered by most recent game first.
    pub async fn list_blunders(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StoredMistake>, StorageError> {
        let rows = sqlx::query(
            "SELECT m.id, m.game_id, m.ply, m.fen_before, m.user_move, m.best_move,
                    m.eval_before_cp, m.eval_before_mate, m.eval_after_cp, m.eval_after_mate,
                    m.classification
             FROM mistakes m
             JOIN games g ON m.game_id = g.id
             WHERE m.classification = 'blunder'
             ORDER BY g.created_at DESC, m.ply ASC
             LIMIT ?1 OFFSET ?2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredMistake {
                id: r.get("id"),
                game_id: r.get("game_id"),
                ply: r.get("ply"),
                fen_before: r.get("fen_before"),
                user_move: r.get("user_move"),
                best_move: r.get("best_move"),
                eval_before_cp: r.get("eval_before_cp"),
                eval_before_mate: r.get("eval_before_mate"),
                eval_after_cp: r.get("eval_after_cp"),
                eval_after_mate: r.get("eval_after_mate"),
                classification: r.get("classification"),
            })
            .collect())
    }

    /// Returns the count of mistakes for a game.
    pub async fn mistake_count(&self, game_id: &str) -> Result<i64, StorageError> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM mistakes WHERE game_id = ?1")
            .bind(game_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("cnt"))
    }

    /// Lists all analyzed games (have `analysis_completed_at`).
    pub async fn list_analyzed_games(&self) -> Result<Vec<StoredGame>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, pgn, user_color, user_result, time_control, rated, created_at, analysis_source, analysis_completed_at
             FROM games WHERE analysis_completed_at IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredGame {
                id: r.get("id"),
                pgn: r.get("pgn"),
                user_color: r.get("user_color"),
                user_result: r.get("user_result"),
                time_control: r.get("time_control"),
                rated: r.get::<i32, _>("rated") != 0,
                created_at: r.get("created_at"),
                analysis_source: r.get("analysis_source"),
                analysis_completed_at: r.get("analysis_completed_at"),
            })
            .collect())
    }

    // -------------------------------------------------------------------
    // Puzzle methods (Slice 5)
    // -------------------------------------------------------------------

    /// Inserts a puzzle. Returns the new puzzle's row ID.
    pub async fn insert_puzzle(&self, puzzle: &PuzzleInsert) -> Result<i64, StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let result = sqlx::query(
            "INSERT INTO puzzles (mistake_id, fen, solution_moves, themes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(puzzle.mistake_id)
        .bind(&puzzle.fen)
        .bind(&puzzle.solution_moves)
        .bind(&puzzle.themes)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Lists puzzles, ordered by creation time (newest first).
    pub async fn list_puzzles(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StoredPuzzle>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, mistake_id, fen, solution_moves, themes, created_at
             FROM puzzles ORDER BY created_at DESC LIMIT ?1 OFFSET ?2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredPuzzle {
                id: r.get("id"),
                mistake_id: r.get("mistake_id"),
                fen: r.get("fen"),
                solution_moves: r.get("solution_moves"),
                themes: r.get("themes"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    /// Returns the count of puzzles in the database.
    pub async fn puzzle_count(&self) -> Result<i64, StorageError> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM puzzles")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("cnt"))
    }

    /// Checks if a puzzle already exists for a given mistake.
    pub async fn puzzle_exists_for_mistake(&self, mistake_id: i64) -> Result<bool, StorageError> {
        let row = sqlx::query("SELECT COUNT(*) as cnt FROM puzzles WHERE mistake_id = ?1")
            .bind(mistake_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("cnt") > 0)
    }

    /// Updates the best_move column on a mistake record (populated during puzzle generation).
    pub async fn update_mistake_best_move(
        &self,
        mistake_id: i64,
        best_move: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("UPDATE mistakes SET best_move = ?1 WHERE id = ?2")
            .bind(best_move)
            .bind(mistake_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Lists all games that have not been analyzed (no `analysis_completed_at`).
    pub async fn list_unanalyzed_games(&self) -> Result<Vec<StoredGame>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, pgn, user_color, user_result, time_control, rated, created_at, analysis_source, analysis_completed_at
             FROM games WHERE analysis_completed_at IS NULL",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredGame {
                id: r.get("id"),
                pgn: r.get("pgn"),
                user_color: r.get("user_color"),
                user_result: r.get("user_result"),
                time_control: r.get("time_control"),
                rated: r.get::<i32, _>("rated") != 0,
                created_at: r.get("created_at"),
                analysis_source: r.get("analysis_source"),
                analysis_completed_at: r.get("analysis_completed_at"),
            })
            .collect())
    }

    // -------------------------------------------------------------------
    // Puzzle attempt methods (Slice 6)
    // -------------------------------------------------------------------

    /// Records a puzzle attempt.
    pub async fn record_attempt(
        &self,
        puzzle_id: i64,
        success: bool,
        time_taken_ms: i64,
        move_played: &str,
    ) -> Result<i64, StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let result = sqlx::query(
            "INSERT INTO puzzle_attempts (puzzle_id, attempted_at, success, time_taken_ms, move_played)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(puzzle_id)
        .bind(now)
        .bind(success as i32)
        .bind(time_taken_ms)
        .bind(move_played)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Retrieves all attempts for a specific puzzle, ordered by time.
    pub async fn get_attempts_for_puzzle(
        &self,
        puzzle_id: i64,
    ) -> Result<Vec<StoredAttempt>, StorageError> {
        let rows = sqlx::query(
            "SELECT id, puzzle_id, attempted_at, success, time_taken_ms, move_played
             FROM puzzle_attempts WHERE puzzle_id = ?1 ORDER BY attempted_at",
        )
        .bind(puzzle_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| StoredAttempt {
                id: r.get("id"),
                puzzle_id: r.get("puzzle_id"),
                attempted_at: r.get("attempted_at"),
                success: r.get::<i32, _>("success") != 0,
                time_taken_ms: r.get("time_taken_ms"),
                move_played: r.get("move_played"),
            })
            .collect())
    }

    /// Returns aggregate statistics for puzzle attempts.
    pub async fn get_attempts_summary(&self) -> Result<AttemptsSummary, StorageError> {
        let row = sqlx::query(
            "SELECT
                COUNT(*) as total_attempts,
                COALESCE(SUM(success), 0) as total_successes,
                COUNT(DISTINCT puzzle_id) as puzzles_attempted
             FROM puzzle_attempts",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_attempts: i64 = row.get("total_attempts");
        let total_successes: i64 = row.get("total_successes");
        let puzzles_attempted: i64 = row.get("puzzles_attempted");

        let success_rate = if total_attempts > 0 {
            total_successes as f64 / total_attempts as f64
        } else {
            0.0
        };

        // Count distinct puzzles attempted today (UTC day boundary)
        let today_start = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            now - (now % 86400)
        };

        let today_row = sqlx::query(
            "SELECT COUNT(DISTINCT puzzle_id) as cnt
             FROM puzzle_attempts WHERE attempted_at >= ?1",
        )
        .bind(today_start)
        .fetch_one(&self.pool)
        .await?;

        let puzzles_attempted_today: i64 = today_row.get("cnt");

        Ok(AttemptsSummary {
            total_attempts,
            total_successes,
            success_rate,
            puzzles_attempted,
            puzzles_attempted_today,
        })
    }

    /// Returns a random unattempted puzzle, or a random already-attempted puzzle
    /// if all puzzles have been seen. Returns `None` if there are no puzzles at all.
    pub async fn get_next_puzzle(&self) -> Result<Option<StoredPuzzle>, StorageError> {
        // Try unattempted puzzles first
        let row = sqlx::query(
            "SELECT p.id, p.mistake_id, p.fen, p.solution_moves, p.themes, p.created_at
             FROM puzzles p
             WHERE p.id NOT IN (SELECT DISTINCT puzzle_id FROM puzzle_attempts)
             ORDER BY RANDOM()
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            return Ok(Some(StoredPuzzle {
                id: r.get("id"),
                mistake_id: r.get("mistake_id"),
                fen: r.get("fen"),
                solution_moves: r.get("solution_moves"),
                themes: r.get("themes"),
                created_at: r.get("created_at"),
            }));
        }

        // Fall back to random already-attempted puzzle
        let row = sqlx::query(
            "SELECT id, mistake_id, fen, solution_moves, themes, created_at
             FROM puzzles
             ORDER BY RANDOM()
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| StoredPuzzle {
            id: r.get("id"),
            mistake_id: r.get("mistake_id"),
            fen: r.get("fen"),
            solution_moves: r.get("solution_moves"),
            themes: r.get("themes"),
            created_at: r.get("created_at"),
        }))
    }

    // -------------------------------------------------------------------
    // Settings methods (Slice 7)
    // -------------------------------------------------------------------

    /// Returns the user's settings. The table is seeded with defaults on
    /// first migration, so this always returns a value.
    pub async fn get_settings(&self) -> Result<UserSettings, StorageError> {
        let row = sqlx::query(
            "SELECT lichess_username, max_games, use_stockfish,
                    inaccuracy_threshold_cp, mistake_threshold_cp, blunder_threshold_cp
             FROM user_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|r| UserSettings {
                lichess_username: r.get("lichess_username"),
                max_games: r.get("max_games"),
                use_stockfish: r.get::<i32, _>("use_stockfish") != 0,
                inaccuracy_threshold_cp: r.get("inaccuracy_threshold_cp"),
                mistake_threshold_cp: r.get("mistake_threshold_cp"),
                blunder_threshold_cp: r.get("blunder_threshold_cp"),
            })
            .unwrap_or_default())
    }

    /// Persists the user's settings, overwriting the existing single row.
    pub async fn save_settings(&self, settings: &UserSettings) -> Result<(), StorageError> {
        sqlx::query(
            "UPDATE user_settings SET
                lichess_username = ?1,
                max_games = ?2,
                use_stockfish = ?3,
                inaccuracy_threshold_cp = ?4,
                mistake_threshold_cp = ?5,
                blunder_threshold_cp = ?6,
                updated_at = strftime('%s', 'now')
             WHERE id = 1",
        )
        .bind(&settings.lichess_username)
        .bind(settings.max_games)
        .bind(settings.use_stockfish as i32)
        .bind(settings.inaccuracy_threshold_cp)
        .bind(settings.mistake_threshold_cp)
        .bind(settings.blunder_threshold_cp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn insert_and_retrieve_game() {
        let storage = Storage::new_in_memory().await.unwrap();

        let game = GameInsert {
            id: "abc12345".to_owned(),
            pgn: "1. e4 e5 *".to_owned(),
            user_color: "white".to_owned(),
            user_result: "win".to_owned(),
            time_control: Some("600+0".to_owned()),
            rated: true,
            created_at: 1700000000,
            analysis_source: Some("lichess".to_owned()),
        };

        let outcome = storage.insert_game(&game).await.unwrap();
        assert!(outcome.was_new);

        let stored = storage.get_game("abc12345").await.unwrap().unwrap();
        assert_eq!(stored.id, "abc12345");
        assert_eq!(stored.pgn, "1. e4 e5 *");
        assert_eq!(stored.user_color, "white");
        assert_eq!(stored.user_result, "win");
        assert!(stored.rated);
    }

    #[tokio::test]
    async fn upsert_does_not_duplicate() {
        let storage = Storage::new_in_memory().await.unwrap();

        let game = GameInsert {
            id: "abc12345".to_owned(),
            pgn: "1. e4 e5 *".to_owned(),
            user_color: "white".to_owned(),
            user_result: "win".to_owned(),
            time_control: Some("600+0".to_owned()),
            rated: true,
            created_at: 1700000000,
            analysis_source: None,
        };

        let outcome1 = storage.insert_game(&game).await.unwrap();
        assert!(outcome1.was_new);

        // Second insert with updated PGN and analysis
        let game2 = GameInsert {
            id: "abc12345".to_owned(),
            pgn: "1. e4 e5 2. Nf3 *".to_owned(),
            user_color: "white".to_owned(),
            user_result: "win".to_owned(),
            time_control: Some("600+0".to_owned()),
            rated: true,
            created_at: 1700000000,
            analysis_source: Some("lichess".to_owned()),
        };

        let outcome2 = storage.insert_game(&game2).await.unwrap();
        assert!(!outcome2.was_new);

        // Only one row
        assert_eq!(storage.game_count().await.unwrap(), 1);

        // Updated fields
        let stored = storage.get_game("abc12345").await.unwrap().unwrap();
        assert_eq!(stored.pgn, "1. e4 e5 2. Nf3 *");
        assert_eq!(stored.analysis_source.as_deref(), Some("lichess"));
    }

    #[tokio::test]
    async fn get_nonexistent_game_returns_none() {
        let storage = Storage::new_in_memory().await.unwrap();
        let result = storage.get_game("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    // -------------------------------------------------------------------
    // Evaluation tests (Slice 3)
    // -------------------------------------------------------------------

    /// Helper: insert a minimal game for eval tests.
    async fn insert_test_game(storage: &Storage, id: &str) {
        let game = GameInsert {
            id: id.to_owned(),
            pgn: "1. e4 e5 *".to_owned(),
            user_color: "white".to_owned(),
            user_result: "win".to_owned(),
            time_control: Some("600+0".to_owned()),
            rated: true,
            created_at: 1700000000,
            analysis_source: None,
        };
        storage.insert_game(&game).await.unwrap();
    }

    #[tokio::test]
    async fn insert_and_get_evaluations() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "eval_test1").await;

        let evals = vec![
            MoveEvaluation {
                game_id: "eval_test1".to_owned(),
                ply: 0,
                eval_cp: Some(30),
                eval_mate: None,
                source: "lichess".to_owned(),
            },
            MoveEvaluation {
                game_id: "eval_test1".to_owned(),
                ply: 1,
                eval_cp: Some(23),
                eval_mate: None,
                source: "lichess".to_owned(),
            },
        ];

        storage
            .insert_evaluations("eval_test1", &evals)
            .await
            .unwrap();

        let stored = storage.get_evaluations("eval_test1").await.unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].ply, 0);
        assert_eq!(stored[0].eval_cp, Some(30));
        assert_eq!(stored[1].ply, 1);
        assert_eq!(stored[1].eval_cp, Some(23));
    }

    #[tokio::test]
    async fn insert_evaluations_replaces_existing() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "eval_replace").await;

        let evals1 = vec![MoveEvaluation {
            game_id: "eval_replace".to_owned(),
            ply: 0,
            eval_cp: Some(30),
            eval_mate: None,
            source: "lichess".to_owned(),
        }];
        storage
            .insert_evaluations("eval_replace", &evals1)
            .await
            .unwrap();

        // Re-insert with different values (e.g., stockfish re-analysis)
        let evals2 = vec![MoveEvaluation {
            game_id: "eval_replace".to_owned(),
            ply: 0,
            eval_cp: Some(45),
            eval_mate: None,
            source: "stockfish".to_owned(),
        }];
        storage
            .insert_evaluations("eval_replace", &evals2)
            .await
            .unwrap();

        let stored = storage.get_evaluations("eval_replace").await.unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].eval_cp, Some(45));
        assert_eq!(stored[0].source, "stockfish");
    }

    #[tokio::test]
    async fn evaluation_count_works() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "count_test").await;

        assert_eq!(storage.evaluation_count("count_test").await.unwrap(), 0);

        let evals = vec![
            MoveEvaluation {
                game_id: "count_test".to_owned(),
                ply: 0,
                eval_cp: Some(30),
                eval_mate: None,
                source: "lichess".to_owned(),
            },
            MoveEvaluation {
                game_id: "count_test".to_owned(),
                ply: 1,
                eval_cp: None,
                eval_mate: Some(3),
                source: "lichess".to_owned(),
            },
        ];
        storage
            .insert_evaluations("count_test", &evals)
            .await
            .unwrap();

        assert_eq!(storage.evaluation_count("count_test").await.unwrap(), 2);
    }

    #[tokio::test]
    async fn update_analysis_status_sets_fields() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "status_test").await;

        // Initially no analysis
        let game = storage.get_game("status_test").await.unwrap().unwrap();
        assert!(game.analysis_completed_at.is_none());

        storage
            .update_analysis_status("status_test", "lichess")
            .await
            .unwrap();

        let game = storage.get_game("status_test").await.unwrap().unwrap();
        assert_eq!(game.analysis_source.as_deref(), Some("lichess"));
        assert!(game.analysis_completed_at.is_some());
    }

    #[tokio::test]
    async fn list_unanalyzed_games_filters() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "unanalyzed1").await;
        insert_test_game(&storage, "unanalyzed2").await;

        // All are unanalyzed
        let unanalyzed = storage.list_unanalyzed_games().await.unwrap();
        assert_eq!(unanalyzed.len(), 2);

        // Mark one as analyzed
        storage
            .update_analysis_status("unanalyzed1", "lichess")
            .await
            .unwrap();

        let unanalyzed = storage.list_unanalyzed_games().await.unwrap();
        assert_eq!(unanalyzed.len(), 1);
        assert_eq!(unanalyzed[0].id, "unanalyzed2");
    }

    #[tokio::test]
    async fn get_evaluations_empty() {
        let storage = Storage::new_in_memory().await.unwrap();
        let evals = storage.get_evaluations("nonexistent").await.unwrap();
        assert!(evals.is_empty());
    }

    // -------------------------------------------------------------------
    // Mistake tests (Slice 4)
    // -------------------------------------------------------------------

    fn test_mistake(game_id: &str, ply: i32, classification: &str) -> MistakeInsert {
        MistakeInsert {
            game_id: game_id.to_owned(),
            ply,
            fen_before: format!("fen_at_ply_{ply}"),
            user_move: "e7e5".to_owned(),
            best_move: "d7d5".to_owned(),
            eval_before_cp: Some(50),
            eval_before_mate: None,
            eval_after_cp: Some(-200),
            eval_after_mate: None,
            classification: classification.to_owned(),
        }
    }

    #[tokio::test]
    async fn insert_and_get_mistakes() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "mistake_test1").await;

        let mistakes = vec![
            test_mistake("mistake_test1", 4, "blunder"),
            test_mistake("mistake_test1", 12, "mistake"),
        ];

        storage
            .insert_mistakes("mistake_test1", &mistakes)
            .await
            .unwrap();

        let stored = storage
            .get_mistakes_for_game("mistake_test1")
            .await
            .unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].ply, 4);
        assert_eq!(stored[0].classification, "blunder");
        assert_eq!(stored[1].ply, 12);
        assert_eq!(stored[1].classification, "mistake");
    }

    #[tokio::test]
    async fn insert_mistakes_replaces_existing() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "mistake_replace").await;

        let mistakes1 = vec![test_mistake("mistake_replace", 4, "blunder")];
        storage
            .insert_mistakes("mistake_replace", &mistakes1)
            .await
            .unwrap();

        // Re-detect: replace with different result
        let mistakes2 = vec![test_mistake("mistake_replace", 6, "mistake")];
        storage
            .insert_mistakes("mistake_replace", &mistakes2)
            .await
            .unwrap();

        let stored = storage
            .get_mistakes_for_game("mistake_replace")
            .await
            .unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].ply, 6);
        assert_eq!(stored[0].classification, "mistake");
    }

    #[tokio::test]
    async fn list_blunders_filters_by_classification() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "blunder_list").await;

        let mistakes = vec![
            test_mistake("blunder_list", 4, "blunder"),
            test_mistake("blunder_list", 8, "mistake"),
            test_mistake("blunder_list", 12, "inaccuracy"),
            test_mistake("blunder_list", 16, "blunder"),
        ];
        storage
            .insert_mistakes("blunder_list", &mistakes)
            .await
            .unwrap();

        let blunders = storage.list_blunders(100, 0).await.unwrap();
        assert_eq!(blunders.len(), 2);
        assert!(blunders.iter().all(|b| b.classification == "blunder"));
    }

    #[tokio::test]
    async fn list_blunders_pagination() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "blunder_page").await;

        let mistakes = vec![
            test_mistake("blunder_page", 2, "blunder"),
            test_mistake("blunder_page", 4, "blunder"),
            test_mistake("blunder_page", 6, "blunder"),
        ];
        storage
            .insert_mistakes("blunder_page", &mistakes)
            .await
            .unwrap();

        let page1 = storage.list_blunders(2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = storage.list_blunders(2, 2).await.unwrap();
        assert_eq!(page2.len(), 1);
    }

    #[tokio::test]
    async fn mistake_count_works() {
        let storage = Storage::new_in_memory().await.unwrap();
        insert_test_game(&storage, "mcount_test").await;

        assert_eq!(storage.mistake_count("mcount_test").await.unwrap(), 0);

        let mistakes = vec![
            test_mistake("mcount_test", 4, "blunder"),
            test_mistake("mcount_test", 8, "mistake"),
        ];
        storage
            .insert_mistakes("mcount_test", &mistakes)
            .await
            .unwrap();

        assert_eq!(storage.mistake_count("mcount_test").await.unwrap(), 2);
    }

    #[tokio::test]
    async fn get_mistakes_empty() {
        let storage = Storage::new_in_memory().await.unwrap();
        let mistakes = storage.get_mistakes_for_game("nonexistent").await.unwrap();
        assert!(mistakes.is_empty());
    }

    // -------------------------------------------------------------------
    // Puzzle tests (Slice 5)
    // -------------------------------------------------------------------

    /// Helper: insert a blunder mistake for puzzle tests. Returns the mistake ID.
    async fn insert_test_blunder(storage: &Storage, game_id: &str, ply: i32) -> i64 {
        insert_test_game(storage, game_id).await;
        let mistakes = vec![test_mistake(game_id, ply, "blunder")];
        storage.insert_mistakes(game_id, &mistakes).await.unwrap();
        let stored = storage.get_mistakes_for_game(game_id).await.unwrap();
        stored[0].id
    }

    #[tokio::test]
    async fn insert_and_list_puzzles() {
        let storage = Storage::new_in_memory().await.unwrap();
        let mistake_id = insert_test_blunder(&storage, "puzzle_test1", 4).await;

        let puzzle = PuzzleInsert {
            mistake_id,
            fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1".to_owned(),
            solution_moves: r#"["d7d5"]"#.to_owned(),
            themes: None,
        };
        let puzzle_id = storage.insert_puzzle(&puzzle).await.unwrap();
        assert!(puzzle_id > 0);

        let puzzles = storage.list_puzzles(100, 0).await.unwrap();
        assert_eq!(puzzles.len(), 1);
        assert_eq!(puzzles[0].mistake_id, mistake_id);
        assert!(puzzles[0].fen.contains("rnbqkbnr"));
    }

    #[tokio::test]
    async fn puzzle_count_works() {
        let storage = Storage::new_in_memory().await.unwrap();
        assert_eq!(storage.puzzle_count().await.unwrap(), 0);

        let mistake_id = insert_test_blunder(&storage, "pcount_test", 4).await;
        let puzzle = PuzzleInsert {
            mistake_id,
            fen: "test_fen".to_owned(),
            solution_moves: r#"["e2e4"]"#.to_owned(),
            themes: None,
        };
        storage.insert_puzzle(&puzzle).await.unwrap();
        assert_eq!(storage.puzzle_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn puzzle_exists_for_mistake_check() {
        let storage = Storage::new_in_memory().await.unwrap();
        let mistake_id = insert_test_blunder(&storage, "pexist_test", 4).await;

        assert!(!storage.puzzle_exists_for_mistake(mistake_id).await.unwrap());

        let puzzle = PuzzleInsert {
            mistake_id,
            fen: "test_fen".to_owned(),
            solution_moves: r#"["e2e4"]"#.to_owned(),
            themes: None,
        };
        storage.insert_puzzle(&puzzle).await.unwrap();
        assert!(storage.puzzle_exists_for_mistake(mistake_id).await.unwrap());
    }

    #[tokio::test]
    async fn update_mistake_best_move_works() {
        let storage = Storage::new_in_memory().await.unwrap();
        let mistake_id = insert_test_blunder(&storage, "bestmove_test", 4).await;

        // Initially best_move is "d7d5" from test_mistake helper
        let stored = storage
            .get_mistakes_for_game("bestmove_test")
            .await
            .unwrap();
        assert_eq!(stored[0].best_move, "d7d5");

        storage
            .update_mistake_best_move(mistake_id, "e2e4")
            .await
            .unwrap();

        let stored = storage
            .get_mistakes_for_game("bestmove_test")
            .await
            .unwrap();
        assert_eq!(stored[0].best_move, "e2e4");
    }

    // -------------------------------------------------------------------
    // Puzzle attempt tests (Slice 6)
    // -------------------------------------------------------------------

    /// Helper: insert a puzzle for attempt tests. Returns the puzzle ID.
    async fn insert_test_puzzle(storage: &Storage, game_id: &str) -> i64 {
        let mistake_id = insert_test_blunder(storage, game_id, 4).await;
        let puzzle = PuzzleInsert {
            mistake_id,
            fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1".to_owned(),
            solution_moves: r#"["d7d5"]"#.to_owned(),
            themes: None,
        };
        storage.insert_puzzle(&puzzle).await.unwrap()
    }

    #[tokio::test]
    async fn record_and_get_attempts() {
        let storage = Storage::new_in_memory().await.unwrap();
        let puzzle_id = insert_test_puzzle(&storage, "attempt_test1").await;

        let attempt_id = storage
            .record_attempt(puzzle_id, true, 5000, "d7d5")
            .await
            .unwrap();
        assert!(attempt_id > 0);

        let attempts = storage.get_attempts_for_puzzle(puzzle_id).await.unwrap();
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].puzzle_id, puzzle_id);
        assert!(attempts[0].success);
        assert_eq!(attempts[0].time_taken_ms, 5000);
        assert_eq!(attempts[0].move_played, "d7d5");
    }

    #[tokio::test]
    async fn multiple_attempts_on_same_puzzle() {
        let storage = Storage::new_in_memory().await.unwrap();
        let puzzle_id = insert_test_puzzle(&storage, "multi_attempt").await;

        storage
            .record_attempt(puzzle_id, false, 3000, "e7e5")
            .await
            .unwrap();
        storage
            .record_attempt(puzzle_id, true, 2000, "d7d5")
            .await
            .unwrap();

        let attempts = storage.get_attempts_for_puzzle(puzzle_id).await.unwrap();
        assert_eq!(attempts.len(), 2);
        assert!(!attempts[0].success);
        assert!(attempts[1].success);
    }

    #[tokio::test]
    async fn get_attempts_empty() {
        let storage = Storage::new_in_memory().await.unwrap();
        let puzzle_id = insert_test_puzzle(&storage, "empty_attempt").await;

        let attempts = storage.get_attempts_for_puzzle(puzzle_id).await.unwrap();
        assert!(attempts.is_empty());
    }

    #[tokio::test]
    async fn get_attempts_summary_empty() {
        let storage = Storage::new_in_memory().await.unwrap();
        let summary = storage.get_attempts_summary().await.unwrap();
        assert_eq!(summary.total_attempts, 0);
        assert_eq!(summary.total_successes, 0);
        assert_eq!(summary.success_rate, 0.0);
        assert_eq!(summary.puzzles_attempted, 0);
        assert_eq!(summary.puzzles_attempted_today, 0);
    }

    #[tokio::test]
    async fn get_attempts_summary_with_data() {
        let storage = Storage::new_in_memory().await.unwrap();
        let puzzle_id1 = insert_test_puzzle(&storage, "summary1").await;
        let puzzle_id2 = insert_test_puzzle(&storage, "summary2").await;

        storage
            .record_attempt(puzzle_id1, true, 3000, "d7d5")
            .await
            .unwrap();
        storage
            .record_attempt(puzzle_id1, false, 4000, "e7e5")
            .await
            .unwrap();
        storage
            .record_attempt(puzzle_id2, true, 2000, "d7d5")
            .await
            .unwrap();

        let summary = storage.get_attempts_summary().await.unwrap();
        assert_eq!(summary.total_attempts, 3);
        assert_eq!(summary.total_successes, 2);
        assert!((summary.success_rate - 2.0 / 3.0).abs() < 0.001);
        assert_eq!(summary.puzzles_attempted, 2);
        // All attempts are "today" since we just recorded them
        assert_eq!(summary.puzzles_attempted_today, 2);
    }

    #[tokio::test]
    async fn get_next_puzzle_returns_unattempted_first() {
        let storage = Storage::new_in_memory().await.unwrap();
        let puzzle_id1 = insert_test_puzzle(&storage, "next_test1").await;
        let puzzle_id2 = insert_test_puzzle(&storage, "next_test2").await;

        // Attempt puzzle 1
        storage
            .record_attempt(puzzle_id1, true, 3000, "d7d5")
            .await
            .unwrap();

        // get_next_puzzle should return the unattempted puzzle (puzzle_id2)
        let next = storage.get_next_puzzle().await.unwrap().unwrap();
        assert_eq!(next.id, puzzle_id2);
    }

    #[tokio::test]
    async fn get_next_puzzle_falls_back_when_all_attempted() {
        let storage = Storage::new_in_memory().await.unwrap();
        let puzzle_id = insert_test_puzzle(&storage, "fallback_test").await;

        // Attempt the only puzzle
        storage
            .record_attempt(puzzle_id, true, 3000, "d7d5")
            .await
            .unwrap();

        // Should still return a puzzle (fallback to already-attempted)
        let next = storage.get_next_puzzle().await.unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().id, puzzle_id);
    }

    #[tokio::test]
    async fn get_next_puzzle_returns_none_when_no_puzzles() {
        let storage = Storage::new_in_memory().await.unwrap();
        let next = storage.get_next_puzzle().await.unwrap();
        assert!(next.is_none());
    }
}
