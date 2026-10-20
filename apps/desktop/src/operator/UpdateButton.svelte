<!--
  @component
  Check for Updates, in About and Settings. It shows each step after the
  click: checking, downloading (with how far along), then Install once the
  new version is downloaded. Finding nothing new, or a failed check, is
  answered by `updates.checkNow` with a message.
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { updates } from "../lib/updates.svelte.ts";

  interface Props {
    /** The button's words before anything has started. */
    label: string;
  }

  let { label }: Props = $props();

  const percent = $derived(Math.round(updates.progress * 100));
</script>

{#if updates.status === "ready"}
  <Button variant="primary" onclick={() => updates.install()}>
    {t("updates.installVersion", { version: updates.version ?? "" })}
  </Button>
{:else}
  <Button disabled={updates.status !== "idle"} onclick={() => updates.checkNow()}>
    {#if updates.status === "checking"}
      {t("about.checking")}
    {:else if updates.status === "downloading"}
      {t("updates.downloading", { percent })}
    {:else if updates.status === "installing"}
      {t("toolbar.updateInstalling")}
    {:else}
      {label}
    {/if}
  </Button>
{/if}
