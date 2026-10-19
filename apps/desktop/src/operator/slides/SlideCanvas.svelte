<!--
  @component
  The slide being edited, drawn exactly as the projector will show it,
  with handles on top:

  - Click an element to select it; drag it to move it; drag a handle to
    resize it (hold Shift on a corner to keep its shape). Edges and centres
    snap to the canvas and to other elements (hold Alt / Option to place
    freely).
  - Double-click a text box to type in it.
  - Click an empty part of the slide to select the slide itself.
-->
<script lang="ts">
  import type { Frame, SlideElement } from "@simpleworship/protocol";
  import {
    BackgroundLayer,
    CANVAS_HEIGHT,
    CANVAS_WIDTH,
    ElementsLayer,
    ScaledCanvas,
  } from "@simpleworship/renderer";
  import { engine } from "../../lib/engine.svelte.ts";
  import type { SlideEditor } from "../../lib/slides/editor.svelte.ts";
  import { slideTheme } from "../../lib/slides/look.ts";
  import { type Guide, snapMove, snapResize } from "../../lib/slides/snapping.ts";
  import TextEditing from "./TextEditing.svelte";

  interface Props {
    editor: SlideEditor;
  }

  let { editor }: Props = $props();

  /** The smallest an element may become, in canvas pixels. */
  const MIN_SIZE = 16;

  type Handle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";
  const HANDLES: Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];

  let overlay: HTMLDivElement;
  let guides: Guide[] = $state([]);
  /** Where the double-click that started editing was, for the caret. */
  let clickAt: { x: number; y: number } | null = $state(null);

  const slide = $derived(editor.slide);
  const theme = $derived(slide ? slideTheme(engine.themes, editor.deck, slide) : null);
  const background = $derived(slide?.background ?? theme?.background ?? null);
  /** The text box being typed in, if any. */
  const editing = $derived(
    editor.editingId && editor.textBox?.id === editor.editingId ? editor.textBox : null,
  );
  /** Everything but the box being typed in (that one is drawn by `TextEditing`). */
  const shown = $derived(slide?.elements.filter((item) => item.id !== editor.editingId) ?? []);

  const percent = (value: number, total: number) => `${(value / total) * 100}%`;

  /** Screen pixels → canvas pixels. */
  function scale(): number {
    return CANVAS_WIDTH / overlay.getBoundingClientRect().width;
  }

  /** Starts moving or resizing an element; follows the pointer until it is released. */
  function startDrag(event: PointerEvent, element: SlideElement, handle: Handle | "move") {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    editor.select(element.id);
    if (element.locked) return;

    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);
    const start = { x: event.clientX, y: event.clientY, frame: { ...element.frame } };
    const ratio = start.frame.width / start.frame.height;
    const factor = scale();
    const others = (slide?.elements ?? [])
      .filter((item) => item.id !== element.id)
      .map((item) => item.frame);
    let moved = false;

    const move = (moveEvent: PointerEvent) => {
      const dx = (moveEvent.clientX - start.x) * factor;
      const dy = (moveEvent.clientY - start.y) * factor;
      if (!moved) {
        if (Math.abs(dx) < 2 && Math.abs(dy) < 2) return;
        // One undo step for the whole drag.
        editor.checkpoint();
        moved = true;
      }
      const snap = !moveEvent.altKey;
      const result =
        handle === "move"
          ? snap
            ? snapMove(offset(start.frame, dx, dy), others)
            : { frame: offset(start.frame, dx, dy), guides: [] }
          : resized(start.frame, handle, dx, dy, moveEvent.shiftKey ? ratio : null, snap, others);
      guides = result.guides;
      element.frame = rounded(result.frame);
    };
    const stop = () => {
      guides = [];
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", stop);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", stop);
  }

  function offset(frame: Frame, dx: number, dy: number): Frame {
    return { ...frame, x: frame.x + dx, y: frame.y + dy };
  }

  function rounded(frame: Frame): Frame {
    return {
      x: Math.round(frame.x),
      y: Math.round(frame.y),
      width: Math.round(frame.width),
      height: Math.round(frame.height),
    };
  }

  /** The frame after dragging `handle` by (dx, dy). */
  function resized(
    from: Frame,
    handle: Handle,
    dx: number,
    dy: number,
    keepRatio: number | null,
    snap: boolean,
    others: Frame[],
  ): { frame: Frame; guides: Guide[] } {
    const edges = {
      left: handle.includes("w"),
      right: handle.includes("e"),
      top: handle.includes("n"),
      bottom: handle.includes("s"),
    };
    let { x, y, width, height } = from;
    if (edges.left) {
      x = Math.min(from.x + dx, from.x + from.width - MIN_SIZE);
      width = from.x + from.width - x;
    }
    if (edges.right) width = Math.max(from.width + dx, MIN_SIZE);
    if (edges.top) {
      y = Math.min(from.y + dy, from.y + from.height - MIN_SIZE);
      height = from.y + from.height - y;
    }
    if (edges.bottom) height = Math.max(from.height + dy, MIN_SIZE);

    // Shift on a corner: keep the shape, following the wider change.
    const corner = handle.length === 2;
    if (keepRatio && corner) {
      if (width / height > keepRatio) height = width / keepRatio;
      else width = height * keepRatio;
      if (edges.left) x = from.x + from.width - width;
      if (edges.top) y = from.y + from.height - height;
      return { frame: { x, y, width, height }, guides: [] };
    }
    const frame = { x, y, width, height };
    return snap ? snapResize(frame, edges, others) : { frame, guides: [] };
  }

  function handleDoubleClick(event: MouseEvent, element: SlideElement) {
    if (element.content.type !== "text") return;
    clickAt = { x: event.clientX, y: event.clientY };
    editor.startEditing(element.id);
  }

  /** A click on the slide itself (not on an element) selects the slide. */
  function handleBackgroundPointer(event: PointerEvent) {
    const target = event.target as HTMLElement;
    if (target.closest("[contenteditable], .hit")) return;
    editor.select(null);
    editor.stopEditing();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions: pointer selection; keys are handled by the editor -->
<div class="stage" onpointerdown={handleBackgroundPointer}>
  <ScaledCanvas>
    {#if background}
      <BackgroundLayer {background} still />
    {/if}
    <ElementsLayer elements={shown} still muted />
    {#if editing}
      {#key editing.id}
        <TextEditing {editor} element={editing} {clickAt} />
      {/key}
    {/if}
  </ScaledCanvas>

  <div class="overlay" bind:this={overlay}>
    {#each shown as element (element.id)}
      {@const selected = element.id === editor.selectedId}
      <!-- biome-ignore lint/a11y/noStaticElementInteractions: pointer dragging; keys are handled by the editor -->
      <div
        class="hit"
        class:selected
        class:locked={element.locked}
        role="presentation"
        style:left={percent(element.frame.x, CANVAS_WIDTH)}
        style:top={percent(element.frame.y, CANVAS_HEIGHT)}
        style:width={percent(element.frame.width, CANVAS_WIDTH)}
        style:height={percent(element.frame.height, CANVAS_HEIGHT)}
        onpointerdown={(event) => startDrag(event, element, "move")}
        ondblclick={(event) => handleDoubleClick(event, element)}
      >
        {#if selected && !element.locked}
          {#each HANDLES as handle (handle)}
            <span
              class="handle {handle}"
              role="presentation"
              onpointerdown={(event) => startDrag(event, element, handle)}
            ></span>
          {/each}
        {/if}
      </div>
    {/each}

    {#each guides as guide, index (index)}
      <span
        class="guide {guide.axis}"
        style:left={guide.axis === "x" ? percent(guide.position, CANVAS_WIDTH) : null}
        style:top={guide.axis === "y" ? percent(guide.position, CANVAS_HEIGHT) : null}
      ></span>
    {/each}
  </div>
</div>

<style>
  .stage {
    position: relative;
    box-shadow: 0 8px 30px rgba(0, 0, 0, 0.5);
  }

  .overlay {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .hit {
    position: absolute;
    pointer-events: auto;
    cursor: move;
    touch-action: none;
  }

  .hit:hover {
    outline: 1px dashed rgba(255, 255, 255, 0.6);
  }

  .hit.selected {
    outline: 2px solid var(--sw-accent);
  }

  .hit.locked {
    cursor: default;
  }

  .handle {
    position: absolute;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    background: #fff;
    border: 1px solid var(--sw-accent);
    border-radius: 2px;
  }

  .nw {
    left: 0;
    top: 0;
    cursor: nwse-resize;
  }

  .n {
    left: 50%;
    top: 0;
    cursor: ns-resize;
  }

  .ne {
    left: 100%;
    top: 0;
    cursor: nesw-resize;
  }

  .e {
    left: 100%;
    top: 50%;
    cursor: ew-resize;
  }

  .se {
    left: 100%;
    top: 100%;
    cursor: nwse-resize;
  }

  .s {
    left: 50%;
    top: 100%;
    cursor: ns-resize;
  }

  .sw {
    left: 0;
    top: 100%;
    cursor: nesw-resize;
  }

  .w {
    left: 0;
    top: 50%;
    cursor: ew-resize;
  }

  .guide {
    position: absolute;
    background: #ff3d8b;
  }

  .guide.x {
    top: 0;
    bottom: 0;
    width: 1px;
  }

  .guide.y {
    left: 0;
    right: 0;
    height: 1px;
  }
</style>
