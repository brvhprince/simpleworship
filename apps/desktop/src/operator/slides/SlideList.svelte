<!--
  @component
  The slides of the presentation being edited, as thumbnails with their
  first line of words. Click to edit one; drag to reorder. ↑/↓ move
  through them.
-->
<script lang="ts">
  import { ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import type { SlideEditor } from "../../lib/slides/editor.svelte.ts";
  import { slideFrame } from "../../lib/slides/look.ts";
  import { slideLabel } from "../../lib/slides/slide-ops.ts";

  interface Props {
    editor: SlideEditor;
  }

  let { editor }: Props = $props();

  /** Payload type for a slide being reordered within the list. */
  const SLIDE_DRAG_TYPE = "application/x-simpleworship-editor-slide";

  /** Where a slide being dragged would land, as an insertion index. */
  let dropAt: number | null = $state(null);
  let list: HTMLDivElement;

  // Keep the current slide in view.
  $effect(() => {
    const current = list?.querySelector(`[data-index="${editor.slideIndex}"]`);
    current?.scrollIntoView({ block: "nearest" });
  });

  function handleKeys(event: KeyboardEvent) {
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key];
    if (step === undefined) return;
    event.preventDefault();
    editor.showSlide(editor.slideIndex + step);
  }

  function dragOver(event: DragEvent, index: number) {
    if (!event.dataTransfer?.types.includes(SLIDE_DRAG_TYPE)) return;
    event.preventDefault();
    const row = (event.currentTarget as HTMLElement).getBoundingClientRect();
    dropAt = event.clientY > row.top + row.height / 2 ? index + 1 : index;
  }

  function drop(event: DragEvent) {
    const from = Number(event.dataTransfer?.getData(SLIDE_DRAG_TYPE));
    const insertion = dropAt;
    dropAt = null;
    if (insertion === null || !Number.isInteger(from)) return;
    event.preventDefault();
    const to = insertion > from ? insertion - 1 : insertion;
    if (to === from) return;
    editor.change((deck) => {
      const [moved] = deck.slides.splice(from, 1);
      if (moved) deck.slides.splice(to, 0, moved);
    });
    editor.showSlide(to);
  }
</script>

<div
  class="list"
  bind:this={list}
  role="listbox"
  tabindex="0"
  aria-label={t("slides.list")}
  onkeydown={handleKeys}
  ondrop={drop}
  ondragleave={() => (dropAt = null)}
>
  {#each editor.deck.slides as slide, index (slide.id)}
    {@const frame = slideFrame(engine.themes, editor.deck, slide)}
    <!-- svelte-ignore a11y_click_events_have_key_events: keys are handled by the listbox -->
    <!-- biome-ignore lint/a11y/useKeyWithClickEvents: keys are handled by the listbox -->
    <div
      class="slide"
      class:selected={index === editor.slideIndex}
      class:drop-before={dropAt === index}
      class:drop-after={dropAt === editor.deck.slides.length &&
        index === editor.deck.slides.length - 1}
      data-index={index}
      role="option"
      tabindex="-1"
      aria-selected={index === editor.slideIndex}
      draggable="true"
      ondragstart={(e) => e.dataTransfer?.setData(SLIDE_DRAG_TYPE, String(index))}
      ondragover={(e) => dragOver(e, index)}
      onclick={() => editor.showSlide(index)}
    >
      <span class="number">{index + 1}</span>
      <span class="body">
        <span class="picture">
          {#if frame}
            <ScaledCanvas><SlideView {frame} /></ScaledCanvas>
          {/if}
        </span>
        <span class="label">{slideLabel(slide)}</span>
      </span>
    </div>
  {:else}
    <p class="empty">{t("slides.none")}</p>
  {/each}
</div>

<style>
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    padding: 2px;
  }

  .slide {
    display: flex;
    gap: var(--sw-space-1);
    cursor: pointer;
  }

  .number {
    width: 1.4em;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
    text-align: right;
  }

  .body {
    flex: 1;
    min-width: 0;
    border: 2px solid transparent;
    border-radius: var(--sw-radius-small);
    overflow: hidden;
    background: var(--sw-surface-raised);
  }

  .selected .body {
    border-color: var(--sw-accent);
  }

  .picture {
    display: block;
  }

  .label {
    display: block;
    padding: 2px 6px;
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .selected .label {
    background: var(--sw-accent);
    color: #fff;
  }

  .drop-before {
    box-shadow: 0 -2px 0 var(--sw-accent);
  }

  .drop-after {
    box-shadow: 0 2px 0 var(--sw-accent);
  }

  .empty {
    color: var(--sw-text-faint);
  }
</style>
