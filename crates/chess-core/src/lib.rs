// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! # chess-core
//!
//! Pure chess logic with no I/O. Provides position types, PGN parsing wrappers,
//! mistake classification (Inaccuracy / Mistake / Blunder), and mistake detection
//! given a sequence of (position, move, eval_before, eval_after) tuples.
//!
//! Depends on `shakmaty` for position representation and move generation.
