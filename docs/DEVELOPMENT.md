# Development Setup

> **Status:** Stub — this document is expanded as the development workflow matures.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) (LTS recommended)
- npm (comes with Node.js)
- Platform-specific Tauri dependencies — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)

## Getting started

```bash
# Clone the repository
git clone https://github.com/Isaachpeterson/harland-chess-trainer.git
cd harland-chess-trainer

# Install frontend dependencies
cd app/src-ui
npm install
cd ../..

# Run the development build
cargo tauri dev --config app/tauri.conf.json
```

## Project structure

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full workspace layout and crate responsibilities.

## Running tests

```bash
# Rust tests (all crates)
cargo test --workspace

# Frontend tests
cd app/src-ui
npm run test
```

## Linting

```bash
# Rust
cargo fmt --check
cargo clippy -- -D warnings

# Frontend
cd app/src-ui
npm run lint
```
