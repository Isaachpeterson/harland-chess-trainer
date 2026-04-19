// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { describe, it, expect } from "vitest";
import {
  matchesSolutionMove,
  formatSolutionDisplay,
} from "../pages/PuzzlePage";

describe("matchesSolutionMove", () => {
  it("matches identical UCI moves", () => {
    expect(matchesSolutionMove("e2e4", "e2e4")).toBe(true);
  });

  it("matches when both have queen promotion", () => {
    expect(matchesSolutionMove("e7e8q", "e7e8q")).toBe(true);
  });

  it("matches when user includes 'q' but solution doesn't", () => {
    expect(matchesSolutionMove("e7e8q", "e7e8")).toBe(true);
  });

  it("matches when solution includes 'q' but user doesn't", () => {
    expect(matchesSolutionMove("e7e8", "e7e8q")).toBe(true);
  });

  it("does not match different moves", () => {
    expect(matchesSolutionMove("e2e4", "d2d4")).toBe(false);
  });

  it("does not match different promotions (non-queen)", () => {
    expect(matchesSolutionMove("e7e8n", "e7e8r")).toBe(false);
  });

  it("does not match knight promotion vs queen promotion", () => {
    expect(matchesSolutionMove("e7e8n", "e7e8q")).toBe(false);
  });

  it("does not match knight promotion vs no promotion specified", () => {
    // "e7e8n" (knight) should not match "e7e8" (implicitly queen)
    expect(matchesSolutionMove("e7e8n", "e7e8")).toBe(false);
  });
});

describe("formatSolutionDisplay", () => {
  it("converts UCI move to SAN for a known position", () => {
    // Starting position: e2e4 should give "e4"
    const fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    expect(formatSolutionDisplay(fen, "e2e4")).toBe("e4");
  });

  it("shows capture notation", () => {
    // Position where Nf6 can capture the pawn on e4
    const fen = "rnbqkb1r/pppppppp/5n2/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 2";
    expect(formatSolutionDisplay(fen, "f6e4")).toBe("Nxe4");
  });

  it("falls back to UCI string for invalid FEN", () => {
    expect(formatSolutionDisplay("invalid", "e2e4")).toBe("e2e4");
  });

  it("shows check symbol", () => {
    // Scholar's mate position: Qxf7#
    const fen =
      "r1bqkbnr/pppp1ppp/2n5/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4";
    const result = formatSolutionDisplay(fen, "h5f7");
    expect(result).toMatch(/Qxf7/);
  });
});
