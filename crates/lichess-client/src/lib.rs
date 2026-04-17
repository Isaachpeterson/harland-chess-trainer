// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! # lichess-client
//!
//! Async wrapper over the Lichess public API. Provides `LichessClient` with methods
//! for fetching user games, game analysis, opening explorer data, and tablebase lookups.
//! Supports ndjson streaming, respects rate limits, and uses exponential backoff on 429s.

mod types;

pub use types::LichessGame;

use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use reqwest::StatusCode;
use std::time::Duration;
use thiserror::Error;

const DEFAULT_BASE_URL: &str = "https://lichess.org";
const APP_USER_AGENT: &str = concat!(
    "HarlandChessTrainer/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/Isaachpeterson/harland-chess-trainer)"
);
const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 1000;

/// Errors that can occur when communicating with the Lichess API.
#[derive(Debug, Error)]
pub enum LichessError {
    #[error("HTTP request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("failed to parse game JSON: {0} — raw line: {1}")]
    Parse(serde_json::Error, String),

    #[error("rate limit exhausted after {0} retries")]
    RateLimitExhausted(u32),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("unexpected status {0}: {1}")]
    UnexpectedStatus(u16, String),
}

/// Async client for the Lichess public API.
pub struct LichessClient {
    client: reqwest::Client,
    base_url: String,
}

impl LichessClient {
    /// Creates a new client with the default Lichess base URL.
    pub fn new() -> Result<Self, LichessError> {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    /// Creates a new client pointing at a custom base URL (useful for testing).
    pub fn with_base_url(base_url: &str) -> Result<Self, LichessError> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(APP_USER_AGENT));
        headers.insert(ACCEPT, HeaderValue::from_static("application/x-ndjson"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
        })
    }

    /// Fetches a user's games from the Lichess API as ndjson, with retry on 429.
    ///
    /// Returns up to `max_games` games for the given `username`. Requests include
    /// evaluation, clock, and opening data.
    pub async fn fetch_user_games(
        &self,
        username: &str,
        max_games: u32,
    ) -> Result<Vec<LichessGame>, LichessError> {
        let url = format!("{}/api/games/user/{}", self.base_url, username);

        let mut attempt = 0u32;
        loop {
            let response = self
                .client
                .get(&url)
                .query(&[
                    ("max", max_games.to_string()),
                    ("evals", "true".to_string()),
                    ("clocks", "true".to_string()),
                    ("opening", "true".to_string()),
                    ("pgnInJson", "true".to_string()),
                ])
                .send()
                .await?;

            match response.status() {
                StatusCode::OK => {
                    return self.parse_ndjson_stream(response).await;
                }
                StatusCode::NOT_FOUND => {
                    return Err(LichessError::UserNotFound(username.to_owned()));
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    attempt += 1;
                    if attempt > MAX_RETRIES {
                        return Err(LichessError::RateLimitExhausted(MAX_RETRIES));
                    }
                    let backoff = Duration::from_millis(INITIAL_BACKOFF_MS * 2u64.pow(attempt - 1));
                    tokio::time::sleep(backoff).await;
                }
                other => {
                    let body = response.text().await.unwrap_or_default();
                    return Err(LichessError::UnexpectedStatus(other.as_u16(), body));
                }
            }
        }
    }

    /// Parses an ndjson response body into a Vec of `LichessGame`.
    async fn parse_ndjson_stream(
        &self,
        response: reqwest::Response,
    ) -> Result<Vec<LichessGame>, LichessError> {
        let mut games = Vec::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // ndjson: each line is a complete JSON object
            while let Some(newline_pos) = buffer.find('\n') {
                let line: String = buffer.drain(..=newline_pos).collect();
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let game: LichessGame = serde_json::from_str(line)
                    .map_err(|e| LichessError::Parse(e, line.to_owned()))?;
                games.push(game);
            }
        }

        // Handle any trailing data without a final newline
        let remaining = buffer.trim();
        if !remaining.is_empty() {
            let game: LichessGame = serde_json::from_str(remaining)
                .map_err(|e| LichessError::Parse(e, remaining.to_owned()))?;
            games.push(game);
        }

        Ok(games)
    }
}
