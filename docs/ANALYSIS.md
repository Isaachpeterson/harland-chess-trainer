# Analysis Pipeline

> **Status:** Implemented in Slice 3.

This document describes how Harland Chess Trainer produces per-move evaluations for each stored game.

---

## Strategy: Lichess-first, Stockfish fallback

The analysis pipeline follows a **Lichess-first** strategy:

1. **Extract Lichess evaluations from PGN.** When games are fetched with `evals=true`, the PGN text contains embedded `[%eval ...]` comments. These are parsed and used as the primary evaluation source — they're free, fast (no local computation), and produced by Lichess's cloud analysis at strong depth.

2. **Fall back to local Stockfish.** If the PGN does not contain `%eval` comments for some or all plies (e.g., the game was never analyzed on Lichess), those plies are analyzed locally using the bundled Stockfish engine at depth 20.

3. **Force Stockfish mode.** The user can set `force_stockfish: true` to skip Lichess evals entirely and re-analyze every position locally. This gives consistent depth and avoids dependency on Lichess's analysis being present.

### Evaluation convention

All evaluations are stored **from White's perspective** (positive = White is better):

- **Lichess `%eval`** values are already from White's perspective in the PGN.
- **Stockfish** scores are from the side-to-move's perspective. After an even ply (White just moved → Black to move), the engine score is negated. After an odd ply (Black just moved → White to move), the score is kept as-is.

### Mate scores

Mate scores follow the same sign convention:
- `eval_mate = 3` → White can force mate in 3.
- `eval_mate = -2` → Black can force mate in 2.

When `eval_mate` is present, `eval_cp` is `NULL`.

---

## Data flow

```
StoredGame (PGN + user_color)
  │
  ▼
chess_core::parse_pgn(pgn, user_color)
  │  Produces Vec<ParsedMove> with:
  │  - ply, fen_before, fen_after, move_uci
  │  - is_user_move flag
  │  - lichess_eval (from %eval comments, if present)
  │
  ├─ Has Lichess eval? ──► MoveEvaluation { source: "lichess" }
  │
  └─ Missing eval? ──► Engine::analyze(fen_after, depth=20)
                         ──► convert to White's perspective
                         ──► MoveEvaluation { source: "stockfish" }
  │
  ▼
Storage::insert_evaluations(game_id, evals)
  │  move_evaluations table: (game_id, ply, eval_cp, eval_mate, source)
  │
  ▼
Storage::update_analysis_status(game_id, source)
  │  Sets analysis_source + analysis_completed_at on the games table
```

## Tauri commands

| Command | Description |
|---------|-------------|
| `analyze_game(game_id, force_stockfish)` | Analyzes a single game. Returns `AnalyzeGameResult`. |
| `analyze_pending_games(force_stockfish)` | Analyzes all unanalyzed games. Emits `analysis-progress` events. Returns `AnalyzeBatchResult`. |

### Progress events

During `analyze_pending_games`, the backend emits `analysis-progress` events:

```json
{
  "game_id": "abcd1234",
  "games_done": 5,
  "games_total": 20,
  "status": "analyzing"
}
```

The final event has `status: "complete"` and `games_done == games_total`.

---

## PGN parsing details

The `chess-core` crate's PGN parser:
- Strips PGN headers (lines matching `[Tag "value"]`)
- Tokenizes movetext into moves, comments, and game results
- Replays the game using `shakmaty` for position validation and FEN generation
- Extracts `[%eval X.XX]` and `[%eval #N]` from PGN comments
- Handles both centipawn (float × 100) and mate score formats
- Tags each move with `is_user_move` based on the supplied `user_color`

### Eval comment format

| PGN comment | Parsed as |
|-------------|-----------|
| `[%eval 0.35]` | `eval_cp: 35` |
| `[%eval -1.5]` | `eval_cp: -150` |
| `[%eval #3]` | `eval_mate: 3` |
| `[%eval #-2]` | `eval_mate: -2` |
| `[%eval 0.0]` | `eval_cp: 0` |

---

## Database schema

```sql
CREATE TABLE move_evaluations (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    ply INTEGER NOT NULL,
    eval_cp INTEGER,
    eval_mate INTEGER,
    source TEXT NOT NULL,  -- 'lichess' | 'stockfish'
    PRIMARY KEY (game_id, ply)
);
```

Ply indexing: ply 0 = evaluation after White's first move, ply 1 = after Black's first move, etc. This matches the Lichess analysis array convention.

---

## Blunder Detection (Slice 4)

After evaluations are stored, the blunder detection pipeline classifies each of the user's moves by the centipawn drop from their perspective.

### Classification thresholds (1600-level defaults)

| Classification | Eval drop (cp) | v0.1 action |
|---------------|----------------|-------------|
| Inaccuracy | 50–99 | Stored, not turned into puzzles |
| Mistake | 100–199 | Stored, not turned into puzzles in v0.1 |
| Blunder | 200+ | Stored, becomes puzzle candidate in Slice 5 |

Thresholds are configurable via `MistakeThresholds` and will be user-settable in the UI (Slice 7).

### Drop calculation

For each of the user's moves at ply P:

1. `eval_before` = stored evaluation at ply P-1 (position after opponent's previous move), converted to the user's perspective.
2. `eval_after` = stored evaluation at ply P (position after the user's move), converted to the user's perspective.
3. `drop = eval_before - eval_after` (positive means the user's position got worse).
4. Classify based on the drop and the active thresholds.

The first move of the game (ply 0 for White, ply 1 for Black) is skipped if no prior evaluation exists.

### Mate score handling

- Mate scores are treated as ±10,000 centipawns for comparison purposes.
- **Mate-to-mate same side:** if both `eval_before` and `eval_after` are mate scores favoring the same side (e.g., user has mate-in-3 → mate-in-5), the transition is **not** flagged. Going from "winning mate" to "longer winning mate" is not a meaningful mistake at the puzzle level.
- **Mate-to-opposite or mate-to-cp:** transitioning from "user mates" to "opponent mates" or to a losing cp score **is** flagged (typically a blunder due to the enormous cp-equivalent drop).

### Already-losing-position cap

When the user's position is already significantly losing (`eval_before < -500cp` from user's perspective), a higher drop is required to classify as a blunder:

- Effective blunder threshold = `blunder_cp + losing_extra_cp` (default: 200 + 100 = 300cp).
- Inaccuracy and mistake thresholds are not affected.
- Rationale: flagging every move in an already-lost position creates noise rather than instructive puzzles.

### `best_move` field

The `best_move` column in the `mistakes` table is empty in Slice 4. It will be populated in Slice 5 when the engine re-analyzes the pre-blunder position with multi-PV to generate puzzles and determine the engine's recommended move.

### Database schema

```sql
CREATE TABLE mistakes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    ply INTEGER NOT NULL,
    fen_before TEXT NOT NULL,
    user_move TEXT NOT NULL,
    best_move TEXT NOT NULL,
    eval_before_cp INTEGER,
    eval_before_mate INTEGER,
    eval_after_cp INTEGER,
    eval_after_mate INTEGER,
    classification TEXT NOT NULL  -- 'inaccuracy' | 'mistake' | 'blunder'
);
```

### Tauri commands

| Command | Description |
|---------|-------------|
| `detect_mistakes(game_id)` | Detects mistakes in a single game. Returns `DetectMistakesResult`. |
| `detect_all_mistakes()` | Detects mistakes in all analyzed games. Returns `DetectAllMistakesResult`. |
