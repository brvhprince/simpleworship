/**
 * The media library as seen by the operator window: the list of images and
 * videos, import progress, and thumbnails.
 *
 * New media gets its thumbnail (and, for videos, its size and length) from
 * the webview right after import. Media that arrived another way (inside a
 * schedule file) gets one the first time its thumbnail fails to load.
 */

import type {
  MediaFit,
  MediaItem,
  MediaProgress,
  MediaRef,
  MediaScaling,
} from "@simpleworship/protocol";
import { scalingFit } from "@simpleworship/renderer";
import { listen } from "@tauri-apps/api/event";
import * as api from "./api.ts";
import { engine } from "./engine.svelte.ts";
import { link } from "./link.svelte.ts";
import { audioDetails, imageDetails, videoDetails } from "./thumbnails.ts";

/** Must match `MEDIA_EVENT` and `MEDIA_PROGRESS_EVENT` in `src-tauri/src/media.rs`. */
const MEDIA_EVENT = "media://changed";
const MEDIA_PROGRESS_EVENT = "media://progress";

class MediaStore {
  /** Every media item, newest first. */
  list: MediaItem[] = $state.raw([]);
  /** The file being copied in, if an import is running. */
  progress: MediaProgress | null = $state(null);
  /** Bumped when thumbnails change, so `<img>` tags reload them. */
  thumbnailVersion = $state(0);
  /**
   * Each file's own fit (its Scaling in the Media tab), by id, for files not
   * on automatic. The theme designer and slide editor draw backgrounds with
   * it (see `setFileFitLookup` in `App.svelte`); the engine does the same
   * for the screens.
   */
  fileFits: Map<string, MediaFit> = $derived(
    new Map(
      this.list.flatMap((item) => {
        const fit = scalingFit(item.scaling);
        return fit ? [[item.id, fit] as const] : [];
      }),
    ),
  );

  /** Items whose thumbnail is being made, so it is only made once. */
  #preparing = new Set<string>();
  #urls: { file(media: MediaRef): string } | null = null;

  /** Starts following changes. `urls` builds file URLs (from `App.svelte`). */
  async connect(urls: { file(media: MediaRef): string }): Promise<() => void> {
    this.#urls = urls;
    const stopChanges = await listen(MEDIA_EVENT, () => void this.refresh());
    const stopProgress = await listen<MediaProgress>(MEDIA_PROGRESS_EVENT, ({ payload }) => {
      this.progress = payload.copied < payload.total ? payload : null;
    });
    await this.refresh();
    if (!link.joined) await this.#moveOldFits();
    return () => {
      stopChanges();
      stopProgress();
    };
  }

  /**
   * SimpleWorship 0.5.x kept each file's Scaling in this window's browser
   * storage, where the screens and other computers could not see it. Moves
   * any found there into the library. A choice that could not be saved
   * stays in storage and is tried again the next time the window opens;
   * choices for files no longer in the library are dropped.
   */
  async #moveOldFits(): Promise<void> {
    const key = "simpleworship.media_fit";
    let saved: Record<string, string>;
    try {
      saved = JSON.parse(localStorage.getItem(key) ?? "null") ?? {};
    } catch {
      return;
    }
    const known = new Set(this.list.map((item) => item.id));
    const valid = new Set<string>(["stretch", "maintain", "zoom"]);
    const moving = Object.entries(saved).filter(
      ([id, scaling]) => known.has(id) && valid.has(scaling),
    );
    // Each file is saved on its own, so they can all be saved at once.
    const results = await Promise.allSettled(
      moving.map(([id, scaling]) => api.setMediaScaling(id, scaling as MediaScaling)),
    );
    const failed = moving.filter((_, index) => results[index]?.status === "rejected");
    try {
      if (failed.length === 0) {
        localStorage.removeItem(key);
      } else {
        localStorage.setItem(key, JSON.stringify(Object.fromEntries(failed)));
      }
    } catch {
      // Storage can be unavailable; then the old choices stay where they are.
    }
  }

  async refresh(): Promise<void> {
    this.list = await api.listMedia();
  }

  /** Copies files into the library and prepares their thumbnails. */
  async import(paths: string[]): Promise<void> {
    try {
      const items = await api.importMedia(paths);
      await Promise.all(items.map((item) => this.prepare(item)));
    } catch (error) {
      engine.showError(error);
    } finally {
      this.progress = null;
    }
  }

  /**
   * Makes a thumbnail and measures the item. Safe to call repeatedly; each
   * item is prepared once per session. Problems (a video the webview cannot
   * play) are shown to the operator.
   */
  async prepare(item: MediaItem): Promise<void> {
    // A joined computer shows the main computer's media, which makes its
    // own thumbnails.
    if (link.joined) return;
    if (!this.#urls || this.#preparing.has(item.id)) return;
    this.#preparing.add(item.id);

    try {
      const url = this.#urls.file({ id: item.id, file: item.file });
      if (item.kind === "audio") {
        // Sound has no picture; only its length is worth knowing.
        const { durationMs } = await audioDetails(url);
        await api.updateMediaDetails(item.id, { width: null, height: null, durationMs });
        return;
      }
      const details = item.kind === "video" ? await videoDetails(url) : await imageDetails(url);
      await api.saveMediaThumbnail(item.id, details.jpeg);
      await api.updateMediaDetails(item.id, {
        width: details.width,
        height: details.height,
        durationMs: details.durationMs,
      });
      this.thumbnailVersion += 1;
    } catch (error) {
      engine.showError(`${item.name}: ${error instanceof Error ? error.message : error}`);
    }
  }
}

export const media = new MediaStore();
