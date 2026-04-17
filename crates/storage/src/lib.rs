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
}
