// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Quality filters for puzzle candidate selection.
//!
//! Each filter is a pure function that can be unit-tested independently.

use engine::MultiPvLine;
use shakmaty::fen::Fen;
use shakmaty::uci::UciMove;
use shakmaty::{CastlingMode, Chess};

use crate::uci_destination_square;

/// Computes the centipawn gap between the best and second-best move from multi-PV results.
///
/// Returns `i32::MAX` if there's only one PV line (unique move by definition).
/// Returns 0 if no PV lines are available.
///
/// Mate scores are converted to ±10,000 cp sentinels for comparison.
pub fn unique_best_move_gap(multipv: &[MultiPvLine]) -> i32 {
    if multipv.is_empty() {
        return 0;
    }
    if multipv.len() < 2 {
        return i32::MAX;
    }

    let score1 = pv_score_cp(&multipv[0]);
    let score2 = pv_score_cp(&multipv[1]);

    // Gap = how much better the best move is than the second-best.
    // Higher score = better for the side to move.
    score1 - score2
}

/// Converts a PV line's score to a centipawn equivalent.
///
/// Mate scores are mapped to ±10,000 cp.
fn pv_score_cp(pv: &MultiPvLine) -> i32 {
    if let Some(mate) = pv.mate_in {
        if mate > 0 {
            10_000 - mate.abs() // closer mate = higher score
        } else {
            -10_000 + mate.abs() // being mated: further away is "less bad"
        }
    } else {
        pv.score_cp.unwrap_or(0)
    }
}

/// Checks if the engine's best move is a trivial recapture.
///
/// A "trivial recapture" is when:
/// 1. The opponent's previous move captured a piece on square S.
/// 2. The engine's best response is to capture back on square S.
///
/// This is detected by checking if `best_move` captures on the same destination
/// square as `previous_move`, and that the best move is indeed a capture in the
/// given position.
///
/// Returns `false` if the FEN is invalid or if the move cannot be parsed.
pub fn is_trivial_recapture(fen: &str, best_move_uci: &str, previous_move_uci: &str) -> bool {
    let prev_dest = match uci_destination_square(previous_move_uci) {
        Some(sq) => sq,
        None => return false,
    };

    let best_dest = match uci_destination_square(best_move_uci) {
        Some(sq) => sq,
        None => return false,
    };

    // Destination squares must match
    if prev_dest != best_dest {
        return false;
    }

    // Verify the best move is actually a capture in the current position
    let fen_parsed: Fen = match fen.parse() {
        Ok(f) => f,
        Err(_) => return false,
    };

    let position: Chess = match fen_parsed.into_position(CastlingMode::Standard) {
        Ok(p) => p,
        Err(_) => return false,
    };

    let best_uci: UciMove = match best_move_uci.parse() {
        Ok(m) => m,
        Err(_) => return false,
    };

    let legal_move = match best_uci.to_move(&position) {
        Ok(m) => m,
        Err(_) => return false,
    };

    legal_move.is_capture()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------
    // unique_best_move_gap tests
    // -------------------------------------------------------------------

    fn make_pv(index: u32, score_cp: Option<i32>, mate_in: Option<i32>) -> MultiPvLine {
        MultiPvLine {
            pv_index: index,
            score_cp,
            mate_in,
            depth: 20,
            pv: vec!["e2e4".to_string()],
        }
    }

    #[test]
    fn gap_with_clear_best_move() {
        let pvs = vec![make_pv(1, Some(200), None), make_pv(2, Some(50), None)];
        assert_eq!(unique_best_move_gap(&pvs), 150);
    }

    #[test]
    fn gap_with_ambiguous_position() {
        let pvs = vec![make_pv(1, Some(100), None), make_pv(2, Some(80), None)];
        assert_eq!(unique_best_move_gap(&pvs), 20);
    }

    #[test]
    fn gap_with_single_pv() {
        let pvs = vec![make_pv(1, Some(100), None)];
        assert_eq!(unique_best_move_gap(&pvs), i32::MAX);
    }

    #[test]
    fn gap_with_no_pvs() {
        assert_eq!(unique_best_move_gap(&[]), 0);
    }

    #[test]
    fn gap_with_mate_vs_cp() {
        // Best move is mate in 3 (side to move mates), second is +200cp
        let pvs = vec![make_pv(1, None, Some(3)), make_pv(2, Some(200), None)];
        // mate in 3 = 10000 - 3 = 9997
        assert_eq!(unique_best_move_gap(&pvs), 9997 - 200);
    }

    #[test]
    fn gap_with_both_mates() {
        // Both moves lead to mate but at different distances
        let pvs = vec![
            make_pv(1, None, Some(2)), // mate in 2 = 9998
            make_pv(2, None, Some(5)), // mate in 5 = 9995
        ];
        assert_eq!(unique_best_move_gap(&pvs), 3);
    }

    #[test]
    fn gap_with_negative_mate() {
        // Best is +500cp, second gets mated in 3
        let pvs = vec![
            make_pv(1, Some(500), None),
            make_pv(2, None, Some(-3)), // being mated in 3 = -10000 + 3 = -9997
        ];
        assert_eq!(unique_best_move_gap(&pvs), 500 - (-9997));
    }

    // -------------------------------------------------------------------
    // is_trivial_recapture tests
    // -------------------------------------------------------------------

    #[test]
    fn detects_simple_recapture() {
        // White to move. Black just captured on d5 (e.g., e6d5). White's best move
        // is to recapture on d5. Position has a black pawn on d5.
        let fen = "rnbqkb1r/pppp1ppp/5n2/3p4/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 3";
        assert!(is_trivial_recapture(fen, "e4d5", "e6d5"));
    }

    #[test]
    fn non_recapture_different_squares() {
        let fen = "rnbqkb1r/pppp1ppp/5n2/3p4/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 0 3";
        // Previous move went to d5, but best move goes to e5 — different squares
        assert!(!is_trivial_recapture(fen, "d2d4", "e6d5"));
    }

    #[test]
    fn not_a_capture_same_square() {
        // Best move goes to same destination but isn't a capture
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        // e2e4 is not a capture even though destination might match
        assert!(!is_trivial_recapture(fen, "e7e4", "e2e4"));
    }

    #[test]
    fn invalid_fen_returns_false() {
        assert!(!is_trivial_recapture("not a fen", "e2e4", "e7e5"));
    }

    #[test]
    fn short_uci_returns_false() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert!(!is_trivial_recapture(fen, "e4", "e7"));
    }
}
