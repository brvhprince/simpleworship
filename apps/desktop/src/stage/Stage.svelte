<!--
  @component
  The stage display ("confidence monitor") for musicians and speakers:
  the live slide in large type, the next slide underneath, and a clock.
  It shows text only, never backgrounds, so it is readable from a distance.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { closeOnEscape } from "../lib/output-window.ts";

  let now = $state(new Date());

  onMount(() => {
    const connection = engine.connect();
    const clock = setInterval(() => (now = new Date()), 1000);
    return () => {
      clearInterval(clock);
      void connection.then((disconnect) => disconnect());
    };
  });

  const slides = $derived(engine.live?.presentation.slides ?? []);
  const index = $derived(engine.live?.selected ?? 0);
  const current = $derived(slides[index] ?? null);
  const next = $derived(slides[index + 1] ?? null);
  const flags = $derived(engine.output?.flags);

  /** A short status note when the audience is not seeing the slide text. */
  const status = $derived.by(() => {
    if (flags?.black) return "Screen is black";
    if (flags?.logo) return "Showing logo";
    if (flags?.hideText) return "Text hidden";
    return null;
  });

  const time = $derived(now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }));
</script>

<svelte:window onkeydown={closeOnEscape("stage")} />

<main class="stage">
  <header>
    <span class="title">{engine.live?.presentation.title ?? ""}</span>
    {#if status}
      <span class="status">{status}</span>
    {/if}
    <span class="clock">{time}</span>
  </header>

  <section class="current">
    {#if current?.section}
      <span class="section">{current.section.name}</span>
    {/if}
    <p>{current?.text ?? ""}</p>
  </section>

  <section class="next">
    <span class="section">Next{next?.section ? ` · ${next.section.name}` : ""}</span>
    <p>{next?.text ?? "—"}</p>
  </section>
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

  .clock {
    font-variant-numeric: tabular-nums;
    font-weight: 700;
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
    font-size: 7vh;
    font-weight: 600;
  }

  .next {
    border-top: 1px solid #333;
    color: #9a9a9a;
  }

  .next p {
    font-size: 4.5vh;
  }
</style>
