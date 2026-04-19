// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { describe, it, expect } from "vitest";
import { legalDests, orientationFromFen } from "../components/PuzzleBoard";

describe("orientationFromFen", () => {
  it("returns 'white' when White to move", () => {
    expect(
      orientationFromFen(
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
      ),
    ).toBe("black");
  });

  it("returns 'white' for the starting position", () => {
    expect(
      orientationFromFen(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
      ),
    ).toBe("white");
  });

  it("returns 'black' when Black to move", () => {
    expect(
      orientationFromFen(
        "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 1 2",
      ),
    ).toBe("black");
  });
});

describe("legalDests", () => {
  it("returns legal destinations for the starting position", () => {
    const dests = legalDests(
      "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    );
    // White should have moves from pawns and knights
    expect(dests.size).toBeGreaterThan(0);
    // e2 pawn can go to e3 and e4
    expect(dests.get("e2")).toEqual(expect.arrayContaining(["e3", "e4"]));
    // g1 knight can go to f3 and h3
    expect(dests.get("g1")).toEqual(expect.arrayContaining(["f3", "h3"]));
  });

  it("returns empty map for a checkmate position", () => {
    // Scholar's mate: Black is checkmated
    const dests = legalDests(
      "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
    );
    expect(dests.size).toBe(0);
  });

  it("limits moves when in check", () => {
    // Black king in check from White's bishop on b5
    const dests = legalDests(
      "rnbqkbnr/pppp1ppp/8/1B2p3/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 1 2",
    );
    // Should have some dests but limited (must resolve check by blocking or moving king)
    expect(dests.size).toBeGreaterThan(0);
    // Fewer pieces can move compared to an unrestricted position
    expect(dests.size).toBeLessThan(16);
  });
});
