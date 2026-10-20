<!--
  @component
  The song library: search, preview, create, edit and delete songs.

  - Type to search titles, authors and lyrics.
  - ↑/↓ move through the results; Enter or double-click sends the song to
    the preview pane.
  - Drag a song onto the schedule, or use “Add to Schedule”.
  - With the assistant on, Ask AI finds songs from a description ("the
    song about the blood that never loses its power"): the assistant names
    songs, and they are looked up in this library.
-->
<script lang="ts">
  import type { Song, SongSummary } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import {
    addSongToSchedule,
    deleteSong,
    getSong,
    newSong,
    searchSongs,
    showSongInPreview,
  } from "../../lib/api.ts";
  import { assistant } from "../../lib/assistant.svelte.ts";
  import { SONG_DRAG_TYPE } from "../../lib/drag.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { importer } from "../../lib/importer.svelte.ts";
  import { library } from "../../lib/library.svelte.ts";
  import { link } from "../../lib/link.svelte.ts";
  import SongEditor from "./SongEditor.svelte";

  /** Wait this long after the last keystroke before searching, in milliseconds. */
  const SEARCH_DELAY_MS = 120;

  let query = $state("");
  let results: SongSummary[] = $state([]);
  let selectedId: string | null = $state(null);
  /** The song open in the editor, if any. */
  let editing: Song | null = $state(null);
  let confirmingDelete = $state(false);
  /** The results are the assistant's suggestions for this description. */
  let suggested: { about: string; missing: string[] } | null = $state(null);

  const selected = $derived(results.find((song) => song.id === selectedId) ?? null);

  // Search as the operator types, and again when the library changes elsewhere.
  $effect(() => {
    const currentQuery = query;
    void library.version;
    const timer = setTimeout(() => refresh(currentQuery), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  });

  async function refresh(currentQuery = query) {
    try {
      results = await searchSongs(currentQuery);
      suggested = null;
      if (!results.some((song) => song.id === selectedId)) {
        selectedId = results[0]?.id ?? null;
      }
    } catch (error) {
      engine.showError(error);
    }
  }

  /** Letters only, for comparing titles loosely. */
  function loose(text: string): string {
    return text.toLowerCase().replace(/[^\p{L}]/gu, "");
  }

  /** Ask AI: songs that fit the description, from this library. */
  async function askAssistant() {
    const about = query.trim();
    if (!about) return;
    try {
      const ideas = await assistant.findSongs(about);
      const found: SongSummary[] = [];
      const missing: string[] = [];
      for (const idea of ideas) {
        // By title first; then by a line of its lyrics.
        const byTitle = (await searchSongs(idea.title)).find(
          (song) =>
            loose(song.title).includes(loose(idea.title)) ||
            loose(idea.title).includes(loose(song.title)),
        );
        const song = byTitle ?? (await searchSongs(idea.line))[0];
        if (!song) missing.push(idea.title);
        else if (!found.some((other) => other.id === song.id)) found.push(song);
      }
      // Still the same question (the operator may have typed on).
      if (query.trim() !== about) return;
      results = found;
      selectedId = found[0]?.id ?? null;
      suggested = { about, missing };
    } catch (error) {
      engine.showError(error);
    }
  }

  async function preview(id: string | null = selectedId) {
    if (!id) return;
    try {
      await showSongInPreview(id);
    } catch (error) {
      engine.showError(error);
    }
  }

  async function addToSchedule(id: string | null = selectedId) {
    if (!id) return;
    try {
      await addSongToSchedule(id);
    } catch (error) {
      engine.showError(error);
    }
  }

  function startDrag(event: DragEvent, id: string) {
    event.dataTransfer?.setData(SONG_DRAG_TYPE, id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "copy";
  }

  async function edit(id: string | null = selectedId) {
    if (!id) return;
    try {
      editing = await getSong(id);
    } catch (error) {
      engine.showError(error);
    }
  }

  async function remove() {
    if (!selectedId) return;
    confirmingDelete = false;
    try {
      await deleteSong(selectedId);
      await refresh();
    } catch (error) {
      engine.showError(error);
    }
  }

  async function handleSaved(song: Song) {
    editing = null;
    selectedId = song.id;
    await refresh();
  }

  /** The DOM id of a song's row, for aria-activedescendant. */
  function optionId(songId: string): string {
    return `song-${songId}`;
  }

  /** Moves the selection by `step` rows, staying inside the list. */
  function moveSelection(step: number) {
    const index = results.findIndex((song) => song.id === selectedId);
    const next = results[Math.min(Math.max(index + step, 0), results.length - 1)];
    if (next) selectedId = next.id;
  }

  /** List navigation for both the search box and the list itself. Handled
   *  here (and marked as handled) so the arrows don't change the live slide. */
  function handleListKeys(event: KeyboardEvent) {
    switch (event.key) {
      case "ArrowDown":
        moveSelection(1);
        break;
      case "ArrowUp":
        moveSelection(-1);
        break;
      case "Enter":
        preview();
        break;
      default:
        return;
    }
    event.preventDefault();
  }
</script>

<div class="songs-tab">
  <div class="toolbar">
    <input
      type="search"
      bind:value={query}
      onkeydown={handleListKeys}
      placeholder={t("songs.search")}
      aria-label={t("songs.searchLabel")}
    >
    {#if assistant.ready}
      <Button
        disabled={!query.trim() || assistant.busy}
        title={t("assistant.findSongsHint")}
        onclick={askAssistant}
      >
        {assistant.busy ? t("assistant.thinking") : t("assistant.ask")}
      </Button>
    {/if}
    <Button onclick={() => (editing = newSong())}>{t("songs.new")}</Button>
    {#if !link.joined}
      <Button onclick={() => importer.open()}>{t("common.import")}</Button>
    {/if}
    <Button disabled={!selected} onclick={() => edit()}>{t("common.edit")}</Button>
    {#if link.joined}
      <!-- Songs are deleted on the main computer. -->
    {:else if confirmingDelete}
      <span class="confirm">{t("songs.confirmDelete", { title: selected?.title ?? "" })}</span>
      <Button variant="live" active onclick={remove}>{t("common.delete")}</Button>
      <Button onclick={() => (confirmingDelete = false)}>{t("songs.keep")}</Button>
    {:else}
      <Button disabled={!selected} onclick={() => (confirmingDelete = true)}
        >{t("common.delete")}</Button
      >
    {/if}
    <span class="spacer"></span>
    <Button disabled={!selected} onclick={() => addToSchedule()}
      >{t("common.addToSchedule")}</Button
    >
    <Button variant="primary" disabled={!selected} onclick={() => preview()}>
      {t("common.showInPreview")}
    </Button>
  </div>

  <!--
    A listbox in the "active descendant" pattern: the list itself has focus and
    handles the arrow keys; aria-activedescendant tells screen readers which
    option is current.
  -->
  {#if suggested}
    <p class="suggested">
      {t("assistant.songsFor", { about: suggested.about })}
      {#if suggested.missing.length > 0}
        {t("assistant.notInLibrary", { titles: suggested.missing.join(", ") })}
      {/if}
    </p>
  {/if}

  <div
    class="results"
    role="listbox"
    aria-label={t("songs.list")}
    tabindex="0"
    aria-activedescendant={selectedId ? optionId(selectedId) : undefined}
    onkeydown={handleListKeys}
  >
    {#each results as song (song.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events: keys are handled by the listbox (handleListKeys) -->
      <!-- biome-ignore lint/a11y/useKeyWithClickEvents: keys are handled by the listbox (handleListKeys) -->
      <div
        id={optionId(song.id)}
        role="option"
        tabindex="-1"
        aria-selected={song.id === selectedId}
        draggable="true"
        ondragstart={(event) => startDrag(event, song.id)}
        onclick={() => (selectedId = song.id)}
        ondblclick={() => preview(song.id)}
      >
        <span class="title">{song.title}</span>
        <span class="author">{song.author}</span>
        <span class="excerpt">{song.excerpt}</span>
      </div>
    {:else}
      <p class="empty">
        {query ? t("songs.noMatch") : t("songs.empty")}
      </p>
    {/each}
  </div>
</div>

{#if editing}
  <SongEditor song={editing} onsave={handleSaved} oncancel={() => (editing = null)} />
{/if}

<style>
  .songs-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
  }

  input[type="search"] {
    width: 340px;
  }

  .spacer {
    flex: 1;
  }

  .suggested {
    margin: 0;
    padding: 0 var(--sw-space-3) var(--sw-space-2);
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
  }

  .confirm {
    color: var(--sw-warning);
  }

  .results {
    overflow-y: auto;
    flex: 1;
    border-top: 1px solid var(--sw-border);
  }

  .results:focus-visible {
    outline: 2px solid var(--sw-accent);
    outline-offset: -2px;
  }

  [role="option"] {
    display: grid;
    grid-template-columns: minmax(180px, 2fr) minmax(120px, 1fr) 3fr;
    gap: var(--sw-space-3);
    padding: var(--sw-space-1) var(--sw-space-3);
    cursor: default;
    user-select: none;
  }

  [role="option"]:nth-child(even) {
    background: rgba(255, 255, 255, 0.02);
  }

  [role="option"][aria-selected="true"] {
    background: var(--sw-accent);
  }

  .title {
    font-weight: 600;
  }

  .author,
  .excerpt {
    color: var(--sw-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  [aria-selected="true"] .author,
  [aria-selected="true"] .excerpt {
    color: inherit;
  }

  .empty {
    margin: 0;
    padding: var(--sw-space-4);
    color: var(--sw-text-faint);
    text-align: center;
  }
</style>
