<!--
  @component
  Presentations: typed announcements and sermon points made in the editor,
  and slides imported from PDF (or PowerPoint and Keynote, when
  LibreOffice is installed). Imported pages can be edited too: reorder or
  drop them, or put typed slides between them.

  Click to select, double-click to preview, drag onto the schedule (or onto
  a schedule item, to add the slides to it).
-->
<script lang="ts">
  import type { Presentation, SlideDeck } from "@simpleworship/protocol";
  import { getMediaUrls, ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { Button } from "@simpleworship/ui";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import {
    addPresentationPage,
    addPresentationToSchedule,
    canConvertPresentations,
    createPresentation,
    deckForEditing,
    deckPresentation,
    deletePresentation,
    finishPresentation,
    getPresentation,
    listPresentations,
    preparePresentation,
    showPresentationInPreview,
    uuidV7,
  } from "../../lib/api.ts";
  import { PRESENTATION_DRAG_TYPE } from "../../lib/drag.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { frameAt } from "../../lib/frames.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { link } from "../../lib/link.svelte.ts";
  import { renderPdfPages } from "../../lib/pdf.ts";
  import { prompts } from "../../lib/prompts.svelte.ts";
  import SlideEditor from "../slides/SlideEditor.svelte";

  /** Must match `PRESENTATIONS_EVENT` in `src-tauri/src/commands/presentations.rs`. */
  const PRESENTATIONS_EVENT = "presentations://changed";

  const urls = getMediaUrls();
  let decks: SlideDeck[] = $state.raw([]);
  /** Each deck's slides as shown, for the cover pictures (by deck id). */
  let covers: Record<string, Presentation> = $state.raw({});
  let selectedId: string | null = $state(null);
  let query = $state("");
  let canConvert = $state(false);
  /** What an import in progress is doing, e.g. "Page 3 of 40". */
  let progress: string | null = $state(null);
  /** The presentation open in the editor, if any. */
  let editing = $state<SlideDeck | null>(null);

  const selected = $derived(decks.find((deck) => deck.id === selectedId) ?? null);
  /** Decks whose name or text matches the search. */
  const shown = $derived.by(() => {
    const words = query.trim().toLowerCase();
    if (!words) return decks;
    return decks.filter(
      (deck) =>
        deck.name.toLowerCase().includes(words) ||
        deck.slides.some((slide) => slide.text.toLowerCase().includes(words)),
    );
  });

  onMount(() => {
    void refresh();
    canConvertPresentations().then((value) => (canConvert = value));
    const stop = listen(PRESENTATIONS_EVENT, () => void refresh());
    return () => void stop.then((unlisten) => unlisten());
  });

  // Redraw covers when the themes change.
  $effect(() => {
    void engine.themes;
    void drawCovers(decks);
  });

  async function refresh() {
    decks = await listPresentations();
  }

  async function drawCovers(list: SlideDeck[]) {
    const entries = await Promise.all(
      list.map(async (deck) => [deck.id, await deckPresentation(deck)] as const),
    );
    covers = Object.fromEntries(entries);
  }

  function newPresentation() {
    const now = Date.now();
    editing = {
      id: uuidV7(),
      name: "",
      themeId: null,
      // The editor adds a first slide with a text box in the theme's style.
      slides: [],
      autoAdvance: null,
      createdAt: now,
      updatedAt: 0,
    };
  }

  async function edit(deck: SlideDeck) {
    // The latest version, in case it changed since the list was loaded,
    // with slides from before the canvas editor turned into text boxes.
    const latest = await getPresentation(deck.id).catch(() => deck);
    editing = await deckForEditing(latest);
  }

  async function chooseAndImport() {
    const extensions = canConvert ? ["pdf", "pptx", "ppt", "key", "odp"] : ["pdf"];
    const path = await open({
      filters: [{ name: t("decks.filesFilter"), extensions }],
      multiple: false,
    });
    if (typeof path !== "string") return;

    let deck: string | null = null;
    let file: string | null = null;
    try {
      progress =
        canConvert && !path.toLowerCase().endsWith(".pdf")
          ? t("decks.converting")
          : t("decks.opening");
      const prepared = await preparePresentation(path);
      file = prepared.file;
      deck = await createPresentation(prepared.name);
      const id = deck;
      await renderPdfPages(urls.file({ id, file: prepared.file }), async (index, total, jpeg) => {
        progress = t("decks.page", { page: index + 1, total });
        await addPresentationPage(id, index, jpeg);
      });
      await finishPresentation(prepared.file, deck);
      selectedId = deck;
    } catch (error) {
      engine.showError(error);
      if (file) await finishPresentation(file, deck, true).catch(() => {});
    } finally {
      progress = null;
    }
  }

  async function remove(deck: SlideDeck) {
    const answer = await prompts.ask({
      title: t("decks.deleteTitle"),
      message: t("decks.deleteMessage", { name: deck.name, count: deck.slides.length }),
      buttons: [
        { id: "cancel", label: t("common.cancel") },
        { id: "delete", label: t("common.delete"), variant: "live" },
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

  function startDrag(event: DragEvent, deck: SlideDeck) {
    event.dataTransfer?.setData(PRESENTATION_DRAG_TYPE, deck.id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "copy";
  }
</script>

<div class="presentations-tab">
  <div class="toolbar">
    {#if !link.joined}
      <Button onclick={newPresentation}>{t("decks.new")}</Button>
      <Button disabled={progress !== null} onclick={chooseAndImport}>{t("common.import")}</Button>
    {/if}
    <input
      type="search"
      bind:value={query}
      placeholder={t("decks.search")}
      aria-label={t("decks.searchLabel")}
    >
    {#if progress}
      <span class="progress">{progress}</span>
    {:else if !canConvert}
      <span class="hint">{t("decks.needsLibreOffice")}</span>
    {/if}
    <span class="spacer"></span>
    {#if !link.joined}
      <Button disabled={!selected} onclick={() => selected && remove(selected)}
        >{t("common.delete")}</Button
      >
      <Button disabled={!selected} onclick={() => selected && edit(selected)}
        >{t("common.edit")}</Button
      >
    {/if}
    <Button
      disabled={!selected}
      onclick={() => selected && attempt(() => addPresentationToSchedule(selected.id))}
    >
      {t("common.addToSchedule")}
    </Button>
    <Button
      variant="primary"
      disabled={!selected}
      onclick={() => selected && attempt(() => showPresentationInPreview(selected.id))}
    >
      {t("common.showInPreview")}
    </Button>
  </div>

  <ul class="grid" aria-label={t("decks.list")}>
    {#each shown as deck (deck.id)}
      {@const slides = covers[deck.id]}
      {@const cover = slides ? frameAt(slides, 0) : null}
      <li>
        <button
          type="button"
          class="card"
          class:selected={deck.id === selectedId}
          draggable="true"
          ondragstart={(event) => startDrag(event, deck)}
          onclick={() => (selectedId = deck.id)}
          ondblclick={() => attempt(() => showPresentationInPreview(deck.id))}
        >
          <span class="picture">
            {#if cover}
              <ScaledCanvas><SlideView frame={cover} /></ScaledCanvas>
            {/if}
          </span>
          <span class="caption">
            <span class="title">{deck.name}</span>
            <span class="meta">
              {t("common.slides", { count: deck.slides.length })}{deck.autoAdvance
                ? t("decks.everySeconds", { seconds: deck.autoAdvance.seconds })
                : ""}
            </span>
          </span>
        </button>
      </li>
    {:else}
      <li class="empty">
        {query ? t("decks.noMatch") : t("decks.empty")}
      </li>
    {/each}
  </ul>
</div>

{#if editing}
  <SlideEditor
    deck={editing}
    onsave={(saved) => {
      editing = null;
      selectedId = saved.id;
    }}
    oncancel={() => (editing = null)}
  />
{/if}

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

  .toolbar input[type="search"] {
    width: 200px;
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
