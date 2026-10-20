/**
 * The Settings, Song Usage and About dialogs, opened from the menu.
 */

import { engine } from "./engine.svelte.ts";
import { t } from "./i18n/i18n.svelte.ts";
import { link } from "./link.svelte.ts";
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
      if (id !== "settings" && id !== "usage" && id !== "about") return;
      // The song usage report is kept by the main computer.
      if (id === "usage" && link.joined) {
        engine.showError(t("link.onMainComputer", { name: link.status.hostName }));
        return;
      }
      this.open = id;
    });
  }
}

export const dialogs = new Dialogs();
