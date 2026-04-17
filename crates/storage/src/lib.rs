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
}
