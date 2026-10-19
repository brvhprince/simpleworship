/**
 * New, Open, Save, Save As and Quit for schedules, and everything around
 * them: the "save changes?" question, the window title, and offering to
 * restore a schedule after a crash.
 *
 * All of these flows live here, in one place, so they read top to bottom.
 * The Rust side does the file work (see `src-tauri/src/commands/schedule.rs`).
 */

import type { AutosaveInfo, ScheduleFileStatus } from "@simpleworship/protocol";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as api from "./api.ts";
import { engine } from "./engine.svelte.ts";
import { t } from "./i18n/i18n.svelte.ts";
import { OPEN_RECENT_PREFIX, onMenuClick } from "./menu.ts";
import { prompts } from "./prompts.svelte.ts";

/** Must match `SCHEDULE_FILE_EVENT` in `src-tauri/src/schedule_file.rs`. */
const SCHEDULE_FILE_EVENT = "schedule-file://status";

/** Must match `OPEN_FILE_EVENT` in `src-tauri/src/launch.rs`. */
const OPEN_FILE_EVENT = "app://open-schedule";

/** File type filter for the open and save dialogs. */
const SCHEDULE_FILTERS = [{ name: "SimpleWorship Schedule", extensions: ["sws"] }];

class ScheduleFileStore {
  /** The current file name and whether it has unsaved changes. */
  status: ScheduleFileStatus = $state({ path: null, name: "Untitled", dirty: false });

  /**
   * Starts following the file status, handles File menu clicks and the
   * window's close button, and offers to restore a crashed session.
   * Call once, from the operator window. Resolves to a cleanup function.
   */
  async connect(): Promise<() => void> {
    const stopStatus = await listen<ScheduleFileStatus>(SCHEDULE_FILE_EVENT, ({ payload }) =>
      this.#setStatus(payload),
    );
    const stopMenu = await onMenuClick((id) => this.#handleMenu(id));
    const stopClose = await getCurrentWindow().onCloseRequested(async (event) => {
      // Stop the window closing by itself; `quit` closes it if appropriate.
      event.preventDefault();
      await this.quit();
    });
    // A schedule file double-clicked (or dropped on the app's icon).
    const stopOpenFile = await listen<string>(OPEN_FILE_EVENT, ({ payload }) => {
      void this.open(payload);
    });

    this.#setStatus(await api.scheduleFileStatus());
    await this.#offerRestore();

    // The file SimpleWorship was started with, if any.
    const launchedWith = await api.launchReady();
    if (launchedWith) await this.open(launchedWith);

    return () => {
      stopStatus();
      stopMenu();
      stopClose();
      stopOpenFile();
    };
  }

  /** Starts an empty schedule, after asking about unsaved changes. */
  async newSchedule(): Promise<void> {
    if (await this.#readyToReplace()) {
      await this.#attempt(api.newSchedule());
    }
  }

  /**
   * Opens a schedule, after asking about unsaved changes. Without a `path`
   * the operator picks a file.
   */
  async open(path?: string): Promise<void> {
    if (!(await this.#readyToReplace())) return;

    const chosen = path ?? (await open({ filters: SCHEDULE_FILTERS, multiple: false }));
    if (typeof chosen === "string") {
      await this.#attempt(api.openSchedule(chosen));
    }
  }

  /** Saves to the current file, or asks where if it was never saved.
   *  Resolves to whether the schedule was saved. */
  async save(): Promise<boolean> {
    if (!this.status.path) return this.saveAs();
    return this.#attempt(api.saveSchedule(null));
  }

  /** Asks where to save, then saves. Resolves to whether it was saved. */
  async saveAs(): Promise<boolean> {
    const path = await save({
      filters: SCHEDULE_FILTERS,
      defaultPath: `${this.status.name}.sws`,
    });
    return path ? this.#attempt(api.saveSchedule(path)) : false;
  }

  /** Quits the app, after asking about unsaved changes. */
  async quit(): Promise<void> {
    const answer = await this.#askToSave();
    if (answer === "cancel") return;
    if (answer === "save" && !(await this.save())) return;
    if (answer === "discard") await api.discardAutosave();

    // Destroying the operator window quits the app (see `windows.rs`).
    await getCurrentWindow().destroy();
  }

  // ── Internals ──────────────────────────────────────────────────────────

  #handleMenu(id: string): void {
    if (id.startsWith(OPEN_RECENT_PREFIX)) {
      void this.open(id.slice(OPEN_RECENT_PREFIX.length));
      return;
    }
    switch (id) {
      case "schedule-new":
        void this.newSchedule();
        break;
      case "schedule-open":
        void this.open();
        break;
      case "schedule-save":
        void this.save();
        break;
      case "schedule-save-as":
        void this.saveAs();
        break;
      case "quit":
        void this.quit();
        break;
    }
  }

  #setStatus(status: ScheduleFileStatus): void {
    this.status = status;
    const unsaved = status.dirty ? " •" : "";
    void getCurrentWindow().setTitle(`${status.name}${unsaved} — SimpleWorship`);
  }

  /**
   * Whether the current schedule may be replaced: there are no unsaved
   * changes, or the operator saved or discarded them.
   */
  async #readyToReplace(): Promise<boolean> {
    const answer = await this.#askToSave();
    if (answer === "save") return this.save();
    return answer !== "cancel";
  }

  /** Asks "save changes?" if there are unsaved changes. */
  async #askToSave(): Promise<"save" | "discard" | "cancel" | "clean"> {
    if (!this.status.dirty) return "clean";
    const answer = await prompts.ask({
      title: t("file.unsavedTitle"),
      message: t("file.unsavedMessage", {
        name: this.status.path ? this.status.name : t("schedule.untitled"),
      }),
      buttons: [
        { id: "discard", label: t("common.dontSave") },
        { id: "cancel", label: t("common.cancel") },
        { id: "save", label: t("common.save"), variant: "primary" },
      ],
      cancel: "cancel",
    });
    return answer as "save" | "discard" | "cancel";
  }

  /** After a crash, offers the autosaved schedule back. */
  async #offerRestore(): Promise<void> {
    const leftover = await api.autosaveLeftover();
    if (!leftover) return;

    const answer = await prompts.ask({
      title: t("file.restoreTitle"),
      message: restoreMessage(leftover),
      buttons: [
        { id: "discard", label: t("common.discard") },
        { id: "restore", label: t("file.restore"), variant: "primary" },
      ],
      cancel: "discard",
    });
    await this.#attempt(answer === "restore" ? api.restoreAutosave() : api.discardAutosave());
  }

  /** Runs a file operation, showing any failure. Resolves to whether it worked. */
  async #attempt(operation: Promise<void>): Promise<boolean> {
    try {
      await operation;
      return true;
    } catch (error) {
      engine.showError(error);
      return false;
    }
  }
}

function restoreMessage(leftover: AutosaveInfo): string {
  const when = new Date(leftover.savedAt).toLocaleString([], {
    dateStyle: "medium",
    timeStyle: "short",
  });
  return t("file.restoreMessage", { when });
}

export const scheduleFile = new ScheduleFileStore();
