// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

/**
 * Typed wrappers for analysis-related Tauri `invoke` calls.
 * Frontend code should use these functions rather than calling `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";

/** Result of analyzing a single game. */
export interface AnalyzeGameResult {
  game_id: string;
  evals_stored: number;
  source: string;
}

/** Result of analyzing a batch of pending games. */
export interface AnalyzeBatchResult {
  games_analyzed: number;
  games_skipped: number;
  total_evals: number;
  errors: string[];
}

/** Progress event payload emitted during batch analysis. */
export interface AnalysisProgress {
  game_id: string;
  games_done: number;
  games_total: number;
  status: string;
}

/**
 * Analyzes a single game by its ID.
 * Extracts Lichess evals from PGN when available, falls back to Stockfish.
 */
export async function analyzeGame(
  gameId: string,
  forceStockfish: boolean,
): Promise<AnalyzeGameResult> {
  return invoke<AnalyzeGameResult>("analyze_game", {
    gameId,
    forceStockfish,
  });
}

/**
 * Analyzes all stored games that lack evaluations.
 * Emits `analysis-progress` events during processing.
 */
export async function analyzePendingGames(
  forceStockfish: boolean,
): Promise<AnalyzeBatchResult> {
  return invoke<AnalyzeBatchResult>("analyze_pending_games", {
    forceStockfish,
  });
}

/** Result of detecting mistakes in a single game. */
export interface DetectMistakesResult {
  game_id: string;
  inaccuracies: number;
  mistakes: number;
  blunders: number;
}

/** Result of detecting mistakes across all analyzed games. */
export interface DetectAllMistakesResult {
  games_processed: number;
  total_inaccuracies: number;
  total_mistakes: number;
  total_blunders: number;
  errors: string[];
}

/**
 * Runs blunder detection on a single game's stored evaluations.
 */
export async function detectMistakes(
  gameId: string,
): Promise<DetectMistakesResult> {
  return invoke<DetectMistakesResult>("detect_mistakes", { gameId });
}

/**
 * Runs blunder detection on all analyzed games.
 */
export async function detectAllMistakes(): Promise<DetectAllMistakesResult> {
  return invoke<DetectAllMistakesResult>("detect_all_mistakes");
}
