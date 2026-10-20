<!--
  @component
  The stage display on a tablet: the live words large, the next slide
  below, the clock, the speaker's timer and messages from the operator.
  View only. Keep the tablet's auto-lock off during the service.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import type { StageOptions } from "@simpleworship/protocol";
  import {
    formatClock,
    remainingMs,
    now as serviceNow,
    stageTimerMs,
  } from "@simpleworship/renderer";
  import { onMount } from "svelte";
  import { remote } from "../lib/remote.svelte.ts";

  let now = $state(serviceNow());
  let options: StageOptions = $state({ showClock: true, showNext: true, textScale: 100 });

  const live = $derived(remote.live);
  const slides = $derived(live?.presentation.slides ?? []);
  const index = $derived(live?.selected ?? 0);
  const current = $derived(slides[index] ?? null);
  const next = $derived(slides[index + 1] ?? null);
  const flags = $derived(remote.output?.flags);
  const countdown = $derived(remote.output?.frame?.countdown ?? null);
  const timer = $derived(remote.stage.timer);
  const timerMs = $derived(timer ? stageTimerMs(timer, now) : 0);
  const time = $derived(
    new Date(now).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
  );

  const status = $derived.by(() => {
    if (flags?.black) return t("stageView.black");
    if (flags?.logo) return t("stageView.logo");
    if (flags?.hideText) return t("stageView.textHidden");
    return null;
  });

  onMount(() => {
    const tick = setInterval(() => (now = serviceNow()), 250);
    remote
      .call<StageOptions>("stage.options")
      .then((saved) => (options = saved))
      .catch(() => {});
    return () => clearInterval(tick);
  });
</script>

<main class="stage" style:--scale={options.textScale / 100}>
  <header>
    <span class="title">{live?.presentation.title ?? ""}</span>
    {#if status}
      <span class="status">{status}</span>
    {/if}
    {#if timer}
      <span class="timer" class:over={timerMs < 0}>{formatClock(timerMs)}</span>
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
    {:else}
      <p>{current?.text ?? ""}</p>
    {/if}
  </section>

  {#if options.showNext}
    <section class="next">
      <span class="section">
        {next?.section
          ? t("stageView.nextSection", { section: next.section.name })
          : t("stageView.next")}
      </span>
      <p>{next?.text || (next ? t("stageView.picture") : "—")}</p>
    </section>
  {/if}

  {#if remote.stage.message}
    <aside class="message">{remote.stage.message}</aside>
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
  }

  header {
    display: flex;
    align-items: center;
    gap: 2vw;
    padding: 1.5vh 2vw;
    padding-top: max(1.5vh, env(safe-area-inset-top));
    font-size: 3vh;
    border-bottom: 1px solid #333;
  }

  .title {
    flex: 1;
    color: #aaa;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status {
    padding: 0.2em 0.6em;
    border-radius: 0.3em;
    background: var(--sw-live);
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
  }

  .timer.over {
    background: var(--sw-live);
  }

  section {
    padding: 2vh 3vw;
    overflow: hidden;
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
    font-size: calc(16vh * var(--scale));
    font-variant-numeric: tabular-nums;
  }

  .next {
    border-top: 1px solid #333;
    color: #9a9a9a;
  }

  .next p {
    font-size: calc(4.5vh * var(--scale));
  }

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
    font-size: calc(8vh * var(--scale));
    font-weight: 800;
    text-align: center;
  }
</style>
