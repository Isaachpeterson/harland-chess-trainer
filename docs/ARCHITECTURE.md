# Architecture

> **Status:** Updated through Slice 4 — Blunder detection.

Harland Chess Trainer is a Tauri 2 desktop application organized as a Cargo workspace with multiple crates.

## Workspace layout

```
harland-chess-trainer/
├── Cargo.toml              # workspace root
├── crates/
│   ├── chess-core/          # pure chess logic: PGN parsing, eval extraction
│   ├── lichess-client/      # async Lichess API wrapper
│   ├── engine/              # Stockfish UCI process management
│   ├── puzzle-gen/          # puzzle generation from analyzed games
│   └── storage/             # SQLite persistence layer
├── app/                     # Tauri application
│   ├── src/                 # Rust backend (Tauri commands)
│   ├── src-ui/              # React + TypeScript frontend
│   │   └── src/api/         # Typed invoke wrappers (frontend never calls invoke directly)
│   └── tauri.conf.json
├── docs/
│   └── ANALYSIS.md          # analysis pipeline design (Lichess-first strategy)
└── resources/
    └── stockfish/           # bundled Stockfish binaries per platform
```

## Crate dependency graph

```
app (Tauri commands, orchestration)
 ├── chess-core       (shakmaty, thiserror)
 ├── engine           (tokio, thiserror)
 ├── lichess-client   (reqwest, serde, tokio, futures, thiserror)
 └── storage          (sqlx + sqlite, tokio, thiserror, serde)
```

Other crates (`puzzle-gen`) are scaffolded but have no dependencies or code yet.

## Data flow (Slice 1: game sync)

```
Frontend (React)
  │  syncGames(username, maxGames)
  │  via typed wrapper in src-ui/src/api/lichess.ts
  ▼
Tauri invoke → sync_games command (app/src/lib.rs)
  │
  ├─► LichessClient::fetch_user_games()
  │     GET /api/games/user/{username}?evals=true&clocks=true&opening=true&pgnInJson=true
  │     ndjson stream → Vec<LichessGame>
  │
  └─► Storage::insert_game() (upsert per game)
        SQLite via sqlx — crates/storage/migrations/0001_initial.sql
        Returns UpsertOutcome { was_new }
  │
  ▼
SyncResult { fetched, new, updated } → frontend
```

### Key patterns established

- **Async Rust throughout.** All I/O uses `tokio`. Tauri commands are `async`.
- **Error types per crate.** `LichessError` (thiserror) in lichess-client, `StorageError` in storage. The app crate maps these to `String` for the Tauri boundary.
- **Typed frontend API.** `src-ui/src/api/lichess.ts` wraps `invoke` with TypeScript types. Components never call `invoke` directly.
- **ndjson streaming.** Lichess returns games as newline-delimited JSON; the client parses the stream incrementally.
- **Upsert semantics.** `Storage::insert_game` checks for existence and updates on conflict, preserving analysis fields.

## Engine component (Slice 2)

The `engine` crate wraps a Stockfish (or compatible UCI) child process in an async Rust API.

```
Engine::new(stockfish_path)
  │  spawn process, capture stdin/stdout
  │  send "uci" → wait for "uciok"
  │  send "isready" → wait for "readyok"
  ▼
Engine::analyze(fen, config)
  │  send "position fen {fen}"
  │  send "go depth N" (or "go movetime M")
  │  collect "info ..." lines until "bestmove ..."
  │  parse info lines → MultiPvLine structs
  ▼
AnalysisResult { best_move, score_cp, mate_in, depth_reached, pv, multipv_results }
```

### Key patterns

- **Async stdio.** Uses `tokio::process` with `BufReader` for non-blocking line reads.
- **Timeout safety.** Every read has a 30-second timeout to avoid hangs.
- **Graceful shutdown.** `Engine::shutdown()` sends `quit`; `Drop` impl calls `kill_on_drop` as fallback.
- **Multi-PV support.** `AnalyzeConfig::multipv` sets the UCI `MultiPV` option; reset to 1 after each analysis.
- **UCI parsing module.** `parse.rs` handles info line extraction as pure functions, fully unit-tested.

## Analysis pipeline (Slice 3)

Slice 3 ties `chess-core`, `engine`, and `storage` together in the app crate.

### chess-core crate

Now active. Depends on `shakmaty` for position representation and `thiserror` for errors.

- **PGN parser** (`pgn` module): tokenizes PGN movetext, replays moves with `shakmaty::Chess`, extracts `[%eval ...]` comments from Lichess-annotated PGN.
- Produces `Vec<ParsedMove>` with `fen_before`, `fen_after`, `move_uci`, `is_user_move`, and optional `lichess_eval`.

### Data flow: Lichess-first analysis

See [docs/ANALYSIS.md](ANALYSIS.md) for the full strategy.

```
analyze_game(game_id, force_stockfish)
  │
  ├─► Storage::get_game(game_id)
  │     Load PGN + user_color
  │
  ├─► chess_core::parse_pgn(pgn, user_color)
  │     → Vec<ParsedMove> with optional Lichess evals
  │
  ├─ Has %eval? ──► MoveEvaluation { source: "lichess" }
  │
  ├─ Missing? ──► Engine::analyze(fen_after, depth=20)
  │               → convert score to White's perspective
  │               → MoveEvaluation { source: "stockfish" }
  │
  ├─► Storage::insert_evaluations(game_id, evals)
  │     move_evaluations table (0002_evaluations.sql)
  │
  └─► Storage::update_analysis_status(game_id, source)

analyze_pending_games(force_stockfish)
  │  Iterates over Storage::list_unanalyzed_games()
  │  Calls analyze_game logic per game
  │  Emits "analysis-progress" Tauri events
  └─► AnalyzeBatchResult { games_analyzed, games_skipped, total_evals, errors }
```

### Key patterns (Slice 3)

- **Lichess-first strategy.** Extract `%eval` from PGN first; only invoke Stockfish for missing plies.
- **Lazy engine initialization.** The `Engine` is created on first use and cached in `AppState`.
- **White-perspective normalization.** All stored evals are from White's POV. Stockfish scores are negated when the side to move was Black.
- **Tauri event emission.** `analyze_pending_games` emits `analysis-progress` events for frontend progress tracking.
- **Typed frontend API split.** Analysis wrappers live in `src-ui/src/api/analysis.ts`, separate from sync wrappers in `lichess.ts`.

### Blunder detection (Slice 4)

Slice 4 adds mistake classification as a pure-logic layer on top of stored evaluations.

```
detect_mistakes(game_id) / detect_all_mistakes()
  │
  ├─► Storage::get_game(game_id)
  │     Load PGN + user_color
  │
  ├─► Storage::get_evaluations(game_id)
  │     Per-ply eval_cp / eval_mate values
  │
  ├─► chess_core::parse_pgn(pgn, user_color)
  │     → Vec<ParsedMove> (fen_before, move_uci, is_user_move)
  │
  ├─► For each user move at ply P:
  │     eval_before = evaluations[ply P-1]  (position before user moved)
  │     eval_after  = evaluations[ply P]    (position after user moved)
  │     chess_core::classify_mistake(before, after, user_is_white, thresholds)
  │       → None | Some(Inaccuracy | Mistake | Blunder)
  │
  └─► Storage::insert_mistakes(game_id, mistakes)
        mistakes table (0003_mistakes.sql)
```

### chess-core crate (updated in Slice 4)

Now includes the `mistakes` module alongside `pgn`:

- **`MistakeClassification`** enum: `Inaccuracy`, `Mistake`, `Blunder` (ordered by severity).
- **`MistakeThresholds`** struct with configurable cp drop thresholds, already-losing cap, and 1600-level defaults.
- **`classify_mistake()`** — pure function handling mate scores (±10,000 cp sentinel), mate-to-mate same-side filtering, already-losing position cap, and standard threshold comparison.
- Property-based tests via `proptest` verify monotonicity and symmetry of classification.

### Key patterns (Slice 4)

- **Pure logic, no I/O.** Blunder detection reads from storage and writes to storage; no engine or network calls.
- **Eval-before / eval-after pairing.** `eval_before` = evaluation at ply P-1 (after opponent's move), `eval_after` = evaluation at ply P (after user's move). User perspective conversion handles Black/White differences.
- **Idempotent re-detection.** `insert_mistakes` deletes existing mistakes for the game before inserting, so re-running is safe.
- **`best_move` deferred.** The engine's recommended move is populated in Slice 5 during puzzle generation; stored as empty string in Slice 4.
