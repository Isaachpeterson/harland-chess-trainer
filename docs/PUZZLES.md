# Puzzle Generation (Slice 5)

## Overview

The puzzle-gen crate transforms detected blunders into training puzzles. For each blunder, the pre-blunder position is re-analyzed with multi-PV (2 lines) at depth ≥ 20, and quality filters determine whether the position makes a good puzzle.

## Quality Filters

A blunder is rejected as a puzzle candidate if any of the following apply:

| Filter | Threshold | Rationale |
|--------|-----------|-----------|
| **Best move matches user move** | Exact UCI match | Engine agrees user played correctly — false positive from blunder detection |
| **Eval gap** | Best move must be ≥ 50cp better than second-best | Ensures a uniquely correct answer |
| **Trivial recapture** | Best move captures on the same square the opponent just moved to | Too obvious to be instructive |
| **Insufficient depth** | Engine analysis < 18 depth | Unreliable "correct" answer |
| **Only one legal move** | Engine returns < 2 PV lines | No decision to make |

All thresholds are configurable via `PuzzleGenConfig`.

## Data Flow

```
Storage (blunders) → puzzle-gen → Storage (puzzles table)
                        ↕
                    Engine (multi-PV)
```

1. `generate_puzzles` Tauri command loads all blunders from storage
2. Filters out blunders that already have puzzles (idempotent)
3. Parses PGN to find previous move (for recapture detection)
4. Calls `puzzle_gen::generate_puzzles()` with engine access
5. Stores accepted puzzles and backfills `best_move` on mistake records

## Puzzle Schema

```sql
CREATE TABLE puzzles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mistake_id INTEGER NOT NULL REFERENCES mistakes(id) ON DELETE CASCADE,
    fen TEXT NOT NULL,
    solution_moves TEXT NOT NULL,  -- JSON array of UCI moves
    themes TEXT,                   -- JSON array (empty in v0.1)
    created_at INTEGER NOT NULL
);
```

## Frontend API

```typescript
import { generatePuzzles } from "./api/puzzles";
const result = await generatePuzzles();
// result: { puzzles_created, puzzles_rejected, puzzles_skipped, errors }
```
