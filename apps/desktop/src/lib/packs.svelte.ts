/**
 * Sharing themes and lower thirds as `.swpack` files (see
 * `src-tauri/src/packs.rs`): which ones the export dialog has open with,
 * and importing a pack.
 *
 * Neither works on a computer that joined another: the library it shows
 * is the main computer's, and the file is on this one.
 */

import { open } from "@tauri-apps/plugin-dialog";
import { importPack } from "./api.ts";
import { engine } from "./engine.svelte.ts";
import { t } from "./i18n/i18n.svelte.ts";
import { link } from "./link.svelte.ts";
import { lowerThirds } from "./lower-thirds.svelte.ts";
import { prompts } from "./prompts.svelte.ts";

/** The file type in open and save dialogs. */
export const PACK_FILTERS = [{ name: "SimpleWorship Theme Pack", extensions: ["swpack"] }];

/** What the export dialog starts with ticked. */
export interface PackChoice {
  themeIds: string[];
  lowerThirdIds: string[];
}

class Packs {
  /** The export dialog, open with these ticked; `null` when closed. */
  exporting: PackChoice | null = $state(null);

  /** Opens the export dialog with `choice` ticked. */
  startExport(choice: PackChoice): void {
    this.exporting = choice;
  }

  /**
   * Imports a `.swpack` file (or asks for one) and says what it added. On
   * a joined computer it explains why it cannot instead.
   */
  async import(file?: string): Promise<void> {
    if (link.joined) {
      await this.#tell(t("pack.importTitle"), t("pack.notWhenJoined"));
      return;
    }
    const path = file ?? (await open({ filters: PACK_FILTERS, multiple: false }));
    if (typeof path !== "string") return;
    try {
      // The import writes the lower thirds itself: the list is locked and
      // reloaded around it.
      const result = await lowerThirds.whileWrittenElsewhere(() => importPack(path));
      const lines = [
        t("pack.addedThemes", { count: result.themesAdded }),
        t("pack.addedLowerThirds", { count: result.lowerThirdsAdded }),
      ];
      if (result.alreadyHere > 0) lines.push(t("pack.alreadyHere", { count: result.alreadyHere }));
      if (result.missingFonts.length > 0) {
        lines.push(t("pack.missingFonts", { fonts: result.missingFonts.join(", ") }));
      }
      await this.#tell(t("pack.importedTitle"), lines.join("\n"));
    } catch (error) {
      engine.showError(error);
    }
  }

  /** Shows a message with an OK button. */
  async #tell(title: string, message: string): Promise<void> {
    await prompts.ask({
      title,
      message,
      buttons: [{ id: "ok", label: t("common.ok"), variant: "primary" }],
      cancel: "ok",
    });
  }
}

export const packs = new Packs();
