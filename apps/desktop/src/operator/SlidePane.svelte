<!--
  @component
  The preview or live pane: a large view of the selected slide above a grid
  of every slide in the presentation.

  The live pane's large view shows the real output (including Black, Logo
  and transitions), so the operator sees exactly what the room sees.

  - Click a slide to select it.
  - Double-click a slide (preview only) to send it live.
  - For a presentation or slideshow from the schedule, drag slides to
    reorder them, or remove one with its × (preview only). This changes the
    schedule item, not the library's presentation.
-->
<script lang="ts">
  import type { OutputState, PaneState } from "@simpleworship/protocol";
  import { OutputView, ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import type { Snippet } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { frameAt } from "../lib/frames.ts";

  interface Props {
    kind: "preview" | "live";
    pane: PaneState | null;
    /** The output state; only used by the live pane. */
    output?: OutputState | null;
    onselect: (index: number) => void;
    onactivate?: (index: number) => void;
    /** Buttons for the pane header. */
    actions?: Snippet;
    /** Controls shown under the large view (the live pane's video controls). */
    below?: Snippet;
    /** The live view's alert ran its course. */
    onalertend?: () => void;
    /** The live view's countdown reached zero. */
    oncountdownend?: () => void;
  }

  let {
    kind,
    pane,
    output = null,
    onselect,
    onactivate,
    actions,
    below,
    onalertend,
    oncountdownend,
  }: Props = $props();

  const selectedFrame = $derived(pane ? frameAt(pane.presentation, pane.selected) : null);

  /** Payload type for a slide being reordered within a schedule item. */
  const SLIDE_DRAG_TYPE = "application/x-simpleworship-item-slide";

  /**
   * The schedule item whose slides can be reordered and removed here: a
   * presentation or slideshow from the schedule.
   */
  const editableItem = $derived(
    pane?.scheduleItem && pane.presentation.source.type === "deck" ? pane.scheduleItem : null,
  );
  /** Where a slide being dragged would land, as an insertion index. */
  let dropAt: number | null = $state(null);

  function dragOverSlide(event: DragEvent, index: number) {
    if (!editableItem || !event.dataTransfer?.types.includes(SLIDE_DRAG_TYPE)) return;
    event.preventDefault();
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    dropAt = event.clientX > box.left + box.width / 2 ? index + 1 : index;
  }

  function dropSlide(event: DragEvent) {
    const from = Number(event.dataTransfer?.getData(SLIDE_DRAG_TYPE));
    const insertion = dropAt;
    dropAt = null;
    if (!editableItem || insertion === null || !Number.isInteger(from)) return;
    event.preventDefault();
    const to = insertion > from ? insertion - 1 : insertion;
    if (to !== from) engine.dispatch({ type: "moveItemSlide", id: editableItem, from, to });
  }

  function removeSlide(index: number) {
    if (editableItem) engine.dispatch({ type: "removeItemSlide", id: editableItem, index });
  }

  let slideList: HTMLOListElement;

  // Keep the selected slide visible as the operator steps through a long song.
  $effect(() => {
    if (pane === null) return;
    const selected = slideList.querySelector(`[data-index="${pane.selected}"]`);
    selected?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  });
</script>

<section class="pane {kind}" aria-label={kind === "live" ? "Live" : "Preview"}>
  <header>
    <span class="badge">{kind === "live" ? "Live" : "Preview"}</span>
    <span class="title">{pane?.presentation.title ?? ""}</span>
    <span class="actions">{@render actions?.()}</span>
  </header>

  <div class="main-view">
    <ScaledCanvas>
      {#if kind === "live" && output}
        <!-- Muted: only the projector plays sound. -->
        <OutputView {output} muted {onalertend} {oncountdownend} />
      {:else if selectedFrame}
        <SlideView frame={selectedFrame} />
      {/if}
    </ScaledCanvas>
  </div>

  {@render below?.()}

  <ol class="slides" bind:this={slideList} ondrop={dropSlide} ondragleave={() => (dropAt = null)}>
    {#if pane}
      {#each pane.presentation.slides as slide, index (index)}
        {@const frame = frameAt(pane.presentation, index)}
        <li
          class:drop-before={dropAt === index}
          class:drop-after={dropAt === pane.presentation.slides.length &&
            index === pane.presentation.slides.length - 1}
          ondragover={(event) => dragOverSlide(event, index)}
        >
          {#if editableItem && kind === "preview" && pane.presentation.slides.length > 1}
            <button
              type="button"
              class="remove"
              title="Remove this slide from the schedule item"
              aria-label="Remove slide {index + 1}"
              onclick={() => removeSlide(index)}
            >
              ×
            </button>
          {/if}
          <button
            type="button"
            class="slide"
            class:selected={index === pane.selected}
            data-index={index}
            draggable={editableItem !== null}
            ondragstart={(event) => event.dataTransfer?.setData(SLIDE_DRAG_TYPE, String(index))}
            onclick={() => onselect(index)}
            ondblclick={() => onactivate?.(index)}
          >
            {#if frame}
              <ScaledCanvas><SlideView {frame} /></ScaledCanvas>
            {/if}
            <span class="caption">
              <span class="number">{index + 1}</span>
              {#if slide.startsSection && slide.section}
                <span class="section">{slide.section.name}</span>
              {:else if slide.caption}
                <span class="section">{slide.caption}</span>
              {/if}
            </span>
          </button>
        </li>
      {/each}
    {:else}
      <li class="empty">
        {kind === "live" ? "Nothing is live." : "Choose a song or some text to preview."}
      </li>
    {/if}
  </ol>
</section>

<style>
  li {
    position: relative;
  }

  /* Where a slide being reordered will land. */
  .drop-before {
    box-shadow: -3px 0 0 var(--sw-accent);
  }

  .drop-after {
    box-shadow: 3px 0 0 var(--sw-accent);
  }

  .remove {
    position: absolute;
    top: 4px;
    right: 4px;
    z-index: 1;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 50%;
    background: rgba(0, 0, 0, 0.7);
    color: #fff;
    line-height: 20px;
    cursor: pointer;
    opacity: 0;
  }

  li:hover .remove,
  .remove:focus-visible {
    opacity: 1;
  }

  .pane {
    --pane-color: var(--sw-preview);
    background: var(--sw-surface);
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }

  .preview {
    grid-area: preview;
  }

  .live {
    grid-area: live;
    --pane-color: var(--sw-live);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
    border-bottom: 1px solid var(--sw-border);
    min-height: 44px;
  }

  .badge {
    padding: 2px var(--sw-space-2);
    border-radius: var(--sw-radius-small);
    background: var(--pane-color);
    font-size: var(--sw-font-size-small);
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .main-view {
    /* As wide as possible, but never taller than about a third of the window. */
    width: min(100% - 2 * var(--sw-space-3), 36vh * 16 / 9);
    margin: var(--sw-space-3) auto;
    border: 2px solid var(--pane-color);
    border-radius: var(--sw-radius-small);
    overflow: hidden;
  }

  .slides {
    list-style: none;
    margin: 0;
    padding: var(--sw-space-3);
    padding-top: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    align-content: start;
    gap: var(--sw-space-2);
    overflow-y: auto;
    flex: 1;
  }

  .slide {
    width: 100%;
    padding: 0;
    border: 2px solid transparent;
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    cursor: pointer;
    text-align: left;
    overflow: hidden;
  }

  .slide:hover {
    border-color: var(--sw-surface-hover);
  }

  .slide.selected {
    border-color: var(--pane-color);
  }

  .caption {
    display: flex;
    gap: var(--sw-space-2);
    padding: 2px var(--sw-space-2);
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-muted);
  }

  .section {
    color: var(--sw-text);
    font-weight: 600;
  }

  .empty {
    grid-column: 1 / -1;
    padding: var(--sw-space-4);
    text-align: center;
    color: var(--sw-text-faint);
  }
</style>
