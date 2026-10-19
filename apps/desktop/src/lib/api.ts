/**
 * Typed wrappers around the desktop app's IPC commands (other than the
 * engine itself, see `engine.svelte.ts`). Each function matches a
 * `#[tauri::command]` in `src-tauri/src/commands/`.
 */

import type {
  AutosaveInfo,
  BibleInfo,
  BookOutline,
  MonitorInfo,
  OutputKind,
  OutputTarget,
  Passage,
  Presentation,
  RecentSchedule,
  ReferenceFeedback,
  ScheduleFileStatus,
  ScriptureOptions,
  Song,
  SongSummary,
  Verse,
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

// ── Schedule ─────────────────────────────────────────────────────────────
// Moving, removing and previewing items are engine commands (see
// `engine.dispatch`); these need the library or the disk.

/** Adds a library song to the schedule at `index`, or at the end. */
export const addSongToSchedule = (id: string, index: number | null = null) =>
  invoke<void>("schedule_add_song", { id, index });

/** Adds free text to the schedule at `index`, or at the end. */
export const addTextToSchedule = (title: string, text: string, index: number | null = null) =>
  invoke<void>("schedule_add_text", { title, text, index });

/** Ids of songs in the schedule that are not in this computer's library. */
export const missingScheduleSongs = () => invoke<string[]>("schedule_missing_songs");

/** Adds the schedule's missing songs to the library; resolves to how many. */
export const addMissingScheduleSongs = () => invoke<number>("schedule_add_missing_songs");

/** The current schedule file name and unsaved-changes flag. */
export const scheduleFileStatus = () => invoke<ScheduleFileStatus>("schedule_file_status");

/** Starts a new, empty schedule (without asking about unsaved changes). */
export const newSchedule = () => invoke<void>("schedule_new");

/** Opens a `.sws` file (without asking about unsaved changes). */
export const openSchedule = (path: string) => invoke<void>("schedule_open", { path });

/** Saves to `path` ("Save As"), or to the current file when `path` is null. */
export const saveSchedule = (path: string | null) => invoke<void>("schedule_save", { path });

/** Recently used schedule files, most recent first. */
export const recentSchedules = () => invoke<RecentSchedule[]>("schedule_recent");

/** The schedule left behind by a run that did not close normally, if any. */
export const autosaveLeftover = () => invoke<AutosaveInfo | null>("autosave_leftover");

/** Brings back the autosaved schedule. */
export const restoreAutosave = () => invoke<void>("autosave_restore");

/** Throws the autosaved schedule away. */
export const discardAutosave = () => invoke<void>("autosave_discard");

// ── Scripture ────────────────────────────────────────────────────────────

/** Every installed Bible. */
export const listBibles = () => invoke<BibleInfo[]>("bibles_list");

/** Imports a Bible file; resolves when it is ready (this takes a few seconds). */
export const importBible = (path: string) => invoke<BibleInfo>("bible_import", { path });

/** Renames a Bible. */
export const renameBible = (id: string, name: string, abbreviation: string) =>
  invoke<void>("bible_rename", { id, name, abbreviation });

/** Deletes a Bible and its verses. */
export const deleteBible = (id: string) => invoke<void>("bible_delete", { id });

/** Which books and chapters a Bible has. */
export const bibleOutline = (id: string) => invoke<BookOutline[]>("bible_outline", { id });

/** Every verse of one chapter. */
export const bibleChapter = (id: string, book: number, chapter: number) =>
  invoke<Verse[]>("bible_chapter", { id, book, chapter });

/** Verses containing the given words, best matches first. */
export const searchBible = (id: string, query: string) =>
  invoke<Verse[]>("bible_search", { id, query });

/** Parses a reference as it is typed ("jn 3:16" → John 3:16). */
export const parseReference = (text: string) =>
  invoke<ReferenceFeedback>("scripture_parse", { text });

/** Loads a passage into the preview pane, in the given translations. */
export const showScriptureInPreview = (
  passage: Passage,
  bibleIds: string[],
  options: ScriptureOptions,
) => invoke<void>("scripture_show_in_preview", { passage, bibleIds, options });

/** Adds a passage to the schedule at `index`, or at the end. */
export const addScriptureToSchedule = (
  passage: Passage,
  bibleIds: string[],
  options: ScriptureOptions,
  index: number | null = null,
) => invoke<void>("schedule_add_scripture", { passage, bibleIds, options, index });

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
