/**
 * Opening the "Import from Other Software" dialog: from the File menu, the
 * Songs tab, or the first-launch offer when SimpleWorship finds another
 * program's library on this computer.
 */

import type { FoundSource } from "@simpleworship/protocol";
import { firstLaunchImports } from "./api.ts";
import { onMenuClick } from "./menu.ts";
import { prompts } from "./prompts.svelte.ts";

class Importer {
  /** Whether the dialog is open. */
  isOpen = $state(false);
  /** Sources to start with (`null`: scan the usual places when it opens). */
  initialSources: FoundSource[] | null = $state(null);

  /** Opens the dialog, optionally with sources already found. */
  open(sources: FoundSource[] | null = null): void {
    this.initialSources = sources;
    this.isOpen = true;
  }

  close(): void {
    this.isOpen = false;
  }

  /** Listens for File › Import from Other Software…. */
  connect(): Promise<() => void> {
    return onMenuClick((id) => {
      if (id === "import") this.open();
    });
  }

  /**
   * On the first launch only: if another program's library is on this
   * computer, offers to bring it across.
   */
  async offerOnFirstLaunch(): Promise<void> {
    const found = await firstLaunchImports();
    if (found.length === 0) return;

    const songs = found.filter((source) => source.kind !== "easyWorshipBible");
    const bibles = found.length - songs.length;
    const parts = [
      ...songs.map((source) => source.label),
      ...(bibles > 0 ? [`${bibles} ${bibles === 1 ? "Bible" : "Bibles"}`] : []),
    ];
    const answer = await prompts.ask({
      title: "Bring your library across?",
      message:
        `SimpleWorship found ${parts.join(", ")} on this computer. ` +
        "Would you like to import them? Nothing in the other program is changed.",
      buttons: [
        { id: "later", label: "Not Now" },
        { id: "import", label: "Review and Import", variant: "primary" },
      ],
      cancel: "later",
    });
    if (answer === "import") this.open(found);
  }
}

export const importer = new Importer();
