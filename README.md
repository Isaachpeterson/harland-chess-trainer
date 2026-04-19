# Harland Chess Trainer

![Status: Pre-v0.1](https://img.shields.io/badge/status-pre--v0.1-orange)

Harland Chess Trainer is a local-first desktop application that analyzes a player's own Lichess games, identifies their recurring mistakes, and turns those mistakes into targeted training puzzles. All analysis happens locally via Stockfish and the Lichess public API — no server to operate, no data to collect, and no recurring cost to users.

## Documentation

See the [docs/](docs/README.md) directory for:

- [Architecture](docs/ARCHITECTURE.md) — system design and crate layout
- [Development](docs/DEVELOPMENT.md) — local setup and workflow
- [Conventions](docs/CONVENTIONS.md) — documentation standards

## License

This project is licensed under the [GNU General Public License v3.0](LICENSE).

### Third-party attributions

- [Stockfish](https://stockfishchess.org/) (GPL-3.0) — bundled chess engine ([source](https://github.com/official-stockfish/Stockfish))
- [chessground](https://github.com/lichess-org/chessground) (GPL-3.0) — chess board UI
- [chess.js](https://github.com/jhlywa/chess.js) (BSD-2-Clause) — frontend chess move validation
- [shakmaty](https://github.com/niklasf/shakmaty) (GPL-3.0) — Rust chess library
