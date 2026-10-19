<!--
  @component
  An alert over the slides: a still banner, or a ticker scrolling from
  right to left. Every screen positions the ticker from the alert's start
  time, so they all scroll in step.

  `onend` fires once when the alert has run its course (its seconds are up,
  or the ticker has scrolled across the set number of times). A still
  banner set to end "after passes" counts each pass as ten seconds.
-->
<script lang="ts">
  import type { Alert } from "@simpleworship/protocol";
  import { onMount } from "svelte";
  import { CANVAS_WIDTH } from "./canvas.ts";
  import { TICKER_SPEED } from "./timing.ts";

  /** How long one "pass" of a still banner lasts, in milliseconds. */
  const BANNER_PASS_MS = 10_000;

  interface Props {
    alert: Alert;
    onend?: () => void;
  }

  let { alert, onend }: Props = $props();

  let text: HTMLSpanElement;
  let offset = $state(CANVAS_WIDTH);
  let ended = false;

  onMount(() => {
    let frame = requestAnimationFrame(function step() {
      const elapsed = Date.now() - alert.startedAt;
      if (alert.style === "ticker" && text) {
        // One pass: from off the right edge until the end leaves the left.
        const passLength = CANVAS_WIDTH + text.offsetWidth;
        const travelled = (elapsed / 1000) * TICKER_SPEED;
        offset = CANVAS_WIDTH - (travelled % passLength);
        const passes = travelled / passLength;
        if (alert.end.type === "afterPasses" && passes >= alert.end.passes) finish();
      }
      if (alert.end.type === "afterSeconds" && elapsed >= alert.end.seconds * 1000) finish();
      if (
        alert.style === "banner" &&
        alert.end.type === "afterPasses" &&
        elapsed >= alert.end.passes * BANNER_PASS_MS
      ) {
        finish();
      }
      frame = requestAnimationFrame(step);
    });
    return () => cancelAnimationFrame(frame);
  });

  function finish() {
    if (!ended) {
      ended = true;
      onend?.();
    }
  }
</script>

<div class="alert {alert.position}" class:ticker={alert.style === "ticker"}>
  <span
    bind:this={text}
    class="text"
    style:transform={alert.style === "ticker" ? `translateX(${offset}px)` : undefined}
  >
    {alert.text}
  </span>
</div>

<style>
  .alert {
    position: absolute;
    left: 0;
    right: 0;
    height: 110px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.82);
    color: #fff;
    font-family: "Open Sans", Helvetica, Arial, sans-serif;
    font-size: 54px;
    font-weight: 600;
    overflow: hidden;
    white-space: nowrap;
  }

  .top {
    top: 0;
  }

  .bottom {
    bottom: 0;
  }

  .ticker {
    justify-content: flex-start;
  }

  .ticker .text {
    position: absolute;
    left: 0;
    will-change: transform;
  }
</style>
