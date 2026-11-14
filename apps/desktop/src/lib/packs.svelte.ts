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

  /** Asks for a `.swpack` file, imports it and says what it added. */
  async import(): Promise<void> {
    const path = await open({ filters: PACK_FILTERS, multiple: false });
    if (typeof path !== "string") return;
    try {
      const result = await importPack(path);
      if (result.lowerThirdsAdded > 0) await lowerThirds.load();
      const lines = [
        t("pack.addedThemes", { count: result.themesAdded }),
        t("pack.addedLowerThirds", { count: result.lowerThirdsAdded }),
      ];
      if (result.alreadyHere > 0) lines.push(t("pack.alreadyHere", { count: result.alreadyHere }));
      if (result.missingFonts.length > 0) {
        lines.push(t("pack.missingFonts", { fonts: result.missingFonts.join(", ") }));
      }
      await prompts.ask({
        title: t("pack.importedTitle"),
        message: lines.join("\n"),
        buttons: [{ id: "ok", label: t("common.ok"), variant: "primary" }],
        cancel: "ok",
      });
    } catch (error) {
      engine.showError(error);
    }
  }
}

export const packs = new Packs();
