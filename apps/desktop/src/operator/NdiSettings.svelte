<!--
  @component
  NDI: send the stream look (and, if wanted, the projector's picture) to
  vMix, OBS, a TriCaster or any NDI receiver on the network. Needs the free
  NDI Runtime or NDI Tools. Shown in the toolbar's Stream panel and in
  Settings › Stream.
-->
<script lang="ts">
  import { openProjectPage } from "../lib/api.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { ndi } from "../lib/ndi.svelte.ts";

  const status = $derived(ndi.status);
  const settings = $derived(status?.settings ?? { enabled: false, main: false });
</script>

{#if status && !status.supported}
  <p class="hint">{t("ndi.notSupported")}</p>
{:else if status}
  <label class="option">
    <input
      type="checkbox"
      checked={settings.enabled}
      onchange={(e) => ndi.set({ ...settings, enabled: e.currentTarget.checked })}
    >
    {t("ndi.send")}
  </label>
  {#if settings.enabled}
    <label class="option indent">
      <input
        type="checkbox"
        checked={settings.main}
        onchange={(e) => ndi.set({ ...settings, main: e.currentTarget.checked })}
      >
      {t("ndi.sendMain")}
    </label>
    {#if status.error}
      <p class="problem">
        {status.runtime ? status.error : t("ndi.noRuntime")}
        {#if !status.runtime}
          <button type="button" class="link" onclick={() => openProjectPage("ndi")}>
            {t("ndi.getRuntime")}
          </button>
        {/if}
      </p>
    {:else}
      <ul class="sources">
        {#each status.sources as source (source.source)}
          <li>
            <span class="name">{source.name}</span>
            <span class="receivers">{t("ndi.receivers", { count: source.receivers })}</span>
          </li>
        {/each}
      </ul>
      <p class="hint">{t("ndi.hint")}</p>
    {/if}
  {/if}
{/if}

<style>
  .option {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    margin-bottom: var(--sw-space-2);
  }

  .indent {
    margin-left: var(--sw-space-4);
  }

  .hint {
    margin: var(--sw-space-1) 0 var(--sw-space-2);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .problem {
    margin: var(--sw-space-1) 0 var(--sw-space-2);
    color: var(--sw-warning, #f59f00);
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--sw-accent);
    text-decoration: underline;
    cursor: pointer;
  }

  .sources {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .sources li {
    display: flex;
    justify-content: space-between;
    gap: var(--sw-space-2);
    padding: 2px 0;
  }

  .receivers {
    color: var(--sw-text-muted);
  }
</style>
