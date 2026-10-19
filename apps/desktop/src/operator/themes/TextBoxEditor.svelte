<!--
  @component
  An overlay for the theme designer's preview that shows the text box and
  lets the operator move it (drag the middle) or resize it (drag a corner).

  Place it over a 16:9 preview of the same size. Positions are in canvas
  pixels (1920×1080), like the theme itself.
-->
<script lang="ts">
  import type { TextBox } from "@simpleworship/protocol";
  import { CANVAS_HEIGHT, CANVAS_WIDTH } from "@simpleworship/renderer";

  interface Props {
    box: TextBox;
    onchange: (box: TextBox) => void;
  }

  let { box, onchange }: Props = $props();

  /** The smallest the box may become, in canvas pixels. */
  const MIN_SIZE = 120;

  type Handle = "move" | "top-left" | "top-right" | "bottom-left" | "bottom-right";

  let overlay: HTMLDivElement;

  /** Starts dragging; follows the pointer until it is released. */
  function startDrag(event: PointerEvent, handle: Handle) {
    event.preventDefault();
    const start = { x: event.clientX, y: event.clientY, box: { ...box } };
    // Screen pixels → canvas pixels.
    const scale = CANVAS_WIDTH / overlay.getBoundingClientRect().width;
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);

    const move = (moveEvent: PointerEvent) => {
      const dx = Math.round((moveEvent.clientX - start.x) * scale);
      const dy = Math.round((moveEvent.clientY - start.y) * scale);
      onchange(dragged(start.box, handle, dx, dy));
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
  <div
    class="box"
    role="presentation"
    style:top={percent(box.top, CANVAS_HEIGHT)}
    style:right={percent(box.right, CANVAS_WIDTH)}
    style:bottom={percent(box.bottom, CANVAS_HEIGHT)}
    style:left={percent(box.left, CANVAS_WIDTH)}
    onpointerdown={(e) => startDrag(e, "move")}
  >
    {#each ["top-left", "top-right", "bottom-left", "bottom-right"] as const as handle (handle)}
      <span
        class="handle {handle}"
        role="presentation"
        onpointerdown={(e) => {
          e.stopPropagation();
          startDrag(e, handle);
        }}
      ></span>
    {/each}
  </div>
</div>

<style>
  .overlay {
    position: absolute;
    inset: 0;
  }

  .box {
    position: absolute;
    border: 1px dashed rgba(255, 255, 255, 0.8);
    outline: 1px dashed rgba(0, 0, 0, 0.6);
    cursor: move;
    touch-action: none;
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
