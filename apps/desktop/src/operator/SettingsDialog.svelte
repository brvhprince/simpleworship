<!--
  @component
  The church's settings: name and CCLI licence (shown in song footers),
  the alert templates in the Alert panel, updates, the stage display
  options, the stream look, the phone remote, and the interface language
  (kept on this computer).
-->
<script lang="ts">
  import type { AlertTemplate, AppSettings } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";
  import { getSettings, saveSettings } from "../lib/api.ts";
  import { dialogs } from "../lib/dialogs.svelte.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { i18n, LANGUAGES, type Language, t } from "../lib/i18n/i18n.svelte.ts";
  import { link } from "../lib/link.svelte.ts";
  import { ROLES, remote } from "../lib/remote.svelte.ts";
  import StreamSettings from "./StreamSettings.svelte";
  import ColorField from "./themes/ColorField.svelte";
  import UpdateButton from "./UpdateButton.svelte";

  let settings: AppSettings | null = $state(null);
  let version = $state("");
  /** The language chosen here ("" follows the computer); applied on Save. */
  let language: Language | "" = $state(i18n.chosen ?? "");

  /** "Twi (12% translated)" for a language that is not finished. */
  function languageName(code: Language, name: string): string {
    const percent = Math.floor(i18n.coverage(code) * 100);
    return percent >= 100 ? name : t("settings.languagePartial", { name, percent });
  }

  onMount(async () => {
    settings = await getSettings();
    version = await getVersion();
  });

  function addTemplate() {
    settings?.alertTemplates.push({
      name: t("settings.newAlert"),
      text: "",
      style: "ticker",
      position: "bottom",
      end: { type: "afterPasses", passes: 3 },
      background: "rgba(0, 0, 0, 0.82)",
      color: "#ffffff",
    });
  }

  function removeTemplate(template: AlertTemplate) {
    if (!settings) return;
    settings.alertTemplates = settings.alertTemplates.filter((other) => other !== template);
  }

  async function save() {
    if (!settings) return;
    try {
      await saveSettings($state.snapshot(settings));
      i18n.choose(language || null);
      dialogs.close();
    } catch (error) {
      engine.showError(error);
    }
  }
</script>

<Dialog title={t("settings.title")} onclose={() => dialogs.close()}>
  {#if settings}
    <div class="settings">
      {#if link.joined}
        <p class="hint">{t("link.settingsThere", { name: link.status.hostName })}</p>
      {/if}
      {#if !link.joined}
        <section>
          <h3>{t("settings.church")}</h3>
          <label class="field">
            {t("common.name")}
            <input bind:value={settings.churchName} placeholder={t("settings.churchPlaceholder")}>
          </label>
          <label class="field">
            {t("settings.ccli")}
            <input bind:value={settings.ccliLicense} placeholder={t("settings.ccliPlaceholder")}>
            <span class="hint">{t("settings.ccliHint")}</span>
          </label>
        </section>

        <section>
          <h3>{t("settings.alerts")}</h3>
          <p class="hint">{t("settings.alertsHint")}</p>
          {#each settings.alertTemplates as template, index (index)}
            <div class="template">
              <input class="template-name" bind:value={template.name} aria-label={t("common.name")}>
              <input
                class="template-text"
                bind:value={template.text}
                aria-label={t("alert.message")}
              >
              <select bind:value={template.style} aria-label={t("alert.style")}>
                <option value="ticker">{t("alert.scrolling")}</option>
                <option value="banner">{t("alert.still")}</option>
              </select>
              <select bind:value={template.position} aria-label={t("alert.position")}>
                <option value="bottom">{t("alert.bottom")}</option>
                <option value="top">{t("alert.top")}</option>
              </select>
              <ColorField
                label={t("alert.background")}
                value={template.background}
                onchange={(value) => (template.background = value)}
              />
              <ColorField
                label={t("alert.textColour")}
                value={template.color}
                onchange={(value) => (template.color = value)}
              />
              <Button onclick={() => removeTemplate(template)}>{t("settings.remove")}</Button>
            </div>
          {/each}
          <Button onclick={addTemplate}>{t("settings.addTemplate")}</Button>
        </section>
      {/if}

      <section>
        <h3>{t("settings.updates")}</h3>
        <label class="option">
          <input type="checkbox" bind:checked={settings.checkForUpdates}>
          {t("settings.autoUpdates")}
        </label>
        <div class="option">
          <span class="hint">{t("settings.thisVersion", { version })}</span>
          <UpdateButton label={t("settings.checkNow")} />
        </div>
      </section>

      {#if !link.joined}
        <section>
          <h3>{t("settings.stage")}</h3>
          <label class="option">
            <input type="checkbox" bind:checked={settings.stage.showClock}>
            {t("settings.showClock")}
          </label>
          <label class="option">
            <input type="checkbox" bind:checked={settings.stage.showNext}>
            {t("settings.showNext")}
          </label>
          <label class="field narrow">
            {t("settings.textSize")}
            <select bind:value={settings.stage.textScale}>
              <option value={80}>{t("settings.smaller")}</option>
              <option value={100}>{t("settings.normal")}</option>
              <option value={125}>{t("settings.larger")}</option>
              <option value={150}>{t("settings.largest")}</option>
            </select>
          </label>
        </section>
      {/if}

      <section>
        <h3>{t("settings.stream")}</h3>
        <StreamSettings />
      </section>

      {#if !link.joined}
        <section>
          <h3>{t("settings.remote")}</h3>
          {#if remote.status}
            {@const status = remote.status}
            <label class="option">
              <input
                type="checkbox"
                checked={status.settings.enabled}
                onchange={(e) => remote.setEnabled(e.currentTarget.checked, status.settings.port)}
              >
              {t("settings.remoteOn")}
            </label>
            <label class="field narrow">
              {t("settings.remotePort")}
              <input
                type="number"
                min="1024"
                max="65535"
                value={status.settings.port}
                onchange={(e) =>
                  remote.setEnabled(status.settings.enabled, e.currentTarget.valueAsNumber || 7270)}
              >
            </label>
            {#if status.running && status.addresses.length > 0}
              <p class="hint">
                {t("settings.remoteAddresses", { addresses: status.addresses.join("  ·  ") })}
              </p>
            {:else if status.error}
              <p class="hint">{status.error}</p>
            {/if}
            <p class="hint">{t("settings.remoteHint")}</p>
            {#each status.devices as device (device.id)}
              <div class="template">
                <span class="device-name">
                  {device.name}
                  {#if status.connected.includes(device.id)}
                    <span class="connected">· {t("settings.remoteConnected")}</span>
                  {/if}
                </span>
                <select
                  value={device.role}
                  aria-label={t("settings.remoteRole", { name: device.name })}
                  onchange={(e) =>
                    remote.setRole(
                      device.id,
                      e.currentTarget.value as (typeof ROLES)[number]["role"],
                    )}
                >
                  {#each ROLES as option (option.role)}
                    <option value={option.role}>{t(option.name)}</option>
                  {/each}
                </select>
                <Button onclick={() => remote.remove(device.id)}>{t("settings.remove")}</Button>
              </div>
            {:else}
              <p class="hint">{t("settings.remoteNoDevices")}</p>
            {/each}
          {/if}
        </section>
      {/if}

      <section>
        <h3>{t("settings.language")}</h3>
        <label class="field">
          <select bind:value={language} aria-label={t("settings.language")}>
            <option value="">{t("settings.languageSystem")}</option>
            {#each LANGUAGES as option (option.code)}
              <option value={option.code}>{languageName(option.code, option.name)}</option>
            {/each}
          </select>
          <span class="hint">{t("settings.languageHint")}</span>
        </label>
      </section>
    </div>
  {/if}

  {#snippet actions()}
    <Button onclick={() => dialogs.close()}>{t("common.cancel")}</Button>
    <Button variant="primary" disabled={!settings} onclick={save}>{t("common.save")}</Button>
  {/snippet}
</Dialog>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-4);
  }

  h3 {
    margin: 0 0 var(--sw-space-2);
    font-size: var(--sw-font-size-large, 15px);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    max-width: 420px;
    margin-bottom: var(--sw-space-3);
    color: var(--sw-text-muted);
  }

  .field input,
  .field select {
    color: var(--sw-text);
  }

  .narrow {
    max-width: 180px;
  }

  .hint {
    margin: 0 0 var(--sw-space-2);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .template {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-1);
    margin-bottom: var(--sw-space-2);
  }

  .device-name {
    flex: 1;
  }

  .connected {
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
  }

  .template-name {
    width: 140px;
  }

  .template-text {
    flex: 1;
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    margin-bottom: var(--sw-space-2);
  }
</style>
