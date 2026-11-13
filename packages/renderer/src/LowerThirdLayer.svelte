<!--
  @component
  A lower third (see `crates/sw-core/src/lower_thirds.rs`): a title and an
  optional second line in an animated strap at the bottom of the canvas.

  Each style's entrance is a CSS animation, so the GPU moves it smoothly.
  Every screen starts the animation part-way through, as far as the start
  time says, so they all move in step, even an output opened later (which
  then shows it already in place). The exit is `lowerThirdOut`, played by
  the output around this layer.

  - `still`: no movement, just the lower third in place (NDI output, which
    is not redrawn often enough for animation).
  - `onend`: fires once when its seconds are up (if it has a time).
-->
<script lang="ts">
  import type { LowerThirdShown } from "@simpleworship/protocol";
  import { onMount, untrack } from "svelte";
  import { now as serviceNow } from "./timing.ts";

  interface Props {
    shown: LowerThirdShown;
    still?: boolean;
    onend?: () => void;
  }

  let { shown, still = false, onend }: Props = $props();

  const item = $derived(shown.item);
  const look = $derived(shown.item.look);
  /** How far into the entrance this screen joined, as a negative delay
   *  (read once: each lower third mounts afresh, keyed on its start time). */
  const joined = -Math.max(0, serviceNow() - untrack(() => shown.startedAt));

  onMount(() => {
    const seconds = shown.item.hideAfterSeconds;
    if (seconds == null) return;
    const left = seconds * 1000 - (serviceNow() - shown.startedAt);
    const timer = setTimeout(() => onend?.(), Math.max(0, left));
    return () => clearTimeout(timer);
  });
</script>

<div
  class="lower-third {look.style} {look.side}"
  class:still={still}
  class:single={!item.subtitle}
  style:--accent={look.accent}
  style:--box={look.background}
  style:--ink={look.color}
  style:--font={look.fontFamily ? `"${look.fontFamily}", var(--sw-render-font, sans-serif)` : null}
  style:--joined="{joined}ms"
>
  <div class="accent"></div>
  <div class="box">
    <div class="title">{item.title}</div>
    {#if item.subtitle}
      <div class="subtitle">{item.subtitle}</div>
    {/if}
  </div>
</div>

<style>
  /* All sizes are canvas pixels (1920 × 1080). */
  .lower-third {
    position: absolute;
    bottom: 110px;
    display: flex;
    align-items: stretch;
    max-width: 1300px;
    color: var(--ink);
    font-family: var(--font, "Montserrat", "Open Sans", sans-serif);
  }

  .left {
    left: 110px;
  }

  .centre {
    left: 50%;
    translate: -50% 0;
  }

  .right {
    right: 110px;
    flex-direction: row-reverse;
  }

  .box {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 6px;
    padding: 22px 34px;
    background: var(--box);
  }

  .title {
    font-size: 56px;
    font-weight: 700;
    line-height: 1.1;
  }

  .subtitle {
    font-size: 34px;
    font-weight: 500;
    line-height: 1.2;
    opacity: 0.88;
  }

  .accent {
    flex: none;
    background: var(--accent);
  }

  /* Every animated part starts at its own delay after the start time, less
     how far into the entrance this screen joined. Each animation holds its
     first frame until then ("both" in every shorthand: a shorthand resets
     the fill mode, so it cannot be set once for all). */

  /* ── Bar: an accent bar grows, the box slides out of it ─────────────── */
  .bar .accent {
    width: 12px;
    animation: grow-y 320ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
  }

  .bar .box {
    animation: reveal-x 520ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(220ms + var(--joined));
  }

  .bar :is(.title, .subtitle) {
    animation: slide-in 520ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(380ms + var(--joined));
  }

  /* ── Wipe: the box is wiped on, an accent edge leading ──────────────── */
  .wipe .accent {
    width: 6px;
  }

  .wipe {
    animation: reveal-x 620ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
    animation-fill-mode: both;
    animation-timing-function: cubic-bezier(0.65, 0, 0.35, 1);
  }

  .wipe :is(.title, .subtitle) {
    animation: fade-up 460ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(340ms + var(--joined));
  }

  /* ── Underline: the text rises, an accent line draws under the title ── */
  .underline .accent {
    display: none;
  }

  .underline .title {
    position: relative;
    padding-bottom: 12px;
    animation: fade-up 480ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
  }

  .underline .title::after {
    content: "";
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: 6px;
    background: var(--accent);
    transform-origin: left;
    animation: grow-x 560ms both cubic-bezier(0.22, 1, 0.36, 1);
    animation-delay: calc(260ms + var(--joined));
  }

  .underline .subtitle {
    animation: fade-up 480ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(420ms + var(--joined));
  }

  .underline .box {
    animation: fade-in 300ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
  }

  /* ── Rise: the whole card rises into place, an accent edge on top ───── */
  .rise {
    flex-direction: column;
    animation: rise 560ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
    animation-fill-mode: both;
    animation-timing-function: cubic-bezier(0.22, 1, 0.36, 1);
  }

  .rise .accent {
    height: 8px;
  }

  /* ── Strap: full width, the title on an accent tag at its start ─────── */
  .strap {
    left: 0;
    right: 0;
    bottom: 80px;
    max-width: none;
    translate: none;
    flex-direction: row;
    background: var(--box);
    animation: reveal-x 480ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
    animation-fill-mode: both;
    animation-timing-function: cubic-bezier(0.65, 0, 0.35, 1);
  }

  .strap .accent {
    display: none;
  }

  .strap .box {
    flex-direction: row;
    align-items: stretch;
    gap: 0;
    padding: 0;
    background: none;
  }

  .strap .title {
    display: flex;
    align-items: center;
    padding: 18px 40px 18px 110px;
    background: var(--accent);
    animation: fade-in 300ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(260ms + var(--joined));
  }

  .strap .subtitle {
    display: flex;
    align-items: center;
    padding: 0 40px;
    font-size: 40px;
    animation: slide-in 460ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(360ms + var(--joined));
  }

  /* ── Block: the title on an accent block, the second line below ─────── */
  .block {
    flex-direction: column;
    align-items: flex-start;
  }

  .block.right {
    align-items: flex-end;
  }

  .block .accent {
    display: none;
  }

  .block .box {
    padding: 0;
    gap: 0;
    background: none;
  }

  .block .title {
    padding: 16px 30px;
    background: var(--accent);
    animation: reveal-x 460ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
  }

  .block .subtitle {
    align-self: flex-start;
    padding: 12px 30px;
    background: var(--box);
    opacity: 1;
    animation: reveal-x 460ms cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(200ms + var(--joined));
  }

  .block.right .subtitle {
    align-self: flex-end;
  }

  @keyframes grow-y {
    from {
      transform: scaleY(0);
    }
  }

  @keyframes grow-x {
    from {
      transform: scaleX(0);
    }
  }

  @keyframes reveal-x {
    from {
      clip-path: inset(0 100% 0 0);
    }
    to {
      clip-path: inset(0 0 0 0);
    }
  }

  @keyframes slide-in {
    from {
      opacity: 0;
      transform: translateX(-24px);
    }
  }

  @keyframes fade-up {
    from {
      opacity: 0;
      transform: translateY(18px);
    }
  }

  @keyframes fade-in {
    from {
      opacity: 0;
    }
  }

  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(46px);
    }
  }

  /* Still outputs (NDI): everything in place, nothing moving. Last, and
     more specific than every style above. */
  .lower-third.still,
  .lower-third.still :is(.accent, .box, .title, .subtitle),
  .lower-third.still .title::after {
    animation: none;
  }
</style>
