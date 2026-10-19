<!--
  @component
  Connect a Phone: a QR code to scan with the phone's camera (or the address
  and 6-digit code to type). The code works once, for five minutes. When the
  phone asks to connect, SimpleWorship asks which role to give it.
-->
<script lang="ts">
  import type { PairingCode } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { remote } from "../lib/remote.svelte.ts";

  /** Codes last five minutes; a fresh one is made a little before. */
  const REFRESH_MS = 4.5 * 60 * 1000;

  let pairing = $state<PairingCode | null>(null);
  let problem = $state<string | null>(null);

  const address = $derived(pairing?.url.split("/#")[0] ?? "");
  const qrImage = $derived(
    pairing ? `data:image/svg+xml;charset=utf-8,${encodeURIComponent(pairing.qrSvg)}` : "",
  );

  async function refresh() {
    try {
      pairing = await remote.startPairing();
      problem = null;
    } catch (error) {
      problem = String(error);
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(refresh, REFRESH_MS);
    return () => {
      clearInterval(timer);
      remote.cancelPairing();
    };
  });
</script>

<Dialog title={t("connect.title")} size="small" onclose={() => (remote.connecting = false)}>
  {#if problem}
    <p class="problem">{problem}</p>
  {:else if pairing}
    <div class="connect">
      <img class="qr" src={qrImage} alt={t("connect.qrAlt")} width="240" height="240">
      <ol>
        <li>{t("connect.step1")}</li>
        <li>{t("connect.step2")}</li>
        <li>{t("connect.step3")}</li>
      </ol>
      <p class="manual">{t("connect.manual")}</p>
      <p class="address">{address}</p>
      <p class="code"><span class="visually-hidden">{t("remote.code")}: </span>{pairing.code}</p>
      <p class="hint">{t("connect.sameWifi")}</p>
    </div>
  {/if}

  {#snippet actions()}
    <Button onclick={() => (remote.connecting = false)}>{t("common.done")}</Button>
  {/snippet}
</Dialog>

<style>
  .connect {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
  }

  .qr {
    border-radius: var(--sw-radius);
    background: #fff;
    padding: 8px;
  }

  ol {
    margin: var(--sw-space-3) 0 0;
    padding-left: 1.2em;
    text-align: left;
    line-height: 1.6;
  }

  .manual {
    margin: var(--sw-space-3) 0 0;
    color: var(--sw-text-muted);
  }

  .address {
    margin: var(--sw-space-1) 0;
    font-family: var(--sw-font-mono);
    user-select: all;
  }

  .code {
    margin: 0;
    font-size: 34px;
    font-weight: 700;
    letter-spacing: 0.25em;
    font-variant-numeric: tabular-nums;
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  .hint {
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .problem {
    color: var(--sw-live);
  }
</style>
