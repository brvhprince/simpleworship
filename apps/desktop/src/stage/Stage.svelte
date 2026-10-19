<!--
  @component
  The stage display ("confidence monitor") for musicians and speakers:
  the live slide in large type, the next slide underneath, the next
  schedule item, a clock, the speaker's timer, and messages from the
  operator ("Wrap up").
  It shows text only, never backgrounds, so it is readable from a distance.
  What it shows is set in Settings.
-->
<script lang="ts">
  import type { AppSettings, StageOptions } from "@simpleworship/protocol";
  import { formatClock, getMediaUrls, remainingMs, stageTimerMs } from "@simpleworship/renderer";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { getSettings } from "../lib/api.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { closeOnEscape } from "../lib/output-window.ts";
  import { itemTitle } from "../lib/schedule.ts";

  /** Must match `SETTINGS_EVENT` in `src-tauri/src/commands/settings.rs`. */
  const SETTINGS_EVENT = "settings://changed";

  const urls = getMediaUrls();
  let now = $state(Date.now());
  let options: StageOptions = $state({ showClock: true, showNext: true, textScale: 100 });

  onMount(() => {
    const connection = engine.connect();
    const clock = setInterval(() => (now = Date.now()), 250);
    getSettings()
      .then((settings) => (options = settings.stage))
      .catch(() => {});
    const stopSettings = listen<AppSettings>(SETTINGS_EVENT, ({ payload }) => {
      options = payload.stage;
    });
    return () => {
      clearInterval(clock);
      void connection.then((disconnect) => disconnect());
      void stopSettings.then((unlisten) => unlisten());
    };
  });

  const slides = $derived(engine.live?.presentation.slides ?? []);
  const index = $derived(engine.live?.selected ?? 0);
  const current = $derived(slides[index] ?? null);
  const next = $derived(slides[index + 1] ?? null);
  const flags = $derived(engine.output?.flags);
  const countdown = $derived(engine.output?.frame?.countdown ?? null);
  const timer = $derived(engine.stage.timer);
  const timerMs = $derived(timer ? stageTimerMs(timer, now) : 0);

  /** A presentation page's picture, when the live slide is one. */
  const picture = $derived.by(() => {
    const background = current?.background;
    return background?.type === "image" ? urls.file(background.media) : null;
  });

  /** The schedule item after the live one, so the band knows what is coming. */
  const nextItem = $derived.by(() => {
    const items = engine.schedule.items;
    const index = items.findIndex((item) => item.id === engine.live?.scheduleItem);
    return index === -1 ? null : (items[index + 1] ?? null);
  });

  /** A short status note when the audience is not seeing the slide text. */
  const status = $derived.by(() => {
    if (flags?.black) return "Screen is black";
    if (flags?.logo) return "Showing logo";
    if (flags?.hideText) return "Text hidden";
    return null;
  });

  const time = $derived(
    new Date(now).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
  );
</script>

<svelte:window onkeydown={closeOnEscape("stage")} />

<main class="stage" class:no-next={!options.showNext} style:--scale={options.textScale / 100}>
  <header>
    <span class="title">{engine.live?.presentation.title ?? ""}</span>
    {#if status}
      <span class="status">{status}</span>
    {/if}
    {#if nextItem}
      <span class="next-item">Next: {itemTitle(nextItem)}</span>
    {/if}
    {#if timer}
      <span class="timer" class:over={timerMs < 0} class:paused={!timer.running}>
        {formatClock(timerMs)}
      </span>
    {/if}
    {#if options.showClock}
      <span class="clock">{time}</span>
    {/if}
  </header>

  <section class="current">
    {#if current?.section}
      <span class="section">{current.section.name}</span>
    {/if}
    {#if countdown}
      <p>{countdown.message}</p>
      <p class="countdown">{formatClock(remainingMs(countdown.endsAt, now))}</p>
    {:else if picture}
      <img src={picture} alt="">
      <span class="section">Slide {index + 1} of {slides.length}</span>
    {:else}
      <p>{current?.text ?? ""}</p>
    {/if}
  </section>

  {#if options.showNext}
    <section class="next">
      <span class="section">Next{next?.section ? ` · ${next.section.name}` : ""}</span>
      <p>{next?.text || (next ? "(picture)" : "—")}</p>
    </section>
  {/if}

  {#if engine.stage.message}
    <aside class="message">{engine.stage.message}</aside>
  {/if}
</main>

<style>
  .stage {
    position: fixed;
    inset: 0;
    display: grid;
    grid-template-rows: auto 3fr 2fr;
    background: #000;
    color: #fff;
    font-family: var(--sw-font);
    cursor: none;
  }

  .stage.no-next {
    grid-template-rows: auto 1fr;
  }

  header {
    display: flex;
    align-items: center;
    gap: 2vw;
    padding: 1.5vh 2vw;
    font-size: 3vh;
    border-bottom: 1px solid #333;
  }

  .title {
    flex: 1;
    color: #aaa;
  }

  .status {
    padding: 0.2em 0.6em;
    border-radius: 0.3em;
    background: var(--sw-live);
  }

  .next-item {
    color: #f59f00;
  }

  .clock,
  .timer {
    font-variant-numeric: tabular-nums;
    font-weight: 700;
  }

  .timer {
    padding: 0.1em 0.5em;
    border-radius: 0.3em;
    background: #1c7ed6;
    font-size: 1.3em;
  }

  .timer.over {
    background: var(--sw-live);
  }

  .timer.paused {
    opacity: 0.6;
  }

  section {
    padding: 2vh 3vw;
    overflow: hidden;
    min-height: 0;
  }

  .current {
    display: flex;
    flex-direction: column;
  }

  .section {
    color: #f59f00;
    font-size: 2.6vh;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  p {
    margin: 1vh 0 0;
    white-space: pre-line;
    line-height: 1.25;
  }

  .current p {
    font-size: calc(7vh * var(--scale));
    font-weight: 600;
  }

  .current .countdown {
    font-size: calc(18vh * var(--scale));
    font-variant-numeric: tabular-nums;
  }

  .current img {
    flex: 1;
    min-height: 0;
    object-fit: contain;
    margin-bottom: 1vh;
  }

  .next {
    border-top: 1px solid #333;
    color: #9a9a9a;
  }

  .next p {
    font-size: calc(4.5vh * var(--scale));
  }

  /* An operator message: unmissable, across the middle of the screen. */
  .message {
    position: fixed;
    left: 4vw;
    right: 4vw;
    top: 50%;
    transform: translateY(-50%);
    padding: 3vh 3vw;
    border-radius: 1.5vh;
    background: #f59f00;
    color: #000;
    font-size: calc(9vh * var(--scale));
    font-weight: 800;
    text-align: center;
    animation: pulse 1.6s ease-in-out infinite;
  }

  @keyframes pulse {
    50% {
      opacity: 0.75;
    }
  }
</style>
