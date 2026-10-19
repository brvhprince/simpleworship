<!--
  @component
  A phone asks to connect: allow it, choosing what it may do, or refuse.
  Shown above everything, so it is not missed during the service.
-->
<script lang="ts">
  import type { PairRequest } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { ROLES, remote } from "../lib/remote.svelte.ts";

  interface Props {
    request: PairRequest;
  }

  let { request }: Props = $props();
</script>

<Dialog title={t("pairRequest.title")} size="small" onclose={() => remote.answer(request, null)}>
  <p class="question">{t("pairRequest.question", { name: request.name })}</p>
  <div class="roles">
    {#each ROLES as option (option.role)}
      <button type="button" onclick={() => remote.answer(request, option.role)}>
        <strong>{t(option.name)}</strong>
        <span>{t(option.description)}</span>
      </button>
    {/each}
  </div>

  {#snippet actions()}
    <Button variant="live" onclick={() => remote.answer(request, null)}>
      {t("pairRequest.refuse")}
    </Button>
  {/snippet}
</Dialog>

<style>
  .question {
    margin: 0 0 var(--sw-space-3);
    font-size: 1.05em;
  }

  .roles {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
  }

  .roles button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    padding: var(--sw-space-2) var(--sw-space-3);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    background: var(--sw-surface-raised);
    color: var(--sw-text);
    text-align: left;
    cursor: pointer;
  }

  .roles button:hover {
    border-color: var(--sw-accent);
  }

  .roles span {
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
  }
</style>
