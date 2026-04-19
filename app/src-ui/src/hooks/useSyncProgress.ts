// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

/**
 * React hook that subscribes to `"sync-progress"` Tauri events emitted by
 * the `full_sync` backend command.
 */

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { SyncProgress } from "../api/sync";

export interface UseSyncProgressResult {
  /** Latest progress payload, or null if no sync has started yet. */
  progress: SyncProgress | null;
  /** True while the pipeline is running (stage is not "complete" or "error"). */
  isRunning: boolean;
}

/**
 * Subscribes to `"sync-progress"` events for the lifetime of the component.
 * Returns the most recent progress state.
 */
export function useSyncProgress(): UseSyncProgressResult {
  const [progress, setProgress] = useState<SyncProgress | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<SyncProgress>("sync-progress", (event) => {
      setProgress(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  const running =
    progress !== null &&
    progress.stage !== "complete" &&
    progress.stage !== "error";

  return { progress, isRunning: running };
}
