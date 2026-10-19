import type { Command, EngineEvent, EngineSnapshot } from "./generated/bindings.ts";

/**
 * Name of the channel engine events are broadcast on.
 * Must match `ENGINE_EVENT` in `apps/desktop/src-tauri/src/commands/engine.rs`.
 */
export const ENGINE_EVENT = "engine://event";

/**
 * A connection to the SimpleWorship engine.
 *
 * UI code depends only on this interface, so the same components work in
 * the desktop app (Tauri IPC) and, later, a phone remote (WebSocket).
 */
export interface Transport {
  /** Sends a command to the engine. Rejects with a message if it is refused. */
  dispatch(command: Command): Promise<void>;

  /** Fetches the complete current engine state. */
  snapshot(): Promise<EngineSnapshot>;

  /**
   * Calls `listener` for every engine event, in order.
   * Resolves to a function that stops listening.
   */
  subscribe(listener: (event: EngineEvent) => void): Promise<() => void>;
}
