/**
 * Typed wrappers around the desktop app's IPC commands (other than the
 * engine itself, see `engine.svelte.ts`). Each function matches a
 * `#[tauri::command]` in `src-tauri/src/commands/`.
 */

import type {
  MonitorInfo,
  OutputKind,
  OutputTarget,
  Presentation,
  Song,
  SongSummary,
} from "@simpleworship/protocol";
import { invoke } from "@tauri-apps/api/core";

// ── Songs ────────────────────────────────────────────────────────────────

/** Searches titles, authors and lyrics. An empty query lists every song. */
export const searchSongs = (query: string) => invoke<SongSummary[]>("songs_search", { query });

/** Loads a song for editing. */
export const getSong = (id: string) => invoke<Song>("song_get", { id });

/** Creates or updates a song; resolves to the saved copy. */
export const saveSong = (song: Song) => invoke<Song>("song_save", { song });

/** Deletes a song. */
export const deleteSong = (id: string) => invoke<boolean>("song_delete", { id });

/** Splits a (possibly unsaved) song into slides, for the editor's preview. */
export const songPresentation = (song: Song) => invoke<Presentation>("song_presentation", { song });

/** Loads a song into the preview pane. */
export const showSongInPreview = (id: string) => invoke<void>("song_show_in_preview", { id });

// ── Free text ────────────────────────────────────────────────────────────

/** Loads free text into the preview pane; blank lines separate slides. */
export const showTextInPreview = (title: string, text: string) =>
  invoke<void>("text_show_in_preview", { title, text });

// ── Outputs ──────────────────────────────────────────────────────────────

/** Lists the connected monitors. */
export const listMonitors = () => invoke<MonitorInfo[]>("monitors_list");

/** Opens (or moves) the projector or stage window. */
export const openOutput = (kind: OutputKind, target: OutputTarget) =>
  invoke<void>("output_open", { kind, target });

/** Closes the projector or stage window. */
export const closeOutput = (kind: OutputKind) => invoke<void>("output_close", { kind });

/** A new, empty song with a fresh id (ids are UUID v7, like the backend's). */
export function newSong(): Song {
  return {
    id: uuidV7(),
    title: "",
    author: "",
    copyright: "",
    ccliNumber: "",
    lyrics: "",
    arrangement: "",
    createdAt: 0,
    updatedAt: 0,
  };
}

/**
 * Generates a UUID v7: a 48-bit millisecond timestamp followed by random
 * bits, so ids sort by creation time (RFC 9562, section 5.7).
 */
export function uuidV7(now: number = Date.now()): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  for (let i = 0; i < 6; i++) {
    bytes[i] = Math.floor(now / 2 ** (8 * (5 - i))) & 0xff;
  }
  bytes[6] = 0x70 | ((bytes[6] ?? 0) & 0x0f); // version 7
  bytes[8] = 0x80 | ((bytes[8] ?? 0) & 0x3f); // RFC 4122 variant

  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
