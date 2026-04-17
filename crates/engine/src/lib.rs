// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! # engine
//!
//! Stockfish process management via the UCI protocol. Spawns a Stockfish process,
//! handles the UCI handshake, and provides `analyze(fen, config)` returning
//! centipawn evaluation, best move, and principal variation.
//! Supports multi-PV analysis and graceful shutdown.

mod parse;

use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::time::{timeout, Duration};

/// Default timeout for UCI commands (30 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Errors that can occur when interacting with the engine.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Failed to spawn the Stockfish process.
    #[error("failed to spawn engine process: {0}")]
    SpawnFailed(#[source] std::io::Error),

    /// The engine violated the UCI protocol (unexpected or missing response).
    #[error("UCI protocol violation: {0}")]
    ProtocolViolation(String),

    /// A command timed out waiting for a response.
    #[error("engine timed out after {0:?}")]
    Timeout(Duration),

    /// The engine process ended unexpectedly (EOF on stdout).
    #[error("unexpected EOF from engine")]
    UnexpectedEof,

    /// An I/O error while communicating with the engine.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration for a position analysis request.
#[derive(Debug, Clone)]
pub struct AnalyzeConfig {
    /// Search to this depth. If `None`, `movetime_ms` must be set.
    pub depth: Option<u32>,
    /// Search for this many milliseconds. If `None`, `depth` must be set.
    pub movetime_ms: Option<u32>,
    /// Number of principal variations to return. Defaults to 1.
    pub multipv: u32,
}

impl Default for AnalyzeConfig {
    fn default() -> Self {
        Self {
            depth: Some(20),
            movetime_ms: None,
            multipv: 1,
        }
    }
}

/// A single principal variation line from multi-PV analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiPvLine {
    /// PV rank (1-based).
    pub pv_index: u32,
    /// Centipawn score from the side to move's perspective.
    pub score_cp: Option<i32>,
    /// Mate-in-N from the side to move's perspective (positive = mating, negative = being mated).
    pub mate_in: Option<i32>,
    /// Depth reached for this line.
    pub depth: u32,
    /// Principal variation moves in UCI notation.
    pub pv: Vec<String>,
}

/// The result of analyzing a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisResult {
    /// Best move in UCI notation (e.g., "e2e4").
    pub best_move: String,
    /// Centipawn score from the side to move's perspective.
    pub score_cp: Option<i32>,
    /// Mate-in-N from the side to move's perspective.
    pub mate_in: Option<i32>,
    /// Depth reached by the engine.
    pub depth_reached: u32,
    /// Principal variation (best line) in UCI notation.
    pub pv: Vec<String>,
    /// All multi-PV lines (populated when `multipv > 1`).
    pub multipv_results: Vec<MultiPvLine>,
}

/// An async wrapper around a Stockfish (or compatible UCI) engine process.
pub struct Engine {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    shutdown_called: bool,
}

impl Engine {
    /// Spawns the engine process at `stockfish_path` and performs the UCI handshake.
    ///
    /// Sends `uci` and waits for `uciok`, then sends `isready` and waits for `readyok`.
    pub async fn new(stockfish_path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let mut child = tokio::process::Command::new(stockfish_path.as_ref())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(EngineError::SpawnFailed)?;

        let stdin = child.stdin.take().ok_or_else(|| {
            EngineError::ProtocolViolation("failed to capture engine stdin".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            EngineError::ProtocolViolation("failed to capture engine stdout".into())
        })?;
        let reader = BufReader::new(stdout);

        let mut engine = Self {
            child,
            stdin,
            reader,
            shutdown_called: false,
        };

        // UCI handshake
        engine.send_command("uci").await?;
        engine.wait_for_line("uciok").await?;

        engine.send_command("isready").await?;
        engine.wait_for_line("readyok").await?;

        Ok(engine)
    }

    /// Analyzes the given FEN position with the specified configuration.
    ///
    /// Returns the engine's evaluation including best move, score, depth, and PV lines.
    pub async fn analyze(
        &mut self,
        fen: &str,
        config: &AnalyzeConfig,
    ) -> Result<AnalysisResult, EngineError> {
        // Set multi-PV if requested
        if config.multipv > 1 {
            self.send_command(&format!("setoption name MultiPV value {}", config.multipv))
                .await?;
        }

        // Set position
        self.send_command(&format!("position fen {fen}")).await?;

        // Build the go command
        let mut go_cmd = String::from("go");
        if let Some(depth) = config.depth {
            go_cmd.push_str(&format!(" depth {depth}"));
        }
        if let Some(movetime) = config.movetime_ms {
            go_cmd.push_str(&format!(" movetime {movetime}"));
        }

        self.send_command(&go_cmd).await?;

        // Collect info lines until we get bestmove
        let mut info_lines: Vec<String> = Vec::new();
        let best_move;

        loop {
            let line = self.read_line().await?;
            if let Some(bm) = line.strip_prefix("bestmove ") {
                best_move = bm.split_whitespace().next().unwrap_or("").to_owned();
                break;
            }
            if line.starts_with("info ") {
                info_lines.push(line);
            }
        }

        if best_move.is_empty() {
            return Err(EngineError::ProtocolViolation(
                "bestmove response was empty".into(),
            ));
        }

        // Parse info lines to extract PV data
        let multipv_lines = parse::extract_final_pvs(&info_lines, config.multipv);

        // Build result from PV1 (best line)
        let (score_cp, mate_in, depth_reached, pv) = multipv_lines
            .first()
            .map(|line| (line.score_cp, line.mate_in, line.depth, line.pv.clone()))
            .unwrap_or((None, None, 0, Vec::new()));

        // Reset multi-PV to 1 after analysis
        if config.multipv > 1 {
            self.send_command("setoption name MultiPV value 1").await?;
        }

        Ok(AnalysisResult {
            best_move,
            score_cp,
            mate_in,
            depth_reached,
            pv,
            multipv_results: multipv_lines,
        })
    }

    /// Sends the `quit` command and waits for the process to exit.
    pub async fn shutdown(&mut self) -> Result<(), EngineError> {
        if self.shutdown_called {
            return Ok(());
        }
        self.shutdown_called = true;
        let _ = self.send_command("quit").await;
        let _ = timeout(Duration::from_secs(5), self.child.wait()).await;
        Ok(())
    }

    /// Sends a UCI command string (appends newline).
    async fn send_command(&mut self, cmd: &str) -> Result<(), EngineError> {
        self.stdin.write_all(format!("{cmd}\n").as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Reads a single line from the engine's stdout with a timeout.
    async fn read_line(&mut self) -> Result<String, EngineError> {
        let mut line = String::new();
        let bytes_read = timeout(DEFAULT_TIMEOUT, self.reader.read_line(&mut line))
            .await
            .map_err(|_| EngineError::Timeout(DEFAULT_TIMEOUT))?
            .map_err(EngineError::Io)?;

        if bytes_read == 0 {
            return Err(EngineError::UnexpectedEof);
        }

        Ok(line.trim_end().to_owned())
    }

    /// Reads lines until one matching `target` is found, discarding all others.
    async fn wait_for_line(&mut self, target: &str) -> Result<(), EngineError> {
        loop {
            let line = self.read_line().await?;
            if line.trim() == target {
                return Ok(());
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if !self.shutdown_called {
            // Best-effort kill if shutdown wasn't called.
            // `kill_on_drop(true)` on the child handles the actual process termination.
            let _ = self.child.start_kill();
        }
    }
}
