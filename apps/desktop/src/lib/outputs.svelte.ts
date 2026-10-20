/**
 * The output windows: which screen each one goes on, and which are open.
 * Used by the toolbar's Screens panel.
 *
 * - **Projector**: what the congregation sees.
 * - **Stage**: the confidence monitor (current and next slide, clock).
 * - **Stream**: the stream look, for a capture card or video switcher.
 *
 * At startup, with a projector or TV connected, the projector output opens
 * on the first screen that is not the operator's own. With only the main
 * screen, nothing opens (it would cover the operator window).
 */

import type { MonitorInfo, OutputKind, OutputTarget } from "@simpleworship/protocol";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { closeOutput, listMonitors, openOutput } from "./api.ts";
import { engine } from "./engine.svelte.ts";
import type { MessageKey } from "./i18n/i18n.svelte.ts";

/** Must match `OUTPUTS_EVENT` in `src-tauri/src/windows.rs`. */
const OUTPUTS_EVENT = "outputs://changed";

/** The screen choice for a normal window instead of a whole screen. */
export const WINDOWED = "windowed";

/** Every output, with its name and what it is for. */
export const OUTPUTS: { kind: OutputKind; name: MessageKey; hint: MessageKey }[] = [
  { kind: "projector", name: "screens.projector", hint: "screens.projectorHint" },
  { kind: "stage", name: "screens.stage", hint: "screens.stageHint" },
  { kind: "stream", name: "screens.stream", hint: "screens.streamHint" },
];

class Outputs {
  /** The connected monitors. */
  monitors: MonitorInfo[] = $state.raw([]);
  /** The output windows that are open. */
  open: OutputKind[] = $state.raw([]);
  /** The screen chosen for each output: a monitor index, or {@link WINDOWED}. */
  screens: Record<OutputKind, string> = $state({
    projector: WINDOWED,
    stage: WINDOWED,
    stream: WINDOWED,
  });

  /** Starts following the output windows. Resolves to a function that stops. */
  async connect(): Promise<() => void> {
    const stop = await listen<OutputKind[]>(OUTPUTS_EVENT, ({ payload }) => {
      this.open = payload;
    });
    this.open = await invoke<OutputKind[]>("outputs_open");
    await this.refreshMonitors();
    const secondary = this.monitors.find((monitor) => !monitor.isPrimary);
    if (secondary && !this.open.includes("projector")) {
      this.screens.projector = String(secondary.index);
      await this.show("projector");
    }
    return stop;
  }

  /** Re-reads the monitors, so a projector plugged in mid-service shows up. */
  async refreshMonitors(): Promise<void> {
    try {
      this.monitors = await listMonitors();
    } catch (error) {
      engine.showError(error);
    }
  }

  isOpen(kind: OutputKind): boolean {
    return this.open.includes(kind);
  }

  /** Opens `kind` on its chosen screen (or moves it there). */
  async show(kind: OutputKind): Promise<void> {
    const screen = this.screens[kind];
    const target: OutputTarget =
      screen === WINDOWED ? { type: "windowed" } : { type: "monitor", index: Number(screen) };
    try {
      await openOutput(kind, target);
    } catch (error) {
      engine.showError(error);
    }
  }

  async hide(kind: OutputKind): Promise<void> {
    try {
      await closeOutput(kind);
    } catch (error) {
      engine.showError(error);
    }
  }

  /** Chooses the screen for `kind`; an open output moves there at once. */
  async choose(kind: OutputKind, screen: string): Promise<void> {
    this.screens[kind] = screen;
    if (this.isOpen(kind)) await this.show(kind);
  }
}

export const outputs = new Outputs();
