# Contributing to Harland Chess Trainer

Thanks for your interest in contributing! This project is open source under GPL-3.0 and welcomes contributions of all kinds — bug reports, feature ideas, code, documentation, and testing.

## Ways to contribute

### Report bugs or request features

Open a [GitHub Issue](https://github.com/Isaachpeterson/harland-chess-trainer/issues). Include:

- **Bug reports:** steps to reproduce, expected vs. actual behavior, your OS, and any error messages.
- **Feature requests:** describe the problem you're trying to solve, not just the solution you want. Context helps us design the right thing.

### Contribute code

1. **Check the issue tracker.** Look for issues labeled `good first issue` or `help wanted`. If you want to work on something, comment on the issue so others know.
2. **Read the docs first.** Skim these before writing code:
   - [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — how the crates fit together
   - [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) — what's currently being worked on and what's coming
   - [docs/CONVENTIONS.md](docs/CONVENTIONS.md) — documentation standards
   - [.github/copilot-instructions.md](.github/copilot-instructions.md) — detailed project context, coding conventions, and tech stack rationale
3. **Fork and branch.** Create a feature branch off `main`. Name it descriptively (e.g., `fix/puzzle-promotion-bug`, `feat/spaced-repetition`).
4. **Make your changes.** Follow the conventions below.
5. **Open a Pull Request.** PRs are squash-merged into `main`. Write a clear description of what changed and why.

### Improve documentation

Docs live in the `docs/` directory. Typo fixes, clarifications, and new guides are all welcome — no issue needed, just open a PR.

### Test on your machine

Download a release (or build from source) and try it with your own Lichess account. Report anything that breaks, feels confusing, or could be better.

## Development setup

### Prerequisites

- **Rust** stable toolchain ([rustup.rs](https://rustup.rs/))
- **Node.js** 20+ ([nodejs.org](https://nodejs.org/))
- **Stockfish** binary — set `STOCKFISH_PATH` env var or ensure `stockfish` is on your system PATH
- **Tauri 2 system dependencies** — see the [Tauri prerequisites guide](https://tauri.app/start/prerequisites/) for your OS

### Build and run

```bash
# Install frontend dependencies
cd app/src-ui && npm install && cd ../..

# Run in development mode (hot-reload)
cargo tauri dev

# Run all tests
cargo test --workspace
cd app/src-ui && npm run test
```

### Useful commands

| Command | Purpose |
|---------|---------|
| `cargo test --workspace` | Run all Rust tests |
| `cargo clippy --workspace -- -D warnings` | Lint Rust code |
| `cargo fmt --check` | Check Rust formatting |
| `cd app/src-ui && npm run test` | Run frontend (Vitest) tests |
| `cd app/src-ui && npx tsc --noEmit` | TypeScript type check |
| `cd app/src-ui && npm run prettier:check` | Check frontend formatting |

All of these run automatically in CI on every push.

## Coding conventions

### Git

- **Conventional Commits** — prefix your commit messages: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test:`.
- Feature branches off `main`. PRs merged via squash.
- `main` always builds green.

### Rust

- `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- Use `Result<T, E>` for fallible functions. No `unwrap()` in non-test code without a comment.
- `thiserror` for library crate errors, `anyhow` for the app crate.
- Write tests alongside code in `#[cfg(test)]` modules.

### TypeScript / React

- Strict TypeScript (`strict: true`). No `any` without a comment.
- All Tauri `invoke` calls go through typed wrappers in `src-ui/src/api/` — never call `invoke` directly from components.
- Functional components with hooks. No class components.

### Testing

- Every PR should include tests for new behavior.
- Integration tests for external dependencies (Lichess, Stockfish) are `#[ignore]`-marked and run manually.

## What happens after you open a PR

1. **CI runs automatically** — formatting, linting, and all tests must pass.
2. **The maintainer reviews** — expect feedback within a few days. Small PRs get reviewed faster.
3. **Squash-merge into `main`** — your commits are combined into one clean commit.

## Code of conduct

Be kind and constructive. This is a hobby project built in the open. Disagree respectfully, assume good intent, and remember that everyone is here to learn and build something useful.

## Questions?

Open a [Discussion](https://github.com/Isaachpeterson/harland-chess-trainer/discussions) or comment on a relevant issue. There are no dumb questions.
