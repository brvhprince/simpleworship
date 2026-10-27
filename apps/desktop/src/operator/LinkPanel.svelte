<!--
  @component
  The Remote panel on a computer that joined another: which computer, as
  what, whether it is connected, and Leave (this computer then runs its
  own service again).
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { link } from "../lib/link.svelte.ts";
  import { ROLES } from "../lib/remote.svelte.ts";

  const status = $derived(link.status);
  const role = $derived(ROLES.find((entry) => entry.role === status.role));
</script>

<p class="joined">{t("link.joinedTo", { name: status.hostName })}</p>
<p class="meta">
  {role ? t(role.name) : ""}
  ·
  <span class:offline={!status.connected}>
    {status.connected ? t("link.connected") : t("link.reconnecting")}
  </span>
</p>
<p class="hint">{t("link.hint")}</p>
<Button onclick={() => link.leave()}>{t("link.leave")}</Button>

<style>
  .joined {
    margin: 0;
    font-weight: 600;
  }

  .meta {
    margin: var(--sw-space-1) 0 var(--sw-space-2);
    color: var(--sw-text-muted);
  }

  .offline {
    color: var(--sw-warning-text);
  }

  .hint {
    margin: 0 0 var(--sw-space-2);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }
</style>
