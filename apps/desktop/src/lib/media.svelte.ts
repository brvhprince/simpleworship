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
import {
  audioDetails,
  imageDetails,
  isDarkThumbnail,
  TimedOut,
  videoDetails,
} from "./thumbnails.ts";

/** Must match `MEDIA_EVENT` and `MEDIA_PROGRESS_EVENT` in `src-tauri/src/media.rs`. */
const MEDIA_EVENT = "media://changed";
const MEDIA_PROGRESS_EVENT = "media://progress";

/** How the store finds a file and its thumbnail (see `setMediaUrls`). */
type MediaFileUrls = {
  file(media: MediaRef): string;
  thumbnail(media: MediaRef): string;
};

/**
 * How many files are prepared (thumbnail and measurements) at once. WebKit
 * decodes only a few videos together; importing twenty at once left most
 * of them waiting for ever, without thumbnails.
 */
const AT_ONCE = 2;
/** How many times a file is tried in one session before giving up. */
const MAX_TRIES = 2;

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

  /** Items waiting for or having their thumbnail made, and when it is done. */
  #queued = new Map<string, Promise<boolean>>();
  /** Items done this session, so they are not done again. */
  #prepared = new Set<string>();
  /** What is waiting its turn (see {@link AT_ONCE}). */
  #waiting: { item: MediaItem; done: (worked: boolean) => void }[] = [];
  /** How many are being prepared right now. */
  #running = 0;
  /** How many times each item was tried this session (see {@link MAX_TRIES}). */
  #tries = new Map<string, number>();
  #urls: MediaFileUrls | null = null;

  /** Starts following changes. `urls` builds file URLs (from `App.svelte`). */
  async connect(urls: MediaFileUrls): Promise<() => void> {
    this.#urls = urls;
    const stopChanges = await listen(MEDIA_EVENT, () => void this.refresh());
    const stopProgress = await listen<MediaProgress>(MEDIA_PROGRESS_EVENT, ({ payload }) => {
      this.progress = payload.copied < payload.total ? payload : null;
    });
    await this.refresh();
    if (!link.joined) {
      await this.#moveOldFits();
      void this.#repairDarkThumbnails();
    }
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
   * Makes a thumbnail and measures the item, in a queue: at most
   * {@link AT_ONCE} at a time, as WebKit decodes only a few videos together
   * and leaves the rest waiting. Resolves to whether it worked. Safe to call
   * repeatedly (a thumbnail that fails to load asks again): an item done
   * this session is not done again, and one that failed is tried at most
   * {@link MAX_TRIES} times.
   */
  prepare(item: MediaItem): Promise<boolean> {
    // A joined computer shows the main computer's media, which makes its
    // own thumbnails.
    if (link.joined || !this.#urls) return Promise.resolve(false);
    const queued = this.#queued.get(item.id);
    if (queued) return queued;
    if (this.#prepared.has(item.id)) return Promise.resolve(true);
    if ((this.#tries.get(item.id) ?? 0) >= MAX_TRIES) return Promise.resolve(false);

    const done = new Promise<boolean>((resolve) => this.#waiting.push({ item, done: resolve }));
    this.#queued.set(item.id, done);
    this.#next();
    return done;
  }

  /**
   * Makes the item's thumbnail again, e.g. one that came out black before
   * thumbnails waited for the frame. Resolves to whether it worked.
   */
  remake(item: MediaItem): Promise<boolean> {
    if (!this.#queued.has(item.id)) {
      this.#tries.delete(item.id);
      this.#prepared.delete(item.id);
    }
    return this.prepare(item);
  }

  /**
   * Once on each computer: makes again the video thumbnails that came out
   * black before thumbnails waited for the frame (0.5.x). Runs in the
   * background, through the same queue. Marked done only when every one was
   * remade, so an interrupted or partly failed pass runs again next time.
   */
  async #repairDarkThumbnails(): Promise<void> {
    const key = "simpleworship.thumbnails_repaired";
    try {
      if (localStorage.getItem(key)) return;
    } catch {
      // Unreadable storage: check anyway; it just cannot be remembered.
    }
    const urls = this.#urls;
    if (!urls) return;
    const remakes: Promise<boolean>[] = [];
    for (const item of this.list.filter((item) => item.kind === "video")) {
      if (await isDarkThumbnail(urls.thumbnail({ id: item.id, file: item.file }))) {
        remakes.push(this.remake(item));
      }
    }
    const results = await Promise.all(remakes);
    if (!results.every(Boolean)) return;
    try {
      localStorage.setItem(key, "1");
    } catch {
      // Storage can be unavailable; then the check runs again next time.
    }
  }

  /**
   * Starts the next waiting items, while fewer than {@link AT_ONCE} run. An
   * item that timed out goes to the back of the queue for its next try, and
   * its caller hears only the final outcome.
   */
  #next(): void {
    while (this.#running < AT_ONCE) {
      const next = this.#waiting.shift();
      if (!next) return;
      this.#running += 1;
      void this.#prepareNow(next.item).then((outcome) => {
        this.#running -= 1;
        if (outcome === "retry") {
          this.#waiting.push(next);
        } else {
          this.#queued.delete(next.item.id);
          next.done(outcome === "done");
        }
        this.#next();
      });
    }
  }

  /**
   * Prepares one item: `"done"`, `"retry"` (it took too long, and has tries
   * left), or `"failed"`. A file the webview cannot play, or one that timed
   * out on its last try, is reported to the operator.
   */
  async #prepareNow(item: MediaItem): Promise<"done" | "retry" | "failed"> {
    const tries = (this.#tries.get(item.id) ?? 0) + 1;
    this.#tries.set(item.id, tries);
    try {
      const url = this.#urls?.file({ id: item.id, file: item.file });
      if (!url) return "failed";
      if (item.kind === "audio") {
        // Sound has no picture; only its length is worth knowing.
        const { durationMs } = await audioDetails(url);
        await api.updateMediaDetails(item.id, { width: null, height: null, durationMs });
      } else {
        const details = item.kind === "video" ? await videoDetails(url) : await imageDetails(url);
        await api.saveMediaThumbnail(item.id, details.jpeg);
        await api.updateMediaDetails(item.id, {
          width: details.width,
          height: details.height,
          durationMs: details.durationMs,
        });
        this.thumbnailVersion += 1;
      }
      this.#prepared.add(item.id);
      return "done";
    } catch (error) {
      if (error instanceof TimedOut && tries < MAX_TRIES) return "retry";
      engine.showError(`${item.name}: ${error instanceof Error ? error.message : error}`);
      return "failed";
    }
  }
}

export const media = new MediaStore();
