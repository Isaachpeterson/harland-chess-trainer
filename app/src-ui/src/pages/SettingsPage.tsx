// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { useState, useEffect } from "react";
import { getSettings, saveSettings, DEFAULT_SETTINGS } from "../api/settings";
import type { UserSettings } from "../api/settings";

export function SettingsPage() {
  const [settings, setSettings] = useState<UserSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getSettings()
      .then(setSettings)
      .catch((e: unknown) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setSaved(false);
    setError(null);
    try {
      await saveSettings(settings);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  function update<K extends keyof UserSettings>(
    key: K,
    value: UserSettings[K],
  ) {
    setSettings((prev) => ({ ...prev, [key]: value }));
  }

  if (loading) {
    return <p>Loading settings…</p>;
  }

  return (
    <div className="page">
      <h2>Settings</h2>
      <form onSubmit={handleSave} className="settings-form">
        <fieldset>
          <legend>Lichess Account</legend>

          <label>
            Lichess username
            <input
              type="text"
              value={settings.lichess_username}
              onChange={(e) =>
                update("lichess_username", e.currentTarget.value.trim())
              }
              placeholder="e.g. Isaachpeterson"
              autoComplete="off"
              spellCheck={false}
              disabled={saving}
            />
          </label>

          <label>
            Games to fetch
            <input
              type="number"
              value={settings.max_games}
              onChange={(e) =>
                update("max_games", Number(e.currentTarget.value))
              }
              min={1}
              max={300}
              disabled={saving}
            />
            <span className="hint">
              Maximum recent games to fetch per sync (1–300)
            </span>
          </label>
        </fieldset>

        <fieldset>
          <legend>Analysis</legend>

          <label className="toggle-label">
            <input
              type="checkbox"
              checked={settings.use_stockfish}
              onChange={(e) => update("use_stockfish", e.currentTarget.checked)}
              disabled={saving}
            />
            Always use local Stockfish
            <span className="hint">
              Slower but consistent. By default, Lichess embedded evals are used
              when available.
            </span>
          </label>
        </fieldset>

        <fieldset>
          <legend>Mistake Thresholds (centipawns)</legend>

          <label>
            Inaccuracy (cp drop ≥)
            <input
              type="number"
              value={settings.inaccuracy_threshold_cp}
              onChange={(e) =>
                update("inaccuracy_threshold_cp", Number(e.currentTarget.value))
              }
              min={10}
              max={500}
              disabled={saving}
            />
          </label>

          <label>
            Mistake (cp drop ≥)
            <input
              type="number"
              value={settings.mistake_threshold_cp}
              onChange={(e) =>
                update("mistake_threshold_cp", Number(e.currentTarget.value))
              }
              min={10}
              max={500}
              disabled={saving}
            />
          </label>

          <label>
            Blunder (cp drop ≥)
            <input
              type="number"
              value={settings.blunder_threshold_cp}
              onChange={(e) =>
                update("blunder_threshold_cp", Number(e.currentTarget.value))
              }
              min={10}
              max={1000}
              disabled={saving}
            />
          </label>
        </fieldset>

        {error && <p className="error-msg">Error: {error}</p>}
        {saved && <p className="success-msg">Settings saved.</p>}

        <button type="submit" disabled={saving}>
          {saving ? "Saving…" : "Save Settings"}
        </button>
      </form>
    </div>
  );
}
