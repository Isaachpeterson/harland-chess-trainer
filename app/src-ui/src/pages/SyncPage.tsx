// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { fullSync } from "../api/sync";
import type { FullSyncResult } from "../api/sync";
import { useSyncProgress } from "../hooks/useSyncProgress";
import { stageName, formatPercent } from "../utils/syncStages";

export function SyncPage() {
  const navigate = useNavigate();
  const { progress, isRunning } = useSyncProgress();
  const [result, setResult] = useState<FullSyncResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [started, setStarted] = useState(false);

  async function handleSync() {
    setStarted(true);
    setResult(null);
    setError(null);
    try {
      const res = await fullSync();
      setResult(res);
    } catch (e: unknown) {
      setError(String(e));
    }
  }

  const percent = progress ? formatPercent(progress.fraction) : "0%";
  const stageLabel = progress ? stageName(progress.stage) : "";

  return (
    <div className="page">
      <h2>Sync &amp; Analyze</h2>
      <p>
        Fetch your recent Lichess games, analyze them with Stockfish, detect
        blunders, and generate training puzzles — all in one click.
      </p>
      <p className="hint">
        Make sure your username is set in{" "}
        <button
          className="link-btn"
          onClick={() => navigate("/settings")}
          disabled={isRunning}
        >
          Settings
        </button>{" "}
        before syncing.
      </p>

      <button
        onClick={handleSync}
        disabled={isRunning}
        className="primary-btn"
      >
        {isRunning ? "Running…" : "Fetch & Analyze"}
      </button>

      {/* Progress bar — shown while running or after completion */}
      {started && (
        <div className="progress-section">
          <div className="progress-bar-track">
            <div
              className="progress-bar-fill"
              style={{
                width: progress ? formatPercent(progress.fraction) : "0%",
              }}
            />
          </div>

          <p className="progress-label">
            {isRunning ? (
              <>
                <span className="stage-name">{stageLabel}</span>
                {" — "}
                <span>{progress?.message ?? "Starting…"}</span>
                <span className="progress-percent"> {percent}</span>
              </>
            ) : progress?.stage === "complete" ? (
              <>✓ {progress.message}</>
            ) : null}
          </p>

          {error && <p className="error-msg">Error: {error}</p>}
        </div>
      )}

      {/* Summary after completion */}
      {result && (
        <div className="sync-results">
          <h3>Results</h3>
          <table>
            <tbody>
              <tr>
                <td>Games fetched</td>
                <td>{result.fetched}</td>
              </tr>
              <tr>
                <td>New games</td>
                <td>{result.new_games}</td>
              </tr>
              <tr>
                <td>Games analyzed</td>
                <td>{result.games_analyzed}</td>
              </tr>
              <tr>
                <td>Blunders detected</td>
                <td>{result.total_blunders}</td>
              </tr>
              <tr>
                <td>Puzzles generated</td>
                <td>{result.puzzles_created}</td>
              </tr>
            </tbody>
          </table>

          {result.errors.length > 0 && (
            <details>
              <summary>{result.errors.length} error(s) during sync</summary>
              <ul className="error-list">
                {result.errors.map((e, i) => (
                  <li key={i}>{e}</li>
                ))}
              </ul>
            </details>
          )}
        </div>
      )}
    </div>
  );
}
