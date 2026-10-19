<!--
  @component
  The media library: images and videos for backgrounds, the logo, and
  schedule items, and sound files for background music.

  - Import… copies files into the library (videos can take a moment).
  - Click to select, double-click to preview, drag onto the schedule.
  - ⌘/Ctrl-click or Shift-click to pick several; dragging them onto the
    schedule (or "Add as Slideshow") makes one slideshow item.
  - "Set as Logo" makes the selected image the Logo button's picture.
  - Sound plays in the music bar at the bottom of the window ("Play"), or
    goes in the schedule as background music.
-->
<script lang="ts">
  import type { MediaItem } from "@simpleworship/protocol";
  import { getMediaUrls } from "@simpleworship/renderer";
  import { Button } from "@simpleworship/ui";
  import { open } from "@tauri-apps/plugin-dialog";
  import { SvelteSet } from "svelte/reactivity";
  import {
    addMediaToSchedule,
    addPlaylistToSchedule,
    addSlideshowToSchedule,
    deleteMedia,
    renameMedia,
    setLogo,
    showMediaInPreview,
  } from "../../lib/api.ts";
  import { MEDIA_DRAG_TYPE } from "../../lib/drag.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { formatBytes, formatDuration } from "../../lib/format.ts";
  import { media } from "../../lib/media.svelte.ts";
  import { prompts } from "../../lib/prompts.svelte.ts";

  const MEDIA_FILTERS = [
    {
      name: "Images, videos and sound",
      extensions: [
        ...["png", "jpg", "jpeg", "gif", "webp", "bmp", "avif"],
        ...["mp4", "m4v", "mov", "webm"],
        ...["mp3", "m4a", "aac", "wav", "ogg", "oga", "flac"],
      ],
    },
  ];

  type Filter = "all" | "image" | "video" | "audio";

  const urls = getMediaUrls();
  let filter: Filter = $state("all");
  /** The item last clicked: the one the single-item buttons act on. */
  let selectedId: string | null = $state(null);
  /** Every picked item (more than one with ⌘/Ctrl- or Shift-click). */
  const picked = new SvelteSet<string>();

  const shown = $derived(media.list.filter((item) => filter === "all" || item.kind === filter));
  const selected = $derived(media.list.find((item) => item.id === selectedId) ?? null);
  /** The picked items, in the order they are shown. */
  const pickedItems = $derived(shown.filter((item) => picked.has(item.id)));
  const logoId = $derived(engine.output?.logo?.id ?? null);

  async function chooseAndImport() {
    const paths = await open({ filters: MEDIA_FILTERS, multiple: true });
    if (paths && paths.length > 0) await media.import(paths);
  }

  /** Runs an action on the selected item, showing any error. */
  async function onSelected(action: (item: MediaItem) => Promise<unknown>) {
    if (!selected) return;
    try {
      await action(selected);
    } catch (error) {
      engine.showError(error);
    }
  }

  async function remove(item: MediaItem) {
    const answer = await prompts.ask({
      title: "Delete media?",
      message: `Delete “${item.name}”? Themes and schedule items that use it will show nothing in its place.`,
      buttons: [
        { id: "cancel", label: "Cancel" },
        { id: "delete", label: "Delete", variant: "live" },
      ],
      cancel: "cancel",
    });
    if (answer === "delete") await deleteMedia(item.id);
  }

  function thumbnailUrl(item: MediaItem): string {
    return `${urls.thumbnail({ id: item.id, file: item.file })}?v=${media.thumbnailVersion}`;
  }

  /** Plays sound in the music bar: the chosen file, then the rest of the sound shown. */
  function play(item: MediaItem) {
    const sounds = shown.filter((other) => other.kind === "audio");
    const start = Math.max(0, sounds.indexOf(item));
    const tracks = [...sounds.slice(start), ...sounds.slice(0, start)];
    engine.dispatch({ type: "setPlaylist", tracks, play: true });
  }

  /** Double-click: preview a picture or video, play a sound. */
  function activate(item: MediaItem) {
    if (item.kind === "audio") play(item);
    else showMediaInPreview(item.id).catch((error) => engine.showError(error));
  }

  function kindLabel(item: MediaItem): string {
    return { image: "Image", video: "Video", audio: "Sound" }[item.kind];
  }

  /** Click: pick one. ⌘/Ctrl-click: add or remove one. Shift-click: a range. */
  function pick(event: MouseEvent, item: MediaItem) {
    if (event.metaKey || event.ctrlKey) {
      if (picked.has(item.id)) picked.delete(item.id);
      else picked.add(item.id);
    } else if (event.shiftKey && selectedId) {
      const from = shown.findIndex((other) => other.id === selectedId);
      const to = shown.findIndex((other) => other.id === item.id);
      const [start, end] = from < to ? [from, to] : [to, from];
      for (const other of shown.slice(start, end + 1)) picked.add(other.id);
    } else {
      picked.clear();
      picked.add(item.id);
    }
    selectedId = item.id;
  }

  /** The picked items as one schedule item: music for sound, else a slideshow. */
  async function addPicked() {
    const ids = pickedItems.map((item) => item.id);
    try {
      if (pickedItems.every((item) => item.kind === "audio")) {
        await addPlaylistToSchedule(`Music (${ids.length} tracks)`, ids);
      } else {
        await addSlideshowToSchedule(ids);
      }
    } catch (error) {
      engine.showError(error);
    }
  }

  async function removePicked() {
    const answer = await prompts.ask({
      title: "Delete media?",
      message: `Delete ${pickedItems.length} items? Themes and schedule items that use them will show nothing in their place.`,
      buttons: [
        { id: "cancel", label: "Cancel" },
        { id: "delete", label: "Delete", variant: "live" },
      ],
      cancel: "cancel",
    });
    if (answer !== "delete") return;
    for (const item of pickedItems) await deleteMedia(item.id);
    picked.clear();
  }

  function startDrag(event: DragEvent, item: MediaItem) {
    // Dragging one of the picked items drags them all.
    const dragged = picked.has(item.id) ? pickedItems : [item];
    event.dataTransfer?.setData(MEDIA_DRAG_TYPE, dragged.map((one) => one.id).join("\n"));
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "copy";
  }
</script>

<div class="media-tab">
  <div class="toolbar">
    <div class="filters" role="radiogroup" aria-label="Show">
      {#each [
        ["all", "All"],
        ["image", "Images"],
        ["video", "Videos"],
        ["audio", "Sound"],
      ] as [value, label] (value)}
        <label>
          <input type="radio" bind:group={filter} {value}>
          {label}
        </label>
      {/each}
    </div>

    <Button onclick={chooseAndImport} disabled={media.progress !== null}>Import…</Button>
    {#if media.progress}
      <span class="progress">
        Copying {media.progress.name}…
        {Math.round((media.progress.copied / media.progress.total) * 100)}%
      </span>
    {/if}

    <span class="spacer"></span>

    {#if pickedItems.length > 1}
      <span class="progress">{pickedItems.length} selected</span>
      <Button onclick={removePicked}>Delete</Button>
      <Button variant="primary" onclick={addPicked}>
        {pickedItems.every((item) => item.kind === "audio") ? "Add as Music" : "Add as Slideshow"}
      </Button>
    {:else}
      {#if selected}
        <input
          class="name"
          value={selected.name}
          aria-label="Name"
          onchange={(e) => onSelected((item) => renameMedia(item.id, e.currentTarget.value))}
        >
      {/if}
      {#if selected?.kind === "image"}
        {#if selected.id === logoId}
          <Button onclick={() => onSelected(() => setLogo(null))}>Remove Logo</Button>
        {:else}
          <Button onclick={() => onSelected((item) => setLogo(item.id))}>Set as Logo</Button>
        {/if}
      {/if}
      <Button disabled={!selected} onclick={() => onSelected(remove)}>Delete</Button>
      <Button
        disabled={!selected}
        onclick={() => onSelected((item) => addMediaToSchedule(item.id))}
      >
        Add to Schedule
      </Button>
      {#if selected?.kind === "audio"}
        <Button variant="primary" onclick={() => selected && play(selected)}>Play</Button>
      {:else}
        <Button
          variant="primary"
          disabled={!selected}
          onclick={() => onSelected((item) => showMediaInPreview(item.id))}
        >
          Show in Preview
        </Button>
      {/if}
    {/if}
  </div>

  <ul class="grid" aria-label="Media">
    {#each shown as item (item.id)}
      <li>
        <button
          type="button"
          class="card"
          class:selected={picked.has(item.id)}
          draggable="true"
          ondragstart={(event) => startDrag(event, item)}
          onclick={(event) => pick(event, item)}
          ondblclick={() => activate(item)}
        >
          <span class="picture">
            {#if item.kind === "audio"}
              <span class="note" aria-hidden="true">♪</span>
            {:else}
              <img src={thumbnailUrl(item)} alt="" onerror={() => media.prepare(item)}>
            {/if}
            {#if item.kind !== "image" && item.durationMs}
              <span class="badge">{formatDuration(item.durationMs)}</span>
            {/if}
            {#if item.id === logoId}
              <span class="badge logo">Logo</span>
            {/if}
          </span>
          <span class="caption">
            <span class="title">{item.name}</span>
            <span class="meta">
              {kindLabel(item)}
              {#if item.width && item.height}
                · {item.width}×{item.height}
              {/if}
              · {formatBytes(item.sizeBytes)}
            </span>
          </span>
        </button>
      </li>
    {:else}
      <li class="empty">
        Nothing here yet. Import images or videos to use as backgrounds, logos, or schedule items,
        and MP3 or M4A files for background music. MP4 (H.264) videos play everywhere.
      </li>
    {/each}
  </ul>
</div>

<style>
  .media-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
  }

  .filters {
    display: flex;
    gap: var(--sw-space-3);
    margin-right: var(--sw-space-3);
    color: var(--sw-text-muted);
  }

  .filters label {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
  }

  .progress {
    color: var(--sw-text-muted);
  }

  .spacer {
    flex: 1;
  }

  .name {
    width: 220px;
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: var(--sw-space-3);
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    align-content: start;
    gap: var(--sw-space-3);
    overflow-y: auto;
    flex: 1;
    border-top: 1px solid var(--sw-border);
  }

  .card {
    width: 100%;
    padding: 0;
    border: 2px solid transparent;
    border-radius: var(--sw-radius);
    background: var(--sw-surface-raised);
    cursor: pointer;
    text-align: left;
    overflow: hidden;
  }

  .card:hover {
    border-color: var(--sw-surface-hover);
  }

  .card.selected {
    border-color: var(--sw-accent);
  }

  .picture {
    position: relative;
    display: block;
    aspect-ratio: 16 / 9;
    background: #000;
  }

  .picture img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .note {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-size: 48px;
    color: var(--sw-text-faint);
    background: var(--sw-surface);
  }

  .badge {
    position: absolute;
    right: 6px;
    bottom: 6px;
    padding: 1px 6px;
    border-radius: var(--sw-radius-small);
    background: rgba(0, 0, 0, 0.75);
    font-size: var(--sw-font-size-small);
    font-variant-numeric: tabular-nums;
  }

  .badge.logo {
    left: 6px;
    right: auto;
    background: var(--sw-accent);
  }

  .caption {
    display: flex;
    flex-direction: column;
    padding: var(--sw-space-1) var(--sw-space-2) var(--sw-space-2);
  }

  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .empty {
    grid-column: 1 / -1;
    color: var(--sw-text-faint);
    padding: var(--sw-space-4);
    line-height: 1.5;
  }
</style>
