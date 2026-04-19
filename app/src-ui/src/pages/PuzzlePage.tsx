// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { useState, useEffect, useCallback, useRef } from "react";
import {
  PuzzleBoard,
  legalDests,
  orientationFromFen,
} from "../components/PuzzleBoard";
import { getNextPuzzle, submitPuzzleAttempt } from "../api/puzzles";
import type { PuzzleResponse } from "../api/puzzles";
import type { Key, Color } from "chessground/types";
import { Chess } from "chess.js";

type PuzzleState = "loading" | "solving" | "correct" | "incorrect" | "empty";

export function PuzzlePage() {
  const [puzzle, setPuzzle] = useState<PuzzleResponse | null>(null);
  const [state, setState] = useState<PuzzleState>("loading");
  const [fen, setFen] = useState<string>("");
  const [orientation, setOrientation] = useState<Color>("white");
  const [dests, setDests] = useState<Map<Key, Key[]>>(new Map());
  const [lastMove, setLastMove] = useState<Key[] | undefined>(undefined);
  const [check, setCheck] = useState<Color | boolean>(false);
  const [, setMovePlayed] = useState<string>("");
  const [solutionDisplay, setSolutionDisplay] = useState<string>("");
  const [puzzleCount, setPuzzleCount] = useState(0);
  const [correctCount, setCorrectCount] = useState(0);
  const [error, setError] = useState<string | null>(null);

  // Timer for puzzle attempts
  const startTimeRef = useRef<number>(0);

  const loadPuzzle = useCallback(async () => {
    setState("loading");
    setError(null);
    setLastMove(undefined);
    setCheck(false);
    setMovePlayed("");
    setSolutionDisplay("");
    try {
      const p = await getNextPuzzle();
      if (!p) {
        setState("empty");
        return;
      }
      setPuzzle(p);
      setFen(p.fen);
      const orient = orientationFromFen(p.fen);
      setOrientation(orient);
      setDests(legalDests(p.fen));
      setState("solving");
      startTimeRef.current = Date.now();
    } catch (e: unknown) {
      setError(String(e));
      setState("empty");
    }
  }, []);

  // Load first puzzle on mount
  useEffect(() => {
    loadPuzzle();
  }, [loadPuzzle]);

  // Keyboard shortcuts
  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === " " || e.key === "Spacebar") {
        e.preventDefault();
        if (state === "correct" || state === "incorrect" || state === "empty") {
          loadPuzzle();
        }
      }
    }
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [state, loadPuzzle]);

  const handleMove = useCallback(
    async (orig: Key, dest: Key) => {
      if (!puzzle || state !== "solving") return;

      const chess = new Chess(fen);
      // Try the move — chess.js needs {from, to} format
      const moveResult = chess.move({ from: orig, to: dest, promotion: "q" });
      if (!moveResult) return;

      const uciMove =
        orig +
        dest +
        (moveResult.promotion && moveResult.promotion !== "q"
          ? moveResult.promotion
          : moveResult.isPromotion()
            ? "q"
            : "");
      setMovePlayed(uciMove);

      const timeTaken = Date.now() - startTimeRef.current;
      const solutionMoves = puzzle.solution_moves;

      // Check if the user's move matches the first solution move
      const isCorrect = matchesSolutionMove(uciMove, solutionMoves[0]);

      // Update the board to show the resulting position
      const newFen = chess.fen();
      setFen(newFen);
      setLastMove([orig, dest]);
      setDests(new Map()); // disable further moves
      setCheck(
        chess.inCheck() ? (chess.turn() === "w" ? "white" : "black") : false,
      );

      // Record the attempt
      try {
        await submitPuzzleAttempt(puzzle.id, isCorrect, timeTaken, uciMove);
      } catch {
        // Don't block the UI for a recording failure
      }

      setPuzzleCount((c) => c + 1);

      if (isCorrect) {
        setCorrectCount((c) => c + 1);
        setState("correct");
      } else {
        // Show the correct move
        const solution = solutionMoves[0];
        setSolutionDisplay(formatSolutionDisplay(puzzle.fen, solution));
        setState("incorrect");

        // Animate the correct move on the board after a short delay
        setTimeout(() => {
          showSolutionOnBoard(puzzle.fen, solution);
        }, 800);
      }
    },
    [puzzle, state, fen],
  );

  /** Animates the solution move onto the board. */
  function showSolutionOnBoard(puzzleFen: string, solutionUci: string) {
    const chess = new Chess(puzzleFen);
    const from = solutionUci.slice(0, 2);
    const to = solutionUci.slice(2, 4);
    const promo = solutionUci.length > 4 ? solutionUci[4] : undefined;
    const result = chess.move({ from, to, promotion: promo });
    if (result) {
      setFen(chess.fen());
      setLastMove([from as Key, to as Key]);
      setCheck(
        chess.inCheck() ? (chess.turn() === "w" ? "white" : "black") : false,
      );
    }
  }

  const interactive = state === "solving";

  return (
    <div className="page puzzle-page">
      <h2>Train</h2>

      {error && <p className="error-msg">{error}</p>}

      {state === "empty" && !error && (
        <div className="puzzle-empty">
          <p>No puzzles available yet.</p>
          <p className="hint">
            Run a sync from the Sync page to generate puzzles from your games.
          </p>
        </div>
      )}

      {state !== "empty" && (
        <>
          <div className="puzzle-board-container">
            {fen && (
              <PuzzleBoard
                fen={fen}
                orientation={orientation}
                dests={dests}
                onMove={handleMove}
                interactive={interactive}
                lastMove={lastMove}
                check={check}
              />
            )}
          </div>

          <div className="puzzle-feedback">
            {state === "loading" && (
              <p className="puzzle-status">Loading puzzle…</p>
            )}
            {state === "solving" && (
              <p className="puzzle-status puzzle-prompt">
                Find the best move for{" "}
                {orientation === "white" ? "White" : "Black"}.
              </p>
            )}
            {state === "correct" && (
              <p className="puzzle-status puzzle-correct">Correct!</p>
            )}
            {state === "incorrect" && (
              <div>
                <p className="puzzle-status puzzle-incorrect">Incorrect.</p>
                {solutionDisplay && (
                  <p className="puzzle-solution">
                    Best move: <strong>{solutionDisplay}</strong>
                  </p>
                )}
              </div>
            )}
          </div>

          <div className="puzzle-controls">
            <button
              onClick={loadPuzzle}
              disabled={state === "loading" || state === "solving"}
              className="primary-btn"
            >
              Next Puzzle
            </button>
            <span className="puzzle-score hint">
              {puzzleCount > 0 && `${correctCount}/${puzzleCount} correct`}
            </span>
            <span className="hint keyboard-hint">
              Press <kbd>Space</kbd> for next puzzle
            </span>
          </div>
        </>
      )}
    </div>
  );
}

/**
 * Checks whether a user's UCI move matches the solution UCI move.
 * Handles promotion normalization (e.g., "e7e8q" matches "e7e8q" or "e7e8").
 */
function matchesSolutionMove(userUci: string, solutionUci: string): boolean {
  // Normalize: strip trailing 'q' from promotions for comparison since queen
  // is the default promotion and backends may or may not include it.
  const normalize = (uci: string) => {
    if (uci.length === 5 && uci[4] === "q") return uci.slice(0, 4);
    return uci;
  };
  return normalize(userUci) === normalize(solutionUci);
}

/**
 * Converts a UCI solution move into a human-readable SAN string for display.
 */
function formatSolutionDisplay(fen: string, solutionUci: string): string {
  try {
    const chess = new Chess(fen);
    const from = solutionUci.slice(0, 2);
    const to = solutionUci.slice(2, 4);
    const promo = solutionUci.length > 4 ? solutionUci[4] : undefined;
    const result = chess.move({ from, to, promotion: promo });
    return result ? result.san : solutionUci;
  } catch {
    return solutionUci;
  }
}

// Exported for testing
export { matchesSolutionMove, formatSolutionDisplay };
