<!--
  @component
  Connecting this phone: the 6-digit code from the operator's screen (filled
  in when the QR code was scanned) and a name the operator will recognise.
  Then it waits until the operator allows it.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import { remote } from "../lib/remote.svelte.ts";

  /** The code in the address, when the QR code was scanned. */
  const scanned = new URLSearchParams(location.hash.slice(1)).get("code") ?? "";

  let code = $state(scanned);
  let name = $state(guessName());

  function guessName(): string {
    const agent = navigator.userAgent;
    if (/iPad/.test(agent)) return "iPad";
    if (/iPhone/.test(agent)) return "iPhone";
    if (/Android/.test(agent)) return "Android phone";
    return "";
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (await remote.pair(code, name)) history.replaceState(null, "", "/");
  }
</script>

<section class="pairing">
  <img class="icon" src="/icon-192.png" alt="" width="72" height="72">
  <h1>{t("remote.pairTitle")}</h1>

  {#if remote.status === "waiting"}
    <p class="waiting">{t("remote.waitingForOperator")}</p>
  {:else}
    <p>{t("remote.pairIntro")}</p>
    <form onsubmit={submit}>
      <label>
        {t("remote.code")}
        <input
          bind:value={code}
          inputmode="numeric"
          autocomplete="one-time-code"
          maxlength="6"
          placeholder="123456"
        >
      </label>
      <label>
        {t("remote.deviceName")}
        <input bind:value={name} placeholder={t("remote.deviceNamePlaceholder")} maxlength="60">
      </label>
      <button type="submit" disabled={code.trim().length !== 6 || !name.trim()}>
        {t("remote.connect")}
      </button>
    </form>
  {/if}
</section>

<style>
  .pairing {
    margin: auto;
    padding: 24px;
    width: min(400px, 100%);
    box-sizing: border-box;
    text-align: center;
  }

  .icon {
    border-radius: 16px;
  }

  h1 {
    margin: 12px 0;
    font-size: 22px;
  }

  p {
    color: var(--sw-text-muted);
    line-height: 1.5;
  }

  .waiting {
    color: var(--sw-text);
    font-size: 18px;
  }

  form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-top: 24px;
    text-align: left;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--sw-text-muted);
  }

  input {
    min-height: 48px;
    padding: 0 12px;
    border: 1px solid var(--sw-border);
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
  }

  input[inputmode="numeric"] {
    font-size: 28px;
    letter-spacing: 0.3em;
    text-align: center;
  }

  button {
    min-height: 52px;
    border: none;
    border-radius: 10px;
    background: var(--sw-accent);
    color: #fff;
    font-weight: 600;
  }

  button:disabled {
    opacity: 0.5;
  }
</style>
