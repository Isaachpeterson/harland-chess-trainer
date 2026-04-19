// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { describe, it, expect } from "vitest";
import { stageName, isRunning, formatPercent } from "../utils/syncStages";
import type { SyncStage } from "../api/sync";

describe("stageName", () => {
  const cases: Array<[SyncStage, string]> = [
    ["fetching", "Fetching games"],
    ["analyzing", "Analyzing positions"],
    ["detecting", "Detecting blunders"],
    ["generating", "Generating puzzles"],
    ["complete", "Complete"],
    ["error", "Error"],
  ];

  it.each(cases)("stage '%s' → '%s'", (stage, expected) => {
    expect(stageName(stage)).toBe(expected);
  });
});

describe("isRunning", () => {
  it("returns true for active stages", () => {
    expect(isRunning("fetching")).toBe(true);
    expect(isRunning("analyzing")).toBe(true);
    expect(isRunning("detecting")).toBe(true);
    expect(isRunning("generating")).toBe(true);
  });

  it("returns false for terminal stages", () => {
    expect(isRunning("complete")).toBe(false);
    expect(isRunning("error")).toBe(false);
  });
});

describe("formatPercent", () => {
  it("formats a fraction as a percentage string", () => {
    expect(formatPercent(0)).toBe("0%");
    expect(formatPercent(0.25)).toBe("25%");
    expect(formatPercent(0.5)).toBe("50%");
    expect(formatPercent(0.75)).toBe("75%");
    expect(formatPercent(1)).toBe("100%");
  });

  it("clamps values outside [0, 1]", () => {
    expect(formatPercent(-0.5)).toBe("0%");
    expect(formatPercent(1.5)).toBe("100%");
  });

  it("rounds fractional percentages", () => {
    expect(formatPercent(0.333)).toBe("33%");
    expect(formatPercent(0.666)).toBe("67%");
  });
});
