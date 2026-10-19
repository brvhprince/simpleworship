<!--
  @component
  Imported presentations: sermon slides and announcements from PDF (or
  PowerPoint and Keynote, when LibreOffice is installed). Each page becomes
  a slide; step through them like any other presentation.
-->
<script lang="ts">
  import type { SlideDeck } from "@simpleworship/protocol";
  import { getMediaUrls } from "@simpleworship/renderer";
  import { Button } from "@simpleworship/ui";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import {
    addPresentationPage,
    addPresentationToSchedule,
    canConvertPresentations,
    createPresentation,
    deletePresentation,
    finishPresentation,
    listPresentations,
    preparePresentation,
    showPresentationInPreview,
  } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { renderPdfPages } from "../../lib/pdf.ts";
  import { prompts } from "../../lib/prompts.svelte.ts";

  /** Must match `PRESENTATIONS_EVENT` in `src-tauri/src/commands/presentations.rs`. */
  const PRESENTATIONS_EVENT = "presentations://changed";

  const urls = getMediaUrls();
  let decks: SlideDeck[] = $state.raw([]);
  let selectedId: string | null = $state(null);
  let canConvert = $state(false);
  /** What an import in progress is doing, e.g. "Page 3 of 40". */
  let progress: string | null = $state(null);

  const selected = $derived(decks.find((deck) => deck.id === selectedId) ?? null);

  onMount(() => {
    void refresh();
    canConvertPresentations().then((value) => (canConvert = value));
    const stop = listen(PRESENTATIONS_EVENT, () => void refresh());
    return () => void stop.then((unlisten) => unlisten());
  });

  async function refresh() {
    decks = await listPresentations();
  }

  async function chooseAndImport() {
    const extensions = canConvert ? ["pdf", "pptx", "ppt", "key", "odp"] : ["pdf"];
    const path = await open({ filters: [{ name: "Presentations", extensions }], multiple: false });
    if (typeof path !== "string") return;

    let deck: string | null = null;
    let file: string | null = null;
    try {
      progress = canConvert && !path.toLowerCase().endsWith(".pdf") ? "Converting…" : "Opening…";
      const prepared = await preparePresentation(path);
      file = prepared.file;
      deck = await createPresentation(prepared.name);
      const id = deck;
      await renderPdfPages(urls.file({ id, file: prepared.file }), async (index, total, jpeg) => {
        progress = `Page ${index + 1} of ${total}`;
        await addPresentationPage(id, index, jpeg);
      });
      await finishPresentation(prepared.file);
      selectedId = deck;
    } catch (error) {
      engine.showError(error);
      if (file) await finishPresentation(file, deck).catch(() => {});
    } finally {
      progress = null;
    }
  }

  async function remove(deck: SlideDeck) {
    const answer = await prompts.ask({
      title: "Delete presentation?",
      message: `Delete “${deck.name}” and its ${deck.pages.length} slides? Schedules that use it keep their copy.`,
      buttons: [
        { id: "cancel", label: "Cancel" },
        { id: "delete", label: "Delete", variant: "live" },
      ],
      cancel: "cancel",
    });
    if (answer === "delete") await attempt(() => deletePresentation(deck.id));
  }

  async function attempt(action: () => Promise<void>) {
    try {
      await action();
    } catch (error) {
      engine.showError(error);
    }
  }
</script>

<div class="presentations-tab">
  <div class="toolbar">
    <Button disabled={progress !== null} onclick={chooseAndImport}>Import…</Button>
    {#if progress}
      <span class="progress">{progress}</span>
    {:else}
      <span class="hint">
        {canConvert
          ? "PDF, PowerPoint and Keynote files can be imported."
          : "Import PDF files. To import PowerPoint or Keynote, save as PDF first (or install LibreOffice)."}
      </span>
    {/if}
    <span class="spacer"></span>
    <Button disabled={!selected} onclick={() => selected && remove(selected)}>Delete</Button>
    <Button
      disabled={!selected}
      onclick={() => selected && attempt(() => addPresentationToSchedule(selected.id))}
    >
      Add to Schedule
    </Button>
    <Button
      variant="primary"
      disabled={!selected}
      onclick={() => selected && attempt(() => showPresentationInPreview(selected.id))}
    >
      Show in Preview
    </Button>
  </div>

  <ul class="grid" aria-label="Presentations">
    {#each decks as deck (deck.id)}
      {@const cover = deck.pages[0]}
      <li>
        <button
          type="button"
          class="card"
          class:selected={deck.id === selectedId}
          onclick={() => (selectedId = deck.id)}
          ondblclick={() => attempt(() => showPresentationInPreview(deck.id))}
        >
          <span class="picture">
            {#if cover}
              <img src={urls.file({ id: cover.id, file: cover.file })} alt="">
            {/if}
          </span>
          <span class="caption">
            <span class="title">{deck.name}</span>
            <span class="meta">{deck.pages.length} slides</span>
          </span>
        </button>
      </li>
    {:else}
      <li class="empty">
        No presentations yet. Import a PDF of your sermon slides or announcements.
      </li>
    {/each}
  </ul>
</div>

<style>
  .presentations-tab {
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

  .hint,
  .progress {
    color: var(--sw-text-muted);
  }

  .spacer {
    flex: 1;
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: var(--sw-space-3);
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
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

  .card.selected {
    border-color: var(--sw-accent);
  }

  .picture {
    display: block;
    aspect-ratio: 16 / 9;
    background: #000;
  }

  .picture img {
    width: 100%;
    height: 100%;
    object-fit: contain;
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
    padding: var(--sw-space-4);
    color: var(--sw-text-faint);
  }
</style>
