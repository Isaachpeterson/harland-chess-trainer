// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

/**
 * Typed wrappers for settings-related Tauri `invoke` calls.
 * Frontend code should use these functions rather than calling `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";

/** User preferences stored in the local database. */
export interface UserSettings {
  lichess_username: string;
  max_games: number;
  /** When true, always use local Stockfish instead of Lichess embedded evals. */
  use_stockfish: boolean;
  inaccuracy_threshold_cp: number;
  mistake_threshold_cp: number;
  blunder_threshold_cp: number;
}

/** Default settings matching the backend's MistakeThresholds::default(). */
export const DEFAULT_SETTINGS: UserSettings = {
  lichess_username: "",
  max_games: 50,
  use_stockfish: false,
  inaccuracy_threshold_cp: 50,
  mistake_threshold_cp: 100,
  blunder_threshold_cp: 200,
};

/** Returns the user's stored settings. */
export async function getSettings(): Promise<UserSettings> {
  return invoke<UserSettings>("get_settings");
}

/** Persists the user's settings. */
export async function saveSettings(settings: UserSettings): Promise<void> {
  return invoke<void>("save_settings", { settings });
}
