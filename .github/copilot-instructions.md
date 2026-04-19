# Harland Chess Trainer — Project Context

> This document is the canonical reference for the Harland Chess Trainer project. It describes the mission, architecture, scope, and conventions. It is intended to be read by human contributors and used as context by AI coding assistants (primarily GitHub Copilot).

**Author:** Isaac Peterson ([@Isaachpeterson](https://github.com/Isaachpeterson))
**License:** GPL-3.0
**Status:** Pre-v0.1 (planning / initial scaffolding)

---

## Related Documents

This file is the high-level project context. For day-to-day implementation guidance, refer to these companion documents:

- **`docs/IMPLEMENTATION_PLAN.md`** — the vertical-slice roadmap. When implementing a feature, read the currently-active slice and implement **only** that slice. Do not pull work forward from later slices.
- **`docs/VERSIONING.md`** — the versioning scheme. When changing `Cargo.toml` versions, `tauri.conf.json`, or `CHANGELOG.md`, follow the rules in this document rather than guessing.
- **`docs/CONVENTIONS.md`** — documentation conventions (update before create, index discipline, etc.).
- **`docs/ARCHITECTURE.md`** — evolving description of how the crates fit together. Update this whenever a new crate or major module is added.
- **`CHANGELOG.md`** — every slice completion adds an entry to the `[Unreleased]` section.

**For AI assistants:** Always check `docs/IMPLEMENTATION_PLAN.md` for the current slice before writing code. Always check `docs/VERSIONING.md` before bumping version numbers.

---

## 1. Mission

Harland Chess Trainer is a local-first desktop application that analyzes a player's own Lichess games, identifies their recurring mistakes, and turns those mistakes into targeted training puzzles. The goal is to help the user improve at chess by repeatedly training on the specific tactical and positional patterns they actually miss in their own play — rather than generic puzzle sets unrelated to their weaknesses.

The project is a passion project by a 1600-rated rapid player (Lichess) who wants to both improve at chess and build something useful that others can freely download, use, and contribute to.

### Why this project exists

Paid services (Chessable, Aimchess, etc.) offer similar functionality, but they're subscription-based and proprietary. A local, open-source alternative gives users full control of their data, costs nothing to run, and can be extended by the community. Since all analysis happens locally via Stockfish and the Lichess public API, there is no server to operate, no data to collect, and no recurring cost to users.

### Core values

These are the principles that should guide design decisions and code review:

- **Craft over speed.** The code should be understandable, idiomatic, and well-tested. Shipping something half-built is not a goal.
- **User ownership.** The user's data stays on their machine. The user supplies their own credentials. The user can audit exactly what the app does.
- **No dark patterns.** No telemetry, no upsells, no "pro features." If the project ever accepts donations, donations are voluntary and don't unlock functionality.
- **Extensibility.** The chess-analysis core should be reusable. A well-structured core enables later features (opening explorer, endgame trainer, alternative game sources like Chess.com) without rewriting.
- **Open forever.** GPL-3 ensures the project and any derivatives stay open source.

---

## 2. Tech Stack

| Layer | Choice | Rationale |
|-------|--------|-----------|
| App framework | **Tauri 2** | Small binaries (~10MB), native webview, Rust backend for system work, first-class cross-platform builds |
| Frontend language | **TypeScript** (strict mode) | Type safety across the web layer |
| Frontend framework | **React 18+** | The author works in React daily; largest Copilot training data |
| Styling | **Tailwind CSS** | Utility-first, fast iteration, pairs well with component libraries |
| Chess board UI | **chessground** | The same open-source board Lichess uses; GPL-3.0 licensed (compatible with this project) |
| Chess move validation (frontend) | **chess.js** | Canonical JavaScript chess library |
| Backend language | **Rust** (stable toolchain) | Memory safety, strong compiler feedback, good for long-lived desktop processes |
| Chess logic (backend) | **shakmaty** | High-quality Rust chess library; position representation, move generation, PGN parsing |
| Engine | **Stockfish** (bundled binary) | Strongest open-source chess engine; communicates via UCI over stdin/stdout |
| Async runtime | **tokio** | Standard async runtime in Rust ecosystem |
| HTTP client | **reqwest** | Standard Rust HTTP client, async-first |
| Database | **SQLite** via **sqlx** or **rusqlite** | Embedded, file-based, no setup; sqlx preferred for compile-time checked queries |
| Credential storage | **keyring** crate (OS-native keychain) | Avoids plaintext tokens on disk |
| Testing | `cargo test`, `proptest`, Vitest (frontend) | Rust's built-in test framework + property-based testing for chess invariants |
| Linting | `cargo clippy`, `cargo fmt`, ESLint, Prettier | Enforced in CI |
| CI/CD | **GitHub Actions** with `tauri-action` | Automated cross-platform builds on release tags |

### Notes on Rust (for a Rust-new developer)

The author has no production Rust experience. Copilot will generate most of the Rust code, guided by architectural review. To make this workflow productive:

- **Trust the compiler.** If `cargo build` fails, read the error — the Rust compiler's errors are exceptionally informative. Do not accept Copilot suggestions that silence errors with `unwrap()` or `unsafe` unless there's a documented reason.
- **Run clippy on every commit.** `cargo clippy -- -D warnings` in CI. Clippy catches non-idiomatic patterns Copilot often produces.
- **Use `Result<T, E>` everywhere.** Functions that can fail should return `Result`. Use the `thiserror` crate for library-style errors and `anyhow` for application-level error aggregation.
- **Prefer owned types at module boundaries, borrow internally.** This simplifies lifetimes for someone learning.
- **Write tests alongside code.** Rust's `#[cfg(test)]` modules make this easy. Use `proptest` for any chess position logic that has invariants.

---

## 3. Architecture

The project is a **Cargo workspace** with multiple crates, enabling independent testing and future reuse. Frontend lives in a separate directory but is part of the same repo.

```
harland-chess-trainer/
├── Cargo.toml                 # workspace root
├── crates/
│   ├── chess-core/            # position types, PGN parsing, mistake detection (no I/O)
│   ├── lichess-client/        # Lichess API wrapper (games, opening explorer, tablebase)
│   ├── engine/                # Stockfish UCI wrapper
│   ├── puzzle-gen/            # puzzle generation from analyzed games
│   └── storage/               # SQLite schema + queries for games, puzzles, progress
├── app/                       # Tauri app (Rust backend + React frontend)
│   ├── src/                   # Tauri Rust commands, orchestration
│   ├── src-ui/                # React + TypeScript frontend
│   │   ├── src/
│   │   ├── package.json
│   │   └── vite.config.ts
│   └── tauri.conf.json
├── resources/
│   └── stockfish/             # bundled Stockfish binaries per platform
├── .github/
│   ├── copilot-instructions.md  # this file (auto-loaded by Copilot in VS Code)
│   └── workflows/             # CI/CD pipelines
├── docs/
│   ├── README.md              # docs index
│   ├── CONVENTIONS.md         # documentation conventions
│   ├── ARCHITECTURE.md        # evolving architecture notes
│   ├── DEVELOPMENT.md         # local dev setup
│   ├── IMPLEMENTATION_PLAN.md # current slice and roadmap
│   └── VERSIONING.md          # version bump rules
├── LICENSE                    # GPL-3.0
├── CONTRIBUTORS.md
├── CHANGELOG.md
└── README.md
```

### Crate responsibilities

**`chess-core`** — pure chess logic with no I/O. Depends on `shakmaty`. Exports:
- Position and move types (likely re-exports from shakmaty with project-specific extensions)
- PGN parsing wrappers
- `MistakeClassification` enum (Inaccuracy, Mistake, Blunder) with configurable thresholds
- Mistake detection given a sequence of (position, move, eval_before, eval_after) tuples
- Future home for counter-threat detection logic (v0.3+)

**`lichess-client`** — async wrapper over the Lichess public API. Depends on `reqwest`, `tokio`, `serde`. Exports:
- `LichessClient` struct with methods: `fetch_user_games`, `fetch_game_analysis`, `fetch_opening_explorer`, `fetch_tablebase`
- PGN streaming support (Lichess returns ndjson/PGN streams for bulk game fetch)
- Respects rate limits and sets a descriptive User-Agent: `HarlandChessTrainer/{version} (+https://github.com/Isaachpeterson/harland-chess-trainer)`
- Caching layer keyed by game ID to avoid re-fetching

**`engine`** — Stockfish process management. Exports:
- `Engine` struct that spawns Stockfish, handles UCI handshake, and manages command/response
- `analyze_position(fen, depth_or_time)` returning centipawn eval + best move + principal variation
- Supports multi-PV analysis (top N moves) for puzzle quality filtering
- Handles graceful shutdown; no orphaned processes

**`puzzle-gen`** — takes analyzed games and produces puzzles. Exports:
- `PuzzleCandidate` struct (position FEN, solution move(s), mistake classification, source game ID + ply)
- Generation rules (blunder-only for v0.1, expanded later)
- Quality filters (eval gap between best and second-best move, uniqueness of solution)

**`storage`** — SQLite persistence. Owns the database schema and migrations. Exports typed query functions; no raw SQL leaks into other crates.

**`app`** — the Tauri application. Ties everything together. Defines Tauri commands exposed to the frontend (`fetch_games`, `analyze_game`, `get_next_puzzle`, `submit_puzzle_attempt`, etc.). The frontend calls these via `@tauri-apps/api/core`'s `invoke`.

---

## 4. MVP Scope (v0.1)

The v0.1 target is a working, usable-by-the-author release. It must do the full loop end-to-end but with minimal polish and minimal features.

> **For the current work-in-progress slice breakdown, see `docs/IMPLEMENTATION_PLAN.md`. This section describes v0.1 as a whole; the implementation plan describes how it's being built.**

### In scope for v0.1

1. **User enters their Lichess username** (no OAuth yet — public games only).
2. **App fetches games** from `/api/games/user/{username}` — default to last 50 rapid games, configurable.
3. **For each game:**
   - Parse PGN.
   - If Lichess server-side analysis is available in the response, use it. **(Default behavior.)**
   - If not, queue for local Stockfish analysis.
   - Allow the user to toggle a setting: "Always use local Stockfish" (for consistency or when Lichess analysis isn't available).
4. **Detect blunders only** (eval drop ≥ 200 centipawns from the user's perspective on the user's moves) — see Section 6 for thresholds.
5. **Generate puzzle candidates** from detected blunders. Position = position before the blunder. Solution = engine's best move. Filter out puzzles where the second-best move is within 50cp of the best (low-quality puzzles).
6. **Store everything in SQLite:**
   - Fetched games (PGN + metadata + analysis status).
   - Puzzle candidates.
   - User attempts per puzzle (correct / incorrect, timestamp, time taken).
7. **Basic UI:**
   - Settings page: username entry, game count, Stockfish preference.
   - Sync page: "Fetch & Analyze" button, progress indicator.
   - Puzzle page: chessground board, puzzle counter, correct/incorrect feedback, "next puzzle" button.
   - Simple stats page: total puzzles, success rate, puzzles solved today.
8. **Bundled Stockfish binary** for Windows, macOS, and Linux.
9. **GPL-3 license + attribution** in the app's About dialog and in the repo.
10. **CI/CD**: builds on push, releases on tag (see Section 9).

### Explicitly out of scope for v0.1

- OAuth / private games / Lichess studies.
- Counter-threat / missed-opportunity puzzles (v0.3+).
- Opening explorer integration (v0.4).
- Endgame tablebase integration (v0.4).
- Chess.com support (v0.5+).
- Puzzle theming / categorization (fork, pin, back-rank, etc.) — too complex for v0.1.
- Spaced repetition scheduling (v0.2).
- Auto-update / Tauri updater.
- Code-signed binaries (unsigned Windows release for v0.1 with a README note about SmartScreen).

### Definition of "v0.1 is done"

The author can install a release from the repo on a clean Windows machine, enter their Lichess username, click Sync, wait for analysis, and then work through puzzles generated from their own recent blunders. Puzzle attempts are recorded. App does not crash on any of the author's games.

---

## 5. Roadmap

### v0.2 — Quality & Retention
- Spaced repetition: missed puzzles come back on a schedule.
- Failure analytics: "you missed this puzzle 3 times" → surface repeated weaknesses.
- Include mistakes (100–200cp drops) in addition to blunders, with difficulty labeling.
- Better stats dashboard (weakness by phase: opening/middlegame/endgame).

### v0.3 — Smarter Puzzles
- **Counter-threat puzzles.** Detect positions where the opponent's next move introduced a serious threat the user didn't anticipate, and generate puzzles asking the user to find the prophylactic move *before* the threat materialized. This is a real design challenge: involves looking at multi-ply eval swings, identifying opponent "plans," and extracting positions where a defensive/preventive move was clearly best.
- Puzzle theming (tactical motif detection via patterns or a classifier).
- OAuth for private games and studies.

### v0.4 — Opening & Endgame Trainers
- Opening explorer using Lichess's `/api/opening/explorer` endpoint.
- Personal opening repertoire tracker: detect the user's most-played openings and identify where they deviate from theory.
- Endgame trainer using Lichess's 7-piece Syzygy tablebase endpoint.

### v0.5+ — Platform Expansion
- Chess.com support (their public API is more limited; PGN import as fallback).
- Import arbitrary PGN files.
- Export puzzles to PGN for sharing.

---

## 6. Chess Analysis Details

### Mistake classification (1600-level defaults)

The author is 1600 rapid on Lichess. Default centipawn thresholds are tuned for instructive mistakes at this level:

| Classification | Eval drop | v0.1 action |
|---------------|-----------|-------------|
| Inaccuracy | 50–99 cp | Ignored in v0.1 (too subtle to be instructive puzzles at 1600 level) |
| Mistake | 100–199 cp | Stored, but not turned into puzzles in v0.1 (added in v0.2) |
| Blunder | 200+ cp | Becomes puzzle candidate |

Thresholds should be **configurable in settings** (stored in user config, not hardcoded). As the user improves, they'll want tighter thresholds. A 2000-rated user should probably include mistakes; a 1200-rated user might want only blunders of 300cp+.

### Mistake detection algorithm

For each of the user's moves in a game:

1. Get `eval_before` = engine evaluation of the position before the user's move, from the user's perspective.
2. Get `eval_after` = engine evaluation of the position after the user's move, from the user's perspective.
3. `drop = eval_before - eval_after`
4. Classify based on drop.

Important edge cases:
- **Mate scores** need special handling. A position evaluated as "mate in 3" vs "mate in 5" is not a meaningful drop. Treat mate scores as ±10000 cp but don't flag "mate in N → mate in M" transitions (both winning) as mistakes.
- **Already-losing positions.** If the position was already -500cp and the user plays a move that makes it -700cp, this is technically a 200cp drop but may not be an instructive blunder. Consider capping: only flag blunders where the position transitions from drawn/winning to losing (or losing to much-more-losing by a large margin).
- **Time trouble moves.** Not detectable from PGN alone unless clock data is present. Lichess PGN includes clock comments. Optionally filter or tag these.

### Puzzle quality filters

A good puzzle has one clearly best move. Filters applied before storing a puzzle:

1. **Unique-best-move gap:** the engine's best move must be at least 50cp better than the second-best (configurable). If two moves are within 50cp, the position is ambiguous.
2. **Not a "recapture" puzzle:** filter out positions where the obvious recapture is the best move (these are too easy to be instructive).
3. **Depth floor:** engine analysis must reach at least depth 18 for v0.1 puzzles. Higher depth for training positions = more reliable "correct" answers.

### Lichess-first vs. Stockfish-first analysis

Default: use Lichess's server-side analysis if present in the game data. This is fast (no local computation), free, and deep (Lichess runs its cloud engine at a strong depth). The PGN returned by `/api/games/user/...?evals=true` includes evaluation comments when the game has been analyzed on Lichess.

Fallback: if `evals=true` doesn't return evaluations for a game (not every game is auto-analyzed), queue it for local Stockfish analysis.

User option: "Always use local Stockfish." Slower but gives consistent depth and avoids dependency on Lichess's analysis being present. Useful for power users.

---

## 7. Data Model

SQLite schema (illustrative — finalize in the storage crate):

```sql
-- Fetched games from Lichess
CREATE TABLE games (
    id TEXT PRIMARY KEY,            -- Lichess game ID
    pgn TEXT NOT NULL,
    user_color TEXT NOT NULL,       -- 'white' | 'black'
    user_result TEXT NOT NULL,      -- 'win' | 'loss' | 'draw'
    time_control TEXT,
    rated INTEGER NOT NULL,
    created_at INTEGER NOT NULL,    -- unix timestamp
    analysis_source TEXT,           -- 'lichess' | 'stockfish' | NULL
    analysis_completed_at INTEGER
);

-- Detected mistakes within games
CREATE TABLE mistakes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    ply INTEGER NOT NULL,                   -- half-move index
    fen_before TEXT NOT NULL,
    user_move TEXT NOT NULL,                -- SAN or UCI
    best_move TEXT NOT NULL,
    eval_before INTEGER NOT NULL,           -- centipawns
    eval_after INTEGER NOT NULL,
    classification TEXT NOT NULL            -- 'inaccuracy' | 'mistake' | 'blunder'
);

-- Puzzles generated from mistakes
CREATE TABLE puzzles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mistake_id INTEGER NOT NULL REFERENCES mistakes(id) ON DELETE CASCADE,
    fen TEXT NOT NULL,
    solution_moves TEXT NOT NULL,           -- JSON array of UCI moves
    themes TEXT,                            -- JSON array, populated later
    created_at INTEGER NOT NULL
);

-- User attempts at solving puzzles
CREATE TABLE puzzle_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    puzzle_id INTEGER NOT NULL REFERENCES puzzles(id) ON DELETE CASCADE,
    attempted_at INTEGER NOT NULL,
    success INTEGER NOT NULL,               -- 0 or 1
    time_taken_ms INTEGER NOT NULL,
    move_played TEXT NOT NULL               -- what the user actually played
);
```

Migrations should be version-controlled and idempotent. Use sqlx's migration system or embed them manually. **Any migration that changes an existing table's columns requires a minor version bump minimum — see `docs/VERSIONING.md`.**

---

## 8. License & Legal

### Why GPL-3.0

- Stockfish is GPL-3. Bundling a GPL binary with a non-GPL app is legally defensible under "mere aggregation" but ambiguous. Matching licenses removes all doubt.
- Prevents someone from forking the project, adding a subscription wall, and selling it without contributing back.
- Aligns with the project's values: stays open forever.

### Compliance checklist

- [ ] `LICENSE` file at repo root containing full GPL-3.0 text.
- [ ] GPL-3 copyright header at the top of every source file (use a short form referencing the LICENSE file).
- [ ] Third-party attributions in README and About dialog:
  - Stockfish (GPL-3) — link to source
  - chessground (GPL-3.0) — link to source
  - chess.js (BSD-2-Clause) — link to source
  - shakmaty (GPL-3) — link to source
  - Other dependencies as they're added
- [ ] Include Stockfish's LICENSE in the `resources/stockfish/` directory of each release.
- [ ] README "License" section clearly stating GPL-3 and how to obtain source.

### Source header template

```rust
// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.
```

### Contributor License Agreement

Not required for v0.1. If the project grows and dual-licensing ever becomes relevant, a CLA can be introduced later (all contributors from that point forward would sign).

---

## 9. Distribution & Release

### Release targets

- Windows x86_64 (.msi installer via Tauri)
- macOS universal (.dmg) — Apple Silicon + Intel
- Linux x86_64 (.AppImage and .deb)

### GitHub Actions workflow

Two workflows:

**`ci.yml`** — runs on every push and PR:
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test --workspace`
- Frontend: `npm run lint`, `npm run typecheck`, `npm run test`
- Does NOT build binaries (too slow for every push).

**`release.yml`** — runs on version tag push (`v*.*.*`):
- Uses `tauri-apps/tauri-action` to build for all three platforms in parallel (matrix).
- Creates a GitHub Release with the artifacts attached.
- Release body auto-generated from CHANGELOG.md entry for that version.

### Versioning

**See `docs/VERSIONING.md` for the full scheme, bump rules, and release checklist.** In short: semantic versioning, with pre-1.0 treating `0.MINOR` as the "effective major" version. User data compatibility (SQLite migrations) must never break silently.

### Release notes

For each release, update `CHANGELOG.md` before tagging. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

### Code signing (deferred)

Unsigned Windows binaries trigger SmartScreen warnings. For v0.1, document this in the README with a screenshot and instructions to bypass. Consider a code signing certificate once the project has a user base.

### Donations (deferred)

Add GitHub Sponsors integration (via `.github/FUNDING.yml`) after v0.1 ships and receives real usage. Do not add donation links to an empty repo.

---

## 10. Development Conventions

### Rust

- **Error handling:** `thiserror` for typed errors in libraries; `anyhow` in the `app` crate and binaries. No `unwrap()` or `expect()` in non-test code without a comment explaining why it's safe.
- **Async:** `tokio` for all async work. Do not mix async runtimes.
- **Naming:** follow standard Rust conventions (snake_case modules and functions, PascalCase types, SCREAMING_SNAKE for constants).
- **Modules:** one concept per module. Prefer many small modules over few large ones.
- **Documentation:** every public function and type in a library crate gets a `///` doc comment. Run `cargo doc --workspace` to verify it builds.
- **Testing:** every library crate has unit tests in `#[cfg(test)]` modules. Integration tests for Stockfish and Lichess client go in `tests/` directories. Property-based tests with `proptest` for chess invariants (e.g., "PGN parse → serialize → parse roundtrip is identity").

### TypeScript / React

- **Strict TypeScript:** `strict: true` in `tsconfig.json`. No `any` without a comment.
- **Components:** functional components with hooks. No class components.
- **State:** local `useState` for component state. Zustand for cross-component state if needed. No Redux.
- **Tauri calls:** all `invoke` calls go through a typed wrapper in `src-ui/src/api/` — never call `invoke` directly from components.
- **Styling:** Tailwind utility classes inline. Extract to a component when a pattern repeats 3+ times. Use CSS modules only if Tailwind can't express the style (rare).
- **Forms:** native HTML inputs with controlled state. No form libraries for v0.1.

### Git

- Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`).
- Feature branches off `main`. PRs merged via squash.
- `main` always builds green.
- Commit messages that introduce breaking changes use the `!` marker (e.g., `feat!: change user settings schema`) — these trigger version bump decisions per `docs/VERSIONING.md`.

### Testing philosophy

Chess logic has precise invariants. Write property-based tests that assert:
- PGN parsing is a pure function.
- Position hashing is stable.
- Move generation from a position is deterministic.
- Mistake classification is monotonic in eval drop.

Integration tests for external dependencies (Lichess, Stockfish) should be marked `#[ignore]` by default and run explicitly in CI with appropriate setup.

---

## 11. Security & Privacy

### Credential storage

- Lichess OAuth tokens (when added in v0.3) stored via the `keyring` crate (uses OS-native keychain).
- **Never** written to plaintext JSON or SQLite.
- **Never** logged, even at debug level.

### User data

- All user data (games, puzzles, attempts) stays in a local SQLite file in the Tauri app's data directory.
- No telemetry. No analytics. No crash reporting to a remote service. (A local crash log file is fine.)
- README explicitly documents: "This app does not send any data to any server controlled by the author. It only communicates with the Lichess public API on the user's behalf."

### Dependencies

- Use `cargo audit` in CI to catch known vulnerabilities in Rust dependencies.
- Use `npm audit` for frontend dependencies.
- Keep dependency count low. Evaluate each new dependency: is it worth the surface area?

---

## 12. Working with GitHub Copilot

This section is specifically for guiding AI-assisted development.

### Required reading before acting

When asked to implement a feature:

1. Read `docs/IMPLEMENTATION_PLAN.md` and identify the current slice. Implement **only** that slice's scope.
2. Consult `docs/VERSIONING.md` before touching any version number in `Cargo.toml`, `tauri.conf.json`, or `package.json`.
3. Update `CHANGELOG.md`'s `[Unreleased]` section as part of the same change. Do not defer this.
4. If a change would require work outside the current slice, stop and flag it — do not pull work forward.

### What Copilot is good at in this project

- Writing Tauri command handlers that follow existing patterns.
- Generating SQL queries and sqlx-annotated Rust structs.
- Writing React components from well-named props.
- Boilerplate-heavy code (UCI message parsing, HTTP request structures).
- Writing tests when given a clear spec.

### What needs human review

- **Chess logic correctness.** Copilot does not understand chess. Any code involving moves, positions, or evaluation must be verified against known positions (use FEN strings from famous games as test cases).
- **Mistake detection algorithms.** The exact thresholds and edge cases (mate scores, already-losing positions) must be human-designed. Copilot will produce naive implementations that miss edge cases.
- **Rust ownership and lifetimes.** Copilot sometimes suggests `.clone()` liberally to make code compile. Review whether the clone is necessary or if borrowing would work.
- **Error handling.** Copilot often reaches for `unwrap()`. Replace with proper `Result` propagation.
- **Security-sensitive code.** Credential handling, file path construction (avoid path traversal), SQL injection prevention (use parameterized queries — sqlx makes this automatic).
- **Version bumps.** Bumping versions is a human decision point. Copilot may propose a bump per `docs/VERSIONING.md` but must not commit version changes without human confirmation.

### Copilot prompting patterns that work well

- **Start with the test.** Write a test that describes the behavior, then let Copilot fill in the implementation. This pins down the spec.
- **Reference this document.** When asking Copilot for architectural guidance, point it at the relevant section of this file.
- **Explicitly state the error type.** "This function should return `Result<Puzzle, PuzzleGenError>`" produces better code than "this function should handle errors."
- **One crate at a time.** Keep Copilot focused on the crate you're working in. Cross-crate refactors need human orchestration.

### Anti-patterns to watch for

- Silently suppressing errors with `.unwrap_or_default()` on results that should be propagated.
- Writing synchronous code where async is required (Stockfish and HTTP both need async).
- Re-inventing functionality that exists in `shakmaty` (move generation, FEN parsing, etc.).
- Generating React components that fetch data directly via `fetch()` instead of going through the typed Tauri `invoke` wrapper.
- Adding new top-level dependencies without reason.
- Starting work on a later slice because "it's easy to do at the same time."
- Bumping the version without consulting `docs/VERSIONING.md`.

---

## 13. Open Questions / Decisions to Revisit

These are intentionally deferred. Revisit when relevant.

- **Stockfish version pinning.** Which Stockfish version to bundle? (Probably latest stable at time of release; document in release notes.)
- **Multi-PV analysis strategy.** For puzzle quality filtering, how many PVs to request from Stockfish? Tradeoff between analysis time and filter quality. Start with 3, tune from there.
- **Game filtering.** Should v0.1 analyze only rated rapid games? Or all rated games? Or include blitz? (Blitz games have more low-quality blunders — might produce too much noise.)
- **Puzzle presentation order.** Random? Weighted by recency? By difficulty? (Start random; add weighting in v0.2.)
- **Theme detection.** How to classify puzzles by tactical theme (fork, pin, skewer, etc.)? Hand-written heuristics, a pattern library, or an ML classifier? Deferred to v0.3.
- **Multi-user support.** Current assumption: one app install = one user. If users want to track multiple accounts on one machine, this needs a profile system. Not a v0.1 concern.

---

## 14. Glossary

- **Centipawn (cp):** 1/100th of a pawn. Standard unit of chess evaluation. +100cp = "up a pawn."
- **UCI:** Universal Chess Interface — the text protocol engines like Stockfish speak over stdin/stdout.
- **PGN:** Portable Game Notation — the standard text format for chess games.
- **FEN:** Forsyth–Edwards Notation — a compact string representation of a single chess position.
- **Ply:** a half-move (one player's move). A full move is two plies.
- **PV:** Principal Variation — the engine's predicted best line of play.
- **Multi-PV:** engine mode where it returns the top N lines instead of only the best.
- **Blunder / Mistake / Inaccuracy:** Lichess's standard error categories, tuned by eval drop.
- **Prophylaxis:** a move that prevents the opponent's plan before it happens. Central to the "counter-threat" puzzle idea.

---

*Last updated: project inception, pre-v0.1. Update this file as architectural decisions change.*