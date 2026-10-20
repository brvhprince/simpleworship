<!--
  @component
  Offers to install a downloaded update, with what is new in it (see
  `lib/updates.svelte.ts`). Render it once, in the operator window.
-->
<script lang="ts">
  import { Button, Dialog } from "@simpleworship/ui";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { updates } from "../lib/updates.svelte.ts";
  import ReleaseNotes from "./ReleaseNotes.svelte";
</script>

{#if updates.offer}
  {@const offer = updates.offer}
  <Dialog
    title={t("updates.installTitle", { version: offer.version })}
    onclose={() => offer.answer(false)}
  >
    <div class="body">
      {#if offer.notes}
        <h3>{t("updates.whatsNew")}</h3>
        <div class="notes">
          <ReleaseNotes markdown={offer.notes} />
        </div>
      {/if}
      <p class="restart">{t("updates.installMessage")}</p>
    </div>

    {#snippet actions()}
      <Button onclick={() => offer.answer(false)}>{t("common.later")}</Button>
      <Button variant="primary" onclick={() => offer.answer(true)}>{t("updates.install")}</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .body {
    width: min(560px, 80vw);
  }

  h3 {
    margin: 0 0 var(--sw-space-2);
    font-size: var(--sw-font-size);
    color: var(--sw-text-muted);
  }

  /* Long notes scroll; the question below them stays in view. */
  .notes {
    max-height: min(50vh, 420px);
    overflow-y: auto;
    padding-right: var(--sw-space-2);
  }

  .restart {
    margin: var(--sw-space-3) 0 0;
    color: var(--sw-text-muted);
  }
</style>
