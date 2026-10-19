/**
 * The Settings and Song Usage dialogs, opened from the menu.
 */

import { onMenuClick } from "./menu.ts";

class Dialogs {
  /** The open dialog, if any. */
  open: "settings" | "usage" | null = $state(null);

  close(): void {
    this.open = null;
  }

  /** Listens for the menu items that open these dialogs. */
  connect(): Promise<() => void> {
    return onMenuClick((id) => {
      if (id === "settings" || id === "usage") this.open = id;
    });
  }
}

export const dialogs = new Dialogs();
