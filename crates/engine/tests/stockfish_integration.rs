// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Integration tests for the engine crate.
//!
//! These tests require a Stockfish binary. Set the `STOCKFISH_PATH` environment
//! variable to the path of the Stockfish executable (or just `stockfish` if it's
//! on PATH). All tests are `#[ignore]` by default — run with:
//!
//! ```sh
//! STOCKFISH_PATH=stockfish cargo test -p engine -- --ignored
//! ```

use engine::{AnalyzeConfig, Engine};

fn stockfish_path() -> String {
    std::env::var("STOCKFISH_PATH").unwrap_or_else(|_| "stockfish".to_owned())
}

#[tokio::test]
#[ignore]
async fn engine_spawns_and_handshakes() {
    let engine = Engine::new(stockfish_path()).await;
    assert!(engine.is_ok(), "Engine should spawn and handshake: {:?}", engine.err());
    let mut engine = engine.unwrap();
    engine.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
#[ignore]
async fn analyze_starting_position() {
    let mut engine = Engine::new(stockfish_path())
        .await
        .expect("engine should spawn");

    let config = AnalyzeConfig {
        depth: Some(18),
        movetime_ms: None,
        multipv: 1,
    };

    let result = engine
        .analyze(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &config,
        )
        .await
        .expect("analysis should succeed");

    assert!(!result.best_move.is_empty(), "should have a best move");
    assert!(result.score_cp.is_some(), "starting position should have a cp score");
    assert!(result.depth_reached >= 18, "should reach requested depth");
    assert!(!result.pv.is_empty(), "should have a principal variation");

    engine.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
#[ignore]
async fn analyze_mate_in_one() {
    // Back-rank mate: White Rd8# is mate in 1.
    // 1k6/ppp5/8/8/8/8/8/1K1R4 w - - 0 1
    let fen = "1k6/ppp5/8/8/8/8/8/1K1R4 w - - 0 1";

    let mut engine = Engine::new(stockfish_path())
        .await
        .expect("engine should spawn");

    let config = AnalyzeConfig {
        depth: Some(10),
        movetime_ms: None,
        multipv: 1,
    };

    let result = engine.analyze(fen, &config).await.expect("analysis should succeed");

    assert_eq!(result.best_move, "d1d8", "Rd8# is the only mate");
    assert_eq!(result.mate_in, Some(1), "should be mate in 1");
    assert!(result.score_cp.is_none(), "mate score should not have cp");

    engine.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
#[ignore]
async fn analyze_multipv() {
    let mut engine = Engine::new(stockfish_path())
        .await
        .expect("engine should spawn");

    let config = AnalyzeConfig {
        depth: Some(15),
        movetime_ms: None,
        multipv: 3,
    };

    let result = engine
        .analyze(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &config,
        )
        .await
        .expect("analysis should succeed");

    assert!(
        result.multipv_results.len() >= 2,
        "should have multiple PV lines, got {}",
        result.multipv_results.len()
    );
    // First PV should match the main result
    assert_eq!(result.multipv_results[0].pv_index, 1);

    engine.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
#[ignore]
async fn analyze_known_position_depth_18() {
    // Open game position from the manual verification step
    let fen = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";

    let mut engine = Engine::new(stockfish_path())
        .await
        .expect("engine should spawn");

    let config = AnalyzeConfig {
        depth: Some(18),
        movetime_ms: None,
        multipv: 1,
    };

    let result = engine.analyze(fen, &config).await.expect("analysis should succeed");

    assert!(!result.best_move.is_empty());
    assert!(result.depth_reached >= 18, "depth should be >= 18, got {}", result.depth_reached);
    assert!(result.score_cp.is_some(), "should have a cp score for this position");
    // The position is roughly equal, score should be within a pawn
    let cp = result.score_cp.unwrap();
    assert!(
        cp.abs() < 100,
        "1.e4 e5 should be roughly equal, got {cp}cp"
    );

    engine.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
#[ignore]
async fn multiple_analyses_same_engine() {
    let mut engine = Engine::new(stockfish_path())
        .await
        .expect("engine should spawn");

    let config = AnalyzeConfig {
        depth: Some(12),
        movetime_ms: None,
        multipv: 1,
    };

    // Analyze two different positions sequentially
    let r1 = engine
        .analyze(
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
            &config,
        )
        .await
        .expect("first analysis should succeed");

    let r2 = engine
        .analyze(
            "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1",
            &config,
        )
        .await
        .expect("second analysis should succeed");

    assert!(!r1.best_move.is_empty());
    assert!(!r2.best_move.is_empty());

    engine.shutdown().await.expect("shutdown should succeed");
}

#[tokio::test]
#[ignore]
async fn shutdown_is_idempotent() {
    let mut engine = Engine::new(stockfish_path())
        .await
        .expect("engine should spawn");

    engine.shutdown().await.expect("first shutdown should succeed");
    engine.shutdown().await.expect("second shutdown should succeed");
}
