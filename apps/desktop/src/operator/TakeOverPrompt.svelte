<!--
  @component
  The main computer stopped answering: offers to carry on its service on
  this computer, from the last state heard (see `link::link_take_over`).
  Never automatic, so a short network blip never starts a second service.
  It goes away by itself if the main computer answers again; Keep Waiting
  hides it until the next time the connection is lost.

  Only one computer should take over: the prompt says so.
-->
<script lang="ts">
  import type { TakeOverResult } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { link } from "../lib/link.svelte.ts";

  let dismissed = $state(false);
  let taking = $state(false);
  let result: TakeOverResult | null = $state(null);
  let problem: string | null = $state(null);

  // Connected again: the next loss asks again.
  $effect(() => {
    if (link.status.connected) dismissed = false;
  });

  const shown = $derived((link.status.hostLost && !dismissed) || result !== null);

  async function takeOver() {
    taking = true;
    problem = null;
    try {
      result = await link.takeOver();
      // Nothing missing: straight on with the service.
      if (result.missingMedia === 0) window.location.reload();
    } catch (error) {
      problem = String(error);
    } finally {
      taking = false;
    }
  }
</script>

{#if shown}
  {#if result}
    <Dialog title={t("takeOver.doneTitle")} size="small" onclose={() => window.location.reload()}>
      <p>{t("takeOver.missing", { count: result.missingMedia })}</p>
      {#snippet actions()}
        <Button variant="primary" onclick={() => window.location.reload()}>{t("common.ok")}</Button>
      {/snippet}
    </Dialog>
  {:else}
    <Dialog
      title={t("takeOver.title", { name: link.status.hostName })}
      size="small"
      onclose={() => (dismissed = true)}
    >
      <p>{t("takeOver.question")}</p>
      <p class="hint">{t("takeOver.onlyOne")}</p>
      {#if problem}
        <p class="problem">{problem}</p>
      {/if}
      {#snippet actions()}
        <Button onclick={() => (dismissed = true)}>{t("takeOver.keepWaiting")}</Button>
        <Button variant="live" disabled={taking} onclick={takeOver}
          >{t("takeOver.takeOver")}</Button
        >
      {/snippet}
    </Dialog>
  {/if}
{/if}

<style>
  p {
    margin: 0 0 var(--sw-space-2);
    line-height: 1.5;
  }

  .hint {
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
  }

  .problem {
    color: var(--sw-live);
  }
</style>
