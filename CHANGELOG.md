# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial project scaffold: Tauri 2 app with React + TypeScript frontend
- Cargo workspace with library crates: chess-core, lichess-client, engine, puzzle-gen, storage
- Documentation structure (docs/)
- CI/CD workflow stubs
- `lichess-client` crate: `LichessClient` with ndjson streaming, rate-limit retry (exponential backoff on 429), `LichessGame` types, `LichessError` via thiserror
- `storage` crate: SQLite via sqlx, `0001_initial.sql` migration (games table), `Storage::new`, `insert_game` (upsert), `get_game`, `game_count`
- Tauri command `sync_games(username, max_games)` wiring lichess-client → storage, returns `SyncResult { fetched, new, updated }`
- Typed frontend API wrapper (`src-ui/src/api/lichess.ts`) — components never call `invoke` directly
- Temporary sync UI: username input, game count, Sync Games button with results display
- Integration tests for lichess-client (wiremock-based ndjson fixture tests, 429 retry, 404 handling)
- Unit tests for storage (insert, upsert, get nonexistent)
- `#[ignore]`-marked real Lichess API test
