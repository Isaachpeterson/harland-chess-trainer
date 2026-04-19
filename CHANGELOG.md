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
- `classify_mistake()` function with mate score handling (±10,000cp sentinels, mate-to-mate same-side filtering), already-losing-position cap, and user perspective conversion
- Property-based tests via `proptest` for classification monotonicity and symmetry
- `storage` migration `0003_mistakes.sql`: `mistakes` table for detected blunders/mistakes/inaccuracies
- `Storage::insert_mistakes`, `get_mistakes_for_game`, `list_blunders`, `mistake_count`, `list_analyzed_games`
- Tauri command `detect_mistakes(game_id)`: per-game blunder detection from stored evaluations
- Tauri command `detect_all_mistakes()`: batch blunder detection across all analyzed games
- Typed frontend API wrappers for `detectMistakes` and `detectAllMistakes` in `analysis.ts`
- `docs/ANALYSIS.md` updated with blunder classification rules, mate score handling, and already-losing cap
- 24 unit tests for mistake classification edge cases, 4 proptest property-based tests, 8 storage tests for mistake CRUD
- `storage` migration `0005_attempts.sql`: `puzzle_attempts` table for tracking user puzzle solve attempts
- `Storage::record_attempt`, `get_attempts_for_puzzle`, `get_attempts_summary`, `get_next_puzzle` (unattempted-first random selection with fallback)
- `StoredAttempt` and `AttemptsSummary` types in storage crate
- Tauri command `get_next_puzzle()`: returns next unseen puzzle (random unattempted, fallback to random attempted)
- Tauri command `submit_puzzle_attempt(puzzle_id, success, time_taken_ms, move_played)`: records a puzzle attempt
- Tauri command `get_attempts_summary()`: returns aggregate statistics (total attempts, success rate, puzzles attempted today)
- Typed frontend API wrappers (`getNextPuzzle`, `submitPuzzleAttempt`, `getAttemptsSummary`) in `puzzles.ts`
- `docs/ANALYSIS.md` updated with puzzle attempt tracking data model and selection strategy
- 8 storage unit tests for attempt recording, retrieval, summary stats, and next-puzzle selection
