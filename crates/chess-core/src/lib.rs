// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! # chess-core
//!
//! Pure chess logic with no I/O. Provides position types, PGN parsing wrappers,
//! mistake classification (Inaccuracy / Mistake / Blunder), and mistake detection
//! given a sequence of (position, move, eval_before, eval_after) tuples.
//!
//! Depends on `shakmaty` for position representation and move generation.

mod pgn;

pub use pgn::{parse_pgn, ParsedMove, PgnEval};

/// Errors from chess-core operations.
#[derive(Debug, thiserror::Error)]
pub enum ChessCoreError {
    /// A PGN string could not be parsed.
    #[error("PGN parsing error: {0}")]
    PgnParse(String),

    /// An illegal or unrecognized move was encountered in a PGN.
    #[error("illegal move in PGN at ply {ply}: {san}")]
    IllegalMove {
        /// The half-move index where the error occurred.
        ply: u32,
        /// The SAN string that could not be resolved to a legal move.
        san: String,
    },
}
