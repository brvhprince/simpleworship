/**
 * Right-click menus, as the system draws them (macOS, Windows, Linux), so
 * they look and behave like every other app's: what someone coming from
 * EasyWorship reaches for first.
 *
 * Each list builds its own entries from the actions it already has (see
 * `SchedulePanel`, `SongsTab`, `MediaTab`, `PresentationsTab`,
 * `ThemesTab`); this file only turns them into a native menu at the mouse.
 */

import { LogicalPosition } from "@tauri-apps/api/dpi";
import {
  type CheckMenuItemOptions,
  Menu,
  type MenuItemOptions,
  type PredefinedMenuItemOptions,
  type SubmenuOptions,
} from "@tauri-apps/api/menu";

/** One line of a right-click menu. */
export type MenuEntry =
  | {
      label: string;
      /** Runs when the line is chosen. */
      action: () => unknown;
      /** Greyed out when `false`. */
      enabled?: boolean;
      /** Shows a tick when `true` (a choice, such as the current scaling). */
      checked?: boolean;
    }
  | { label: string; submenu: MenuEntry[] }
  | "separator";

/** The menu open last; a new one replaces it (see {@link showContextMenu}). */
let open: Menu | null = null;
/**
 * Counts right-clicks, so that only the newest one shows its menu: menus
 * are made one at a time on the Rust side, and one for an earlier
 * right-click can be ready after a later one's. Shown late, its entries
 * would act on the item clicked before.
 */
let latest = 0;

/**
 * Shows `entries` as a menu where `event` happened, instead of the
 * webview's own menu. Entries that are `false`, `null` or `undefined` are
 * left out, so a list can write `!link.joined && { … }`; separators at the
 * ends or next to each other are dropped.
 */
export async function showContextMenu(
  event: MouseEvent,
  entries: (MenuEntry | false | null | undefined)[],
): Promise<void> {
  event.preventDefault();
  event.stopPropagation();
  const request = ++latest;
  // Taken now: the pointer can move while the menu is made.
  const at = anchor(event);
  const shown = tidy(entries.filter((entry): entry is MenuEntry => Boolean(entry)));
  if (shown.length === 0) return;

  const menu = await Menu.new({ items: shown.map(toItem) });
  if (request !== latest) {
    // Another right-click came while this menu was made: show that one.
    void menu.close();
    return;
  }
  // The previous menu is finished with once another opens; closing it
  // frees what the system kept for it.
  void open?.close();
  open = menu;
  await menu.popup(at);
}

/**
 * Where the menu opens, in the window (the webview fills it, unzoomed):
 * where the right-click was, or, for the keyboard's menu key (which has no
 * pointer position), under the item it was pressed on.
 */
function anchor(event: MouseEvent): LogicalPosition | undefined {
  if (event.clientX !== 0 || event.clientY !== 0) {
    return new LogicalPosition(event.clientX, event.clientY);
  }
  if (event.target instanceof Element) {
    const box = event.target.getBoundingClientRect();
    return new LogicalPosition(box.left, box.bottom);
  }
  return undefined;
}

/** Drops separators at the ends and next to each other (exported for tests). */
export function tidy(entries: MenuEntry[]): MenuEntry[] {
  const result: MenuEntry[] = [];
  for (const entry of entries) {
    const last = result.at(-1);
    if (entry === "separator" && (last === undefined || last === "separator")) continue;
    result.push(entry);
  }
  while (result.at(-1) === "separator") result.pop();
  return result;
}

/** What the menu API takes for one entry. */
type ItemOptions =
  | MenuItemOptions
  | CheckMenuItemOptions
  | SubmenuOptions
  | PredefinedMenuItemOptions;

/** The menu API's description of one entry. */
function toItem(entry: MenuEntry): ItemOptions {
  if (entry === "separator") return { item: "Separator" };
  if ("submenu" in entry) {
    return { text: entry.label, items: tidy(entry.submenu).map(toItem) };
  }
  const action = () => void entry.action();
  if (entry.checked !== undefined) {
    return { text: entry.label, checked: entry.checked, enabled: entry.enabled ?? true, action };
  }
  return { text: entry.label, enabled: entry.enabled ?? true, action };
}
