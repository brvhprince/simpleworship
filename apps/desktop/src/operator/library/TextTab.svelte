<!--
  @component
  Put any text on screen: announcements, a prayer, a quick lyric that is not
  in the library. Blank lines separate slides. The text can be shown right
  away or added to the schedule.
-->
<script lang="ts">
  import type { CountdownEnd, CountdownTarget } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import { addCountdownToSchedule, addTextToSchedule, showTextInPreview } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { withModifier } from "../../lib/platform.ts";

  let title = $state("");
  let text = $state("");

  // Countdown settings.
  let countdownMessage = $state("Service starts in");
  let countdownMode: CountdownTarget["type"] = $state("timeOfDay");
  let countdownMinutes = $state(5);
  let countdownTime = $state("10:30");
  let countdownNext = $state(false);

  async function addCountdown() {
    let target: CountdownTarget;
    let label: string;
    if (countdownMode === "duration") {
      target = { type: "duration", seconds: Math.max(1, Math.round(countdownMinutes * 60)) };
      label = `Countdown (${countdownMinutes} min)`;
    } else {
      const [hour = 0, minute = 0] = countdownTime.split(":").map(Number);
      target = { type: "timeOfDay", hour, minute };
      label = `Countdown to ${countdownTime}`;
    }
    const atEnd: CountdownEnd = countdownNext ? "nextItem" : "stay";
    try {
      await addCountdownToSchedule(label, { message: countdownMessage, target, atEnd });
    } catch (error) {
      engine.showError(error);
    }
  }

  async function preview() {
    try {
      await showTextInPreview(title.trim() || "Text", text);
    } catch (error) {
      engine.showError(error);
    }
  }

  async function addToSchedule() {
    try {
      await addTextToSchedule(title.trim() || "Text", text);
    } catch (error) {
      engine.showError(error);
    }
  }

  /** Cmd/Ctrl+Enter previews without reaching for the mouse. */
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      preview();
    }
  }
</script>

<div class="tab">
  <div class="text-tab">
    <input bind:value={title} placeholder="Title (optional)" aria-label="Title">
    <textarea
      bind:value={text}
      onkeydown={handleKeydown}
      placeholder="Type the text to show. Leave a blank line between slides."
      aria-label="Text"
    ></textarea>
    <div class="actions">
      <span class="hint">{withModifier("Enter")}</span>
      <Button disabled={!text.trim()} onclick={addToSchedule}>Add to Schedule</Button>
      <Button variant="primary" disabled={!text.trim()} onclick={preview}>Show in Preview</Button>
    </div>
  </div>

  <section class="countdown">
    <h4>Countdown</h4>
    <div class="row">
      <input bind:value={countdownMessage} aria-label="Countdown message" class="message">
      <select bind:value={countdownMode} aria-label="Count down">
        <option value="timeOfDay">until</option>
        <option value="duration">for</option>
      </select>
      {#if countdownMode === "timeOfDay"}
        <input type="time" bind:value={countdownTime} aria-label="Time">
      {:else}
        <input
          type="number"
          min="1"
          max="240"
          bind:value={countdownMinutes}
          aria-label="Minutes"
          class="minutes"
        >
        <span class="hint">min</span>
      {/if}
      <label class="option">
        <input type="checkbox" bind:checked={countdownNext}>
        then go to the next item
      </label>
      <Button onclick={addCountdown}>Add to Schedule</Button>
    </div>
  </section>
</div>

<style>
  .tab {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .text-tab {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    padding: var(--sw-space-3);
  }

  .countdown {
    padding: var(--sw-space-2) var(--sw-space-3) var(--sw-space-3);
    border-top: 1px solid var(--sw-border);
  }

  .countdown h4 {
    margin: 0 0 var(--sw-space-2);
    font-size: var(--sw-font-size);
    color: var(--sw-text-muted);
  }

  .countdown .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .countdown .message {
    width: 220px;
  }

  .countdown .minutes {
    width: 64px;
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  input {
    max-width: 400px;
  }

  textarea {
    flex: 1;
    resize: none;
    line-height: 1.5;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: var(--sw-space-3);
  }

  .hint {
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }
</style>
