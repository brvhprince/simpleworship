<!--
  @component
  The browser source: turn it on, and copy the link to paste into OBS,
  vMix or Streamlabs (as a Browser source of 1920×1080). Shown in the
  toolbar's Stream panel and in Settings › Stream.
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { link } from "../lib/link.svelte.ts";
  import { remote } from "../lib/remote.svelte.ts";

  const status = $derived(remote.status);
  const enabled = $derived(status?.settings.browserSource ?? false);
  const links = $derived(status?.streamLinks ?? []);

  /** The link last copied, to say so for a moment. */
  let copied: string | null = $state(null);

  async function copy(link: string) {
    try {
      await navigator.clipboard.writeText(link);
      copied = link;
      setTimeout(() => {
        if (copied === link) copied = null;
      }, 2000);
    } catch {
      // The link stays selectable to copy by hand.
    }
  }
</script>

{#if link.joined}
  <p class="hint">{t("link.browserSourceThere", { name: link.status.hostName })}</p>
{:else}
  <label class="option">
    <input
      type="checkbox"
      checked={enabled}
      onchange={(e) => remote.setBrowserSource(e.currentTarget.checked)}
    >
    {t("stream.browserSource")}
  </label>

  {#if enabled}
    {#if links.length > 0}
      <p class="hint">{t("stream.browserSourceHint")}</p>
      <ul class="links">
        {#each links as link, index (link)}
          <li>
            <span class="where"
              >{index === 0 ? t("stream.thisComputer") : t("stream.otherComputer")}</span
            >
            <code>{link}</code>
            <Button onclick={() => copy(link)}>
              {copied === link ? t("stream.copied") : t("stream.copy")}
            </Button>
          </li>
        {/each}
      </ul>
      <Button onclick={() => remote.renewStreamLink()}>{t("stream.newLink")}</Button>
      <p class="hint">{t("stream.newLinkHint")}</p>
    {:else}
      <p class="hint">{status?.error ?? t("remotePanel.starting")}</p>
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

  .hint {
    margin: var(--sw-space-2) 0;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .links {
    list-style: none;
    margin: 0 0 var(--sw-space-2);
    padding: 0;
  }

  .links li {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--sw-space-2);
    margin-bottom: var(--sw-space-1);
  }

  .where {
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
  }

  code {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--sw-font-mono);
    font-size: var(--sw-font-size-small);
    user-select: all;
  }
</style>
