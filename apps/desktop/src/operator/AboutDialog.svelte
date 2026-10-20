<!--
  @component
  About SimpleWorship: the version, the licence, where to get help, and
  the people and works it builds on.
-->
<script lang="ts">
  import { Button, Dialog } from "@simpleworship/ui";
  import { getVersion } from "@tauri-apps/api/app";
  import { onMount } from "svelte";
  import icon from "../../src-tauri/icons/128x128@2x.png";
  import { openLogsFolder, openProjectPage } from "../lib/api.ts";
  import { dialogs } from "../lib/dialogs.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import UpdateButton from "./UpdateButton.svelte";

  let version = $state("");

  onMount(async () => {
    version = await getVersion();
  });

  const year = new Date().getFullYear();
</script>

<Dialog title={t("about.title")} size="small" onclose={() => dialogs.close()}>
  <div class="about">
    <img class="icon" src={icon} alt="" width="96" height="96">
    <h2>SimpleWorship</h2>
    <p class="version">{t("about.version", { version })}</p>
    <p class="tagline">{t("about.tagline")}</p>

    <div class="links">
      <Button onclick={() => openProjectPage("website")}>{t("about.website")}</Button>
      <Button onclick={() => openProjectPage("changes")}>{t("about.whatsNew")}</Button>
      <Button onclick={() => openProjectPage("report")}>{t("about.report")}</Button>
    </div>

    <dl class="facts">
      <dt>{t("about.license")}</dt>
      <dd>
        {t("about.licenseText")}
        <button type="button" class="link" onclick={() => openProjectPage("license")}>
          {t("about.readLicense")}
        </button>
        <span class="trademark">{t("about.nameTrademark")}</span>
      </dd>
      <dt>{t("about.credits")}</dt>
      <dd>
        {t("about.creditsText")}
        <span class="trademark">{t("about.ndiTrademark")}</span>
      </dd>
      <dt>{t("about.support")}</dt>
      <dd>
        {t("about.supportText")}
        <button type="button" class="link" onclick={() => openLogsFolder()}>
          {t("about.openLogs")}
        </button>
      </dd>
    </dl>

    <p class="copyright">© {year} {t("about.contributors")}</p>
  </div>

  {#snippet actions()}
    <UpdateButton label={t("about.checkUpdates")} />
    <Button variant="primary" onclick={() => dialogs.close()}>{t("common.close")}</Button>
  {/snippet}
</Dialog>

<style>
  .about {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--sw-space-2);
  }

  .icon {
    border-radius: 22px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  h2 {
    margin: var(--sw-space-2) 0 0;
    font-size: 22px;
  }

  .version {
    margin: 0;
    color: var(--sw-text-muted);
    font-variant-numeric: tabular-nums;
  }

  .tagline {
    margin: 0;
    max-width: 320px;
    line-height: 1.5;
  }

  .links {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--sw-space-2);
    margin: var(--sw-space-2) 0;
  }

  .facts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: var(--sw-space-1) var(--sw-space-3);
    margin: 0;
    text-align: left;
    font-size: var(--sw-font-size-small);
    line-height: 1.5;
  }

  dt {
    color: var(--sw-text-muted);
    font-weight: 600;
  }

  dd {
    margin: 0;
    color: var(--sw-text-muted);
  }

  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--sw-accent);
    cursor: pointer;
    font: inherit;
  }

  .copyright {
    margin: var(--sw-space-2) 0 0;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .trademark {
    display: block;
    margin-top: var(--sw-space-1);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }
</style>
