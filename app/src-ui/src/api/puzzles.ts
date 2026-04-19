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
