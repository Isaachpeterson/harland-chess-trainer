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

## Puzzle Solving Interaction Model (Slice 8)

The puzzle solving UI presents positions from generated puzzles on a chessground board. The user must find the best move.

### Libraries

- **chessground** (GPL-3.0) — the same board component used by Lichess. Renders the board, handles drag-and-drop / click-click piece movement, highlights, and animations.
- **chess.js** (BSD-2-Clause) — move validation and legal move generation. Used to compute legal destinations (for chessground's `movable.dests`) and to validate the user's move against the solution.

### Interaction flow

1. `PuzzlePage` calls `getNextPuzzle()` to fetch a puzzle (FEN + solution moves).
2. Board orientation is derived from the FEN's side to move (the user always plays as the side to move).
3. `chess.js` computes legal destinations, passed to chessground as `movable.dests`.
4. User drags or clicks a piece. chessground fires `movable.events.after(orig, dest)`.
5. The move is converted to UCI and compared against `solution_moves[0]`.
6. **Correct:** green "Correct!" feedback. Board shows the resulting position.
7. **Incorrect:** red "Incorrect." feedback. The correct move is shown in SAN notation, then animated onto the board after 800ms.
8. Attempt is recorded via `submitPuzzleAttempt()` with success/failure, time taken, and the move played.
9. User clicks "Next Puzzle" or presses Spacebar to load the next puzzle.

### Promotion handling

All promotions default to queen. Non-queen promotions are rare in puzzles (the quality filters ensure a uniquely best move, which is almost always queen promotion when promotion is involved). If a future puzzle requires under-promotion, the solution matching normalizes queen promotions (`e7e8q` matches `e7e8`).

### Component structure

```
PuzzlePage (src/pages/PuzzlePage.tsx)
  └── PuzzleBoard (src/components/PuzzleBoard.tsx)
        └── chessground instance (lifecycle managed by React ref)
```

`PuzzleBoard` is a controlled component: it receives FEN, orientation, legal destinations, and interaction flags as props. `PuzzlePage` owns all puzzle state (loading, solving, correct, incorrect, empty) and orchestrates the flow.
