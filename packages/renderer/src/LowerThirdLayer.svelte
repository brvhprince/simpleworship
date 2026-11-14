<!--
  @component
  A lower third (see `crates/sw-core/src/lower_thirds.rs`): a title and an
  optional second line in an animated strap at the bottom of the canvas.

  Each style's entrance is a CSS animation, so the GPU moves it smoothly.
  Every screen starts the animation part-way through, as far as the start
  time says, so they all move in step, even an output opened later (which
  then shows it already in place). The exit is `lowerThirdOut`, played by
  the output around this layer.

  Its size and speed are two variables every measure and timing here is
  multiplied by: `--u` (one canvas pixel at its size) and `--pace`.

  - `logo`: the church logo, for the Logo icon (nothing without one).
  - `still`: no movement, just the lower third in place (NDI output, which
    is not redrawn often enough for animation).
  - `onend`: fires once when its seconds are up (if it has a time), with
    the id of the showing this layer was mounted for: a layer still leaving
    after a replacement appeared reports its own showing, never the new one.
-->
<script lang="ts">
  import type { Logo, LowerThirdShown } from "@simpleworship/protocol";
  import { onMount, untrack } from "svelte";
  import { LOWER_THIRD_ICONS } from "./lower-third-icons.ts";
  import { LOWER_THIRD_PACE, LOWER_THIRD_SCALE } from "./lower-thirds.ts";
  import { getMediaUrls } from "./media.ts";
  import { now as serviceNow } from "./timing.ts";

  interface Props {
    shown: LowerThirdShown;
    logo?: Logo | null;
    still?: boolean;
    onend?: (showId: string) => void;
  }

  let { shown, logo = null, still = false, onend }: Props = $props();

  const urls = getMediaUrls();
  const item = $derived(shown.item);
  const look = $derived(shown.item.look);
  /** The drawing beside the title, if it has one. */
  const drawing = $derived(
    look.icon === "none" || look.icon === "logo" ? null : LOWER_THIRD_ICONS[look.icon],
  );
  /** The logo beside the title (a video logo shows its still). */
  const logoSrc = $derived.by(() => {
    if (look.icon !== "logo" || !logo) return null;
    return logo.kind === "video" ? urls.thumbnail(logo.media) : urls.file(logo.media);
  });
  /** How far into the entrance this screen joined, as a negative delay
   *  (read once: each lower third mounts afresh, keyed on its start time). */
  const joined = -Math.max(0, serviceNow() - untrack(() => shown.startedAt));

  onMount(() => {
    // The showing this layer is for, fixed now (each showing mounts afresh).
    const { showId, startedAt, item } = untrack(() => shown);
    const seconds = item.hideAfterSeconds;
    if (seconds == null) return;
    const left = seconds * 1000 - (serviceNow() - startedAt);
    const timer = setTimeout(() => onend?.(showId), Math.max(0, left));
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
  style:--u="{LOWER_THIRD_SCALE[look.size]}px"
  style:--pace={LOWER_THIRD_PACE[look.speed]}
  style:--round="calc({look.rounding} * var(--u))"
>
  <div class="accent"></div>
  <div class="box">
    <div class="title">
      {#if drawing}
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
          {#each drawing as d (d)}
            <path {d} />
          {/each}
        </svg>
      {:else if logoSrc}
        <img class="icon logo" src={logoSrc} alt="">
      {/if}
      <span>{item.title}</span>
    </div>
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
    max-width: calc(1300 * var(--u));
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
    gap: calc(6 * var(--u));
    padding: calc(22 * var(--u)) calc(34 * var(--u));
    background: var(--box);
  }

  .title {
    font-size: calc(56 * var(--u));
    font-weight: 700;
    line-height: 1.1;
  }

  .subtitle {
    font-size: calc(34 * var(--u));
    font-weight: 500;
    line-height: 1.2;
    opacity: 0.88;
  }

  .accent {
    flex: none;
    background: var(--accent);
  }

  /* The icon beside the title: a line drawing in the accent colour (the
     text's colour where the title sits on the accent), or the logo. */
  .title {
    display: flex;
    align-items: center;
    gap: 0.3em;
  }

  /* A long unbroken title wraps rather than being cut off by the card's
     rounded clip. */
  .box,
  .title > span {
    min-width: 0;
  }

  .title > span {
    overflow-wrap: anywhere;
  }

  .icon {
    flex: none;
    width: 0.9em;
    height: 0.9em;
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .icon.logo {
    width: auto;
    max-width: 3em;
    height: 1.1em;
    object-fit: contain;
  }

  /* Round corners: the whole card where the accent and box are one piece,
     each piece where they are separate. */
  .bar,
  .wipe,
  .rise {
    overflow: hidden;
    border-radius: var(--round);
  }

  .underline .box,
  .block :is(.title, .subtitle) {
    border-radius: var(--round);
  }

  .strap .title {
    border-radius: 0 var(--round) var(--round) 0;
  }

  /* Every animated part starts at its own delay after the start time, less
     how far into the entrance this screen joined. Each animation holds its
     first frame until then ("both" in every shorthand: a shorthand resets
     the fill mode, so it cannot be set once for all). */

  /* ── Bar: an accent bar grows, the box slides out of it ─────────────── */
  .bar .accent {
    width: calc(12 * var(--u));
    animation: grow-y calc(320ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
  }

  .bar .box {
    animation: reveal-x calc(520ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(220ms * var(--pace) + var(--joined));
  }

  .bar :is(.title, .subtitle) {
    animation: slide-in calc(520ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(380ms * var(--pace) + var(--joined));
  }

  /* ── Wipe: the box is wiped on, an accent edge leading ──────────────── */
  .wipe .accent {
    width: calc(6 * var(--u));
  }

  .wipe {
    animation: reveal-x calc(620ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
    animation-fill-mode: both;
    animation-timing-function: cubic-bezier(0.65, 0, 0.35, 1);
  }

  .wipe :is(.title, .subtitle) {
    animation: fade-up calc(460ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(340ms * var(--pace) + var(--joined));
  }

  /* ── Underline: the text rises, an accent line draws under the title ── */
  .underline .accent {
    display: none;
  }

  .underline .title {
    position: relative;
    padding-bottom: calc(12 * var(--u));
    animation: fade-up calc(480ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
  }

  .underline .title::after {
    content: "";
    position: absolute;
    left: 0;
    bottom: 0;
    width: 100%;
    height: calc(6 * var(--u));
    background: var(--accent);
    transform-origin: left;
    animation: grow-x calc(560ms * var(--pace)) both cubic-bezier(0.22, 1, 0.36, 1);
    animation-delay: calc(260ms * var(--pace) + var(--joined));
  }

  .underline .subtitle {
    animation: fade-up calc(480ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(420ms * var(--pace) + var(--joined));
  }

  .underline .box {
    animation: fade-in calc(300ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
  }

  /* ── Rise: the whole card rises into place, an accent edge on top ───── */
  .rise {
    flex-direction: column;
    animation: rise calc(560ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
    animation-fill-mode: both;
    animation-timing-function: cubic-bezier(0.22, 1, 0.36, 1);
  }

  .rise .accent {
    height: calc(8 * var(--u));
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
    animation: reveal-x calc(480ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
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
    padding: calc(18 * var(--u)) calc(40 * var(--u)) calc(18 * var(--u)) 110px;
    background: var(--accent);
    animation: fade-in calc(300ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(260ms * var(--pace) + var(--joined));
  }

  .strap .subtitle {
    display: flex;
    align-items: center;
    padding: 0 calc(40 * var(--u));
    font-size: calc(40 * var(--u));
    animation: slide-in calc(460ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(360ms * var(--pace) + var(--joined));
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
    padding: calc(16 * var(--u)) calc(30 * var(--u));
    background: var(--accent);
    animation: reveal-x calc(460ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: var(--joined);
  }

  .block .subtitle {
    align-self: flex-start;
    padding: calc(12 * var(--u)) calc(30 * var(--u));
    background: var(--box);
    opacity: 1;
    animation: reveal-x calc(460ms * var(--pace)) cubic-bezier(0.22, 1, 0.36, 1) both;
    animation-delay: calc(200ms * var(--pace) + var(--joined));
  }

  .block.right .subtitle {
    align-self: flex-end;
  }

  /* Where the title sits on the accent, its icon takes the text's colour. */
  .block .icon,
  .strap .icon {
    stroke: var(--ink);
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
      transform: translateY(calc(18 * var(--u)));
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
      transform: translateY(calc(46 * var(--u)));
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
