/**
 * The Settings, Song Usage and About dialogs, opened from the menu.
 */

import { onMenuClick } from "./menu.ts";

class Dialogs {
  /** The open dialog, if any. */
  open: "settings" | "usage" | "about" | null = $state(null);

  close(): void {
    this.open = null;
  }

  /** Listens for the menu items that open these dialogs. */
  connect(): Promise<() => void> {
    return onMenuClick((id) => {
      if (id === "settings" || id === "usage" || id === "about") this.open = id;
    });
  }
}

export const dialogs = new Dialogs();
