<!--
  @component
  The stage display's timer and messages, from a phone (full control).
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import { formatClock, stageTimerMs } from "@simpleworship/renderer";
  import { onMount } from "svelte";
  import { remote } from "../lib/remote.svelte.ts";

  let minutes = $state(30);
  let message = $state("");
  let now = $state(Date.now());

  const timer = $derived(remote.stage.timer);
  const shown = $derived(timer ? stageTimerMs(timer, now) : 0);
  const quick = $derived([
    t("stage.quick5"),
    t("stage.quick2"),
    t("stage.quickWrap"),
    t("stage.quickLouder"),
  ]);

  onMount(() => {
    const tick = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(tick);
  });

  function send(text: string) {
    void remote.dispatch({ type: "setStageMessage", message: text.trim() || null });
    message = "";
  }
</script>

<section class="stage">
  <h2>{t("stage.timer")}</h2>
  {#if timer}
    <div class="clock" class:over={shown < 0}>{formatClock(shown)}</div>
    <div class="row">
      {#if timer.running}
        <button type="button" onclick={() => void remote.dispatch({ type: "pauseStageTimer" })}>
          {t("common.pause")}
        </button>
      {:else}
        <button type="button" onclick={() => void remote.dispatch({ type: "resumeStageTimer" })}>
          {t("stage.resume")}
        </button>
      {/if}
      <button type="button" onclick={() => void remote.dispatch({ type: "clearStageTimer" })}>
        {t("common.clear")}
      </button>
    </div>
  {:else}
    <div class="row">
      <button
        type="button"
        onclick={() =>
          void remote.dispatch({ type: "startStageTimer", mode: "countUp", durationMs: 0 })}
      >
        {t("stage.countUp")}
      </button>
      <input
        type="number"
        inputmode="numeric"
        min="1"
        max="240"
        bind:value={minutes}
        aria-label={t("stage.minutes")}
      >
      <button
        type="button"
        onclick={() =>
          void remote.dispatch({
            type: "startStageTimer",
            mode: "countDown",
            durationMs: Math.max(1, Math.round(minutes)) * 60_000,
          })}
      >
        {t("stage.countDown")}
      </button>
    </div>
  {/if}

  <h2>{t("stage.messageTitle")}</h2>
  {#if remote.stage.message}
    <div class="row showing">
      <span>{t("stage.quoted", { text: remote.stage.message })}</span>
      <button type="button" onclick={() => send("")}>{t("common.clear")}</button>
    </div>
  {/if}
  <div class="quick">
    {#each quick as text (text)}
      <button type="button" onclick={() => send(text)}>{text}</button>
    {/each}
  </div>
  <form
    class="row"
    onsubmit={(event) => {
      event.preventDefault();
      send(message);
    }}
  >
    <input bind:value={message} placeholder={t("stage.placeholder")} class="message">
    <button type="submit" disabled={!message.trim()}>{t("common.send")}</button>
  </form>
</section>

<style>
  .stage {
    padding: 12px;
  }

  h2 {
    margin: 12px 0 8px;
    font-size: 15px;
    color: var(--sw-text-muted);
  }

  .clock {
    font-size: 48px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .over {
    color: var(--sw-live);
  }

  .row,
  .quick {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 8px;
  }

  button,
  input {
    min-height: 48px;
    padding: 0 14px;
    border: 1px solid var(--sw-border);
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
  }

  input[type="number"] {
    width: 80px;
  }

  .message {
    flex: 1;
    min-width: 0;
  }

  .showing {
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    border-radius: 10px;
    background: rgba(245, 159, 0, 0.15);
  }
</style>
