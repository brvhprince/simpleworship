<!--
  @component
  The toolbar's Lower Third panel: show a saved lower third with one click,
  hide the one on screen, and open the editor to change or add one. The
  list is kept with the settings (see `lib/lower-thirds.svelte.ts`).
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { lowerThirds } from "../lib/lower-thirds.svelte.ts";
  import { panels } from "../lib/panels.svelte.ts";
  import { KIND_LABELS } from "./lower-third-labels.ts";

  const showing = $derived(engine.output?.lowerThird ?? null);

  onMount(() => void lowerThirds.load());

  /** Opens the editor; the panel closes, so the dialog has the screen. */
  function openEditor(open: () => void) {
    open();
    panels.close();
  }
</script>

<div class="lower-thirds">
  {#if showing}
    <div class="showing">
      <span>{t("lowerThird.onScreen", { title: showing.item.title })}</span>
      <Button variant="live" active onclick={() => lowerThirds.hide()}
        >{t("lowerThird.hide")}</Button
      >
    </div>
  {/if}

  {#if !lowerThirds.ready}
    <p class="hint">{t("lowerThird.loading")}</p>
  {:else if lowerThirds.list.length === 0}
    <p class="hint">{t("lowerThird.empty")}</p>
  {:else}
    <ul>
      {#each lowerThirds.list as item (item.id)}
        <li class:on={showing?.item.id === item.id}>
          <button
            type="button"
            class="item"
            title={t("lowerThird.showHint")}
            onclick={() => lowerThirds.show(item)}
          >
            <span class="kind">{t(KIND_LABELS[item.kind])}</span>
            <strong>{item.title}</strong>
            {#if item.subtitle}
              <span class="subtitle">{item.subtitle}</span>
            {/if}
          </button>
          <Button
            disabled={!lowerThirds.ready || lowerThirds.busy}
            onclick={() => openEditor(() => lowerThirds.edit(item))}
            >{t("common.edit")}</Button
          >
        </li>
      {/each}
    </ul>
  {/if}

  <Button
    disabled={!lowerThirds.ready || lowerThirds.busy}
    onclick={() => openEditor(() => lowerThirds.create())}
    >{t("lowerThird.new")}</Button
  >
</div>

<style>
  .lower-thirds {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-3);
  }

  .showing {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2);
    border-radius: var(--sw-radius-small);
    background: var(--sw-live-soft);
  }

  ul {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .item {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: var(--sw-space-2);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    text-align: left;
    cursor: pointer;
  }

  .item:hover {
    background: var(--sw-surface-hover);
  }

  li.on .item {
    border-color: var(--sw-live);
  }

  .item strong,
  .subtitle {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .kind {
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
    text-transform: uppercase;
  }

  .subtitle {
    color: var(--sw-text-muted);
  }

  .hint {
    margin: 0;
    color: var(--sw-text-muted);
  }
</style>
