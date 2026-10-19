/**
 * The desktop app's connection to the engine: Tauri IPC.
 * Implements the shared {@link Transport} interface.
 */

import { ENGINE_EVENT, type EngineEvent, type Transport } from "@simpleworship/protocol";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export const tauriTransport: Transport = {
  dispatch: (command) => invoke("engine_dispatch", { command }),

  snapshot: () => invoke("engine_snapshot"),

  subscribe: (listener) => listen<EngineEvent>(ENGINE_EVENT, (event) => listener(event.payload)),
};
