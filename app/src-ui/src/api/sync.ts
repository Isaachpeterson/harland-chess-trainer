// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

/**
 * Typed wrapper for the `full_sync` Tauri command and its progress event.
 * Frontend code should use this rather than calling `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";

/** Stage names emitted by the `full_sync` pipeline. */
export type SyncStage =
  | "fetching"
  | "analyzing"
  | "detecting"
  | "generating"
  | "complete"
  | "error";

/** Progress event payload emitted by `full_sync` on the `"sync-progress"` channel. */
export interface SyncProgress {
  stage: SyncStage;
  message: string;
  /** Completion fraction in the range [0, 1]. */
  fraction: number;
}

/** Combined result returned by `full_sync` on completion. */
export interface FullSyncResult {
  fetched: number;
  new_games: number;
  games_analyzed: number;
  total_blunders: number;
  puzzles_created: number;
  errors: string[];
}

/**
 * Runs the full sync pipeline: fetch → analyze → detect → generate.
 * Reads the Lichess username and preferences from the stored settings.
 * Emits `"sync-progress"` events during processing.
 */
export async function fullSync(): Promise<FullSyncResult> {
  return invoke<FullSyncResult>("full_sync");
}
