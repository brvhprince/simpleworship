<!--
  @component
  An overlay for the theme designer's preview that shows the boxes text
  is laid out in (the slide text, the reference, the footer) and lets the
  operator move one (drag the middle) or resize it (drag a corner).
  Clicking a box picks it; only the picked box has corner handles.

  Place it over a 16:9 preview of the same size. Positions are in canvas
  pixels (1920×1080), like the theme itself.
-->
<script lang="ts" module>
  import type { TextBox } from "@simpleworship/protocol";

  /** One box on the preview. */
  export interface EditableBox {
    id: string;
    /** Shown on the box, e.g. "Reference". */
    label: string;
    box: TextBox;
  }
</script>

<script lang="ts">
  import { CANVAS_HEIGHT, CANVAS_WIDTH } from "@simpleworship/renderer";

  interface Props {
    boxes: EditableBox[];
    /** The id of the picked box. */
    active: string;
    onselect: (id: string) => void;
    onchange: (id: string, box: TextBox) => void;
  }

  let { boxes, active, onselect, onchange }: Props = $props();

  /** The smallest a box may become, in canvas pixels. */
  const MIN_SIZE = 40;

  type Handle = "move" | "top-left" | "top-right" | "bottom-left" | "bottom-right";

  let overlay: HTMLDivElement;

  /** Starts dragging; follows the pointer until it is released. */
  function startDrag(event: PointerEvent, id: string, box: TextBox, handle: Handle) {
    event.preventDefault();
    onselect(id);
    const start = { x: event.clientX, y: event.clientY, box: { ...box } };
    // Screen pixels → canvas pixels.
    const scale = CANVAS_WIDTH / overlay.getBoundingClientRect().width;
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);

    const move = (moveEvent: PointerEvent) => {
      const dx = Math.round((moveEvent.clientX - start.x) * scale);
      const dy = Math.round((moveEvent.clientY - start.y) * scale);
      onchange(id, dragged(start.box, handle, dx, dy));
    };
    const stop = () => {
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", stop);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", stop);
  }

  /** The box after dragging `handle` by (dx, dy), kept on the canvas. */
  function dragged(from: TextBox, handle: Handle, dx: number, dy: number): TextBox {
    const clamp = (value: number, max: number) => Math.min(Math.max(value, 0), max);
    const width = CANVAS_WIDTH - from.left - from.right;
    const height = CANVAS_HEIGHT - from.top - from.bottom;
    const next = { ...from };

    if (handle === "move") {
      next.left = clamp(from.left + dx, CANVAS_WIDTH - width);
      next.right = CANVAS_WIDTH - width - next.left;
      next.top = clamp(from.top + dy, CANVAS_HEIGHT - height);
      next.bottom = CANVAS_HEIGHT - height - next.top;
      return next;
    }
    if (handle.includes("left")) {
      next.left = clamp(from.left + dx, CANVAS_WIDTH - from.right - MIN_SIZE);
    } else {
      next.right = clamp(from.right - dx, CANVAS_WIDTH - from.left - MIN_SIZE);
    }
    if (handle.includes("top")) {
      next.top = clamp(from.top + dy, CANVAS_HEIGHT - from.bottom - MIN_SIZE);
    } else {
      next.bottom = clamp(from.bottom - dy, CANVAS_HEIGHT - from.top - MIN_SIZE);
    }
    return next;
  }

  const percent = (value: number, total: number) => `${(value / total) * 100}%`;
</script>

<div class="overlay" bind:this={overlay}>
  <!-- The picked box last, so it is on top and easy to grab. -->
  {#each [...boxes].sort(
    (a, b) => Number(a.id === active) - Number(b.id === active),
  ) as item (item.id)}
    <div
      class="box"
      class:active={item.id === active}
      role="presentation"
      style:top={percent(item.box.top, CANVAS_HEIGHT)}
      style:right={percent(item.box.right, CANVAS_WIDTH)}
      style:bottom={percent(item.box.bottom, CANVAS_HEIGHT)}
      style:left={percent(item.box.left, CANVAS_WIDTH)}
      onpointerdown={(e) => startDrag(e, item.id, item.box, "move")}
    >
      {#if boxes.length > 1}
        <span class="tag">{item.label}</span>
      {/if}
      {#if item.id === active}
        {#each ["top-left", "top-right", "bottom-left", "bottom-right"] as const as handle (handle)}
          <span
            class="handle {handle}"
            role="presentation"
            onpointerdown={(e) => {
              e.stopPropagation();
              startDrag(e, item.id, item.box, handle);
            }}
          ></span>
        {/each}
      {/if}
    </div>
  {/each}
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
  }

  .box {
    position: absolute;
    border: 1px dashed rgba(255, 255, 255, 0.45);
    cursor: move;
    touch-action: none;
  }

  .box.active {
    border-color: rgba(255, 255, 255, 0.9);
    outline: 1px dashed rgba(0, 0, 0, 0.6);
  }

  .tag {
    position: absolute;
    top: 2px;
    left: 4px;
    padding: 0 4px;
    border-radius: 2px;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    font-size: 10px;
    pointer-events: none;
  }

  .box.active .tag {
    background: var(--sw-accent);
  }

  .handle {
    position: absolute;
    width: 12px;
    height: 12px;
    background: var(--sw-accent);
    border: 1px solid #fff;
    border-radius: 2px;
  }

  .top-left {
    top: -6px;
    left: -6px;
    cursor: nwse-resize;
  }

  .top-right {
    top: -6px;
    right: -6px;
    cursor: nesw-resize;
  }

  .bottom-left {
    bottom: -6px;
    left: -6px;
    cursor: nesw-resize;
  }

  .bottom-right {
    bottom: -6px;
    right: -6px;
    cursor: nwse-resize;
  }
</style>
