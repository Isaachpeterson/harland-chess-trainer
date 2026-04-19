# Stockfish Binaries

This directory holds Stockfish binaries that are bundled into the release.

**These binaries are not committed to the repository.** They are downloaded
automatically during CI release builds (see `.github/workflows/release.yml`).

## For local development

You do **not** need a binary here for development. Set the `STOCKFISH_PATH`
environment variable to your local Stockfish binary, or ensure `stockfish` is
on your system PATH.

## Expected filenames

The app expects the binary to be named:

- `stockfish.exe` (Windows)
- `stockfish` (macOS / Linux)

The CI workflow downloads the correct binary for each platform and places it
here before the Tauri build step.
