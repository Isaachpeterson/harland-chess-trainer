// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

export function AboutPage() {
  return (
    <div className="about-page">
      <h2>Harland Chess Trainer</h2>
      <p className="about-version">v0.1.0</p>
      <p className="about-author">
        Created by{" "}
        <a
          href="https://github.com/Isaachpeterson"
          target="_blank"
          rel="noopener noreferrer"
        >
          Isaac Peterson
        </a>
      </p>
      <p className="about-description">
        A local-first desktop application that analyzes your Lichess games,
        identifies recurring mistakes, and turns them into targeted training
        puzzles. All analysis happens locally — no data leaves your machine.
      </p>

      <h3>License</h3>
      <p>
        This program is free software licensed under the{" "}
        <a
          href="https://www.gnu.org/licenses/gpl-3.0.en.html"
          target="_blank"
          rel="noopener noreferrer"
        >
          GNU General Public License v3.0
        </a>
        .
      </p>
      <p>
        Source code:{" "}
        <a
          href="https://github.com/Isaachpeterson/harland-chess-trainer"
          target="_blank"
          rel="noopener noreferrer"
        >
          github.com/Isaachpeterson/harland-chess-trainer
        </a>
      </p>

      <h3>Third-party software</h3>
      <table className="about-licenses">
        <thead>
          <tr>
            <th>Library</th>
            <th>License</th>
            <th>Source</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>Stockfish</td>
            <td>GPL-3.0</td>
            <td>
              <a
                href="https://github.com/official-stockfish/Stockfish"
                target="_blank"
                rel="noopener noreferrer"
              >
                Source
              </a>
            </td>
          </tr>
          <tr>
            <td>chessground</td>
            <td>GPL-3.0</td>
            <td>
              <a
                href="https://github.com/lichess-org/chessground"
                target="_blank"
                rel="noopener noreferrer"
              >
                Source
              </a>
            </td>
          </tr>
          <tr>
            <td>shakmaty</td>
            <td>GPL-3.0</td>
            <td>
              <a
                href="https://github.com/niklasf/shakmaty"
                target="_blank"
                rel="noopener noreferrer"
              >
                Source
              </a>
            </td>
          </tr>
          <tr>
            <td>chess.js</td>
            <td>BSD-2-Clause</td>
            <td>
              <a
                href="https://github.com/jhlywa/chess.js"
                target="_blank"
                rel="noopener noreferrer"
              >
                Source
              </a>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  );
}
