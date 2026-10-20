/**
 * Listen, from the operator's side: whether this computer is listening to
 * the preacher, and the references it heard (newest first), for the
 * Scripture tab to offer. The listening itself happens in Rust (see
 * `src-tauri/src/listen.rs`); these commands always run on this computer,
 * even when it has joined another.
 */

import type {
  AudioInput,
  HeardWords,
  ListenSettings,
  ListenStatus,
  Passage,
  SpeechModel,
} from "@simpleworship/protocol";
import { invoke } from "@tauri-apps/api/core";
import { listen as onEvent } from "@tauri-apps/api/event";
import { engine } from "./engine.svelte.ts";

/** Must match the events in `src-tauri/src/listen.rs`. */
const STATUS_EVENT = "listen://status";
const HEARD_EVENT = "listen://heard";
const LEVEL_EVENT = "listen://level";

/** How many references to keep offering. */
const KEEP = 6;

/** A reference heard, and when. */
export interface Heard {
  passage: Passage;
  /** How it is written ("John 3:16"). */
  reference: string;
  /** The words it was found in. */
  said: string;
  /** When it was (last) heard, in milliseconds since the epoch. */
  at: number;
}

class ListenStore {
  status: ListenStatus | null = $state.raw(null);
  devices: AudioInput[] = $state.raw([]);
  /** How loud the microphone is, 0 to 1. */
  level = $state(0);
  /** The last sentence heard, to show that it is hearing. */
  words = $state("");
  /** References heard, newest first. */
  heard: Heard[] = $state.raw([]);
  /** References heard since the Scripture tab was last looked at. */
  unseen = $state(0);
  /**
   * A heard reference to show in the preview, asked for from outside the
   * Scripture tab (the H key). The tab shows it, with its translations,
   * and clears this.
   */
  request: Heard | null = $state.raw(null);

  get listening(): boolean {
    return this.status?.listening ?? false;
  }

  /** Starts following Listen. Resolves to a function that stops. */
  async connect(): Promise<() => void> {
    const stops = await Promise.all([
      onEvent<ListenStatus>(STATUS_EVENT, ({ payload }) => {
        this.status = payload;
        if (!payload.listening) this.level = 0;
      }),
      onEvent<number>(LEVEL_EVENT, ({ payload }) => {
        this.level = payload;
      }),
      onEvent<HeardWords>(HEARD_EVENT, ({ payload }) => this.#hear(payload)),
    ]);
    this.status = await invoke<ListenStatus>("listen_status");
    return () => {
      for (const stop of stops) stop();
    };
  }

  /** Looks for microphones again (one may have been plugged in). */
  async findDevices(): Promise<void> {
    this.devices = await invoke<AudioInput[]>("listen_devices");
  }

  async start(): Promise<void> {
    await this.#run(() => invoke<ListenStatus>("listen_start"));
  }

  async stop(): Promise<void> {
    await this.#run(() => invoke<ListenStatus>("listen_stop"));
  }

  async configure(settings: ListenSettings): Promise<void> {
    await this.#run(() => invoke<ListenStatus>("listen_configure", { settings }));
  }

  async download(model: SpeechModel): Promise<void> {
    await this.#run(() => invoke<ListenStatus>("listen_download", { model }));
  }

  /** The H key: shows the newest reference heard in the preview. */
  previewNewest(): boolean {
    const newest = this.heard[0];
    if (!newest) return false;
    this.request = newest;
    return true;
  }

  /** Forgets what was heard (the Clear button). */
  clear(): void {
    this.heard = [];
    this.unseen = 0;
  }

  #hear(heard: HeardWords) {
    this.words = heard.words;
    const at = Date.now();
    let list = this.heard;
    for (const found of heard.references) {
      // Said again: move it to the front rather than offering it twice.
      list = [
        { ...found, at },
        ...list.filter((other) => other.reference !== found.reference),
      ].slice(0, KEEP);
      this.unseen += 1;
    }
    this.heard = list;
  }

  async #run(action: () => Promise<ListenStatus>) {
    try {
      this.status = await action();
    } catch (error) {
      engine.showError(error);
    }
  }
}

export const listen = new ListenStore();
