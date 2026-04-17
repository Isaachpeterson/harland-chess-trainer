// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! # engine
//!
//! Stockfish process management via the UCI protocol. Spawns a Stockfish process,
//! handles the UCI handshake, and provides `analyze_position(fen, depth_or_time)`
//! returning centipawn evaluation, best move, and principal variation.
//! Supports multi-PV analysis and graceful shutdown.
