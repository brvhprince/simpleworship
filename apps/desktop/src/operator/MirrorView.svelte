<!--
  @component
  The window of a computer that joined another as a mirror (an overflow
  room, the stream booth, a backup): what is live there, large, and this
  computer's own screens (Screens), stream window and NDI (Stream), and
  the link (Remote, to leave). A mirror controls nothing: the service is
  run from the main computer.
-->
<script lang="ts">
  import { OutputView, ScaledCanvas } from "@simpleworship/renderer";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { link } from "../lib/link.svelte.ts";
  import { outputs } from "../lib/outputs.svelte.ts";
  import { panels } from "../lib/panels.svelte.ts";
  import LinkPanel from "./LinkPanel.svelte";
  import NdiSettings from "./NdiSettings.svelte";
  import Popover from "./Popover.svelte";
  import ScreensPanel from "./ScreensPanel.svelte";

  const screensOn = $derived(outputs.open.length);
</script>

<div class="mirror">
  <header>
    <span class="brand">SimpleWorship</span>
    <Popover
      label={screensOn > 0 ? t("toolbar.screensOn", { count: screensOn }) : t("toolbar.screens")}
      align="start"
      open={panels.open === "screens"}
      active={screensOn > 0}
      ontoggle={() => panels.toggle("screens")}
      onclose={() => panels.close()}
    >
      <ScreensPanel />
    </Popover>
    <Popover
      label={t("toolbar.stream")}
      align="start"
      open={panels.open === "stream"}
      ontoggle={() => panels.toggle("stream")}
      onclose={() => panels.close()}
    >
      <NdiSettings />
    </Popover>
    <Popover
      label={t("toolbar.joined")}
      align="start"
      open={panels.open === "remote"}
      active={link.status.connected}
      ontoggle={() => panels.toggle("remote")}
      onclose={() => panels.close()}
    >
      <LinkPanel />
    </Popover>
    <span class="following">
      {t("mirror.following", { name: link.status.hostName })}
    </span>
  </header>

  <main>
    <div class="title">
      <span class="badge">{t("pane.live")}</span>
      {engine.live?.presentation.title ?? t("pane.nothingLive")}
    </div>
    {#if engine.output}
      <div class="screen">
        <ScaledCanvas fit="contain">
          <OutputView output={engine.output} muted />
        </ScaledCanvas>
      </div>
    {/if}
    <p class="hint">{t("mirror.hint")}</p>
  </main>
</div>

<style>
  .mirror {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--sw-bg);
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
    background: var(--sw-surface);
    position: relative;
    z-index: 10;
  }

  .brand {
    font-weight: 700;
    margin-right: var(--sw-space-2);
  }

  .following {
    margin-left: auto;
    color: var(--sw-text-muted);
  }

  main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    padding: var(--sw-space-3);
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    font-weight: 600;
  }

  .badge {
    padding: 1px 8px;
    border-radius: var(--sw-radius-small);
    background: var(--sw-live);
    color: #fff;
    font-size: var(--sw-font-size-small);
  }

  .screen {
    flex: 1;
    min-height: 0;
    border: 2px solid var(--sw-live);
    border-radius: var(--sw-radius);
    overflow: hidden;
  }

  .hint {
    margin: 0;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }
</style>
