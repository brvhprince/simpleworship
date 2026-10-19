<!--
  @component
  The church's settings: name and CCLI licence (shown in song footers),
  the alert templates in the Alert panel, and the stage display options.
-->
<script lang="ts">
  import type { AlertTemplate, AppSettings } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { getSettings, saveSettings } from "../lib/api.ts";
  import { dialogs } from "../lib/dialogs.svelte.ts";
  import { engine } from "../lib/engine.svelte.ts";

  let settings: AppSettings | null = $state(null);

  onMount(async () => {
    settings = await getSettings();
  });

  function addTemplate() {
    settings?.alertTemplates.push({
      name: "New alert",
      text: "",
      style: "ticker",
      position: "bottom",
      end: { type: "afterPasses", passes: 3 },
    });
  }

  function removeTemplate(template: AlertTemplate) {
    if (!settings) return;
    settings.alertTemplates = settings.alertTemplates.filter((other) => other !== template);
  }

  async function save() {
    if (!settings) return;
    try {
      await saveSettings($state.snapshot(settings));
      dialogs.close();
    } catch (error) {
      engine.showError(error);
    }
  }
</script>

<Dialog title="Settings" onclose={() => dialogs.close()}>
  {#if settings}
    <div class="settings">
      <section>
        <h3>Church</h3>
        <label class="field">
          Name
          <input bind:value={settings.churchName} placeholder="e.g. Grace Chapel, Kumasi">
        </label>
        <label class="field">
          CCLI licence number
          <input bind:value={settings.ccliLicense} placeholder="e.g. 1234567">
          <span class="hint">
            Added to every song's copyright line as “CCLI License # …”. Leave empty to leave it off.
          </span>
        </label>
      </section>

      <section>
        <h3>Alert templates</h3>
        <p class="hint">
          One click in the Alert panel. Put <code>___</code> where a number or name goes, e.g.
          “Parent of child ___, please come to the nursery”.
        </p>
        {#each settings.alertTemplates as template, index (index)}
          <div class="template">
            <input class="template-name" bind:value={template.name} aria-label="Name">
            <input class="template-text" bind:value={template.text} aria-label="Message">
            <select bind:value={template.style} aria-label="Style">
              <option value="ticker">Scrolling</option>
              <option value="banner">Still</option>
            </select>
            <select bind:value={template.position} aria-label="Position">
              <option value="bottom">Bottom</option>
              <option value="top">Top</option>
            </select>
            <Button onclick={() => removeTemplate(template)}>Remove</Button>
          </div>
        {/each}
        <Button onclick={addTemplate}>Add Template</Button>
      </section>

      <section>
        <h3>Stage display</h3>
        <label class="option">
          <input type="checkbox" bind:checked={settings.stage.showClock}>
          Show the time of day
        </label>
        <label class="option">
          <input type="checkbox" bind:checked={settings.stage.showNext}>
          Show the next slide
        </label>
        <label class="field narrow">
          Text size
          <select bind:value={settings.stage.textScale}>
            <option value={80}>Smaller</option>
            <option value={100}>Normal</option>
            <option value={125}>Larger</option>
            <option value={150}>Largest</option>
          </select>
        </label>
      </section>
    </div>
  {/if}

  {#snippet actions()}
    <Button onclick={() => dialogs.close()}>Cancel</Button>
    <Button variant="primary" disabled={!settings} onclick={save}>Save</Button>
  {/snippet}
</Dialog>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-4);
  }

  h3 {
    margin: 0 0 var(--sw-space-2);
    font-size: var(--sw-font-size-large, 15px);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    max-width: 420px;
    margin-bottom: var(--sw-space-3);
    color: var(--sw-text-muted);
  }

  .field input,
  .field select {
    color: var(--sw-text);
  }

  .narrow {
    max-width: 180px;
  }

  .hint {
    margin: 0 0 var(--sw-space-2);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .template {
    display: flex;
    gap: var(--sw-space-1);
    margin-bottom: var(--sw-space-2);
  }

  .template-name {
    width: 140px;
  }

  .template-text {
    flex: 1;
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    margin-bottom: var(--sw-space-2);
  }
</style>
