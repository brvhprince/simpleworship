/**
 * The engine state as seen by this window, kept up to date from engine
 * events. Components read `engine.schedule`, `engine.preview`,
 * `engine.live` and `engine.output` and re-render automatically when they
 * change.
 */

import type {
  AudioState,
  Command,
  EngineEvent,
  EngineSnapshot,
  OutputState,
  PaneState,
  Schedule,
  StageState,
  ThemeLibrary,
  Transport,
} from "@simpleworship/protocol";
import { tauriTransport } from "./tauri-transport.ts";

/** How long an error message stays visible, in milliseconds. */
const ERROR_TIMEOUT_MS = 6000;

export class EngineStore {
  /** The order of service. */
  schedule: Schedule = $state.raw({ items: [] });
  /** The themes and the default theme for each kind of content. */
  themes: ThemeLibrary = $state.raw({
    themes: [],
    defaults: { song: null, scripture: null, text: null, presentation: null },
  });
  /** The preview pane, or `null` if empty. */
  preview: PaneState | null = $state.raw(null);
  /** The live pane, or `null` if nothing is live. */
  live: PaneState | null = $state.raw(null);
  /** What the output screens draw. `null` until the first snapshot arrives. */
  output: OutputState | null = $state.raw(null);
  /** The stage display's timer and message. */
  stage: StageState = $state.raw({ timer: null, message: null });
  /** The background audio player. */
  audio: AudioState | null = $state.raw(null);
  /** The last error, shown to the operator for a few seconds. */
  error: string | null = $state(null);

  #transport: Transport;
  #errorTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(transport: Transport) {
    this.#transport = transport;
  }

  /**
   * Starts following the engine. Resolves to a function that stops.
   *
   * Subscribes *before* fetching the snapshot, so no change can slip
   * through in between.
   */
  async connect(): Promise<() => void> {
    const unsubscribe = await this.#transport.subscribe((event) => this.#apply(event));
    this.#applySnapshot(await this.#transport.snapshot());
    return unsubscribe;
  }

  /** Sends a command. Failures are shown in {@link error} instead of thrown. */
  dispatch(command: Command): void {
    this.#transport.dispatch(command).catch((error) => this.showError(error));
  }

  /** Shows an error message to the operator for a few seconds. */
  showError(error: unknown): void {
    this.error = error instanceof Error ? error.message : String(error);
    clearTimeout(this.#errorTimer);
    this.#errorTimer = setTimeout(() => (this.error = null), ERROR_TIMEOUT_MS);
  }

  #apply(event: EngineEvent): void {
    switch (event.type) {
      case "previewChanged":
        this.preview = event.preview;
        break;
      case "liveChanged":
        this.live = event.live;
        break;
      case "outputChanged":
        this.output = event.output;
        break;
      case "scheduleChanged":
        this.schedule = event.schedule;
        break;
      case "themesChanged":
        this.themes = event.themes;
        break;
      case "stageChanged":
        this.stage = event.stage;
        break;
      case "audioChanged":
        this.audio = event.audio;
        break;
      case "songWentLive":
        // Recorded by the app for the CCLI report; nothing to show.
        break;
    }
  }

  #applySnapshot(snapshot: EngineSnapshot): void {
    this.schedule = snapshot.schedule;
    this.themes = snapshot.themes;
    this.stage = snapshot.stage;
    this.audio = snapshot.audio;
    this.preview = snapshot.preview;
    this.live = snapshot.live;
    // An event may already have delivered a newer output than this snapshot.
    if (!this.output || snapshot.output.revision >= this.output.revision) {
      this.output = snapshot.output;
    }
  }
}

/** The engine store for this window. */
export const engine = new EngineStore(tauriTransport);
