# Architecture

> **Status:** Updated through Slice 1 — Lichess fetch + storage.

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
```

Other crates (`chess-core`, `engine`, `puzzle-gen`) are scaffolded but have no dependencies or code yet.

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
