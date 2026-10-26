/**
 * Right-click menus, as the system draws them (macOS, Windows, Linux), so
 * they look and behave like every other app's: what someone coming from
 * EasyWorship reaches for first.
 *
 * Each list builds its own entries from the actions it already has (see
 * `SchedulePanel`, `SongsTab`, `MediaTab`, `PresentationsTab`,
 * `ThemesTab`); this file only turns them into a native menu at the mouse.
 */

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
  const shown = tidy(entries.filter((entry): entry is MenuEntry => Boolean(entry)));
  if (shown.length === 0) return;

  // The previous menu is finished with once another opens; closing it
  // frees what the system kept for it.
  const previous = open;
  open = await Menu.new({ items: shown.map(toItem) });
  void previous?.close();
  await open.popup();
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
