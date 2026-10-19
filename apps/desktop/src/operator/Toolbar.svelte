<!--
  @component
  The bar across the top of the operator window:

  - **Outputs**: choose which screen the projector output and the stage
    display appear on, and show or hide them.
  - **Display controls**: Black, Logo, Clear Text, Clear Background.
  - **Transition**: cut or fade, and how long a fade takes.
-->
<script lang="ts">
  import type {
    MonitorInfo,
    OutputKind,
    OutputTarget,
    TransitionKind,
  } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import { onMount } from "svelte";
  import { closeOutput, listMonitors, openOutput } from "../lib/api.ts";
  import { engine } from "../lib/engine.svelte.ts";
  import { panels } from "../lib/panels.svelte.ts";
  import AlertsPanel from "./AlertsPanel.svelte";
  import Popover from "./Popover.svelte";
  import StagePanel from "./StagePanel.svelte";

  /** Value of the "Windowed" choice in the screen pickers. */
  const WINDOWED = "windowed";

  let monitors: MonitorInfo[] = $state([]);
  let projectorScreen = $state(WINDOWED);
  let stageScreen = $state(WINDOWED);

  const flags = $derived(engine.output?.flags);
  const transition = $derived(engine.output?.transition);

  onMount(async () => {
    await refreshMonitors();
    // Ready to go at startup: with a projector or TV connected, the output
    // opens on the first screen that is not the operator's own. With only
    // the main screen, nothing opens (it would cover the operator window).
    const secondary = monitors.find((monitor) => !monitor.isPrimary);
    if (secondary) {
      projectorScreen = String(secondary.index);
      await show("projector", projectorScreen);
    }
  });

  /** Re-reads the monitor list. Called whenever a picker is opened, so a
   *  projector plugged in mid-service shows up. */
  async function refreshMonitors() {
    try {
      monitors = await listMonitors();
    } catch (error) {
      engine.showError(error);
    }
  }

  function targetFor(screen: string): OutputTarget {
    return screen === WINDOWED ? { type: "windowed" } : { type: "monitor", index: Number(screen) };
  }

  async function show(kind: OutputKind, screen: string) {
    try {
      await openOutput(kind, targetFor(screen));
    } catch (error) {
      engine.showError(error);
    }
  }

  async function hide(kind: OutputKind) {
    try {
      await closeOutput(kind);
    } catch (error) {
      engine.showError(error);
    }
  }

  function setTransitionKind(kind: TransitionKind) {
    if (!transition) return;
    engine.dispatch({ type: "setTransition", transition: { ...transition, kind } });
  }

  function setTransitionDuration(seconds: number) {
    if (!transition || !Number.isFinite(seconds)) return;
    const durationMs = Math.round(Math.min(Math.max(seconds, 0), 10) * 1000);
    engine.dispatch({ type: "setTransition", transition: { ...transition, durationMs } });
  }
</script>

{#snippet screenPicker(
  label: string,
  kind: OutputKind,
  screen: string,
  onchange: (value: string) => void,
)}
  <fieldset class="group" aria-label={label}>
    <span class="label">{label}</span>
    <select
      value={screen}
      onfocus={refreshMonitors}
      onchange={(e) => onchange(e.currentTarget.value)}
    >
      <option value={WINDOWED}>In a window</option>
      {#each monitors as monitor (monitor.index)}
        <option value={String(monitor.index)}>
          {monitor.name}
          ({monitor.width}×{monitor.height}){monitor.isPrimary ? " · main" : ""}
        </option>
      {/each}
    </select>
    <Button onclick={() => show(kind, screen)}>Show</Button>
    <Button onclick={() => hide(kind)}>Hide</Button>
  </fieldset>
{/snippet}

<header class="toolbar">
  <span class="brand">SimpleWorship</span>

  {@render screenPicker("Output", "projector", projectorScreen, (v) => (projectorScreen = v))}
  {@render screenPicker("Stage", "stage", stageScreen, (v) => (stageScreen = v))}

  <span class="spacer"></span>

  {#if flags}
    <fieldset class="group" aria-label="Display controls">
      <Button
        variant="live"
        active={flags.black}
        shortcut="F6"
        onclick={() => engine.dispatch({ type: "setBlack", on: !flags.black })}
      >
        Black
      </Button>
      <Button
        variant="live"
        active={flags.logo}
        shortcut="F7"
        onclick={() => engine.dispatch({ type: "setLogo", on: !flags.logo })}
      >
        Logo
      </Button>
      <Button
        variant="live"
        active={flags.hideText}
        shortcut="F8"
        onclick={() => engine.dispatch({ type: "setHideText", on: !flags.hideText })}
      >
        Clear Text
      </Button>
      <Button
        variant="live"
        active={flags.hideBackground}
        shortcut="F9"
        onclick={() => engine.dispatch({ type: "setHideBackground", on: !flags.hideBackground })}
      >
        Clear Background
      </Button>
    </fieldset>
  {/if}

  <Popover
    label="Alert"
    shortcut="A"
    open={panels.open === "alerts"}
    active={engine.output?.alert != null}
    ontoggle={() => panels.toggle("alerts")}
    onclose={() => panels.close()}
  >
    <AlertsPanel />
  </Popover>
  <Popover
    label="Stage"
    open={panels.open === "stage"}
    active={engine.stage.timer != null || engine.stage.message != null}
    ontoggle={() => panels.toggle("stage")}
    onclose={() => panels.close()}
  >
    <StagePanel />
  </Popover>

  {#if transition}
    <fieldset class="group" aria-label="Transition">
      <span class="label">Transition</span>
      <select
        value={transition.kind}
        onchange={(e) => setTransitionKind(e.currentTarget.value as TransitionKind)}
      >
        <option value="fade">Fade</option>
        <option value="cut">Cut</option>
      </select>
      {#if transition.kind === "fade"}
        <input
          class="duration"
          type="number"
          min="0"
          max="10"
          step="0.1"
          value={transition.durationMs / 1000}
          aria-label="Fade duration in seconds"
          onchange={(e) => setTransitionDuration(e.currentTarget.valueAsNumber)}
        >
        <span class="unit">s</span>
      {/if}
    </fieldset>
  {/if}
</header>

<style>
  .toolbar {
    grid-area: toolbar;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--sw-space-4);
    padding: var(--sw-space-2) var(--sw-space-3);
    background: var(--sw-surface);
  }

  .brand {
    font-weight: 700;
    letter-spacing: 0.02em;
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

  .label,
  .unit {
    color: var(--sw-text-muted);
    margin-right: var(--sw-space-1);
  }

  .spacer {
    flex: 1;
  }

  select {
    max-width: 220px;
  }

  .duration {
    width: 64px;
  }
</style>
