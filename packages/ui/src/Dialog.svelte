<!--
  @component
  A modal dialog built on the native `<dialog>` element, so focus trapping
  and the Escape key work out of the box.

  The dialog is open while it is rendered. To close it, stop rendering it:
  ```svelte
  {#if editing}
    <Dialog title="Edit song" onclose={() => (editing = false)}>…</Dialog>
  {/if}
  ```
  `onclose` runs when the operator presses Escape.

  `size="small"` suits a short question with a few buttons.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title: string;
    /** "large" (default) for editors, "small" for short questions. */
    size?: "large" | "small";
    onclose: () => void;
    children: Snippet;
    /** Buttons shown at the bottom right. */
    actions?: Snippet;
  }

  let { title, size = "large", onclose, children, actions }: Props = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
    return () => dialog.close();
  });

  /** Escape fires "cancel". Stop the browser closing the dialog by itself:
   *  the parent decides, by no longer rendering it. */
  function handleCancel(event: Event) {
    event.preventDefault();
    onclose();
  }
</script>

<dialog bind:this={dialog} class={size} oncancel={handleCancel} aria-label={title}>
  <header>{title}</header>
  <div class="body">
    {@render children()}
  </div>
  {#if actions}
    <footer>{@render actions()}</footer>
  {/if}
</dialog>

<style>
  dialog {
    width: min(1100px, 92vw);
    max-height: 90vh;
    padding: 0;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    background: var(--sw-surface);
    color: var(--sw-text);
  }

  dialog[open] {
    display: flex;
    flex-direction: column;
  }

  dialog.small {
    width: min(440px, 92vw);
  }

  dialog::backdrop {
    background: rgba(0, 0, 0, 0.6);
  }

  header {
    padding: var(--sw-space-3) var(--sw-space-4);
    font-weight: 600;
    border-bottom: 1px solid var(--sw-border);
  }

  .body {
    padding: var(--sw-space-4);
    overflow: auto;
    /* Start from the content's own height, and shrink (scrolling) only when
       the window is too small. Plain `flex: 1` means "start from zero
       height", which WebKit (the macOS and Linux webview) honours literally
       inside an auto-height dialog, collapsing the body to nothing. */
    flex: 1 1 auto;
    min-height: 0;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--sw-space-2);
    padding: var(--sw-space-3) var(--sw-space-4);
    border-top: 1px solid var(--sw-border);
  }
</style>
