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
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { engine } from "../lib/engine.svelte.ts";
  import { scheduleFile } from "../lib/schedule-file.svelte.ts";
  import {
    commandForKey,
    hasNumberedSection,
    isTypingTarget,
    SectionJumper,
  } from "../lib/shortcuts.ts";
  import Library from "./library/Library.svelte";
  import PromptHost from "./PromptHost.svelte";
  import SchedulePanel from "./SchedulePanel.svelte";
  import SlidePane from "./SlidePane.svelte";
  import Toolbar from "./Toolbar.svelte";

  onMount(() => {
    const connections = [engine.connect(), scheduleFile.connect()];
    for (const connection of connections) {
      connection.catch((error) => engine.showError(error));
    }
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
    const isModified = event.metaKey || event.ctrlKey || event.altKey;
    // Space and Enter on a focused button should press that button.
    const isButtonActivation =
      event.target instanceof HTMLButtonElement && (event.key === " " || event.key === "Enter");

    if (event.defaultPrevented || isModified || isButtonActivation) return;
    if (isTypingTarget(event.target) || !engine.output) return;

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
        Go Live
      </Button>
    {/snippet}
  </SlidePane>

  <SlidePane
    kind="live"
    pane={engine.live}
    output={engine.output}
    onselect={(index) => engine.dispatch({ type: "selectLiveSlide", index })}
  >
    {#snippet actions()}
      <Button disabled={!engine.live} onclick={() => engine.dispatch({ type: "clearLive" })}>
        Clear
      </Button>
    {/snippet}
  </SlidePane>

  <Library />

  <PromptHost />

  {#if engine.error}
    <div class="error" role="alert">{engine.error}</div>
  {/if}
</div>

<style>
  .operator {
    display: grid;
    grid-template-columns: 240px 1fr 1fr;
    grid-template-rows: auto minmax(0, 1fr) minmax(220px, 34vh);
    grid-template-areas:
      "toolbar  toolbar toolbar"
      "schedule preview live"
      "library  library library";
    gap: 1px;
    height: 100vh;
    overflow: hidden;
    background: var(--sw-border);
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
