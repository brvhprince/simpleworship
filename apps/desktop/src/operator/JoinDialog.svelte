<!--
  @component
  Join Another Computer: this computer then follows the service running on
  the other one (the main computer), as a second operator or a mirror for
  an overflow room or the stream booth.

  Computers on the network are listed by themselves (or type an address).
  On the main computer, Remote › Connect a Phone shows the code to type;
  its operator then allows this computer and chooses what it may do. Both
  must run the same version of SimpleWorship.
-->
<script lang="ts">
  import type { FoundHost } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { link } from "../lib/link.svelte.ts";

  let hosts: FoundHost[] = $state([]);
  let looking = $state(false);
  let address = $state("");
  let code = $state("");
  let name = $state("");
  let waiting = $state(false);
  let problem: string | null = $state(null);

  async function look() {
    looking = true;
    try {
      hosts = await link.discover();
      const compatible = hosts.find((host) => host.compatible);
      if (!address && compatible) address = compatible.address;
    } catch (error) {
      problem = String(error);
    } finally {
      looking = false;
    }
  }

  async function join(event: SubmitEvent) {
    event.preventDefault();
    waiting = true;
    problem = null;
    try {
      // Every window reloads once joined.
      await link.join(address, code, name);
    } catch (error) {
      problem = String(error);
      waiting = false;
    }
  }

  onMount(() => void look());
</script>

<Dialog title={t("join.title")} size="small" onclose={() => (link.joining = false)}>
  {#if waiting}
    <p class="waiting">
      {t("join.waitingFor", {
        name: hosts.find((host) => host.address === address)?.name ?? address,
      })}
    </p>
    <p class="hint">{address}</p>
  {:else}
    <form id="join" onsubmit={join}>
      <p class="hint">{t("join.intro")}</p>

      <div class="found">
        <span class="label">{t("join.found")}</span>
        <Button disabled={looking} onclick={look}>
          {looking ? t("join.looking") : t("cameras.refresh")}
        </Button>
      </div>
      {#if hosts.length > 0}
        <ul class="hosts">
          {#each hosts as host (host.address)}
            <li>
              <label class:incompatible={!host.compatible}>
                <input
                  type="radio"
                  name="host"
                  value={host.address}
                  disabled={!host.compatible}
                  checked={address === host.address}
                  onchange={() => (address = host.address)}
                >
                <span class="name">{host.name}</span>
                <span class="meta">
                  {host.compatible
                    ? host.address
                    : t("join.otherVersion", { version: host.version })}
                </span>
              </label>
            </li>
          {/each}
        </ul>
      {:else if !looking}
        <p class="hint">{t("join.noneFound")}</p>
      {/if}

      <label class="field">
        {t("join.address")}
        <input bind:value={address} placeholder="192.168.1.20" autocomplete="off">
      </label>
      <label class="field">
        {t("remote.code")}
        <input
          bind:value={code}
          inputmode="numeric"
          maxlength="6"
          placeholder="123456"
          autocomplete="off"
        >
      </label>
      <label class="field">
        {t("join.name")}
        <input bind:value={name} maxlength="60" placeholder={t("join.namePlaceholder")}>
      </label>
      {#if problem}
        <p class="problem">{problem}</p>
      {/if}
    </form>
  {/if}

  {#snippet actions()}
    <Button onclick={() => (link.joining = false)}>{t("common.cancel")}</Button>
    {#if !waiting}
      <Button
        variant="primary"
        type="submit"
        form="join"
        disabled={!address.trim() || code.trim().length !== 6 || !name.trim()}
      >
        {t("join.join")}
      </Button>
    {/if}
  {/snippet}
</Dialog>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
  }

  .hint {
    margin: 0;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .found {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .label {
    color: var(--sw-text-muted);
  }

  .hosts {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .hosts label {
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: var(--sw-space-2);
    padding: var(--sw-space-1) 0;
    cursor: pointer;
  }

  .hosts .meta {
    grid-column: 2;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .incompatible {
    opacity: 0.6;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .field input {
    color: var(--sw-text);
  }

  .waiting {
    margin: var(--sw-space-3) 0;
  }

  .problem {
    margin: 0;
    color: var(--sw-live);
  }
</style>
