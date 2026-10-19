/**
 * Clicks on the native menu bar.
 *
 * The Rust side forwards every click as an event carrying the item's id
 * (see `src-tauri/src/menu.rs`). Components can temporarily take over an
 * item: while the song editor is open, ⌘/Ctrl+S saves the song instead of
 * the schedule.
 */

import { listen } from "@tauri-apps/api/event";

/** Must match `MENU_EVENT` in `src-tauri/src/menu.rs`. */
const MENU_EVENT = "menu://action";

/** Prefix of "Open Recent" item ids; the rest is the file path. */
export const OPEN_RECENT_PREFIX = "open-recent:";

const overrides = new Map<string, () => void>();

/**
 * Makes menu item `id` call `handler` instead of its normal action, until
 * the returned function is called. Use it from an `$effect` so it is
 * undone when the component goes away.
 */
export function overrideMenuItem(id: string, handler: () => void): () => void {
  overrides.set(id, handler);
  return () => {
    if (overrides.get(id) === handler) overrides.delete(id);
  };
}

/**
 * Calls `handle` with the id of every menu click that is not overridden.
 * Resolves to a function that stops listening.
 */
export function onMenuClick(handle: (id: string) => void): Promise<() => void> {
  return listen<string>(MENU_EVENT, ({ payload: id }) => {
    const override = overrides.get(id);
    if (override) override();
    else handle(id);
  });
}

/**
 * Edit › Undo and Redo: undo typing in the focused text field, unless an
 * editor took them over with {@link overrideMenuItem}.
 */
export function connectEditMenu(): Promise<() => void> {
  return onMenuClick((id) => {
    // execCommand is old, but it is how a page asks the webview to undo
    // typing in a text field, and every webview still supports it.
    if (id === "undo") document.execCommand("undo");
    if (id === "redo") document.execCommand("redo");
  });
}
