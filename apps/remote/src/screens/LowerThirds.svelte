<!--
  @component
  Lower thirds from a phone (full control only): tap one to show it on the
  outputs it is set for, tap Hide to take it off. They are made and edited
  on the computer (Lower Third in the toolbar); this screen only shows them.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import type { LowerThird } from "@simpleworship/protocol";
  import { onMount } from "svelte";
  import { remote } from "../lib/remote.svelte.ts";

  let items: LowerThird[] = $state([]);
  let loaded = $state(false);

  const showing = $derived(remote.output?.lowerThird ?? null);

  onMount(async () => {
    items = await remote.call<LowerThird[]>("lowerThirds.list").catch(() => []);
    loaded = true;
  });

  function show(item: LowerThird) {
    void remote.dispatch({ type: "showLowerThird", item: $state.snapshot(item) as LowerThird });
  }
</script>

<section class="lower-thirds">
  {#if showing}
    <div class="showing">
      <span>{t("lowerThird.onScreen", { title: showing.item.title })}</span>
      <button type="button" onclick={() => void remote.dispatch({ type: "hideLowerThird" })}>
        {t("lowerThird.hide")}
      </button>
    </div>
  {/if}

  {#if loaded && items.length === 0}
    <p class="hint">{t("remote.noLowerThirds")}</p>
  {/if}

  <div class="items">
    {#each items as item (item.id)}
      <button type="button" class:on={showing?.item.id === item.id} onclick={() => show(item)}>
        <strong>{item.title}</strong>
        {#if item.subtitle}
          <span>{item.subtitle}</span>
        {/if}
      </button>
    {/each}
  </div>
</section>

<style>
  .lower-thirds {
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
    flex: none;
    min-height: 44px;
    padding: 0 16px;
    border: none;
    border-radius: 8px;
    background: var(--sw-live);
    color: #fff;
  }

  .items {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .items button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    min-height: 64px;
    padding: 10px 14px;
    border: 1px solid var(--sw-border);
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
    text-align: left;
  }

  .items button.on {
    border-color: var(--sw-live);
  }

  .items strong {
    font-size: 17px;
  }

  .items span {
    color: var(--sw-text-muted);
  }

  .hint {
    margin: 0;
    color: var(--sw-text-muted);
  }
</style>
