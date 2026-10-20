<!--
  @component
  Lays out its children on the 1920×1080 reference canvas and scales the
  canvas to fit the available space.

  - `fit="width"` (default): fills the parent's width and takes a 16:9 height.
    Used for previews and thumbnails.
  - `fit="contain"`: fills the parent in both directions, letterboxing if
    the parent is not 16:9. Used for full-screen output windows.
  - `background`: what shows around and behind the slide; black unless the
    page must be see-through (the stream output).
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { CANVAS_HEIGHT, CANVAS_WIDTH } from "./canvas.ts";

  interface Props {
    fit?: "width" | "contain";
    background?: string;
    children: Snippet;
  }

  let { fit = "width", background = "#000", children }: Props = $props();

  let width = $state(0);
  let height = $state(0);

  const scale = $derived(
    fit === "contain"
      ? Math.min(width / CANVAS_WIDTH, height / CANVAS_HEIGHT)
      : width / CANVAS_WIDTH,
  );
</script>

<div
  class="viewport {fit}"
  style:background={background}
  bind:clientWidth={width}
  bind:clientHeight={height}
>
  <div
    class="canvas"
    style:width="{CANVAS_WIDTH}px"
    style:height="{CANVAS_HEIGHT}px"
    style:transform="translate(-50%, -50%) scale({scale})"
  >
    {@render children()}
  </div>
</div>

<style>
  .viewport {
    position: relative;
    overflow: hidden;
  }

  .width {
    width: 100%;
    aspect-ratio: 16 / 9;
  }

  .contain {
    width: 100%;
    height: 100%;
  }

  .canvas {
    position: absolute;
    top: 50%;
    left: 50%;
    transform-origin: center;
  }
</style>
