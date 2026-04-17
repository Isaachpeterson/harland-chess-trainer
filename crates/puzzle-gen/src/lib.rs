// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! # puzzle-gen
//!
//! Takes analyzed games and produces training puzzles. Generates `PuzzleCandidate`
//! structs from detected blunders, applies quality filters (eval gap between best
//! and second-best move, solution uniqueness), and prepares puzzles for storage.
