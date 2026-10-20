<!--
  @component
  The operator window, laid out like EasyWorship:

  ┌──────────────────── toolbar ────────────────────┐
  │ schedule │      preview       │       live       │
  ├──────────┴────────────────────┴──────────────────┤
  │ library: songs · text · scripture · media · …    │
  └──────────────────────────────────────────────────┘

  It also owns the keyboard shortcuts (see `lib/shortcuts.ts`) and the
  File menu flows (see `lib/schedule-file.svelte.ts`).
-->
<script lang="ts">
  import type { Command } from "@simpleworship/protocol";
  import { getMediaUrls } from "@simpleworship/renderer";
  import { Button } from "@simpleworship/ui";
  import { onMount, untrack } from "svelte";
  import { setMenuLabels } from "../lib/api.ts";
  import { bibles } from "../lib/bibles.svelte.ts";
  import { dialogs } from "../lib/dialogs.svelte.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { menuLabels, t } from "../lib/i18n/i18n.svelte.ts";
  import { importer } from "../lib/importer.svelte.ts";
  import { link } from "../lib/link.svelte.ts";
  import { listen } from "../lib/listen.svelte.ts";
  import { media } from "../lib/media.svelte.ts";
  import { connectEditMenu } from "../lib/menu.ts";
  import { ndi } from "../lib/ndi.svelte.ts";
  import { outputs } from "../lib/outputs.svelte.ts";
  import { panels } from "../lib/panels.svelte.ts";
  import { remote } from "../lib/remote.svelte.ts";
  import { scheduleFile } from "../lib/schedule-file.svelte.ts";
  import {
    commandForKey,
    hasNumberedSection,
    isTypingTarget,
    SectionJumper,
  } from "../lib/shortcuts.ts";
  import { updates } from "../lib/updates.svelte.ts";
  import AboutDialog from "./AboutDialog.svelte";
  import AudioPlayer from "./AudioPlayer.svelte";
  import ConnectPhoneDialog from "./ConnectPhoneDialog.svelte";
  import ImportDialog from "./ImportDialog.svelte";
  import JoinDialog from "./JoinDialog.svelte";
  import Library from "./library/Library.svelte";
  import MirrorView from "./MirrorView.svelte";
  import PairRequestDialog from "./PairRequestDialog.svelte";
  import PromptHost from "./PromptHost.svelte";
  import SchedulePanel from "./SchedulePanel.svelte";
  import SettingsDialog from "./SettingsDialog.svelte";
  import SlidePane from "./SlidePane.svelte";
  import TakeOverPrompt from "./TakeOverPrompt.svelte";
  import Toolbar from "./Toolbar.svelte";
  import UsageDialog from "./UsageDialog.svelte";
  import VideoControls from "./VideoControls.svelte";

  const urls = getMediaUrls();

  // The native menu bar follows the interface language.
  $effect(() => {
    const labels = menuLabels();
    setMenuLabels(labels).catch((error) => engine.showError(error));
  });

  // Presentations set to move on by themselves: this window keeps time and
  // tells the engine when the live slide's time is up. Stepping through by
  // hand restarts the clock (the engine ignores a timer for a slide the
  // operator already left).
  const advanceTimer = $derived.by(() => {
    const live = engine.live;
    const seconds = live?.presentation.autoAdvance?.seconds;
    return live && seconds
      ? { presentationId: live.presentation.id, from: live.selected, seconds }
      : null;
  });
  // Compared by value, so other engine updates do not restart the clock.
  const advanceKey = $derived(JSON.stringify(advanceTimer));
  $effect(() => {
    void advanceKey;
    const timing = untrack(() => advanceTimer);
    if (!timing) return;
    const { presentationId, from, seconds } = timing;
    const timer = setTimeout(
      () => engine.dispatch({ type: "autoAdvance", presentationId, from }),
      seconds * 1000,
    );
    return () => clearTimeout(timer);
  });

  onMount(() => {
    const connections = [
      engine.connect(),
      scheduleFile.connect(),
      bibles.connect(),
      media.connect(urls),
      importer.connect(),
      dialogs.connect(),
      connectEditMenu(),
      updates.connect(),
      remote.connect(),
      outputs.connect(),
      ndi.connect(),
      listen.connect(),
    ];
    for (const connection of connections) {
      connection.catch((error) => engine.showError(error));
    }
    // After any crash-recovery question, offer another program's library
    // (first launch only).
    void Promise.all(connections)
      .then(() => importer.offerOnFirstLaunch())
      .catch((error) => engine.showError(error));
    return () => {
      for (const connection of connections) {
        void connection.then((disconnect) => disconnect());
      }
    };
  });

  const sectionJumper = new SectionJumper(
    (command) => engine.dispatch(command),
    (letter) => hasNumberedSection(engine.live, letter),
  );

  /** Whether a shortcut command makes sense right now. Pressing → with
   *  nothing live should do nothing, not show an error. */
  function isApplicable(command: Command): boolean {
    switch (command.type) {
      case "nextSlide":
      case "previousSlide":
      case "jumpToSection":
        return engine.live !== null;
      case "goLive":
        return engine.preview !== null;
      default:
        return true;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    // A mirror follows another computer and controls nothing.
    if (link.mirror) return;
    const isModified = event.metaKey || event.ctrlKey || event.altKey;
    // Space and Enter on a focused button should press that button.
    const isButtonActivation =
      event.target instanceof HTMLButtonElement && (event.key === " " || event.key === "Enter");

    if (event.defaultPrevented || isModified || isButtonActivation) return;
    if (isTypingTarget(event.target) || !engine.output) return;

    // A opens the alert panel; Esc takes an alert off the screen.
    if (event.key === "a" || event.key === "A") {
      event.preventDefault();
      panels.toggle("alerts");
      return;
    }
    // H shows the newest reference Listen heard in the preview.
    if ((event.key === "h" || event.key === "H") && listen.previewNewest()) {
      event.preventDefault();
      return;
    }
    if (event.key === "Escape" && engine.output.alert && panels.open === null) {
      event.preventDefault();
      engine.dispatch({ type: "clearAlert" });
      return;
    }

    const command = commandForKey(event.key, engine.output.flags);
    if (command) {
      event.preventDefault();
      if (isApplicable(command)) engine.dispatch(command);
      return;
    }

    if (engine.live && sectionJumper.handleKey(event.key)) {
      event.preventDefault();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if link.mirror}
  <MirrorView />
{:else}
  <div class="operator">
    <Toolbar />

    <SchedulePanel />

    <SlidePane
      kind="preview"
      pane={engine.preview}
      onselect={(index) => engine.dispatch({ type: "selectPreviewSlide", index })}
      onactivate={(index) => {
        engine.dispatch({ type: "selectPreviewSlide", index });
        engine.dispatch({ type: "goLive" });
      }}
    >
      {#snippet actions()}
        <Button
          variant="primary"
          shortcut="Enter"
          disabled={!engine.preview}
          onclick={() => engine.dispatch({ type: "goLive" })}
        >
          {t("pane.goLive")}
        </Button>
      {/snippet}
    </SlidePane>

    <SlidePane
      kind="live"
      pane={engine.live}
      output={engine.output}
      onselect={(index) => engine.dispatch({ type: "selectLiveSlide", index })}
      onalertend={() => engine.dispatch({ type: "clearAlert" })}
      oncountdownend={() => engine.dispatch({ type: "countdownFinished" })}
    >
      {#snippet below()}
        <VideoControls />
      {/snippet}
      {#snippet actions()}
        <Button disabled={!engine.live} onclick={() => engine.dispatch({ type: "clearLive" })}>
          {t("common.clear")}
        </Button>
      {/snippet}
    </SlidePane>

    <Library />

    <div class="player"><AudioPlayer /></div>

    {#if dialogs.open === "settings"}
      <SettingsDialog />
    {:else if dialogs.open === "usage"}
      <UsageDialog />
    {:else if dialogs.open === "about"}
      <AboutDialog />
    {/if}
    {#if remote.connecting}
      <ConnectPhoneDialog />
    {/if}
    {#if importer.isOpen}
      <ImportDialog />
    {/if}
    {#if link.joining}
      <JoinDialog />
    {/if}
    <!-- Last, so it shows above any other window: a phone or computer is waiting. -->
    {#if remote.requests[0]}
      {#key remote.requests[0].id}
        <PairRequestDialog request={remote.requests[0]} />
      {/key}
    {/if}

    <PromptHost />

    {#if engine.error}
      <div class="error" role="alert">{engine.error}</div>
    {/if}
  </div>
{/if}

<TakeOverPrompt />

{#if link.joined && !link.status.connected}
  <div class="offline" role="status">
    {t("link.reconnectingTo", { name: link.status.hostName })}
  </div>
{/if}

<style>
  .offline {
    position: fixed;
    top: 56px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 50;
    padding: var(--sw-space-1) var(--sw-space-3);
    border-radius: var(--sw-radius);
    background: var(--sw-warning, #f59f00);
    color: #000;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.4);
  }

  .operator {
    display: grid;
    grid-template-columns: 240px 1fr 1fr;
    grid-template-rows: auto minmax(0, 1fr) minmax(220px, 34vh) auto;
    grid-template-areas:
      "toolbar  toolbar toolbar"
      "schedule preview live"
      "library  library library"
      "player   player  player";
    gap: 1px;
    height: 100vh;
    overflow: hidden;
    background: var(--sw-border);
  }

  .player {
    grid-area: player;
  }

  .error {
    position: fixed;
    right: var(--sw-space-4);
    bottom: var(--sw-space-4);
    max-width: 420px;
    padding: var(--sw-space-3) var(--sw-space-4);
    border-radius: var(--sw-radius);
    background: var(--sw-live);
    color: #fff;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.4);
  }
</style>
