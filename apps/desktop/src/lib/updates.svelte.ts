/**
 * New versions of SimpleWorship, handled so they never get in the way of a
 * service:
 *
 * - The app looks for one at startup (after a minute) and once a day, if
 *   allowed in Settings, and always from Help › Check for Updates….
 * - A new version downloads quietly in the background; the toolbar then
 *   shows "Update ready".
 * - It is only installed when the operator clicks, and not while anything
 *   is live. The app restarts only then.
 *
 * Updates come from the project's GitHub releases and are checked against
 * the project's signing key (see `src-tauri/tauri.conf.json`), so only
 * real SimpleWorship releases are installed.
 */

import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { getSettings } from "./api.ts";
import { engine } from "./engine.svelte.ts";
import { t } from "./i18n/i18n.svelte.ts";
import { onMenuClick } from "./menu.ts";
import { prompts } from "./prompts.svelte.ts";

/** Wait this long after startup before looking, so starting up stays quick. */
const FIRST_CHECK_DELAY_MS = 60_000;
const DAY_MS = 24 * 60 * 60 * 1000;

type Status = "idle" | "checking" | "downloading" | "ready" | "installing";

class Updates {
  status: Status = $state("idle");
  /** The new version, once one is found. */
  version: string | null = $state(null);
  /** What is new in it, from the release notes. */
  notes: string | null = $state(null);
  /** Download progress, 0 to 1. */
  progress = $state(0);

  #update: Update | null = null;

  /** Starts the automatic checks and listens for the Help menu. */
  async connect(): Promise<() => void> {
    const stopMenu = await onMenuClick((id) => {
      if (id === "check-updates") void this.checkNow();
    });
    // No releases to find while developing.
    if (import.meta.env.DEV) return stopMenu;

    const automatic = async () => {
      const settings = await getSettings().catch(() => null);
      if (settings?.checkForUpdates !== false) await this.#look().catch(() => {});
    };
    const first = setTimeout(automatic, FIRST_CHECK_DELAY_MS);
    const daily = setInterval(automatic, DAY_MS);
    return () => {
      stopMenu();
      clearTimeout(first);
      clearInterval(daily);
    };
  }

  /** Help › Check for Updates…: looks now and says what it found. */
  async checkNow(): Promise<void> {
    try {
      const found = await this.#look();
      if (!found) {
        await prompts.ask({
          title: t("updates.upToDate"),
          message: t("updates.newest"),
          buttons: [{ id: "ok", label: t("common.ok"), variant: "primary" }],
          cancel: "ok",
        });
      }
    } catch (error) {
      engine.showError(
        t("updates.checkFailed", { error: error instanceof Error ? error.message : String(error) }),
      );
    }
  }

  /** Looks for a new version and downloads it. Resolves to whether there is one. */
  async #look(): Promise<boolean> {
    if (this.status !== "idle") return this.version !== null;
    this.status = "checking";
    try {
      const update = await check();
      if (!update) {
        this.status = "idle";
        return false;
      }
      this.#update = update;
      this.version = update.version;
      this.notes = update.body ?? null;
      this.status = "downloading";
      let total = 0;
      let received = 0;
      await update.download((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        if (event.event === "Progress") {
          received += event.data.chunkLength;
          this.progress = total ? received / total : 0;
        }
      });
      this.status = "ready";
      return true;
    } catch (error) {
      this.status = "idle";
      throw error;
    }
  }

  /** Installs the downloaded version and restarts, if nothing is live. */
  async install(): Promise<void> {
    if (!this.#update || this.status !== "ready") return;
    if (engine.live) {
      await prompts.ask({
        title: t("updates.liveTitle"),
        message: t("updates.liveMessage"),
        buttons: [{ id: "ok", label: t("common.ok"), variant: "primary" }],
        cancel: "ok",
      });
      return;
    }
    const answer = await prompts.ask({
      title: t("updates.installTitle", { version: this.version ?? "" }),
      message: `${this.notes ? `${this.notes}\n\n` : ""}${t("updates.installMessage")}`,
      buttons: [
        { id: "later", label: t("common.later") },
        { id: "install", label: t("updates.install"), variant: "primary" },
      ],
      cancel: "later",
    });
    if (answer !== "install") return;
    this.status = "installing";
    try {
      await this.#update.install();
      await relaunch();
    } catch (error) {
      this.status = "ready";
      engine.showError(
        t("updates.installFailed", {
          error: error instanceof Error ? error.message : String(error),
        }),
      );
    }
  }
}

export const updates = new Updates();
