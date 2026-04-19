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
- `engine` crate: async Stockfish UCI wrapper with `Engine::new`, `Engine::analyze`, and `Engine::shutdown`
- UCI message parsing module (`engine::parse`) for `info` line extraction with multi-PV support
- `EngineError` enum via thiserror (spawn failure, protocol violation, timeout, unexpected EOF, I/O)
- `AnalyzeConfig` (depth, movetime, multipv) and `AnalysisResult` (best_move, score_cp, mate_in, depth, pv, multipv_results)
- Mate score handling: distinguished from centipawn scores in `MultiPvLine`
- 13 unit tests for UCI info line parsing and multi-PV extraction
- 7 `#[ignore]`-marked integration tests requiring Stockfish binary (`STOCKFISH_PATH` env var)
- `chess-core` crate: PGN parser using `shakmaty` for position tracking and move validation
- PGN `[%eval ...]` comment extraction (centipawn and mate score formats)
- `ParsedMove` type with `fen_before`, `fen_after`, `move_uci`, `is_user_move`, and optional `lichess_eval`
- `storage` migration `0002_evaluations.sql`: `move_evaluations` table for per-ply evaluations
- `Storage::insert_evaluations`, `get_evaluations`, `evaluation_count`, `update_analysis_status`, `list_unanalyzed_games`
- Tauri command `analyze_game(game_id, force_stockfish)`: Lichess-first eval extraction with Stockfish fallback
- Tauri command `analyze_pending_games(force_stockfish)`: batch analysis with `analysis-progress` event emission
- Typed frontend API wrapper (`src-ui/src/api/analysis.ts`) for analysis commands
- `docs/ANALYSIS.md` describing the Lichess-first analysis strategy and eval conventions
- 22 unit tests for PGN parsing and eval extraction, 5 integration tests for the analysis pipeline
- `puzzle-gen` crate: puzzle generation from detected blunders with multi-PV re-analysis
- Quality filters: unique-best-move gap (≥50cp), trivial recapture detection, depth floor (≥18), single-legal-move rejection, best-move-matches-user-move rejection
- `storage` migration `0004_puzzles.sql`: puzzles table with FK to mistakes
- `Storage::insert_puzzle`, `list_puzzles`, `puzzle_count`, `puzzle_exists_for_mistake`, `update_mistake_best_move`
- Tauri command `generate_puzzles`: loads blunders, runs puzzle-gen with engine, stores accepted puzzles, backfills `best_move` on mistakes
- Frontend API wrapper `src-ui/src/api/puzzles.ts` with typed `generatePuzzles()` function
- `docs/PUZZLES.md` describing the puzzle generation pipeline and quality filters
- 12 unit tests for puzzle quality filters, 5 storage tests for puzzle CRUD
- 6 new storage tests for evaluation CRUD and analysis status tracking
- `chess-core` mistake classification: `MistakeClassification` enum (`Inaccuracy`, `Mistake`, `Blunder`), `MistakeThresholds` struct with configurable 1600-level defaults
- `classify_mistake` pure function with mate-score handling, already-losing cap, and configurable thresholds
- `storage` migration `0003_mistakes.sql`: mistakes table
- `Storage::insert_mistakes`, `get_mistakes_for_game`, `list_blunders`, `mistake_count`, `list_analyzed_games`
- Tauri commands `detect_mistakes(game_id)` and `detect_all_mistakes()`
- Property-based tests with `proptest` for mistake classification (monotonicity, symmetry, zero-drop invariants)
- 8 storage tests for puzzle attempt CRUD (`puzzle_attempts` table, `0005_attempts.sql` migration)
- `Storage::record_attempt`, `get_attempts_for_puzzle`, `get_attempts_summary`, `get_next_puzzle`
- Tauri commands `submit_puzzle_attempt`, `get_next_puzzle`, `get_attempts_summary`
- Frontend API wrappers in `src-ui/src/api/puzzles.ts` for attempt commands
- **Slice 7 — Settings + Sync UI:**
- `storage` migration `0006_settings.sql`: `user_settings` single-row table with seeded defaults
- `UserSettings` struct (serde Serialize + Deserialize) with fields: `lichess_username`, `max_games`, `use_stockfish`, threshold fields
- `Storage::get_settings()` and `Storage::save_settings()` with 3 unit tests
- Tauri commands `get_settings`, `save_settings`, and `full_sync` (chained pipeline with `"sync-progress"` event emission)
- `FullSyncResult` and `SyncProgress` Tauri response types
- Routing: `react-router-dom` v6 with `HashRouter`; routes `/` (Sync) and `/settings` (Settings)
- `SettingsPage`: form for username, max games, Stockfish toggle, and threshold inputs
- `SyncPage`: "Fetch & Analyze" button, progress bar driven by `"sync-progress"` events, results table
- `useSyncProgress` hook encapsulating Tauri event listener for `"sync-progress"`
- `src/utils/syncStages.ts` pure utility functions (`stageName`, `isRunning`, `formatPercent`)
- Vitest setup (`vitest/config`, jsdom, `@testing-library/jest-dom`); 11 unit tests for `syncStages`
- Frontend API wrappers: `src/api/settings.ts` and `src/api/sync.ts`
- `App.tsx` replaced with `HashRouter`-based shell with nav bar linking Sync and Settings pages
- `App.css` replaced with app shell, nav bar, settings form, progress bar, and results table styles
- Fixed chessground license attribution: GPL-3.0 (not MIT) in README, copilot-instructions.md
- **Slice 8 — Puzzle Solving UI:**
- `PuzzleBoard` component wrapping chessground with React lifecycle management (init, update via `Api.set()`, destroy)
- `PuzzlePage` with puzzle state machine (loading → solving → correct/incorrect → next), timer-based attempt tracking, and keyboard shortcuts (Spacebar for next puzzle)
- `legalDests()` and `orientationFromFen()` pure utility functions bridging chess.js and chessground
- `matchesSolutionMove()` with promotion normalization (queen promotion equivalence)
- `formatSolutionDisplay()` converting UCI solution moves to SAN for human-readable feedback
- Incorrect-move feedback: shows correct move in SAN, then animates solution onto the board after 800ms
- chessground CSS imported (base, brown theme, cburnett pieces)
- Puzzle-specific styles: board container with aspect-ratio square, feedback messages, controls bar, keyboard hint
- Route `/puzzles` → `PuzzlePage`; nav bar updated: Sync | Puzzles | Settings
- 18 new Vitest tests: 6 for PuzzleBoard utilities (legalDests, orientationFromFen), 12 for PuzzlePage utilities (matchesSolutionMove, formatSolutionDisplay)
- New frontend dependencies: `chessground` 9.2.1 (GPL-3.0), `chess.js` 1.4.0 (BSD-2-Clause)
- `docs/PUZZLES.md` updated with puzzle solving interaction model
- `docs/ARCHITECTURE.md` updated with Slice 8 component structure and data flow
