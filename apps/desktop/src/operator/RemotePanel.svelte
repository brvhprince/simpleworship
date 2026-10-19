<!--
  @component
  The toolbar's Remote panel: turn the phone remote on, connect a phone,
  and see who is connected (changing a phone's role or removing it is in
  Settings).
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { panels } from "../lib/panels.svelte.ts";
  import { ROLES, remote } from "../lib/remote.svelte.ts";

  const status = $derived(remote.status);
  const connected = $derived(
    status?.devices.filter((device) => status.connected.includes(device.id)) ?? [],
  );

  function roleName(role: string): string {
    const found = ROLES.find((entry) => entry.role === role);
    return found ? t(found.name) : role;
  }
</script>

{#if !status?.settings.enabled}
  <p class="hint">{t("remotePanel.offHint")}</p>
  <Button variant="primary" onclick={() => remote.setEnabled(true, status?.settings.port ?? 7270)}>
    {t("remotePanel.turnOn")}
  </Button>
{:else if !status.running}
  <p class="error">{status.error ?? t("remotePanel.starting")}</p>
  <Button onclick={() => remote.setEnabled(true, status.settings.port)}>
    {t("remotePanel.tryAgain")}
  </Button>
{:else}
  <Button
    variant="primary"
    onclick={() => {
      panels.close();
      remote.connecting = true;
    }}
  >
    {t("remotePanel.connectPhone")}
  </Button>
  <h4>{t("remotePanel.connected", { count: connected.length })}</h4>
  <ul>
    {#each connected as device (device.id)}
      <li><span>{device.name}</span><span class="role">{roleName(device.role)}</span></li>
    {:else}
      <li class="hint">{t("remotePanel.noneConnected")}</li>
    {/each}
  </ul>
  <p class="hint">{t("remotePanel.settingsHint")}</p>
{/if}

<style>
  h4 {
    margin: var(--sw-space-3) 0 var(--sw-space-1);
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    justify-content: space-between;
    gap: var(--sw-space-2);
    padding: 4px 0;
  }

  .role {
    color: var(--sw-text-muted);
  }

  .hint {
    margin: var(--sw-space-2) 0;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .error {
    color: var(--sw-live);
  }
</style>
