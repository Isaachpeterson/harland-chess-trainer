// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

/**
 * Typed wrappers for puzzle-generation Tauri `invoke` calls.
 * Frontend code should use these functions rather than calling `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";

/** Result of generating puzzles from detected blunders. */
export interface GeneratePuzzlesResult {
  puzzles_created: number;
  puzzles_rejected: number;
  puzzles_skipped: number;
  errors: string[];
}

/**
 * Generates training puzzles from all detected blunders that don't already have puzzles.
 * Re-analyzes pre-blunder positions with multi-PV, applies quality filters,
 * and stores accepted puzzles.
 */
export async function generatePuzzles(): Promise<GeneratePuzzlesResult> {
  return invoke<GeneratePuzzlesResult>("generate_puzzles");
}

/** A puzzle returned from the backend. */
export interface PuzzleResponse {
  id: number;
  fen: string;
  solution_moves: string[];
}

/** Result of submitting a puzzle attempt. */
export interface SubmitAttemptResult {
  attempt_id: number;
}

/** Aggregate puzzle attempt statistics. */
export interface AttemptsSummaryResponse {
  total_attempts: number;
  total_successes: number;
  success_rate: number;
  puzzles_attempted: number;
  puzzles_attempted_today: number;
}

/**
 * Returns the next puzzle to solve. Prefers unattempted puzzles; falls back to
 * already-attempted ones if all have been seen. Returns `null` if no puzzles exist.
 */
export async function getNextPuzzle(): Promise<PuzzleResponse | null> {
  return invoke<PuzzleResponse | null>("get_next_puzzle");
}

/**
 * Records a puzzle attempt.
 */
export async function submitPuzzleAttempt(
  puzzleId: number,
  success: boolean,
  timeTakenMs: number,
  movePlayed: string,
): Promise<SubmitAttemptResult> {
  return invoke<SubmitAttemptResult>("submit_puzzle_attempt", {
    puzzleId,
    success,
    timeTakenMs,
    movePlayed,
  });
}

/**
 * Returns aggregate puzzle attempt statistics.
 */
export async function getAttemptsSummary(): Promise<AttemptsSummaryResponse> {
  return invoke<AttemptsSummaryResponse>("get_attempts_summary");
}
