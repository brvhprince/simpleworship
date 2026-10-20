<!--
  @component
  The bar across the top of the operator window, on one line at any width.

  On the left, the things set up before the service, each a panel:

  - **Screens**: which screen the projector, stage display and stream
    window go on, and switching each on or off.
  - **Transitions**: cut or fade (and how long) for each kind of content.
  - **Stream**: the browser source for OBS or vMix.
  - **Remote**: phones and tablets.

  On the right, what the operator reaches for during the service: Black,
  Logo, Clear Text and Clear Background, then the Alert and Stage panels.

  On narrower screens the keyboard hints are left out, then the name.
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import { outputs } from "../lib/outputs.svelte.ts";
  import { panels } from "../lib/panels.svelte.ts";
  import { remote } from "../lib/remote.svelte.ts";
  import { updates } from "../lib/updates.svelte.ts";
  import AlertsPanel from "./AlertsPanel.svelte";
  import Popover from "./Popover.svelte";
  import RemotePanel from "./RemotePanel.svelte";
  import ScreensPanel from "./ScreensPanel.svelte";
  import StagePanel from "./StagePanel.svelte";
  import StreamPanel from "./StreamPanel.svelte";
  import TransitionsPanel from "./TransitionsPanel.svelte";

  const flags = $derived(engine.output?.flags);
  const screensOn = $derived(outputs.open.length);
</script>

<header class="toolbar">
  <div class="bar">
    <span class="brand">SimpleWorship</span>

    <div class="group">
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
        label={t("toolbar.transitions")}
        align="start"
        open={panels.open === "transitions"}
        ontoggle={() => panels.toggle("transitions")}
        onclose={() => panels.close()}
      >
        <TransitionsPanel />
      </Popover>
      <Popover
        label={t("toolbar.stream")}
        align="start"
        open={panels.open === "stream"}
        active={(remote.status?.streamLinks.length ?? 0) > 0}
        ontoggle={() => panels.toggle("stream")}
        onclose={() => panels.close()}
      >
        <StreamPanel />
      </Popover>
      <Popover
        label={remote.connectedCount > 0
          ? t("toolbar.remoteConnected", { count: remote.connectedCount })
          : t("toolbar.remote")}
        align="start"
        open={panels.open === "remote"}
        active={remote.connectedCount > 0}
        ontoggle={() => panels.toggle("remote")}
        onclose={() => panels.close()}
      >
        <RemotePanel />
      </Popover>
    </div>

    <span class="spacer"></span>

    {#if updates.status === "ready" || updates.status === "installing"}
      <Button
        variant="primary"
        disabled={updates.status === "installing"}
        title={t("toolbar.updateHint", { version: updates.version ?? "" })}
        onclick={() => updates.install()}
      >
        {updates.status === "installing" ? t("toolbar.updateInstalling") : t("toolbar.updateReady")}
      </Button>
    {/if}

    {#if flags}
      <fieldset class="group" aria-label={t("toolbar.displayControls")}>
        <Button
          variant="live"
          active={flags.black}
          shortcut="F6"
          onclick={() => engine.dispatch({ type: "setBlack", on: !flags.black })}
        >
          {t("toolbar.black")}
        </Button>
        <Button
          variant="live"
          active={flags.logo}
          shortcut="F7"
          onclick={() => engine.dispatch({ type: "setLogo", on: !flags.logo })}
        >
          {t("toolbar.logo")}
        </Button>
        <Button
          variant="live"
          active={flags.hideText}
          shortcut="F8"
          onclick={() => engine.dispatch({ type: "setHideText", on: !flags.hideText })}
        >
          {t("toolbar.clearText")}
        </Button>
        <Button
          variant="live"
          active={flags.hideBackground}
          shortcut="F9"
          onclick={() => engine.dispatch({ type: "setHideBackground", on: !flags.hideBackground })}
        >
          {t("toolbar.clearBackground")}
        </Button>
      </fieldset>
    {/if}

    <div class="group">
      <Popover
        label={t("toolbar.alert")}
        shortcut="A"
        open={panels.open === "alerts"}
        active={engine.output?.alert != null}
        ontoggle={() => panels.toggle("alerts")}
        onclose={() => panels.close()}
      >
        <AlertsPanel />
      </Popover>
      <Popover
        label={t("toolbar.stage")}
        open={panels.open === "stage"}
        active={engine.stage.timer != null || engine.stage.message != null}
        ontoggle={() => panels.toggle("stage")}
        onclose={() => panels.close()}
      >
        <StagePanel />
      </Popover>
    </div>
  </div>
</header>

<style>
  /* The header is the size container; the bar inside it adapts. */
  .toolbar {
    grid-area: toolbar;
    container-type: inline-size;
    background: var(--sw-surface);
    /* Panels drop down over the panes below. */
    position: relative;
    z-index: 10;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: var(--sw-space-3);
    padding: var(--sw-space-2) var(--sw-space-3);
  }

  .brand {
    font-weight: 700;
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .group {
    margin: 0;
    padding: 0;
    border: none;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
  }

  .spacer {
    flex: 1;
  }

  /* Smaller screens: drop the keyboard hints first, then the name. */
  @container (max-width: 1240px) {
    .bar :global(kbd) {
      display: none;
    }
  }

  @container (max-width: 1000px) {
    .brand {
      display: none;
    }

    .bar {
      gap: var(--sw-space-2);
    }
  }
</style>
