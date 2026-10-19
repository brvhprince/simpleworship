/**
 * The media library as seen by the operator window: the list of images and
 * videos, import progress, and thumbnails.
 *
 * New media gets its thumbnail (and, for videos, its size and length) from
 * the webview right after import. Media that arrived another way (inside a
 * schedule file) gets one the first time its thumbnail fails to load.
 */

import type { MediaItem, MediaProgress, MediaRef } from "@simpleworship/protocol";
import { listen } from "@tauri-apps/api/event";
import * as api from "./api.ts";
import { engine } from "./engine.svelte.ts";
import { imageDetails, videoDetails } from "./thumbnails.ts";

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
    return () => {
      stopChanges();
      stopProgress();
    };
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
    if (!this.#urls || this.#preparing.has(item.id)) return;
    this.#preparing.add(item.id);

    try {
      const url = this.#urls.file({ id: item.id, file: item.file });
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
