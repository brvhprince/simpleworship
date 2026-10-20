<!--
  @component
  Alerts from a phone, for the nursery and ushers: pick a template, type
  the number (a big number pad comes up), Show. Or type any message.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import type { AlertTemplate } from "@simpleworship/protocol";
  import { onMount } from "svelte";
  import { remote } from "../lib/remote.svelte.ts";

  let templates: AlertTemplate[] = $state([]);
  let chosen = $state<AlertTemplate | null>(null);
  let blank = $state("");
  let custom = $state("");

  const showing = $derived(remote.output?.alert ?? null);
  const needsBlank = $derived(chosen?.text.includes("___") ?? false);

  let blankInput: HTMLInputElement | null = $state(null);

  // After choosing a template, typing the number is all that is left to do.
  $effect(() => {
    blankInput?.focus();
  });

  onMount(async () => {
    templates = await remote.call<AlertTemplate[]>("alerts.templates").catch(() => []);
  });

  function show() {
    const template = chosen;
    const text = template
      ? needsBlank
        ? template.text.replace("___", blank.trim())
        : template.text
      : custom.trim();
    if (!text) return;
    const request = template
      ? { ...template, text }
      : {
          text,
          style: "ticker" as const,
          position: "bottom" as const,
          end: { type: "afterPasses" as const, passes: 3 },
          background: "rgba(0, 0, 0, 0.82)",
          color: "#ffffff",
        };
    void remote.dispatch({ type: "showAlert", alert: request });
    blank = "";
    custom = "";
    chosen = null;
  }
</script>

<section class="alerts">
  {#if showing}
    <div class="showing">
      <span>{t("alert.onScreen", { text: showing.text })}</span>
      <button type="button" onclick={() => void remote.dispatch({ type: "clearAlert" })}>
        {t("common.clear")}
      </button>
    </div>
  {/if}

  <div class="templates">
    {#each templates as template (template.name)}
      <button
        type="button"
        class:active={chosen === template}
        onclick={() => (chosen = chosen === template ? null : template)}
      >
        {template.name}
      </button>
    {/each}
  </div>

  {#if chosen}
    <p class="text">{chosen.text}</p>
    {#if needsBlank}
      <input
        class="blank"
        bind:this={blankInput}
        bind:value={blank}
        inputmode="numeric"
        aria-label={t("alert.fillIn")}
        placeholder="—"
      >
    {/if}
  {:else}
    <textarea bind:value={custom} rows="3" placeholder={t("alert.placeholder")}></textarea>
  {/if}

  <button
    type="button"
    class="show"
    disabled={chosen ? needsBlank && !blank.trim() : !custom.trim()}
    onclick={show}
  >
    {t("alert.show")}
  </button>
</section>

<style>
  .alerts {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
  }

  .showing {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 12px;
    border-radius: 10px;
    background: var(--sw-live-soft);
  }

  .showing button {
    min-height: 44px;
    padding: 0 16px;
    border: none;
    border-radius: 8px;
    background: var(--sw-live);
    color: #fff;
  }

  .templates {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .templates button {
    min-height: 56px;
    border: 1px solid var(--sw-border);
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
  }

  .templates button.active {
    border-color: var(--sw-accent);
    color: var(--sw-accent);
  }

  .text {
    margin: 0;
    color: var(--sw-text-muted);
  }

  .blank {
    min-height: 72px;
    border: 1px solid var(--sw-border);
    border-radius: 12px;
    background: var(--sw-surface);
    color: var(--sw-text);
    font-size: 40px;
    text-align: center;
  }

  textarea {
    padding: 12px;
    border: 1px solid var(--sw-border);
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
    resize: none;
  }

  .show {
    min-height: 60px;
    border: none;
    border-radius: 12px;
    background: var(--sw-accent);
    color: #fff;
    font-size: 18px;
    font-weight: 700;
  }

  .show:disabled {
    opacity: 0.4;
  }
</style>
