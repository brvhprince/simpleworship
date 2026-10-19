<!--
  @component
  Put any text on screen: announcements, a prayer, a quick lyric that is not
  in the library. Blank lines separate slides. The text can be shown right
  away or added to the schedule.
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { addTextToSchedule, showTextInPreview } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { withModifier } from "../../lib/platform.ts";

  let title = $state("");
  let text = $state("");

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

<style>
  .text-tab {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    height: 100%;
    padding: var(--sw-space-3);
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
