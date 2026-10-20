<!--
  @component
  A toolbar button with a panel that drops down below it. The panel closes
  on Escape or a click elsewhere.

  - `align`: which edge of the button the panel lines up with: "start"
    for buttons on the left of the toolbar, "end" (default) for those on
    the right, so the panel always opens towards the window.

  The panel never runs past the window: it is at most as wide as the
  window allows, and scrolls when it is taller.
-->
<script lang="ts">
  import { Button } from "@simpleworship/ui";
  import type { Snippet } from "svelte";

  interface Props {
    label: string;
    open: boolean;
    /** Show the button as "on", e.g. while an alert is showing. */
    active?: boolean;
    shortcut?: string;
    align?: "start" | "end";
    ontoggle: () => void;
    onclose: () => void;
    children: Snippet;
  }

  let {
    label,
    open,
    active = false,
    shortcut,
    align = "end",
    ontoggle,
    onclose,
    children,
  }: Props = $props();

  let wrapper: HTMLDivElement;

  function handleWindowClick(event: MouseEvent) {
    // The path is fixed when the click happens, so a button that removes
    // itself (e.g. "Clear") still counts as a click inside.
    if (open && !event.composedPath().includes(wrapper)) onclose();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (open && event.key === "Escape") {
      event.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleKeydown} />

<div class="popover" bind:this={wrapper}>
  <Button variant="live" {active} {shortcut} aria-expanded={open} onclick={ontoggle}
    >{label}</Button
  >
  {#if open}
    <div class="panel {align}" role="dialog" aria-label={label}>
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .popover {
    position: relative;
  }

  .panel {
    position: absolute;
    top: calc(100% + 6px);
    z-index: 20;
    width: 380px;
    max-width: calc(100vw - 24px);
    max-height: calc(100vh - 80px);
    overflow-y: auto;
    box-sizing: border-box;
    padding: var(--sw-space-3);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    background: var(--sw-surface);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.45);
  }

  .start {
    left: 0;
  }

  .end {
    right: 0;
  }
</style>
