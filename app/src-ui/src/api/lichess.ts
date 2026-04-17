// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

/**
 * Typed wrappers for Tauri `invoke` calls.
 * Frontend code should use these functions rather than calling `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";

/** Result of a game sync operation. */
export interface SyncResult {
  fetched: number;
  new: number;
  updated: number;
}

/**
 * Fetches games from Lichess for the given username and stores them locally.
 */
export async function syncGames(
  username: string,
  maxGames: number,
): Promise<SyncResult> {
  return invoke<SyncResult>("sync_games", {
    username,
    maxGames,
  });
}
