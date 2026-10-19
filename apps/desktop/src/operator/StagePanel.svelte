<!--
  @component
  The stage display's extras: a timer for the speaker and messages only the
  people on stage see ("Wrap up", "5 minutes").
-->
<script lang="ts">
  import { formatClock, stageTimerMs } from "@simpleworship/renderer";
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";

  const QUICK_MESSAGES = ["5 minutes", "2 minutes", "Wrap up", "Please speak up"];

  let minutes = $state(30);
  let message = $state("");
  let now = $state(Date.now());

  const timer = $derived(engine.stage.timer);
  const shown = $derived(timer ? stageTimerMs(timer, now) : 0);

  onMount(() => {
    const tick = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(tick);
  });

  function sendMessage(text: string) {
    engine.dispatch({ type: "setStageMessage", message: text.trim() || null });
    message = "";
  }
</script>

<h4>Timer</h4>
{#if timer}
  <div class="timer" class:over={shown < 0}>{formatClock(shown)}</div>
  <div class="row">
    {#if timer.running}
      <Button onclick={() => engine.dispatch({ type: "pauseStageTimer" })}>Pause</Button>
    {:else}
      <Button onclick={() => engine.dispatch({ type: "resumeStageTimer" })}>Resume</Button>
    {/if}
    <Button onclick={() => engine.dispatch({ type: "clearStageTimer" })}>Clear</Button>
  </div>
{:else}
  <div class="row">
    <Button
      onclick={() => engine.dispatch({ type: "startStageTimer", mode: "countUp", durationMs: 0 })}
    >
      Count Up
    </Button>
    <span class="or">or</span>
    <input
      class="minutes"
      type="number"
      min="1"
      max="240"
      bind:value={minutes}
      aria-label="Minutes"
    >
    <span class="or">min</span>
    <Button
      onclick={() =>
        engine.dispatch({
          type: "startStageTimer",
          mode: "countDown",
          durationMs: Math.max(1, Math.round(minutes)) * 60_000,
        })}
    >
      Count Down
    </Button>
  </div>
{/if}

<h4>Message to the stage</h4>
{#if engine.stage.message}
  <div class="row showing">
    <span>“{engine.stage.message}”</span>
    <Button onclick={() => sendMessage("")}>Clear</Button>
  </div>
{/if}
<div class="row">
  {#each QUICK_MESSAGES as quick (quick)}
    <Button onclick={() => sendMessage(quick)}>{quick}</Button>
  {/each}
</div>
<div class="row">
  <input
    class="message"
    bind:value={message}
    placeholder="Type a message"
    onkeydown={(e) => e.key === "Enter" && sendMessage(message)}
  >
  <Button variant="primary" disabled={!message.trim()} onclick={() => sendMessage(message)}
    >Send</Button
  >
</div>

<style>
  h4 {
    margin: 0 0 var(--sw-space-2);
    font-size: var(--sw-font-size);
    color: var(--sw-text-muted);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-1);
    margin-bottom: var(--sw-space-3);
  }

  .timer {
    font-size: 32px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    margin-bottom: var(--sw-space-2);
  }

  .over {
    color: var(--sw-live);
  }

  .or {
    color: var(--sw-text-muted);
  }

  .minutes {
    width: 56px;
  }

  .message {
    flex: 1;
  }

  .showing {
    justify-content: space-between;
    padding: var(--sw-space-2);
    border-radius: var(--sw-radius-small);
    background: rgba(245, 159, 0, 0.12);
  }
</style>
