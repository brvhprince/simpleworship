/**
 * The installed Bibles, kept up to date: the list reloads whenever a Bible
 * is imported, renamed or deleted (including the bundled Bibles that are
 * installed in the background on first launch).
 */

import type { BibleInfo } from "@simpleworship/protocol";
import { listen } from "@tauri-apps/api/event";
import { listBibles } from "./api.ts";

/** Must match `BIBLES_EVENT` in `src-tauri/src/bibles.rs`. */
const BIBLES_EVENT = "bibles://changed";

class BiblesStore {
  /** Every installed Bible, sorted by abbreviation. */
  list: BibleInfo[] = $state.raw([]);
  /** False until the first list has arrived. */
  loaded = $state(false);

  /** Starts following changes. Resolves to a function that stops. */
  async connect(): Promise<() => void> {
    const stop = await listen(BIBLES_EVENT, () => void this.refresh());
    await this.refresh();
    return stop;
  }

  async refresh(): Promise<void> {
    this.list = await listBibles();
    this.loaded = true;
  }
}

export const bibles = new BiblesStore();
