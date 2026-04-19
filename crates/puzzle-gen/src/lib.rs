// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! # puzzle-gen
//!
//! Takes analyzed games and produces training puzzles. Generates `PuzzleCandidate`
//! structs from detected blunders, applies quality filters (eval gap between best
//! and second-best move, solution uniqueness), and prepares puzzles for storage.

mod filters;

use engine::{AnalyzeConfig, Engine};
use thiserror::Error;

pub use filters::{is_trivial_recapture, unique_best_move_gap};

/// Errors from puzzle generation.
#[derive(Debug, Error)]
pub enum PuzzleGenError {
    #[error("engine error: {0}")]
    Engine(#[from] engine::EngineError),

    #[error("invalid FEN: {0}")]
    InvalidFen(String),
}

/// Input for puzzle generation from a detected blunder.
#[derive(Debug, Clone)]
pub struct BlunderInput {
    /// Database ID of the mistake record.
    pub mistake_id: i64,
    /// Lichess game ID.
    pub game_id: String,
    /// Half-move index of the blunder.
    pub ply: i32,
    /// FEN of the position before the user's blunder move.
    pub fen_before: String,
    /// The user's blunder move in UCI notation.
    pub user_move: String,
    /// The opponent's move that led to `fen_before`, in UCI notation.
    /// Used for recapture detection. `None` if the blunder was on the first move.
    pub previous_move_uci: Option<String>,
}

/// A generated puzzle candidate, ready for storage.
#[derive(Debug, Clone)]
pub struct PuzzleCandidate {
    /// Database ID of the originating mistake.
    pub mistake_id: i64,
    /// Lichess game ID of the source game.
    pub source_game_id: String,
    /// Half-move index in the source game.
    pub source_ply: i32,
    /// FEN of the puzzle position (position before the blunder — user must find the best move).
    pub fen: String,
    /// Solution moves in UCI notation. For v0.1, a single-move solution (the engine's best).
    pub solution_uci_moves: Vec<String>,
    /// The engine's best move in UCI notation (also stored in the mistake's `best_move` column).
    pub best_move_uci: String,
    /// Tactical themes (empty for v0.1; populated in v0.3).
    pub themes: Vec<String>,
}

/// Configuration for puzzle quality filters.
#[derive(Debug, Clone)]
pub struct PuzzleGenConfig {
    /// Minimum centipawn gap between the best and second-best move.
    pub min_eval_gap_cp: i32,
    /// Minimum engine depth for the analysis to be considered reliable.
    pub min_depth: u32,
    /// Whether to filter out trivial recapture puzzles.
    pub filter_recaptures: bool,
}

impl Default for PuzzleGenConfig {
    fn default() -> Self {
        Self {
            min_eval_gap_cp: 50,
            min_depth: 18,
            filter_recaptures: true,
        }
    }
}

/// Reason a blunder was rejected as a puzzle candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterReason {
    /// The gap between the best and second-best move was too small.
    EvalGapTooSmall { gap_cp: i32 },
    /// The best move is a trivial recapture.
    TrivialRecapture,
    /// Engine analysis did not reach the required depth.
    InsufficientDepth { depth: u32, required: u32 },
    /// The engine returned an error for this position.
    EngineError(String),
    /// Only one legal move exists — no puzzle value.
    OnlyOneLegalMove,
    /// The engine's best move is the same move the user played — not a real blunder.
    BestMoveMatchesUserMove,
}

/// Result for a single blunder: either an accepted puzzle or a rejection.
#[derive(Debug)]
pub enum PuzzleGenResult {
    Accepted(PuzzleCandidate),
    Rejected {
        mistake_id: i64,
        reason: FilterReason,
    },
}

/// Generates puzzles from a set of detected blunders.
///
/// For each blunder, re-analyzes the pre-blunder position with multi-PV to determine
/// whether the position has a uniquely best move. Applies quality filters and returns
/// accepted puzzle candidates alongside rejection reasons for filtered blunders.
pub async fn generate_puzzles(
    blunders: &[BlunderInput],
    engine: &mut Engine,
    config: &PuzzleGenConfig,
) -> Vec<PuzzleGenResult> {
    let mut results = Vec::with_capacity(blunders.len());

    let analyze_config = AnalyzeConfig {
        depth: Some(config.min_depth.max(20)), // at least depth 20, or min_depth if higher
        movetime_ms: None,
        multipv: 2,
    };

    for blunder in blunders {
        let result = generate_single_puzzle(blunder, engine, &analyze_config, config).await;
        results.push(result);
    }

    results
}

/// Generates a puzzle from a single blunder, applying all quality filters.
async fn generate_single_puzzle(
    blunder: &BlunderInput,
    engine: &mut Engine,
    analyze_config: &AnalyzeConfig,
    config: &PuzzleGenConfig,
) -> PuzzleGenResult {
    // Run multi-PV analysis on the pre-blunder position
    let analysis = match engine.analyze(&blunder.fen_before, analyze_config).await {
        Ok(a) => a,
        Err(e) => {
            return PuzzleGenResult::Rejected {
                mistake_id: blunder.mistake_id,
                reason: FilterReason::EngineError(e.to_string()),
            };
        }
    };

    // Check depth floor
    if analysis.depth_reached < config.min_depth {
        return PuzzleGenResult::Rejected {
            mistake_id: blunder.mistake_id,
            reason: FilterReason::InsufficientDepth {
                depth: analysis.depth_reached,
                required: config.min_depth,
            },
        };
    }

    // Reject if the engine's best move is the same as the user's move.
    // This means the upstream blunder detection was a false positive — the user
    // actually played the best move, so there's nothing to train on.
    if analysis.best_move == blunder.user_move {
        return PuzzleGenResult::Rejected {
            mistake_id: blunder.mistake_id,
            reason: FilterReason::BestMoveMatchesUserMove,
        };
    }

    // Check if there's only one legal move (no puzzle value)
    if analysis.multipv_results.len() < 2 {
        // Engine only returned one PV line → only one legal move (or nearly so)
        return PuzzleGenResult::Rejected {
            mistake_id: blunder.mistake_id,
            reason: FilterReason::OnlyOneLegalMove,
        };
    }

    // Check unique-best-move gap
    let gap = unique_best_move_gap(&analysis.multipv_results);
    if gap < config.min_eval_gap_cp {
        return PuzzleGenResult::Rejected {
            mistake_id: blunder.mistake_id,
            reason: FilterReason::EvalGapTooSmall { gap_cp: gap },
        };
    }

    // Check trivial recapture
    if config.filter_recaptures {
        if let Some(ref prev_move) = blunder.previous_move_uci {
            if is_trivial_recapture(&blunder.fen_before, &analysis.best_move, prev_move) {
                return PuzzleGenResult::Rejected {
                    mistake_id: blunder.mistake_id,
                    reason: FilterReason::TrivialRecapture,
                };
            }
        }
    }

    // All filters passed — create puzzle candidate
    PuzzleGenResult::Accepted(PuzzleCandidate {
        mistake_id: blunder.mistake_id,
        source_game_id: blunder.game_id.clone(),
        source_ply: blunder.ply,
        fen: blunder.fen_before.clone(),
        solution_uci_moves: vec![analysis.best_move.clone()],
        best_move_uci: analysis.best_move.clone(),
        themes: Vec::new(),
    })
}

/// Extracts the destination square from a UCI move string (e.g., "e2e4" → "e4").
pub fn uci_destination_square(uci: &str) -> Option<&str> {
    if uci.len() >= 4 {
        Some(&uci[2..4])
    } else {
        None
    }
}
