// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Types for Lichess API responses.

use serde::{Deserialize, Serialize};

/// A game fetched from the Lichess API (ndjson format with `pgnInJson=true`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LichessGame {
    /// Lichess game ID (8 characters).
    pub id: String,

    /// Whether the game was rated.
    pub rated: bool,

    /// Speed category: "ultraBullet", "bullet", "blitz", "rapid", "classical", "correspondence".
    pub speed: String,

    /// Game status: "created", "started", "mate", "resign", "stalemate", "timeout", "draw", "outoftime", etc.
    pub status: String,

    /// Players object containing white and black player info.
    pub players: Players,

    /// Winner color ("white" or "black"), absent for draws.
    pub winner: Option<String>,

    /// PGN text of the game (present when `pgnInJson=true`).
    pub pgn: Option<String>,

    /// Unix timestamp in milliseconds when the game was created.
    #[serde(rename = "createdAt")]
    pub created_at: i64,

    /// Unix timestamp in milliseconds when the game ended.
    #[serde(rename = "lastMoveAt")]
    pub last_move_at: Option<i64>,

    /// Opening information if requested.
    pub opening: Option<Opening>,

    /// Clock settings if present.
    pub clock: Option<Clock>,

    /// Server-side analysis if available and `evals=true` was set.
    pub analysis: Option<Vec<AnalysisEntry>>,
}

impl LichessGame {
    /// Determines which color the given username played.
    ///
    /// Returns `None` if the username doesn't match either player.
    pub fn user_color(&self, username: &str) -> Option<String> {
        let lower = username.to_lowercase();
        if self
            .players
            .white
            .user
            .as_ref()
            .is_some_and(|u| u.name.to_lowercase() == lower)
        {
            Some("white".to_owned())
        } else if self
            .players
            .black
            .user
            .as_ref()
            .is_some_and(|u| u.name.to_lowercase() == lower)
        {
            Some("black".to_owned())
        } else {
            None
        }
    }

    /// Determines the result for the given username: "win", "loss", or "draw".
    ///
    /// Returns `None` if the username doesn't match either player.
    pub fn user_result(&self, username: &str) -> Option<String> {
        let color = self.user_color(username)?;
        match &self.winner {
            Some(winner) if *winner == color => Some("win".to_owned()),
            Some(_) => Some("loss".to_owned()),
            None => Some("draw".to_owned()),
        }
    }

    /// Whether Lichess server-side analysis is present.
    pub fn has_analysis(&self) -> bool {
        self.analysis.as_ref().is_some_and(|a| !a.is_empty())
    }
}

/// Player pair for white and black.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Players {
    pub white: PlayerInfo,
    pub black: PlayerInfo,
}

/// Information about one side's player in a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub user: Option<UserInfo>,
    pub rating: Option<i32>,
    #[serde(rename = "ratingDiff")]
    pub rating_diff: Option<i32>,
}

/// Basic Lichess user info as embedded in game data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub id: Option<String>,
}

/// Opening information returned by Lichess.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opening {
    pub eco: Option<String>,
    pub name: Option<String>,
    pub ply: Option<u32>,
}

/// Clock settings for a game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clock {
    pub initial: Option<i64>,
    pub increment: Option<i64>,
    #[serde(rename = "totalTime")]
    pub total_time: Option<i64>,
}

/// A single move's server-side analysis entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisEntry {
    pub eval: Option<i32>,
    pub mate: Option<i32>,
    pub best: Option<String>,
    pub variation: Option<String>,
    pub judgment: Option<Judgment>,
}

/// Lichess's judgment on a move.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Judgment {
    pub name: Option<String>,
    pub comment: Option<String>,
}
