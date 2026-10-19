<!--
  @component
  The preview or live pane: a large view of the selected slide above a grid
  of every slide in the presentation.

  The live pane's large view shows the real output (including Black, Logo
  and transitions), so the operator sees exactly what the room sees.

  - Click a slide to select it.
  - Double-click a slide (preview only) to send it live.
-->
<script lang="ts">
  import type { OutputState, PaneState } from "@simpleworship/protocol";
  import { OutputView, ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import type { Snippet } from "svelte";
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
  }

  let { kind, pane, output = null, onselect, onactivate, actions }: Props = $props();

  const selectedFrame = $derived(pane ? frameAt(pane.presentation, pane.selected) : null);

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
        <OutputView {output} />
      {:else if selectedFrame}
        <SlideView frame={selectedFrame} />
      {/if}
    </ScaledCanvas>
  </div>

  <ol class="slides" bind:this={slideList}>
    {#if pane}
      {#each pane.presentation.slides as slide, index (index)}
        {@const frame = frameAt(pane.presentation, index)}
        <li>
          <button
            type="button"
            class="slide"
            class:selected={index === pane.selected}
            data-index={index}
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
