// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { describe, it, expect } from "vitest";
import { formatSuccessRate } from "../pages/StatsPage";

describe("formatSuccessRate", () => {
  it("returns em-dash when there are no attempts", () => {
    expect(formatSuccessRate(0, 0)).toBe("—");
  });

  it("returns 0% for a zero rate with attempts", () => {
    expect(formatSuccessRate(0, 5)).toBe("0%");
  });

  it("returns 100% for a perfect rate", () => {
    expect(formatSuccessRate(1.0, 10)).toBe("100%");
  });

  it("rounds to the nearest percent", () => {
    // 7 successes out of 10 = 0.7 = 70%
    expect(formatSuccessRate(0.7, 10)).toBe("70%");
  });

  it("rounds fractional percentages correctly", () => {
    // 1 success out of 3 ≈ 0.3333… → 33%
    expect(formatSuccessRate(1 / 3, 3)).toBe("33%");
  });

  it("rounds up at midpoint", () => {
    // 0.555 → 56%
    expect(formatSuccessRate(0.555, 100)).toBe("56%");
  });
});
