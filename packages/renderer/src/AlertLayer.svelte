<!--
  @component
  An alert over the slides: a still banner, or a ticker scrolling from
  right to left.

  The ticker is a CSS animation, so the GPU moves it smoothly instead of a
  script nudging it every frame (which made the text shimmer). Every
  screen starts the animation part-way through, as far as the alert's start
  time says, so they all scroll in step, even an output opened later.

  `onend` fires once when the alert has run its course (its seconds are up,
  or the ticker has scrolled across the set number of times). A still
  banner set to end "after passes" counts each pass as ten seconds.

  `still`: show a ticker as a still banner too, shrunk to fit if it is too
  long (NDI output, which is not redrawn often enough to scroll smoothly).
-->
<script lang="ts">
  import type { Alert } from "@simpleworship/protocol";
  import { onMount } from "svelte";
  import { CANVAS_WIDTH } from "./canvas.ts";
  import { now as serviceNow, TICKER_SPEED } from "./timing.ts";

  /** How long one "pass" of a still banner lasts, in milliseconds. */
  const BANNER_PASS_MS = 10_000;

  interface Props {
    alert: Alert;
    still?: boolean;
    onend?: () => void;
  }

  let { alert, still = false, onend }: Props = $props();

  /** Room for a still banner's text, leaving a margin at each side. */
  const STILL_WIDTH = CANVAS_WIDTH * 0.94;

  let text: HTMLSpanElement;
  /** The text's width in canvas pixels, once measured (the ticker waits for it). */
  let textWidth: number | null = $state(null);
  /** How far into the scrolling this screen joined, in milliseconds. */
  let joinedAt = $state(0);
  let ended = false;

  const isTicker = $derived(alert.style === "ticker" && !still);
  /** Shrinks a still banner's text that would not fit. */
  const fit = $derived(
    !isTicker && textWidth !== null && textWidth > STILL_WIDTH ? STILL_WIDTH / textWidth : null,
  );
  /** One pass: from just off the right edge until the end leaves the left. */
  const passMs = $derived(
    textWidth === null ? 0 : ((CANVAS_WIDTH + textWidth) / TICKER_SPEED) * 1000,
  );
  const passes = $derived(alert.end.type === "afterPasses" ? String(alert.end.passes) : "infinite");

  onMount(() => {
    // Measure once the fonts are ready; the width decides the speed.
    void document.fonts.ready.then(() => {
      if (!text) return;
      textWidth = text.offsetWidth;
      joinedAt = serviceNow() - alert.startedAt;
    });

    const total = totalMs();
    if (total === null) return;
    const timer = setTimeout(finish, Math.max(0, total - (serviceNow() - alert.startedAt)));
    return () => clearTimeout(timer);
  });

  /** How long the alert stays up, or `null` for a ticker counting its passes (see `onanimationend`). */
  function totalMs(): number | null {
    switch (alert.end.type) {
      case "afterSeconds":
        return alert.end.seconds * 1000;
      case "afterPasses":
        return isTicker ? null : alert.end.passes * BANNER_PASS_MS;
      case "manual":
        return null;
    }
  }

  function finish() {
    if (!ended) {
      ended = true;
      onend?.();
    }
  }
</script>

<div
  class="alert {alert.position}"
  class:ticker={isTicker}
  style:background={alert.background}
  style:color={alert.color}
>
  <span
    bind:this={text}
    class="text"
    class:scrolling={isTicker && textWidth !== null}
    style:--from="{CANVAS_WIDTH}px"
    style:--to="{-(textWidth ?? 0)}px"
    style:animation-duration="{passMs}ms"
    style:animation-delay="{-joinedAt}ms"
    style:animation-iteration-count={passes}
    style:transform={fit === null ? null : `scale(${fit})`}
    onanimationend={finish}
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
    /* Hidden until measured, so it does not flash in the wrong place. */
    visibility: hidden;
    /* Its own GPU layer: moving it never redraws the text. */
    will-change: transform;
    backface-visibility: hidden;
  }

  .ticker .text.scrolling {
    visibility: visible;
    animation-name: scroll;
    animation-timing-function: linear;
    animation-fill-mode: both;
  }

  @keyframes scroll {
    from {
      transform: translate3d(var(--from), 0, 0);
    }
    to {
      transform: translate3d(var(--to), 0, 0);
    }
  }
</style>
