// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

/**
 * Pure utility functions for sync stage display.
 * These are separate from UI components so they can be unit-tested with Vitest.
 */

import type { SyncStage } from "../api/sync";

/** Human-readable label for each pipeline stage. */
export function stageName(stage: SyncStage): string {
  switch (stage) {
    case "fetching":
      return "Fetching games";
    case "analyzing":
      return "Analyzing positions";
    case "detecting":
      return "Detecting blunders";
    case "generating":
      return "Generating puzzles";
    case "complete":
      return "Complete";
    case "error":
      return "Error";
  }
}

/** Returns true while the pipeline is still running (not yet complete or errored). */
export function isRunning(stage: SyncStage): boolean {
  return stage !== "complete" && stage !== "error";
}

/**
 * Formats a progress fraction (0–1) as a percentage string, e.g. "75%".
 * Clamps to [0, 100].
 */
export function formatPercent(fraction: number): string {
  const clamped = Math.max(0, Math.min(1, fraction));
  return `${Math.round(clamped * 100)}%`;
}
