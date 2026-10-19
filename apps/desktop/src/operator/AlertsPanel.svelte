<!--
  @component
  Show an alert over the slides: pick a saved template (filling in the
  blank, e.g. a child's number) or type a message, then Show. Templates are
  edited in Settings.
-->
<script lang="ts">
  import type { AlertEnd, AlertPosition, AlertStyle, AlertTemplate } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { getSettings } from "../lib/api.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { panels } from "../lib/panels.svelte.ts";

  let templates: AlertTemplate[] = $state([]);
  let text = $state("");
  let blank = $state("");
  let template = $state<AlertTemplate | null>(null);
  let style: AlertStyle = $state("ticker");
  let position: AlertPosition = $state("bottom");
  let endType: AlertEnd["type"] = $state("afterPasses");
  let endAmount = $state(3);

  const showing = $derived(engine.output?.alert ?? null);
  const needsBlank = $derived(template?.text.includes("___") ?? false);

  let blankInput: HTMLInputElement | null = $state(null);

  // After choosing a template, typing the number is all that is left to do.
  $effect(() => {
    blankInput?.focus();
  });

  onMount(async () => {
    templates = (await getSettings()).alertTemplates;
  });

  function useTemplate(chosen: AlertTemplate) {
    template = chosen;
    style = chosen.style;
    position = chosen.position;
    endType = chosen.end.type;
    endAmount = amountOf(chosen.end) ?? endAmount;
    text = chosen.text;
    keepEndingValid();
  }

  /** The number in an ending ("3" passes, "10" seconds), if it has one. */
  function amountOf(ending: AlertEnd): number | null {
    switch (ending.type) {
      case "afterSeconds":
        return ending.seconds;
      case "afterPasses":
        return ending.passes;
      case "manual":
        return null;
    }
  }

  /** A still banner does not scroll, so "after scrolling" becomes seconds. */
  function keepEndingValid() {
    if (style === "banner" && endType === "afterPasses") {
      endType = "afterSeconds";
      endAmount = 10;
    }
  }

  function end(): AlertEnd {
    const amount = Math.max(1, Math.round(endAmount));
    if (endType === "afterSeconds") return { type: "afterSeconds", seconds: amount };
    if (endType === "afterPasses") return { type: "afterPasses", passes: amount };
    return { type: "manual" };
  }

  function show() {
    const message = needsBlank ? text.replace("___", blank.trim()) : text;
    if (!message.trim()) return;
    engine.dispatch({
      type: "showAlert",
      alert: { text: message.trim(), style, position, end: end() },
    });
    blank = "";
    panels.close();
  }
</script>

{#if showing}
  <div class="showing">
    <span>{t("alert.onScreen", { text: showing.text })}</span>
    <Button variant="live" active onclick={() => engine.dispatch({ type: "clearAlert" })}
      >{t("common.clear")}</Button
    >
  </div>
{/if}

{#if templates.length > 0}
  <div class="templates">
    {#each templates as saved (saved.name)}
      <Button active={template === saved} onclick={() => useTemplate(saved)}>{saved.name}</Button>
    {/each}
  </div>
{/if}

{#if needsBlank}
  <label class="field">
    {t("alert.fillIn")}
    <input bind:this={blankInput} bind:value={blank} onkeydown={(e) => e.key === "Enter" && show()}>
  </label>
{/if}

<label class="field">
  {t("alert.message")}
  <textarea bind:value={text} rows="2" placeholder={t("alert.placeholder")}></textarea>
</label>

<div class="row">
  <select bind:value={style} aria-label={t("alert.style")} onchange={keepEndingValid}>
    <option value="ticker">{t("alert.scrolling")}</option>
    <option value="banner">{t("alert.still")}</option>
  </select>
  <select bind:value={position} aria-label={t("alert.position")}>
    <option value="bottom">{t("alert.bottom")}</option>
    <option value="top">{t("alert.top")}</option>
  </select>
  <select bind:value={endType} aria-label={t("alert.ends")}>
    {#if style === "ticker"}
      <option value="afterPasses">{t("alert.afterScrolling")}</option>
    {/if}
    <option value="afterSeconds">{t("alert.afterSeconds")}</option>
    <option value="manual">{t("alert.whenCleared")}</option>
  </select>
  {#if endType !== "manual"}
    <input
      class="amount"
      type="number"
      min="1"
      bind:value={endAmount}
      aria-label={endType === "afterPasses" ? t("alert.times") : t("alert.seconds")}
    >
  {/if}
</div>

<div class="actions">
  <Button variant="primary" disabled={!text.trim() || (needsBlank && !blank.trim())} onclick={show}>
    {t("alert.show")}
  </Button>
</div>

<style>
  .showing {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sw-space-2);
    margin-bottom: var(--sw-space-3);
    padding: var(--sw-space-2);
    border-radius: var(--sw-radius-small);
    background: var(--sw-live-soft);
  }

  .templates {
    display: flex;
    flex-wrap: wrap;
    gap: var(--sw-space-1);
    margin-bottom: var(--sw-space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    margin-bottom: var(--sw-space-2);
    color: var(--sw-text-muted);
  }

  .field input,
  .field textarea {
    color: var(--sw-text);
    resize: none;
  }

  .row {
    display: flex;
    gap: var(--sw-space-1);
    margin-bottom: var(--sw-space-3);
  }

  .amount {
    width: 56px;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
