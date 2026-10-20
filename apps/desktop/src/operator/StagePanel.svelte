<!--
  @component
  The stage display's extras: a timer for the speaker and messages only the
  people on stage see ("Wrap up", "5 minutes").
-->
<script lang="ts">
  import { formatClock, now as serviceNow, stageTimerMs } from "@simpleworship/renderer";
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";

  /** One-click messages, in the interface language. */
  const quickMessages = $derived([
    t("stage.quick5"),
    t("stage.quick2"),
    t("stage.quickWrap"),
    t("stage.quickLouder"),
  ]);

  let minutes = $state(30);
  let message = $state("");
  let now = $state(serviceNow());

  const timer = $derived(engine.stage.timer);
  const shown = $derived(timer ? stageTimerMs(timer, now) : 0);

  onMount(() => {
    const tick = setInterval(() => (now = serviceNow()), 250);
    return () => clearInterval(tick);
  });

  function sendMessage(text: string) {
    engine.dispatch({ type: "setStageMessage", message: text.trim() || null });
    message = "";
  }
</script>

<h4>{t("stage.timer")}</h4>
{#if timer}
  <div class="timer" class:over={shown < 0}>{formatClock(shown)}</div>
  <div class="row">
    {#if timer.running}
      <Button onclick={() => engine.dispatch({ type: "pauseStageTimer" })}
        >{t("common.pause")}</Button
      >
    {:else}
      <Button onclick={() => engine.dispatch({ type: "resumeStageTimer" })}
        >{t("stage.resume")}</Button
      >
    {/if}
    <Button onclick={() => engine.dispatch({ type: "clearStageTimer" })}
      >{t("common.clear")}</Button
    >
  </div>
{:else}
  <div class="row">
    <Button
      onclick={() => engine.dispatch({ type: "startStageTimer", mode: "countUp", durationMs: 0 })}
    >
      {t("stage.countUp")}
    </Button>
    <span class="or">{t("stage.or")}</span>
    <input
      class="minutes"
      type="number"
      min="1"
      max="240"
      bind:value={minutes}
      aria-label={t("stage.minutes")}
    >
    <span class="or">{t("stage.minutesUnit")}</span>
    <Button
      onclick={() =>
        engine.dispatch({
          type: "startStageTimer",
          mode: "countDown",
          durationMs: Math.max(1, Math.round(minutes)) * 60_000,
        })}
    >
      {t("stage.countDown")}
    </Button>
  </div>
{/if}

<h4>{t("stage.messageTitle")}</h4>
{#if engine.stage.message}
  <div class="row showing">
    <span>{t("stage.quoted", { text: engine.stage.message })}</span>
    <Button onclick={() => sendMessage("")}>{t("common.clear")}</Button>
  </div>
{/if}
<div class="row">
  {#each quickMessages as quick (quick)}
    <Button onclick={() => sendMessage(quick)}>{quick}</Button>
  {/each}
</div>
<div class="row">
  <input
    class="message"
    bind:value={message}
    placeholder={t("stage.placeholder")}
    onkeydown={(e) => e.key === "Enter" && sendMessage(message)}
  >
  <Button variant="primary" disabled={!message.trim()} onclick={() => sendMessage(message)}
    >{t("common.send")}</Button
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
