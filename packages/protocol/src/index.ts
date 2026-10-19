/**
 * @module @simpleworship/protocol
 *
 * Everything a SimpleWorship front end needs to talk to the engine:
 *
 * - The data types (`Command`, `EngineEvent`, `Presentation`, …). They are
 *   generated from the Rust code by ts-rs, so they always match the backend.
 *   Run `pnpm gen:bindings` after changing a Rust type.
 * - The {@link Transport} interface, which hides *how* the front end is
 *   connected (Tauri IPC in the desktop app, a WebSocket in the phone
 *   remote).
 */

export type * from "./generated/bible.ts";
export type * from "./generated/bindings.ts";
export type * from "./generated/desktop.ts";
export type * from "./generated/import.ts";
export type * from "./generated/remote.ts";
export * from "./transport.ts";
