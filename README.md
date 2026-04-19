# Harland Chess Trainer

![Status: v0.1.0](https://img.shields.io/badge/status-v0.1.0-green)
![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue)

Harland Chess Trainer is a local-first desktop application that analyzes a player's own Lichess games, identifies recurring mistakes, and turns those mistakes into targeted training puzzles. All analysis happens locally via Stockfish and the Lichess public API — no server to operate, no data to collect, and no recurring cost to users.

## Quickstart

1. **Download** the latest release for your platform from the [Releases](https://github.com/Isaachpeterson/harland-chess-trainer/releases) page.
   - **Windows:** `.msi` installer
   - **macOS:** `.dmg` disk image
   - **Linux:** `.AppImage` or `.deb` package
2. **Install and launch** the application.
3. Go to **Settings** and enter your Lichess username.
4. Go to **Sync** and click **Fetch & Analyze**. This downloads your recent games, runs analysis, detects blunders, and generates puzzles.
5. Go to **Puzzles** and start solving!

### Windows SmartScreen warning

The Windows release is **not code-signed**. When you run the installer for the first time, Windows SmartScreen may display a warning:

> "Windows protected your PC — Microsoft Defender SmartScreen prevented an unrecognized app from starting."

To proceed:
1. Click **"More info"**
2. Click **"Run anyway"**

This is expected for unsigned open-source software. You can verify the download by checking the SHA-256 hash against the one published in the GitHub release notes. Code signing will be added in a future release once the project has a user base.

## Features (v0.1)

- Fetch your recent Lichess games (public, no account required)
- Automatic analysis: uses Lichess server-side evals when available, falls back to local Stockfish
- Blunder detection with configurable thresholds
- Puzzle generation from your blunders with quality filters
- Interactive puzzle solving with chessground board
- Attempt tracking and basic statistics
- Configurable settings (thresholds, game count, Stockfish preference)
- All data stored locally in SQLite — nothing leaves your machine

## Development

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for local setup instructions.

### Prerequisites

- Rust stable toolchain
- Node.js 20+
- Stockfish binary (set `STOCKFISH_PATH` env var or ensure `stockfish` is on PATH)
- Tauri 2 system dependencies ([platform-specific guide](https://tauri.app/start/prerequisites/))

### Quick start (dev)

```bash
cd app/src-ui && npm install
cd ../.. && cargo tauri dev
```

## Documentation

See the [docs/](docs/README.md) directory for:

- [Architecture](docs/ARCHITECTURE.md) — system design and crate layout
- [Development](docs/DEVELOPMENT.md) — local setup and workflow
- [Conventions](docs/CONVENTIONS.md) — documentation standards
- [Implementation Plan](docs/IMPLEMENTATION_PLAN.md) — slice-based roadmap
- [Analysis](docs/ANALYSIS.md) — analysis pipeline design
- [Puzzles](docs/PUZZLES.md) — puzzle generation pipeline

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for how to get started — whether that's reporting bugs, suggesting features, improving docs, or writing code.

If you're looking for something to work on, check the [issue tracker](https://github.com/Isaachpeterson/harland-chess-trainer/issues) for issues labeled `good first issue` or `help wanted`.

## Roadmap

This project follows a slice-based development plan. See [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md) for the full roadmap. Next up after v0.1:

- **v0.2** — Spaced repetition, failure analytics, mistake-level puzzles
- **v0.3** — Counter-threat puzzles, tactical themes, OAuth for private games
- **v0.4** — Opening explorer, endgame trainer
- **v0.5+** — Chess.com support, PGN import/export

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).

### Third-party attributions

- [Stockfish](https://stockfishchess.org/) (GPL-3.0) — bundled chess engine ([source](https://github.com/official-stockfish/Stockfish))
- [chessground](https://github.com/lichess-org/chessground) (GPL-3.0) — chess board UI
- [chess.js](https://github.com/jhlywa/chess.js) (BSD-2-Clause) — frontend chess move validation
- [shakmaty](https://github.com/niklasf/shakmaty) (GPL-3.0) — Rust chess library
