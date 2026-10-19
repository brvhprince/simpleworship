<!--
  @component
  A button in SimpleWorship style.

  - `variant`: "default", "primary" (the main action) or "live" (affects
    what the audience sees).
  - `active`: shows the button as switched on, for toggles such as Black.
  - `shortcut`: a key hint shown next to the label, e.g. "F6".

  Usage:
  ```svelte
  <Button variant="live" active={black} shortcut="F6" onclick={toggleBlack}>Black</Button>
  ```
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";

  interface Props extends HTMLButtonAttributes {
    variant?: "default" | "primary" | "live";
    active?: boolean;
    shortcut?: string;
    children: Snippet;
  }

  let {
    variant = "default",
    active = false,
    shortcut,
    children,
    type = "button",
    ...rest
  }: Props = $props();
</script>

<button class="button {variant}" class:active aria-pressed={active || undefined} {type} {...rest}>
  {@render children()}
  {#if shortcut}
    <kbd>{shortcut}</kbd>
  {/if}
</button>

<style>
  .button {
    display: inline-flex;
    align-items: center;
    gap: var(--sw-space-2);
    padding: var(--sw-space-1) var(--sw-space-3);
    min-height: 28px;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    background: var(--sw-surface-raised);
    cursor: pointer;
    white-space: nowrap;
  }

  .button:hover:not(:disabled) {
    background: var(--sw-surface-hover);
  }

  .button:disabled {
    opacity: 0.45;
    cursor: default;
  }

  .primary {
    background: var(--sw-accent);
    border-color: var(--sw-accent);
  }

  .primary:hover:not(:disabled) {
    background: var(--sw-accent-strong);
  }

  .live.active {
    background: var(--sw-live);
    border-color: var(--sw-live);
  }

  .default.active {
    background: var(--sw-accent);
    border-color: var(--sw-accent);
  }

  kbd {
    font-family: var(--sw-font-mono);
    font-size: var(--sw-font-size-small);
    color: var(--sw-text-muted);
  }

  .active kbd,
  .primary kbd {
    color: inherit;
    opacity: 0.8;
  }
</style>
