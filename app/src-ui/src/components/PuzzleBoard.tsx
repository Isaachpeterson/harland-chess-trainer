// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

import { useRef, useEffect, useCallback } from "react";
import { Chessground } from "chessground";
import { Chess } from "chess.js";
import type { Api } from "chessground/api";
import type { Config } from "chessground/config";
import type { Key, Color } from "chessground/types";

export interface PuzzleBoardProps {
  /** FEN of the puzzle position. */
  fen: string;
  /** Board orientation — which side is at the bottom. */
  orientation: Color;
  /** Legal move destinations from the current position, keyed by origin square. */
  dests: Map<Key, Key[]>;
  /** Called when the user makes a move (origin, destination). */
  onMove: (orig: Key, dest: Key) => void;
  /** Whether the board accepts user input. */
  interactive: boolean;
  /** Optional last-move highlight (pair of squares). */
  lastMove?: Key[];
  /** Optional check highlight color. */
  check?: Color | boolean;
}

/**
 * Wraps a chessground instance in a React component.
 *
 * The component manages the chessground lifecycle (init on mount, destroy on
 * unmount) and forwards config updates via the `Api.set()` method when props
 * change.
 */
export function PuzzleBoard({
  fen,
  orientation,
  dests,
  onMove,
  interactive,
  lastMove,
  check,
}: PuzzleBoardProps) {
  const boardRef = useRef<HTMLDivElement>(null);
  const apiRef = useRef<Api | null>(null);

  // Stable callback ref for onMove so chessground doesn't re-bind every render
  const onMoveRef = useRef(onMove);
  onMoveRef.current = onMove;

  const handleMove = useCallback((orig: Key, dest: Key) => {
    onMoveRef.current(orig, dest);
  }, []);

  // Initialize chessground on mount
  useEffect(() => {
    if (!boardRef.current) return;

    const config: Config = {
      fen,
      orientation,
      turnColor: orientation, // user's color is always the side to move in a puzzle
      movable: {
        free: false,
        color: interactive ? orientation : undefined,
        dests: interactive ? dests : new Map(),
        showDests: true,
        events: {
          after: handleMove,
        },
      },
      highlight: {
        lastMove: true,
        check: true,
      },
      animation: {
        enabled: true,
        duration: 200,
      },
      lastMove,
      check,
      draggable: {
        enabled: true,
        showGhost: true,
      },
      premovable: {
        enabled: false,
      },
    };

    apiRef.current = Chessground(boardRef.current, config);

    return () => {
      apiRef.current?.destroy();
      apiRef.current = null;
    };
    // Only run on mount/unmount — updates are handled by the set() effect below
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Update chessground when props change (without recreating the instance)
  useEffect(() => {
    if (!apiRef.current) return;
    apiRef.current.set({
      fen,
      orientation,
      turnColor: orientation,
      movable: {
        free: false,
        color: interactive ? orientation : undefined,
        dests: interactive ? dests : new Map(),
        showDests: true,
        events: {
          after: handleMove,
        },
      },
      lastMove,
      check,
    });
  }, [fen, orientation, dests, interactive, lastMove, check, handleMove]);

  return <div ref={boardRef} className="puzzle-board" />;
}

/**
 * Computes legal move destinations for chessground from a FEN string via chess.js.
 * Returns a Map<Key, Key[]> suitable for `movable.dests`.
 */
export function legalDests(fen: string): Map<Key, Key[]> {
  const chess = new Chess(fen);
  const dests = new Map<Key, Key[]>();
  for (const move of chess.moves({ verbose: true })) {
    const from = move.from as Key;
    const to = move.to as Key;
    const existing = dests.get(from);
    if (existing) {
      existing.push(to);
    } else {
      dests.set(from, [to]);
    }
  }
  return dests;
}

/**
 * Determines which color should be at the bottom of the board from a FEN string.
 * The side to move in the puzzle position is the "user's" side.
 */
export function orientationFromFen(fen: string): Color {
  const parts = fen.split(" ");
  return parts[1] === "b" ? "black" : "white";
}
