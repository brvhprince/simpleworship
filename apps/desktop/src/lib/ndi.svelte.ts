/**
 * NDI output, from the operator's side: whether it is on, whether the NDI
 * Runtime was found, and how many receivers are watching each source. The
 * sending itself happens in Rust (see `src-tauri/src/ndi.rs`).
 */

import type { NdiSettings, NdiStatus } from "@simpleworship/protocol";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { engine } from "./engine.svelte.ts";

/** Must match `STATUS_EVENT` in `src-tauri/src/ndi.rs`. */
const STATUS_EVENT = "ndi://status";

class NdiStore {
  status: NdiStatus | null = $state.raw(null);

  /** Whether any source is being sent. */
  get sending(): boolean {
    return (this.status?.sources.length ?? 0) > 0;
  }

  /** Starts following NDI. Resolves to a function that stops. */
  async connect(): Promise<() => void> {
    const stop = await listen<NdiStatus>(STATUS_EVENT, ({ payload }) => {
      this.status = payload;
    });
    this.status = await invoke<NdiStatus>("ndi_status");
    return stop;
  }

  async set(settings: NdiSettings): Promise<void> {
    try {
      this.status = await invoke<NdiStatus>("ndi_set", { settings });
    } catch (error) {
      engine.showError(error);
    }
  }
}

export const ndi = new NdiStore();
