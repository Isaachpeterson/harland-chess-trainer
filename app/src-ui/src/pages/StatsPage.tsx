// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { useState, useEffect } from "react";
import { getAttemptsSummary } from "../api/puzzles";
import type { AttemptsSummaryResponse } from "../api/puzzles";

export function StatsPage() {
  const [summary, setSummary] = useState<AttemptsSummaryResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getAttemptsSummary()
      .then(setSummary)
      .catch((e: unknown) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="page">
        <h2>Stats</h2>
        <p>Loading…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="page">
        <h2>Stats</h2>
        <p className="error-msg">{error}</p>
      </div>
    );
  }

  if (!summary) {
    return (
      <div className="page">
        <h2>Stats</h2>
        <p className="hint">No data available.</p>
      </div>
    );
  }

  return (
    <div className="page stats-page">
      <h2>Stats</h2>

      <div className="stats-grid">
        <StatCard
          label="Puzzles Solved"
          value={String(summary.total_successes)}
        />
        <StatCard
          label="Success Rate"
          value={formatSuccessRate(summary.success_rate, summary.total_attempts)}
        />
        <StatCard
          label="Attempted Today"
          value={String(summary.puzzles_attempted_today)}
        />
        <StatCard
          label="Total Attempts"
          value={String(summary.total_attempts)}
        />
      </div>

      {summary.total_attempts === 0 && (
        <p className="hint stats-empty-hint">
          Solve some puzzles on the Train page to see your stats here.
        </p>
      )}
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat-card">
      <span className="stat-value">{value}</span>
      <span className="stat-label">{label}</span>
    </div>
  );
}

/**
 * Formats a success rate fraction as a percentage string.
 * Returns "—" when there are no attempts yet.
 */
export function formatSuccessRate(rate: number, totalAttempts: number): string {
  if (totalAttempts === 0) return "—";
  return `${Math.round(rate * 100)}%`;
}
