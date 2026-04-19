# Implementation Plan — Vertical Slices

> This document breaks v0.1 (and beyond) into small, independent, shippable slices. Each slice is scoped narrowly enough that it can be implemented, reviewed, and committed before the next one starts. The goal is to avoid the "implement everything at once" failure mode and to keep AI-assisted development focused.
>
> **This is a living document.** Update it as the project evolves. When a slice completes, mark it done and record notes. When reality diverges from the plan, rewrite the upcoming slices — don't force the plan onto the code.

**Last updated:** project inception
**Current slice:** Slice 6 — Puzzle attempt tracking
**Target release:** v0.1

---

## How to Use This Document

### For human readers

Scan the "Slice Roadmap" section to see the overall arc. Read the currently-active slice in detail. Don't read ahead past the next 1–2 slices — they'll change.

### For AI coding assistants (GitHub Copilot, Claude, etc.)

When asked to implement a slice, read **only** the spec for that slice plus the context in `copilot-instructions.md`. Do not implement functionality from later slices, even if it seems convenient. Scope discipline is the point of this document.

When a slice is complete:
1. Update the "Status" field of that slice to `Done` with the completion date.
2. Add a brief "Notes" subsection under that slice with anything learned, deviations from the plan, or decisions made during implementation.
3. Update the "Current slice" field at the top.
4. If the next slice needs to change based on what you learned, propose edits to that slice's spec rather than silently deviating.

---

## Slice Rules

These rules apply to every slice, without exception.

1. **One slice at a time.** Do not start the next slice until the current one is committed to git.
2. **Slice boundaries are hard.** If implementing the current slice reveals that a later slice's design is wrong, stop and update the plan. Do not silently pull work forward.
3. **Every slice ends with a commit.** The commit message follows Conventional Commits and references the slice by number (e.g., `feat(slice-2): add Stockfish UCI wrapper`).
4. **Every slice ends with a manual verification step.** Automated tests are required but not sufficient. Each slice has a "Manual verification" section that a human runs before marking the slice done.
5. **No new top-level dependencies without justification.** If a slice needs a new crate or npm package, the slice spec must name it. Adding unplanned dependencies is out of scope.
6. **Documentation updates are part of the slice.** A slice is not done until `ARCHITECTURE.md` and relevant docs reflect the new code.
7. **Tests are part of the slice, not a follow-up.** Write unit tests alongside implementation. Integration tests for external services (Lichess, Stockfish) are marked `#[ignore]` and run manually.

---

## Slice Template

Every slice follows this structure:

```
## Slice N — Short Descriptive Title

**Status:** Not started | In progress | Done (YYYY-MM-DD)
**Depends on:** [prior slices]
**Estimated effort:** S / M / L

### Goal
One-paragraph description of what this slice accomplishes and why.

### In scope
Bulleted list of what will be built.

### Out of scope
Bulleted list of things that might seem related but belong in later slices.

### Deliverables
- Code changes (by crate / directory)
- Tests
- Documentation updates

### Manual verification
Concrete steps a human runs to confirm the slice works.

### Notes
(Filled in after completion. Decisions made, surprises encountered, deviations from plan.)
```

---

## Slice Roadmap (v0.1)

| # | Title | Status | Depends on |
|---|-------|--------|-----------|
| 0 | Project scaffold | Done | — |
| 1 | Lichess fetch + storage | Done (2026-04-17) | 0 |
| 2 | Stockfish engine wrapper | Done (2026-04-17) | 0 |
| 3 | Game analysis pipeline | Done (2026-04-17) | 1, 2 |
| 4 | Blunder detection | Done (2026-04-17) | 3 |
| 5 | Puzzle generation | Done (2026-04-18) | 4 |
| 6 | Puzzle attempt tracking | Not started | 5 |
| 7 | Settings + Sync UI | Not started | 1 |
| 8 | Puzzle solving UI | Not started | 5, 6 |
| 9 | Basic stats UI | Not started | 6 |
| 10 | v0.1 release prep | Not started | all prior |

Post-v0.1 slices are sketched at the end of this document but not yet broken into detail.

---

## Slice 0 — Project Scaffold

**Status:** Done
**Depends on:** —
**Estimated effort:** S

### Goal
Stand up the Tauri + React + TypeScript project with the Cargo workspace structure, documentation skeleton, license, and CI workflows. No application logic.

### In scope
- Tauri 2 app with React + TS + Vite frontend
- Cargo workspace with empty library crates: `chess-core`, `lichess-client`, `engine`, `puzzle-gen`, `storage`
- `docs/` folder with README, CONVENTIONS, ARCHITECTURE stub, DEVELOPMENT stub
- GPL-3.0 LICENSE file
- CONTRIBUTORS.md, CHANGELOG.md, README.md
- `.gitignore`
- GitHub Actions CI + release workflow stubs
- `cargo tauri dev` launches a working window

### Notes
(Recorded after completion.)

---

## Slice 1 — Lichess Fetch + Storage

**Status:** Done (2026-04-17)
**Depends on:** 0
**Estimated effort:** M

### Goal
Prove we can fetch a user's games from Lichess and persist them locally. No analysis, no UI beyond a minimal way to invoke the sync command. This slice establishes the async-Rust, serde, and sqlx patterns the rest of the project will reuse.

### In scope

**`lichess-client` crate:**
- `LichessClient` struct with configurable base URL (default `https://lichess.org`) and a `reqwest` client
- User-Agent set to `HarlandChessTrainer/{version} (+https://github.com/Isaachpeterson/harland-chess-trainer)`
- `fetch_user_games(username, max_games)` hitting `/api/games/user/{username}` with query params `evals=true`, `clocks=true`, `opening=true`
- ndjson stream parsing
- `LichessGame` struct deserialized via serde (id, pgn, players, winner, speed, rated, created_at, analysis presence)
- Rate limit handling: exponential backoff on 429, max 3 retries
- `LichessError` enum via `thiserror` covering network, parse, rate-limit-exhausted, user-not-found

**`storage` crate:**
- sqlx dependency with sqlite feature
- `migrations/0001_initial.sql` creating the `games` table (see PROJECT_CONTEXT.md Section 7)
- `Storage::new(db_path)` opens connection and runs migrations
- `Storage::insert_game(game)` as an upsert (newer analysis fields win)
- `Storage::get_game(id)` returning `Option<StoredGame>`
- Compile-time checked queries via `sqlx::query_as!`

**`app` crate:**
- Tauri command `sync_games(username: String, max_games: u32) -> Result<SyncResult, String>` wiring the two crates together
- `SyncResult { fetched: u32, new: u32, updated: u32 }`
- Typed wrapper in `src-ui/src/api/lichess.ts` — frontend never calls `invoke` directly

### Out of scope
- Running Stockfish analysis (Slice 2/3)
- Detecting mistakes (Slice 4)
- Any UI beyond whatever is needed to trigger the command for verification (a temporary button is fine; polish comes in Slice 7)
- OAuth (deferred to v0.3)

### Deliverables
- Code in the three crates above
- Unit tests in `lichess-client` using a captured ndjson fixture in `crates/lichess-client/tests/fixtures/`
- Integration tests in `storage` against in-memory SQLite
- `#[ignore]`-marked test that hits real Lichess API for one known username
- `docs/ARCHITECTURE.md` updated with the data-flow diagram: frontend invoke → Tauri command → lichess-client → storage

### Manual verification
1. Run `cargo test --workspace` — all non-ignored tests pass.
2. Run `cargo tauri dev`.
3. From the frontend (via a temporary button or devtools console), invoke `sync_games` with `"Isaachpeterson"` and `50`.
4. Open the SQLite file (DB Browser for SQLite, or `sqlite3` CLI) and confirm 50 rows in the `games` table with sensible-looking data.
5. Re-run the same sync — confirm `new: 0, updated: N` (games are upserts, not duplicates).

### Notes
Completed 2026-04-17.

- Used `sqlx` with runtime queries (not compile-time checked `query_as!`) for Slice 1 to avoid needing a `DATABASE_URL` env var during builds. Compile-time checked queries can be revisited later if desired.
- `SqliteConnectOptions::from_str` returns a url parse error, not `sqlx::Error`, so we wrap it manually into `StorageError::Database`.
- Lichess ndjson parsing uses incremental buffering via `bytes_stream()` + manual newline splitting, rather than a line-based codec, to keep dependencies minimal.
- `wiremock 0.6` used for lichess-client integration tests (mock HTTP server). Added as a dev-dependency only.
- The Tauri `setup` hook initializes `Storage` using `block_on` on the current tokio handle. This works because Tauri 2 runs setup within an async context.
- Frontend uses a simple `useState`-driven form for the temporary sync UI. No external form or state libraries needed.
- `analysis` field from Lichess ndjson is captured in the `LichessGame` type for future use (Slice 3) but not stored in the games table yet — analysis storage belongs to Slice 3.

---

## Slice 2 — Stockfish Engine Wrapper

**Status:** Done (2026-04-17)
**Depends on:** 0
**Estimated effort:** M

### Goal
Wrap a bundled Stockfish binary in an ergonomic async Rust API. Prove we can spawn the engine, speak UCI, and get reliable evaluations for arbitrary positions. This slice is independent of Slice 1 — it can be built in parallel conceptually, but sequentially in practice.

### In scope

**`engine` crate:**
- `Engine` struct owning a child process handle and async stdio channels
- `Engine::new(stockfish_path)` spawns the process and performs the UCI handshake (`uci` → wait for `uciok`, `isready` → wait for `readyok`)
- `Engine::analyze(fen, config)` where `AnalyzeConfig { depth: Option<u32>, movetime_ms: Option<u32>, multipv: u32 }`
- Returns `AnalysisResult { best_move: String (UCI), score_cp: i32, mate_in: Option<i32>, depth_reached: u32, pv: Vec<String>, multipv_results: Vec<MultiPvLine> }`
- Proper handling of mate scores (distinguished from cp scores)
- Graceful shutdown: `Engine::shutdown()` sends `quit` and waits for process exit; `Drop` impl kills process if shutdown wasn't called
- `EngineError` via `thiserror`: process spawn failure, UCI protocol violation, timeout, unexpected EOF

### Out of scope
- Bundling the Stockfish binary in the app release (deferred to Slice 10)
- Using the engine on stored games (Slice 3)
- Opening book or tablebase integration (v0.4)
- Any UI

### Deliverables
- `engine` crate implementation
- Unit tests for UCI message parsing (pure functions, no process needed)
- `#[ignore]`-marked integration tests that actually spawn Stockfish from `$STOCKFISH_PATH` env var and verify known positions produce expected best moves (e.g., back-rank mate-in-1 positions)
- `docs/ARCHITECTURE.md` updated with the engine component

### Manual verification
1. With Stockfish on PATH, run the integration tests: `STOCKFISH_PATH=stockfish cargo test -p engine -- --ignored`.
2. All integration tests pass.
3. Analyze a known position (e.g., `rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2`) and confirm `depth_reached >= 18` and a sane `score_cp` value.

### Notes
Completed 2026-04-17.

- Dependencies added to engine crate: `thiserror 2`, `tokio 1` (process, io-util, time, sync features). Dev-dependency: `tokio` with macros + rt-multi-thread for `#[tokio::test]`.
- UCI parsing lives in a separate `parse` module with pure functions, making it fully unit-testable without spawning Stockfish.
- `Engine` uses `kill_on_drop(true)` on the child process as a safety net; `Drop` impl also calls `start_kill` if `shutdown()` wasn't called.
- Multi-PV is set/reset per analysis call via `setoption name MultiPV value N`. Reset to 1 after each analysis to avoid state leakage between calls.
- Info line parsing extracts the deepest result per PV index, discarding intermediate depths.
- Integration tests require `STOCKFISH_PATH` env var (defaults to `stockfish` on PATH).

---

## Slice 3 — Game Analysis Pipeline

**Status:** Done (2026-04-17)
**Depends on:** 1, 2
**Estimated effort:** M

### Goal
For each stored game, produce per-move evaluations and persist them. Prefer Lichess's embedded evaluations; fall back to local Stockfish. This is the first slice that ties multiple crates together and reveals orchestration patterns.

### In scope

**`chess-core` crate:**
- PGN parsing wrapper around `shakmaty` — yields a sequence of `(position_fen, ply, user_move_uci)` tuples
- Identifies which moves were made by the user (based on the game's `user_color`)
- Extracts embedded evaluations from PGN `%eval` comments when present

**`storage` crate:**
- Migration `0002_evaluations.sql` adding a `move_evaluations` table
- Schema: `(game_id, ply, eval_cp, eval_mate, source)` where source is `'lichess'` or `'stockfish'`
- `Storage::insert_evaluations(game_id, evals)` batch insert
- `Storage::get_evaluations(game_id)` retrieval

**`app` crate:**
- Tauri command `analyze_game(game_id: String, force_stockfish: bool)` orchestrating:
  1. Load game from storage
  2. If `force_stockfish` is false, try to extract Lichess evals from PGN first
  3. For any missing plies (or always, if `force_stockfish` is true), invoke `engine` to compute evaluations
  4. Persist evaluations
  5. Mark the game's `analysis_source` and `analysis_completed_at` fields
- Tauri command `analyze_pending_games(force_stockfish: bool)` that runs `analyze_game` for every stored game lacking analysis. Emits progress events to the frontend via Tauri's event system.

### Out of scope
- Detecting which moves were mistakes (Slice 4)
- Generating puzzles (Slice 5)
- UI progress display (will consume the events in Slice 7)

### Deliverables
- Code in the three crates
- Unit tests for PGN parsing and eval extraction
- Integration test: run the pipeline against a real game from the fixture set, confirm evaluations are stored for every ply
- `docs/ARCHITECTURE.md` updated; `docs/ANALYSIS.md` created describing the Lichess-first strategy

### Manual verification
1. With Slice 1's games stored, invoke `analyze_pending_games(false)` from the frontend.
2. Confirm progress events fire to the frontend console.
3. Query SQLite: `SELECT COUNT(*) FROM move_evaluations` — should be roughly (games × avg plies).
4. Pick one game, confirm it has an evaluation for every ply and that the values look sane (no NULLs, no absurd values).

### Notes
Completed 2026-04-17.

- `chess-core` now depends on `shakmaty 0.27` for position tracking and `thiserror 2` for errors.
- PGN parsing is implemented manually (tokenizer + shakmaty move validation) rather than using `pgn-reader`, keeping dependencies minimal. Handles Lichess PGN format including `[%eval ...]` and `[%clk ...]` comments.
- `ParsedMove` includes both `fen_before` and `fen_after` — `fen_after` is needed for Stockfish fallback analysis (engine evaluates the position resulting from each move).
- Stockfish scores are normalized to White's perspective by negating when the side to move is Black (i.e., after even plies).
- The `Engine` is lazily initialized in `AppState` on first use and reused across analysis calls. Resolved via `STOCKFISH_PATH` env var or `"stockfish"` on PATH.
- `Storage::insert_evaluations` deletes existing evals for the game before inserting, making re-analysis idempotent.
- `analyze_pending_games` emits `analysis-progress` Tauri events per game. Errors on individual games are collected but don't abort the batch.
- Frontend typed API wrappers split into `lichess.ts` (sync) and `analysis.ts` (analysis commands).
- Added `docs/ANALYSIS.md` describing the Lichess-first strategy, eval conventions, and data flow.

---

## Slice 4 — Blunder Detection

**Status:** Done (2026-04-17)
**Depends on:** 3
**Estimated effort:** S

### Goal
Identify the user's blunders from stored evaluations and persist them as structured records. Pure logic slice, no external I/O.

### In scope

**`chess-core` crate:**
- `MistakeClassification` enum: `Inaccuracy`, `Mistake`, `Blunder`
- `classify_mistake(eval_before_cp, eval_after_cp, thresholds: &MistakeThresholds) -> Option<MistakeClassification>`
- Mate score handling per PROJECT_CONTEXT.md Section 6 (mate-to-mate transitions do not count; mate-to-losing is always a blunder)
- Already-losing-position cap: if `eval_before < -500cp`, require a larger drop to classify as blunder
- Configurable thresholds struct with v0.1 defaults (100 / 200 / 300 cp for 1600-level play)

**`storage` crate:**
- Migration `0003_mistakes.sql` adding the `mistakes` table (see PROJECT_CONTEXT.md Section 7)
- `Storage::insert_mistakes(game_id, mistakes)` batch insert
- `Storage::get_mistakes_for_game(game_id)` and `Storage::list_blunders(limit, offset)`

**`app` crate:**
- Tauri command `detect_mistakes(game_id: String)` that reads evaluations, runs classification, persists mistakes
- Tauri command `detect_all_mistakes()` for batch processing
- Thresholds are loaded from user settings (add a minimal settings table or JSON file)

### Out of scope
- Generating puzzles from blunders (Slice 5)
- Inaccuracies and mistakes as puzzle sources (v0.2)
- Advanced filters like time-trouble detection (deferred)
- UI for adjusting thresholds (Slice 7)

### Deliverables
- `chess-core` with `classify_mistake` and thorough unit tests covering edge cases
- Property-based tests with `proptest`: classification is monotonic in eval drop
- `storage` migrations and methods
- `app` commands
- `docs/ANALYSIS.md` updated with classification rules

### Manual verification
1. Run classification on all stored games.
2. Spot-check 3 flagged blunders manually: load the game in Lichess's analysis board and confirm the flagged move was indeed a blunder.
3. Confirm that at least one known-good game (no blunders) produces zero mistakes.

### Notes
Completed 2026-04-17.

- `classify_mistake` takes full eval fields (eval_cp, eval_mate for both before and after) plus a `user_is_white` flag, rather than the simpler `(eval_before_cp, eval_after_cp)` in the original spec. This was necessary to handle mate score edge cases properly.
- Mate scores are mapped to ±10,000 cp sentinels for comparison. Mate-to-mate same-sign transitions (both winning or both losing) are filtered out before drop calculation.
- Already-losing-position cap: when `eval_before < -500cp` (user's perspective), the blunder threshold increases by 100cp (configurable via `losing_extra_cp`). This only affects the blunder threshold, not inaccuracy/mistake.
- `proptest` added as a dev-dependency for chess-core. Property-based tests verify: monotonicity of classification in eval drop, improving moves never classified, symmetric behavior for White/Black, zero drop never classified.
- `best_move` column in the mistakes table is stored as empty string — populated in Slice 5 when the engine re-analyzes pre-blunder positions with multi-PV.
- Thresholds currently use `MistakeThresholds::default()` in the Tauri commands. User-configurable settings table deferred to Slice 7 (settings UI).
- Detection skips ply 0 for White (no prior eval) and ply 1 for Black (same reason). The very first move of the game cannot be evaluated without a starting-position eval.
- `detect_all_mistakes` iterates over `list_analyzed_games()` (new storage method) rather than all games, ensuring only games with evaluations are processed.
- `insert_mistakes` deletes existing mistakes for the game before inserting, making re-detection idempotent.

---

## Slice 5 — Puzzle Generation

**Status:** Not started
**Depends on:** 4
**Estimated effort:** M

### Goal
Turn detected blunders into high-quality training puzzles by applying quality filters. A good puzzle has a clearly best move; this slice is where the art of puzzle quality lives.

### In scope

**`puzzle-gen` crate:**
- `PuzzleCandidate` struct: `(fen, solution_uci_moves, mistake_id, source_game_id, source_ply, themes: Vec<String>)`
- Quality filters:
  - **Unique-best-move gap:** requires multi-PV analysis showing the best move is at least 50cp better than the second-best. This means Slice 5 may need to re-run the engine with multipv=2 on the pre-blunder position. Decide during implementation: re-analyze at puzzle-gen time, or change Slice 3 to always compute multipv=2 on candidate positions. Prefer re-analysis here to keep Slice 3 simple.
  - **Not a trivial recapture:** if the best move is recapturing a piece that was just captured, filter it out.
  - **Depth floor:** re-analysis must reach depth ≥ 18.
- `generate_puzzles_for_mistakes(mistakes, engine) -> Vec<PuzzleCandidate>`

**`storage` crate:**
- Migration `0004_puzzles.sql` adding the `puzzles` table
- `Storage::insert_puzzle(puzzle)` and `Storage::list_puzzles(filter)`

**`app` crate:**
- Tauri command `generate_puzzles()` running the full pipeline over all stored blunders

### Out of scope
- Tactical theme detection (v0.3)
- Spaced repetition scheduling (v0.2)
- UI for solving puzzles (Slice 8)

### Deliverables
- `puzzle-gen` crate with filter implementations and unit tests per filter
- Integration test: generate puzzles from a known game, confirm quality filters reject at least one known-ambiguous position
- `storage` migrations and methods
- `docs/PUZZLES.md` describing the generation pipeline and quality filters

### Manual verification
1. Run `generate_puzzles()` against all detected blunders.
2. Review 5 generated puzzles manually — each should have one clearly best move that a human would recognize as the solution.
3. Confirm at least one blunder was rejected by the unique-best-move filter (log this for visibility).

### Notes
Completed 2026-04-18.

- `puzzle-gen` crate depends on `engine` (for multi-PV analysis) and `shakmaty 0.27` (for recapture detection via position/move validation).
- Quality filters implemented in a separate `filters` module with pure functions, fully unit-testable without Stockfish.
- Re-analysis uses `multipv: 2` at `depth: max(min_depth, 20)` per position. This keeps Slice 3's analysis simple (single PV) while getting the multi-PV data needed for puzzle quality.
- Recapture detection parses the FEN with shakmaty, converts the best move UCI to a `Move`, and checks `Move::is_capture()` plus destination-square matching with the opponent's previous move.
- Mate scores mapped to ±10,000cp sentinels for eval gap calculation, consistent with chess-core's approach.
- `generate_puzzles` Tauri command is idempotent: skips blunders that already have puzzles via `puzzle_exists_for_mistake`.
- Previous move UCI (for recapture filter) is obtained by re-parsing the PGN and finding the move at `ply - 1`.
- `best_move` column on mistakes is backfilled during puzzle generation, as planned in Slice 4 notes.
- Storage migration `0004_puzzles.sql` uses `CREATE TABLE IF NOT EXISTS` for safety.
- **Bug found during manual verification:** Some blunders flagged by Slice 4 had the engine's best move matching the user's actual move (e.g., checkmate moves where eval drop was a mate-score artifact). Added `BestMoveMatchesUserMove` filter to reject these false positives. Root cause is likely a mate-score edge case in blunder detection — worth revisiting in a future slice.

---

## Slice 6 — Puzzle Attempt Tracking

**Status:** Not started
**Depends on:** 5
**Estimated effort:** S

### Goal
Record user attempts at solving puzzles, including correct/incorrect and time taken. Enables later features like spaced repetition and weakness analytics. Pure persistence slice.

### In scope

**`storage` crate:**
- Migration `0005_attempts.sql` adding the `puzzle_attempts` table
- `Storage::record_attempt(puzzle_id, success, time_taken_ms, move_played)`
- `Storage::get_attempts_for_puzzle(puzzle_id)`
- `Storage::get_attempts_summary()` returning aggregate stats (total attempts, success rate, puzzles attempted today)

**`app` crate:**
- Tauri commands `submit_puzzle_attempt(...)` and `get_next_puzzle()`
- For v0.1, `get_next_puzzle` returns a random unattempted puzzle, falling back to random already-attempted if all are seen

### Out of scope
- Spaced repetition scheduling (v0.2)
- Difficulty-weighted selection (v0.2)
- UI (Slice 8)

### Deliverables
- Migrations and storage methods with tests
- Tauri commands
- `docs/ANALYSIS.md` updated with the attempts data model

### Manual verification
1. Generate at least 10 puzzles (via Slice 5).
2. Call `get_next_puzzle` repeatedly — confirm it returns different puzzles.
3. Submit attempts (some success, some failure) and confirm they persist.
4. Call `get_attempts_summary` and confirm the numbers match what you submitted.

---

## Slice 7 — Settings + Sync UI

**Status:** Not started
**Depends on:** 1
**Estimated effort:** M

### Goal
First real UI slice. Lets the user enter their Lichess username, configure basic settings, and trigger the sync + analyze pipeline with visible progress.

### In scope

**Frontend:**
- Routing (React Router or equivalent — pick one, document choice)
- Settings page: Lichess username input, max games to fetch, "always use local Stockfish" toggle, mistake thresholds
- Sync page: "Fetch & Analyze" button that chains `sync_games`, `analyze_pending_games`, `detect_all_mistakes`, `generate_puzzles` with a progress bar
- Progress is driven by Tauri events emitted from the backend commands
- Settings persisted via the storage crate (add a `user_settings` table with a single row)

**Backend:**
- A combined command `full_sync()` that orchestrates the full pipeline and emits structured progress events
- Settings CRUD commands

### Out of scope
- Puzzle solving UI (Slice 8)
- Stats UI (Slice 9)
- Detailed per-game analysis view (post v0.1)

### Deliverables
- React pages under `src-ui/src/pages/`
- Typed API wrappers in `src-ui/src/api/`
- Tauri event listeners encapsulated in a hook (`useSyncProgress`)
- Vitest tests for pure UI logic
- `docs/DEVELOPMENT.md` updated with how to run the frontend

### Manual verification
1. Launch the app with no data.
2. Enter a real Lichess username, set max games to 20, click "Fetch & Analyze."
3. Watch the progress bar advance through fetch → analyze → detect → generate.
4. Confirm on completion that puzzles are available in the database.

---

## Slice 8 — Puzzle Solving UI

**Status:** Not started
**Depends on:** 5, 6
**Estimated effort:** L

### Goal
The thing the user actually came here for. Present a puzzle on a chessground board, accept moves, validate against the solution, provide feedback, and record the attempt.

### In scope

**Frontend:**
- Install `chessground` and `chess.js`
- `PuzzleBoard` component wrapping chessground, with move validation via chess.js
- `PuzzlePage` that loads the next puzzle, presents it, times the attempt, handles correct/incorrect feedback (chessground supports arrow/highlight overlays — use them)
- "Next puzzle" button after each attempt
- Keyboard shortcuts: spacebar for next, arrow keys to navigate moves in review

**Integration:**
- Puzzle attempt recording via Slice 6 commands
- If the solution is multi-ply, the opponent's response moves automatically (chessground supports this pattern)

### Out of scope
- Hint system (post v0.1)
- Move annotations / "why this move" (post v0.1)
- Puzzle review mode / browse past puzzles (post v0.1)

### Deliverables
- `PuzzleBoard`, `PuzzlePage` components with tests
- Chessground CSS imported and themed to match app
- Documentation of the puzzle interaction model in `docs/PUZZLES.md`

### Manual verification
1. Load a puzzle. Play the correct move — confirm success feedback.
2. Load another puzzle. Play an incorrect move — confirm failure feedback and that the correct move is shown afterward.
3. Confirm attempts are recorded in the database.
4. Solve 10 puzzles in a row, keyboard-navigating. No jank, no crashes.

---

## Slice 9 — Basic Stats UI

**Status:** Not started
**Depends on:** 6
**Estimated effort:** S

### Goal
A minimal stats page so the user can see their progress. Not a dashboard — just enough to be motivating.

### In scope
- Stats page displaying: total puzzles solved, success rate overall, puzzles attempted today, current streak
- Pulls from `get_attempts_summary` command

### Out of scope
- Weakness-by-phase analysis (v0.2)
- Charts / graphs (v0.2)
- Exportable reports (post v0.1)

### Deliverables
- `StatsPage` component
- Tests for the formatting logic
- Minor doc updates

### Manual verification
1. View the stats page after solving some puzzles. Numbers match what's in the database.

---

## Slice 10 — v0.1 Release Prep

**Status:** Not started
**Depends on:** all prior
**Estimated effort:** M

### Goal
Get the app into a shippable state. Bundle Stockfish, finalize CI release workflow, write release notes, test a clean-machine install.

### In scope
- Stockfish binaries bundled in `resources/stockfish/` for Windows, macOS, Linux
- App resolves bundled binary at runtime via Tauri's path APIs (fall back to PATH in dev)
- Finalize `release.yml` GitHub Actions workflow to produce cross-platform installers on tag push
- Write v0.1 entry in CHANGELOG.md
- README screenshots and quickstart
- SmartScreen note for unsigned Windows binaries
- Attribution: Stockfish license in the distribution bundle, about dialog listing all third-party licenses
- Manual test of the release installer on a clean Windows VM (or separate machine)

### Out of scope
- Auto-updater (post v0.1)
- Code signing (post v0.1)
- Donation links (post v0.1, once usage justifies)

### Deliverables
- Tagged `v0.1.0` release on GitHub with downloadable artifacts for all three platforms
- CHANGELOG entry
- Updated README
- Release announcement draft (optional, for later sharing)

### Manual verification
1. On a machine that has never had this project, download the Windows installer from the GitHub release.
2. Install. Launch. Enter Lichess username. Complete full sync + analyze. Solve puzzles.
3. Uninstall cleanly.

---

## Post-v0.1 Sketches (Not Yet Sliced)

These are reminders of what's coming, not commitments. They'll be sliced when v0.1 ships and the author has real usage feedback.

### v0.2 — Quality & Retention
- Spaced repetition scheduling for missed puzzles
- Failure analytics (puzzles missed 3+ times surface weaknesses)
- Include mistakes (100–200cp drops) as puzzle sources with difficulty labeling
- Weakness-by-game-phase stats (opening / middlegame / endgame)

### v0.3 — Smarter Puzzles
- Counter-threat / missed-prophylaxis puzzles — the big design challenge from the project mission
- Tactical theme detection
- OAuth for private games and Lichess studies

### v0.4 — Opening & Endgame Trainers
- Opening explorer using Lichess's explorer endpoint
- Personal repertoire deviation detection
- Endgame trainer using Lichess's Syzygy tablebase endpoint

### v0.5+ — Platform Expansion
- Chess.com game source
- Arbitrary PGN file import
- Puzzle export / sharing

---

## Adapting This Plan

This plan is a hypothesis, not a contract. Expect revisions. When revising:

1. **Never edit a completed slice's scope retroactively.** It happened. Its "Notes" section records the truth.
2. **Prefer adding new slices over expanding existing ones.** If Slice 5 reveals a missing piece, insert a new Slice 5.5 rather than growing Slice 5 mid-flight.
3. **Renumbering is fine when the plan is restructured.** Update the Slice Roadmap table and all cross-references.
4. **If a slice turns out to be too big, split it.** Signs it's too big: more than a day of focused work, more than ~15 files changed, more than one architectural concept introduced.
5. **When priorities shift, reorder the "Not started" slices freely.** Just keep dependencies respected.

---

*This document is the single source of truth for what the project is currently working on. When in doubt, come here first.*