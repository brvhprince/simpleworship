/**
 * Typed wrappers around the desktop app's IPC commands (other than the
 * engine itself, see `engine.svelte.ts`). Each function matches a
 * `#[tauri::command]` in `src-tauri/src/commands/`.
 */

import type {
  AppSettings,
  AutosaveInfo,
  BibleCatalog,
  BibleChoice,
  BibleInfo,
  BookOutline,
  CameraRef,
  CountdownSlide,
  DeckSlide,
  FoundSource,
  ImportedSong,
  ImportPreview,
  ImportSummary,
  MediaChoice,
  MediaFolderPaths,
  MediaItem,
  MediaScaling,
  MonitorInfo,
  OutputKind,
  OutputTarget,
  Passage,
  PreparedPdf,
  Presentation,
  PreviewMedia,
  RecentSchedule,
  ReferenceFeedback,
  RenderFrame,
  ScheduleFileStatus,
  ScriptureOptions,
  SlideDeck,
  Song,
  SongSummary,
  SongUsage,
  StreamSettings,
  ThemeDefaults,
  ThemeRecord,
  TransitionSettings,
  TranslationText,
  Verse,
} from "@simpleworship/protocol";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { link } from "./link.svelte.ts";

// ── Songs ────────────────────────────────────────────────────────────────

/** Searches titles, authors and lyrics. An empty query lists every song. */

/**
 * Commands that, on a computer that joined another, run on that computer
 * instead (its library, its schedule). Must match `host_calls.rs`.
 */
const ON_HOST = new Set([
  "songs_search",
  "song_get",
  "song_save",
  "song_presentation",
  "schedule_item_thumbnail",
  "song_show_in_preview",
  "schedule_add_song",
  "bibles_list",
  "bible_outline",
  "bible_chapter",
  "bible_search",
  "scripture_parse",
  "scripture_text",
  "scripture_show_in_preview",
  "schedule_add_scripture",
  "text_show_in_preview",
  "schedule_add_text",
  "schedule_add_countdown",
  "media_list",
  "media_show_in_preview",
  "schedule_add_media",
  "schedule_add_slideshow",
  "schedule_add_playlist",
  "schedule_append_media",
  "logo_set",
  "presentations_list",
  "presentation_get",
  "presentation_show_in_preview",
  "schedule_add_presentation",
  "schedule_append_presentation",
  "deck_presentation",
  "theme_save",
  "theme_delete",
  "theme_set_defaults",
  "fonts_list",
  "settings_get",
  "schedule_file_status",
  "transitions_save",
  "stream_save",
]);

/** Runs `command` here, or on the computer this one joined (see {@link ON_HOST}). */
function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (link.joined && ON_HOST.has(command)) {
    return tauriInvoke<T>("link_call", { method: command, params: args ?? {} });
  }
  return tauriInvoke<T>(command, args);
}

export const searchSongs = (query: string) => invoke<SongSummary[]>("songs_search", { query });

/** Loads a song for editing. */
export const getSong = (id: string) => invoke<Song>("song_get", { id });

/** Creates or updates a song; resolves to the saved copy. */
export const saveSong = (song: Song) => invoke<Song>("song_save", { song });

/** Deletes a song. */
export const deleteSong = (id: string) => invoke<boolean>("song_delete", { id });

/** Splits a (possibly unsaved) song into slides, for the editor's preview. */
export const songPresentation = (song: Song) => invoke<Presentation>("song_presentation", { song });

/**
 * The first slide of a schedule item as the screens would draw it, for the
 * schedule's grid; `null` for background music.
 */
export const scheduleItemThumbnail = (id: string) =>
  invoke<RenderFrame | null>("schedule_item_thumbnail", { id });

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

/** Adds a camera's live picture to the schedule at `index` (or at the end). */
export const addCameraToSchedule = (camera: CameraRef, index: number | null = null) =>
  invoke<void>("schedule_add_camera", { camera, index });

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

/**
 * Saves to `path` ("Save As"), or to the current file when `path` is null.
 * With `includeMedia` (the default) images and videos are embedded.
 */
export const saveSchedule = (path: string | null, includeMedia = true) =>
  invoke<void>("schedule_save", { path, includeMedia });

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

/**
 * The Bibles that can be downloaded from eBible.org (freely shared ones
 * only). Fetches the list again when `refresh` is set or it is a week old;
 * without internet, the copy kept from before (marked `offline`).
 */
export const bibleCatalog = (refresh = false) => invoke<BibleCatalog>("bible_catalog", { refresh });

/** Downloads a Bible from eBible.org's list and adds it to the library. */
export const installCatalogBible = (id: string) =>
  invoke<BibleInfo>("bible_catalog_install", { id });

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

/** A passage's verses in one Bible. */
export const scriptureText = (passage: Passage, bibleId: string) =>
  invoke<TranslationText>("scripture_text", { passage, bibleId });

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

// ── Themes and fonts ─────────────────────────────────────────────────────
// The theme library itself arrives with the engine state (`engine.themes`).

/** Creates or updates a theme. */
export const saveTheme = (theme: ThemeRecord) => invoke<void>("theme_save", { theme });

/** Deletes a theme; content that used it falls back to the default. */
export const deleteTheme = (id: string) => invoke<void>("theme_delete", { id });

/** Chooses the default theme for songs, scripture and text. */
export const setThemeDefaults = (defaults: ThemeDefaults) =>
  invoke<void>("theme_set_defaults", { defaults });

/** Font families: the bundled ones first, then the installed ones. */
export const listFonts = () => invoke<string[]>("fonts_list");

/** Sets (or, with `null`, removes) the church logo. */
export const setLogo = (mediaId: string | null) => invoke<void>("logo_set", { mediaId });

// ── Media ────────────────────────────────────────────────────────────────

/** Where media files and thumbnails live. */
export const mediaFolders = () => invoke<MediaFolderPaths>("media_folders");

/** Every media item, newest first. */
export const listMedia = () => invoke<MediaItem[]>("media_list");

/** Copies files into the media library (in the background, with progress). */
export const importMedia = (paths: string[]) => invoke<MediaItem[]>("media_import", { paths });

/** Records a video's size and length, measured by the webview. */
export const updateMediaDetails = (
  id: string,
  details: { width: number | null; height: number | null; durationMs: number | null },
) => invoke<void>("media_update_details", { id, ...details });

/** Stores a media item's thumbnail (JPEG bytes). */
export const saveMediaThumbnail = (id: string, jpeg: Uint8Array) =>
  invoke<void>("media_save_thumbnail", { id, jpeg: Array.from(jpeg) });

/** Renames a media item. */
export const renameMedia = (id: string, name: string) => invoke<void>("media_rename", { id, name });

/** Sets how a picture or video fills the screen wherever it is used. */
export const setMediaScaling = (id: string, scaling: MediaScaling) =>
  invoke<void>("media_set_scaling", { id, scaling });

/** Deletes a media item and its file. */
export const deleteMedia = (id: string) => invoke<void>("media_delete", { id });

/** Shows an image or video on its own in the preview pane. */
export const showMediaInPreview = (id: string) => invoke<void>("media_show_in_preview", { id });

/** Adds an image or video to the schedule at `index`, or at the end. */
export const addMediaToSchedule = (id: string, index: number | null = null) =>
  invoke<void>("schedule_add_media", { id, index });

// ── Importing from other software ────────────────────────────────────────

/** Finds importable libraries in `folder`, or in the usual places. */
export const scanForImports = (folder: string | null = null) =>
  invoke<FoundSource[]>("import_scan", { folder });

/** On the very first launch, what the usual places contain; later, nothing. */
export const firstLaunchImports = () => invoke<FoundSource[]>("import_first_launch");

/** Reads sources and reports their songs (marking duplicates) and Bibles. */
export const previewImport = (sources: FoundSource[]) =>
  invoke<ImportPreview>("import_preview", { sources });

/** Saves the chosen songs, imports the chosen Bibles and copies the chosen media. */
export const commitImport = (
  songs: ImportedSong[],
  bibles: BibleChoice[],
  media: MediaChoice[] = [],
) => invoke<ImportSummary>("import_commit", { songs, bibles, media });

/** Looks for media files that were not found, in `folder` and below. */
export const locateImportMedia = (media: PreviewMedia[], folder: string) =>
  invoke<PreviewMedia[]>("import_locate_media", { media, folder });

// ── Countdowns and background music ──────────────────────────────────────

/** Adds a countdown to the schedule at `index`, or at the end. */
export const addCountdownToSchedule = (
  title: string,
  countdown: CountdownSlide,
  index: number | null = null,
) => invoke<void>("schedule_add_countdown", { title, countdown, index });

/** Adds background music (audio items, in order) to the schedule. */
export const addPlaylistToSchedule = (
  name: string,
  trackIds: string[],
  index: number | null = null,
) => invoke<void>("schedule_add_playlist", { name, trackIds, index });

// ── Presentations ────────────────────────────────────────────────────────

/** Every presentation, most recently changed first. */
export const listPresentations = () => invoke<SlideDeck[]>("presentations_list");

/** Whether PowerPoint/Keynote files can be imported (LibreOffice found). */
export const canConvertPresentations = () => invoke<boolean>("presentation_can_convert");

/** Puts a presentation, as PDF, where the webview can read it. */
export const preparePresentation = (path: string) =>
  invoke<PreparedPdf>("presentation_prepare", { path });

/** Starts an empty presentation; resolves to its id. */
export const createPresentation = (name: string) => invoke<string>("presentation_create", { name });

/** Stores one rendered page. The image goes as raw bytes, not JSON. */
export const addPresentationPage = (deck: string, page: number, jpeg: Uint8Array) =>
  tauriInvoke<MediaItem>("presentation_add_page", jpeg, {
    headers: { "x-deck": deck, "x-page": String(page) },
  });

/**
 * Removes the temporary PDF and saves the imported deck, or deletes the
 * deck if the import `failed`.
 */
export const finishPresentation = (file: string, deck: string | null, failed = false) =>
  invoke<void>("presentation_finish", { file, deck, failed });

/** One presentation. */
export const getPresentation = (id: string) => invoke<SlideDeck>("presentation_get", { id });

/** Saves a presentation (and updates its copies in the schedule). */
export const savePresentation = (deck: SlideDeck) =>
  invoke<SlideDeck>("presentation_save", { deck });

/** The slides a deck makes, styled as shown (it need not be saved). */
export const deckPresentation = (deck: SlideDeck) =>
  invoke<Presentation>("deck_presentation", { deck });

/** A presentation ready for the canvas editor (old text slides become text boxes). */
export const deckForEditing = (deck: SlideDeck) => invoke<SlideDeck>("deck_for_editing", { deck });

/** Picture slides for media items, in order (sound is left out). */
export const pictureSlides = (mediaIds: string[]) =>
  invoke<DeckSlide[]>("deck_picture_slides", { mediaIds });

/** Deletes a presentation and its pages. */
export const deletePresentation = (id: string) => invoke<void>("presentation_delete", { id });

/** Shows a presentation in the preview pane. */
export const showPresentationInPreview = (id: string) =>
  invoke<void>("presentation_show_in_preview", { id });

/** Adds a presentation to the schedule at `index`, or at the end. */
export const addPresentationToSchedule = (id: string, index: number | null = null) =>
  invoke<void>("schedule_add_presentation", { id, index });

/** Adds pictures and videos to the schedule as one slideshow item. */
export const addSlideshowToSchedule = (mediaIds: string[], index: number | null = null) =>
  invoke<void>("schedule_add_slideshow", { mediaIds, index });

/** Adds pictures and videos to the end of schedule item `id`. */
export const appendMediaToItem = (id: string, mediaIds: string[]) =>
  invoke<void>("schedule_append_media", { id, mediaIds });

/** Adds a presentation's slides to the end of schedule item `id`. */
export const appendPresentationToItem = (id: string, deckId: string) =>
  invoke<void>("schedule_append_presentation", { id, deckId });

// ── Settings and reports ─────────────────────────────────────────────────

/** The church's settings. */
export const getSettings = () => invoke<AppSettings>("settings_get");

/** Saves the church's settings. */
export const saveSettings = (settings: AppSettings) => invoke<void>("settings_save", { settings });

/** Saves the transition for each kind of content. */
export const saveTransitions = (transitions: TransitionSettings) =>
  invoke<void>("transitions_save", { transitions });

/** Changes and saves how the stream output shows each kind of content. */
export const saveStream = (settings: StreamSettings) => invoke<void>("stream_save", { settings });

/** Songs used between two dates (`YYYY-MM-DD`, inclusive), most used first. */
export const usageReport = (from: string, to: string) =>
  invoke<SongUsage[]>("usage_report", { from, to });

/** Writes the usage report as CSV to `path`. */
export const exportUsageCsv = (from: string, to: string, path: string) =>
  invoke<void>("usage_export_csv", { from, to, path });

// ── The app ──────────────────────────────────────────────────────────────

/** Tells the app the window can open files; resolves to the one it was started with. */
export const launchReady = () => invoke<string | null>("launch_ready");

/** Rebuilds the menu bar with these translated labels (keys like "menu.file"). */
export const setMenuLabels = (labels: Record<string, string>) =>
  invoke<void>("menu_set_labels", { labels });

/** Opens one of the project's web pages in the browser. */
export const openProjectPage = (
  page:
    | "website"
    | "guide"
    | "moving"
    | "report"
    | "license"
    | "changes"
    | "ndi"
    | "anthropic-keys"
    | "openai-keys"
    | "gemini-keys",
) => invoke<void>("open_project_page", { page });

/** Opens the folder with the log files. */
export const openLogsFolder = () => invoke<void>("open_logs_folder");

/**
 * Whether this copy came from the Microsoft Store, which updates it: the
 * app's own update checks are then hidden.
 */
export const updatesFromStore = () => invoke<boolean>("updates_from_store");

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
    themeId: null,
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
