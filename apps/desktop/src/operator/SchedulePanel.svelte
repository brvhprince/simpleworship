<!--
  @component
  The order of service (left column).

  - Click an item to preview it; double-click to send it live.
  - ↑/↓ move through the items, Enter sends the previewed item live,
    Delete removes it.
  - Drag songs in from the library, and drag items to reorder them.
  - Drag several pictures in at once for one slideshow item (hold Alt /
    Option for one item each). Drop pictures or a presentation onto the
    middle of a picture, text or presentation item to add them as slides.

  The previewed item is marked blue and the live item red, like the panes.
  Notes for the previewed item can be edited at the bottom.
-->
<script lang="ts">
  import type { ScheduleItem } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import {
    addMediaToSchedule,
    addMissingScheduleSongs,
    addPlaylistToSchedule,
    addPresentationToSchedule,
    addSlideshowToSchedule,
    addSongToSchedule,
    appendMediaToItem,
    appendPresentationToItem,
    missingScheduleSongs,
  } from "../lib/api.ts";
  import {
    dropTarget,
    finalIndex,
    ITEM_DRAG_TYPE,
    MEDIA_DRAG_TYPE,
    mediaIds,
    PRESENTATION_DRAG_TYPE,
    SONG_DRAG_TYPE,
  } from "../lib/drag.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { library } from "../lib/library.svelte.ts";
  import { media } from "../lib/media.svelte.ts";
  import { canHoldSlides, itemKind, itemTitle } from "../lib/schedule.ts";
  import { scheduleFile } from "../lib/schedule-file.svelte.ts";
  import ItemLookEditor from "./ItemLookEditor.svelte";

  type ScheduleLayout = "list" | "column" | "grid";
  let layout = $state<ScheduleLayout>(
    (typeof localStorage !== "undefined" &&
      (localStorage.getItem("simpleworship.schedule_layout") as ScheduleLayout)) ||
      "list",
  );

  function setLayout(newLayout: ScheduleLayout) {
    layout = newLayout;
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("simpleworship.schedule_layout", newLayout);
    }
  }

  const items = $derived(engine.schedule.items);
  const previewId = $derived(engine.preview?.scheduleItem ?? null);
  const liveId = $derived(engine.live?.scheduleItem ?? null);
  const previewed = $derived(items.find((item) => item.id === previewId) ?? null);

  /** Where a drag in progress would drop, as an insertion index. */
  let dropAt: number | null = $state(null);
  /** The row a drag in progress would add slides to, if any. */
  let dropInto: number | null = $state(null);
  /** How many songs in the schedule are not in this computer's library. */
  let missingSongs = $state(0);

  // Re-check for missing songs whenever the schedule changes.
  $effect(() => {
    void engine.schedule;
    missingScheduleSongs()
      .then((ids) => (missingSongs = ids.length))
      .catch(() => (missingSongs = 0));
  });

  async function addMissingSongs() {
    try {
      await addMissingScheduleSongs();
      missingSongs = 0;
      library.changed();
    } catch (error) {
      engine.showError(error);
    }
  }

  function preview(item: ScheduleItem) {
    engine.dispatch({ type: "previewScheduleItem", id: item.id });
  }

  function show(item: ScheduleItem) {
    engine.dispatch({ type: "showScheduleItem", id: item.id });
  }

  function remove(item: ScheduleItem) {
    engine.dispatch({ type: "removeScheduleItem", id: item.id });
  }

  function setNotes(item: ScheduleItem, notes: string) {
    engine.dispatch({ type: "setScheduleItemNotes", id: item.id, notes });
  }

  /** Keyboard control while the list has focus. */
  function handleKeys(event: KeyboardEvent) {
    const index = items.findIndex((item) => item.id === previewId);
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key];

    if (step !== undefined) {
      const next = items[index === -1 ? 0 : index + step];
      if (next) preview(next);
    } else if ((event.key === "Delete" || event.key === "Backspace") && previewed) {
      remove(previewed);
    } else {
      // Everything else (Enter, F6…) falls through to the global shortcuts.
      return;
    }
    event.preventDefault();
  }

  // ── Drag and drop ──────────────────────────────────────────────────────

  const OUR_TYPES = [SONG_DRAG_TYPE, ITEM_DRAG_TYPE, MEDIA_DRAG_TYPE, PRESENTATION_DRAG_TYPE];

  function isOurDrag(event: DragEvent): boolean {
    const types = event.dataTransfer?.types ?? [];
    return OUR_TYPES.some((type) => types.includes(type));
  }

  /** Whether the drag carries slides (pictures or a presentation). */
  function carriesSlides(event: DragEvent): boolean {
    const types = event.dataTransfer?.types ?? [];
    return types.includes(MEDIA_DRAG_TYPE) || types.includes(PRESENTATION_DRAG_TYPE);
  }

  function startDrag(event: DragEvent, item: ScheduleItem) {
    event.dataTransfer?.setData(ITEM_DRAG_TYPE, item.id);
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  function dragOverRow(event: DragEvent, rowIndex: number) {
    if (!isOurDrag(event)) return;
    event.preventDefault();
    event.stopPropagation();
    const row = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const item = items[rowIndex];
    const canDropInto = carriesSlides(event) && item !== undefined && canHoldSlides(item);
    const target = dropTarget(rowIndex, event.clientY, row, canDropInto);
    dropAt = target.kind === "between" ? target.index : null;
    dropInto = target.kind === "into" ? target.index : null;
  }

  /** Dragging over the empty space below the items drops at the end. */
  function dragOverList(event: DragEvent) {
    if (!isOurDrag(event)) return;
    event.preventDefault();
    dropAt = items.length;
    dropInto = null;
  }

  function dragLeaveList(event: DragEvent) {
    const list = event.currentTarget as HTMLElement;
    if (!list.contains(event.relatedTarget as Node | null)) {
      dropAt = null;
      dropInto = null;
    }
  }

  async function drop(event: DragEvent) {
    const insertion = dropAt ?? items.length;
    const into = dropInto === null ? null : (items[dropInto] ?? null);
    dropAt = null;
    dropInto = null;
    const data = event.dataTransfer;
    if (!data || !isOurDrag(event)) return;
    event.preventDefault();

    const itemId = data.getData(ITEM_DRAG_TYPE);
    if (itemId) {
      const from = items.findIndex((item) => item.id === itemId);
      if (from !== -1) {
        engine.dispatch({ type: "moveScheduleItem", id: itemId, to: finalIndex(from, insertion) });
      }
      return;
    }

    try {
      const songId = data.getData(SONG_DRAG_TYPE);
      const deckId = data.getData(PRESENTATION_DRAG_TYPE);
      const ids = mediaIds(data.getData(MEDIA_DRAG_TYPE));
      if (songId) {
        await addSongToSchedule(songId, insertion);
      } else if (deckId) {
        if (into) await appendPresentationToItem(into.id, deckId);
        else await addPresentationToSchedule(deckId, insertion);
      } else if (ids.length > 0) {
        await dropMedia(ids, insertion, into, event.altKey);
      }
    } catch (error) {
      engine.showError(error);
    }
  }

  /**
   * Media dropped on the schedule: added to the item it was dropped on, or
   * as one new item (a slideshow of pictures, or music for sound files),
   * or, with Alt held, one item per file.
   */
  async function dropMedia(
    ids: string[],
    insertion: number,
    into: ScheduleItem | null,
    separately: boolean,
  ) {
    if (into) {
      await appendMediaToItem(into.id, ids);
      return;
    }
    if (ids.length === 1 || separately) {
      // One after the other, so they keep their order.
      for (const [offset, id] of ids.entries()) {
        await addMediaToSchedule(id, insertion + offset);
      }
      return;
    }
    const kinds = ids.map((id) => media.list.find((item) => item.id === id)?.kind);
    if (kinds.every((kind) => kind === "audio")) {
      const count = ids.length;
      await addPlaylistToSchedule(t("schedule.musicTracks", { count }), ids, insertion);
    } else {
      await addSlideshowToSchedule(ids, insertion);
    }
  }
</script>

<section class="schedule" aria-label={t("schedule.title")}>
  <header>
    <span class="heading">{t("schedule.title")}</span>
    <div class="layout-toggle" role="group" aria-label={t("schedule.layout")}>
      <button
        type="button"
        class="layout-btn"
        class:active={layout === "list"}
        title={t("schedule.layoutList")}
        aria-label={t("schedule.layoutList")}
        onclick={() => setLayout("list")}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>
      </button>
      <button
        type="button"
        class="layout-btn"
        class:active={layout === "column"}
        title={t("schedule.layoutColumn")}
        aria-label={t("schedule.layoutColumn")}
        onclick={() => setLayout("column")}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="5" rx="1"/><rect x="3" y="15" width="18" height="5" rx="1"/></svg>
      </button>
      <button
        type="button"
        class="layout-btn"
        class:active={layout === "grid"}
        title={t("schedule.layoutGrid")}
        aria-label={t("schedule.layoutGrid")}
        onclick={() => setLayout("grid")}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></svg>
      </button>
    </div>
    <span class="file" title={scheduleFile.status.path ?? t("schedule.notSaved")}>
      {scheduleFile.status.path ? scheduleFile.status.name : t("schedule.untitled")}{scheduleFile
        .status.dirty
        ? " •"
        : ""}
    </span>
  </header>

  {#if missingSongs > 0}
    <div class="notice">
      {t("schedule.missingSongs", { count: missingSongs })}
      <Button onclick={addMissingSongs}>{t("schedule.addToLibrary")}</Button>
    </div>
  {/if}

  <!--
    The list follows the "active descendant" pattern: the list has focus,
    and the previewed item is the current option.
  -->
  <div
    class="items"
    class:layout-list={layout === "list"}
    class:layout-column={layout === "column"}
    class:layout-grid={layout === "grid"}
    role="listbox"
    aria-label={t("schedule.items")}
    tabindex="0"
    aria-activedescendant={previewId ? `schedule-${previewId}` : undefined}
    onkeydown={handleKeys}
    ondragover={dragOverList}
    ondragleave={dragLeaveList}
    ondrop={drop}
  >
    {#each items as item, index (item.id)}
      <!-- svelte-ignore a11y_click_events_have_key_events: keys are handled by the listbox (handleKeys) -->
      <!-- biome-ignore lint/a11y/useKeyWithClickEvents: keys are handled by the listbox (handleKeys) -->
      <div
        id="schedule-{item.id}"
        class="item"
        class:live={item.id === liveId}
        class:drop-before={dropAt === index}
        class:drop-into={dropInto === index}
        class:drop-after={dropAt === items.length && index === items.length - 1}
        role="option"
        tabindex="-1"
        aria-selected={item.id === previewId}
        draggable="true"
        ondragstart={(event) => startDrag(event, item)}
        ondragover={(event) => dragOverRow(event, index)}
        onclick={() => preview(item)}
        ondblclick={() => show(item)}
      >
        <span class="number">{index + 1}</span>
        <span class="kind">{itemKind(item)}</span>
        <span class="title">{itemTitle(item)}</span>
        {#if item.notes}
          <span class="has-notes" title={item.notes}>✎</span>
        {/if}
      </div>
    {:else}
      <p class="empty">
        {t("schedule.empty")}
      </p>
    {/each}
  </div>

  {#if previewed}
    <div class="details">
      <ItemLookEditor item={previewed} />
    </div>
    <label class="notes">
      {t("schedule.notesFor", { title: itemTitle(previewed) })}
      <textarea
        value={previewed.notes}
        placeholder={t("schedule.notesPlaceholder")}
        onchange={(event) => setNotes(previewed, event.currentTarget.value)}
      ></textarea>
    </label>
  {/if}
</section>

<style>
  .schedule {
    grid-area: schedule;
    background: var(--sw-surface);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
    border-bottom: 1px solid var(--sw-border);
    min-height: 44px;
  }

  .heading {
    font-weight: 600;
  }

  .layout-toggle {
    display: inline-flex;
    align-items: center;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    overflow: hidden;
  }

  .layout-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3px 6px;
    background: transparent;
    border: none;
    color: var(--sw-text-muted);
    cursor: pointer;
    line-height: 1;
    transition: background 0.15s, color 0.15s;
  }

  .layout-btn:hover {
    color: var(--sw-text);
    background: rgba(255, 255, 255, 0.05);
  }

  .layout-btn.active {
    background: var(--sw-surface-sunken, rgba(0, 0, 0, 0.2));
    color: var(--sw-accent);
  }

  .file {
    flex: 1;
    text-align: right;
    color: var(--sw-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .notice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--sw-space-2);
    margin: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
    border-radius: var(--sw-radius);
    background: rgba(245, 159, 0, 0.12);
    color: var(--sw-warning);
  }

  .items {
    flex: 1;
    overflow-y: auto;
    padding: var(--sw-space-1) 0;
  }

  .items.layout-column {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    padding: var(--sw-space-2);
  }

  .items.layout-column .item {
    border-radius: var(--sw-radius);
    border: 1px solid var(--sw-border);
    border-left: 4px solid transparent;
    background: var(--sw-surface-sunken, rgba(255, 255, 255, 0.02));
    padding: var(--sw-space-2) var(--sw-space-3);
  }

  .items.layout-column .item[aria-selected="true"] {
    border-left-color: var(--sw-preview);
    border-color: var(--sw-preview);
    background: rgba(76, 110, 245, 0.12);
  }

  .items.layout-column .item.live {
    border-left-color: var(--sw-live);
    border-color: var(--sw-live);
    background: var(--sw-live-soft);
  }

  .items.layout-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(130px, 1fr));
    gap: var(--sw-space-2);
    padding: var(--sw-space-2);
    align-content: start;
  }

  .items.layout-grid .item {
    flex-direction: column;
    align-items: flex-start;
    border-radius: var(--sw-radius);
    border: 1px solid var(--sw-border);
    border-top: 3px solid transparent;
    border-left: none;
    background: var(--sw-surface-sunken, rgba(255, 255, 255, 0.02));
    padding: var(--sw-space-2);
    min-height: 80px;
    gap: var(--sw-space-1);
    position: relative;
  }

  .items.layout-grid .item .title {
    width: 100%;
    white-space: normal;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-height: 1.3;
  }

  .items.layout-grid .item[aria-selected="true"] {
    border-top-color: var(--sw-preview);
    border-color: var(--sw-preview);
    background: rgba(76, 110, 245, 0.12);
  }

  .items.layout-grid .item.live {
    border-top-color: var(--sw-live);
    border-color: var(--sw-live);
    background: var(--sw-live-soft);
  }

  .items.layout-grid .drop-before {
    box-shadow: inset 3px 0 0 var(--sw-accent);
  }

  .items.layout-grid .drop-after {
    box-shadow: inset -3px 0 0 var(--sw-accent);
  }

  .items:focus-visible {
    outline: 2px solid var(--sw-accent);
    outline-offset: -2px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
    border-left: 3px solid transparent;
    cursor: default;
    user-select: none;
  }

  .item[aria-selected="true"] {
    background: rgba(76, 110, 245, 0.18);
    border-left-color: var(--sw-preview);
  }

  .item.live {
    background: var(--sw-live-soft);
    border-left-color: var(--sw-live);
  }

  /* The line showing where a dragged item will land. */
  .drop-before {
    box-shadow: inset 0 2px 0 var(--sw-accent);
  }

  .drop-after {
    box-shadow: inset 0 -2px 0 var(--sw-accent);
  }

  /* The item that dropped pictures or slides will be added to. */
  .drop-into {
    outline: 2px solid var(--sw-accent);
    outline-offset: -2px;
  }

  .number {
    width: 1.5em;
    color: var(--sw-text-faint);
    font-variant-numeric: tabular-nums;
  }

  .kind {
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .has-notes {
    color: var(--sw-text-muted);
  }

  .empty {
    margin: var(--sw-space-3);
    color: var(--sw-text-faint);
    line-height: 1.5;
  }

  .details {
    padding: var(--sw-space-2) var(--sw-space-3) 0;
    border-top: 1px solid var(--sw-border);
  }

  .notes {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    padding: var(--sw-space-2) var(--sw-space-3) var(--sw-space-3);
    border-top: 1px solid var(--sw-border);
    color: var(--sw-text-muted);
  }

  .notes textarea {
    height: 72px;
    resize: none;
    color: var(--sw-text);
  }
</style>
