# Architecture

> **Status:** Updated through Slice 2 — Stockfish engine wrapper.

Harland Chess Trainer is a Tauri 2 desktop application organized as a Cargo workspace with multiple crates.

## Workspace layout

```
harland-chess-trainer/
├── Cargo.toml              # workspace root
├── crates/
│   ├── chess-core/          # pure chess logic, no I/O
│   ├── lichess-client/      # async Lichess API wrapper
│   ├── engine/              # Stockfish UCI process management
│   ├── puzzle-gen/          # puzzle generation from analyzed games
│   └── storage/             # SQLite persistence layer
├── app/                     # Tauri application
│   ├── src/                 # Rust backend (Tauri commands)
│   ├── src-ui/              # React + TypeScript frontend
│   │   └── src/api/         # Typed invoke wrappers (frontend never calls invoke directly)
│   └── tauri.conf.json
└── resources/
    └── stockfish/           # bundled Stockfish binaries per platform
```

## Crate dependency graph

```
app (Tauri commands)
 ├── lichess-client   (reqwest, serde, tokio, futures, thiserror)
 └── storage          (sqlx + sqlite, tokio, thiserror, serde)

engine                (tokio, thiserror)   [standalone, not yet wired into app]
```

Other crates (`chess-core`, `puzzle-gen`) are scaffolded but have no dependencies or code yet.

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
