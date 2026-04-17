// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Integration tests for PGN parsing and evaluation extraction.
//! Tests the full flow of parsing realistic Lichess PGN into structured move data.

#[test]
fn parse_realistic_lichess_pgn_with_evals() {
    let pgn = r#"[Event "Rated Rapid game"]
[Site "https://lichess.org/test1234"]
[White "TestUser"]
[Black "Opponent"]
[Result "1-0"]

1. e4 { [%eval 0.3] [%clk 0:10:00] } 1... e5 { [%eval 0.23] [%clk 0:10:00] } 2. Nf3 { [%eval 0.5] [%clk 0:09:58] } 2... Nc6 { [%eval 0.31] [%clk 0:09:57] } 3. Bc4 { [%eval 0.38] [%clk 0:09:55] } 3... Bc5 { [%eval 0.42] [%clk 0:09:54] } 1-0"#;

    let parsed = chess_core::parse_pgn(pgn, "white").unwrap();
    assert_eq!(parsed.len(), 6, "should have 6 half-moves");

    // All moves should have Lichess evals
    for (i, mv) in parsed.iter().enumerate() {
        assert!(
            mv.lichess_eval.is_some(),
            "move at ply {i} should have a Lichess eval"
        );
    }

    // Verify specific eval values (centipawns, from White's perspective)
    assert_eq!(parsed[0].lichess_eval.as_ref().unwrap().eval_cp, Some(30));
    assert_eq!(parsed[1].lichess_eval.as_ref().unwrap().eval_cp, Some(23));
    assert_eq!(parsed[2].lichess_eval.as_ref().unwrap().eval_cp, Some(50));

    // User move tagging (user is White)
    assert!(parsed[0].is_user_move, "ply 0 (e4) should be user's move");
    assert!(!parsed[1].is_user_move, "ply 1 (e5) should not be user's");
    assert!(parsed[2].is_user_move, "ply 2 (Nf3) should be user's move");

    // FEN tracking: initial position before first move
    assert!(parsed[0]
        .fen_before
        .contains("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR"));
    // fen_after of ply 0 should match fen_before of ply 1
    assert_eq!(parsed[0].fen_after, parsed[1].fen_before);
}

#[test]
fn parse_pgn_without_evals_yields_none() {
    let pgn = "1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 *";
    let parsed = chess_core::parse_pgn(pgn, "white").unwrap();

    assert_eq!(parsed.len(), 6);
    for mv in &parsed {
        assert!(
            mv.lichess_eval.is_none(),
            "moves without %eval should have no lichess_eval"
        );
    }
}

#[test]
fn parse_pgn_partial_evals() {
    let pgn = "1. e4 { [%eval 0.3] } 1... e5 2. Nf3 { [%eval 0.5] } *";
    let parsed = chess_core::parse_pgn(pgn, "black").unwrap();

    assert_eq!(parsed.len(), 3);
    assert!(parsed[0].lichess_eval.is_some());
    assert!(parsed[1].lichess_eval.is_none());
    assert!(parsed[2].lichess_eval.is_some());

    // User is Black
    assert!(!parsed[0].is_user_move);
    assert!(parsed[1].is_user_move);
    assert!(!parsed[2].is_user_move);
}

#[test]
fn parse_pgn_mate_eval() {
    let pgn = "1. e4 { [%eval 0.3] } 1... e5 { [%eval #3] } *";
    let parsed = chess_core::parse_pgn(pgn, "white").unwrap();

    let eval0 = parsed[0].lichess_eval.as_ref().unwrap();
    assert_eq!(eval0.eval_cp, Some(30));
    assert_eq!(eval0.eval_mate, None);

    let eval1 = parsed[1].lichess_eval.as_ref().unwrap();
    assert_eq!(eval1.eval_cp, None);
    assert_eq!(eval1.eval_mate, Some(3));
}

#[test]
fn all_fen_after_values_are_valid() {
    let pgn = "1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. O-O Nf6 *";
    let parsed = chess_core::parse_pgn(pgn, "white").unwrap();

    // Every fen_after should be a valid FEN with the expected structure
    for mv in &parsed {
        assert!(
            mv.fen_after.contains(' '),
            "fen_after at ply {} doesn't look like a valid FEN: {}",
            mv.ply,
            mv.fen_after
        );
    }

    // Chain integrity: fen_after[N] == fen_before[N+1]
    for i in 0..parsed.len() - 1 {
        assert_eq!(
            parsed[i].fen_after,
            parsed[i + 1].fen_before,
            "FEN chain broken between ply {} and {}",
            i,
            i + 1
        );
    }
}
