<!--
  @component
  The library along the bottom of the operator window, one tab per kind of
  content. Tabs for content that is not built yet are shown but disabled,
  so the layout already matches what operators know from EasyWorship.
-->
<script lang="ts">
  import { showTextInPreview } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { listen } from "../../lib/listen.svelte.ts";
  import MediaTab from "./MediaTab.svelte";
  import PresentationsTab from "./PresentationsTab.svelte";
  import ScriptureTab from "./ScriptureTab.svelte";
  import SongsTab from "./SongsTab.svelte";
  import TextTab from "./TextTab.svelte";
  import ThemesTab from "./ThemesTab.svelte";

  interface Tab {
    id: string;
    label: string;
  }

  const tabs: Tab[] = $derived([
    { id: "songs", label: t("library.songs") },
    { id: "text", label: t("library.text") },
    { id: "scripture", label: t("library.scripture") },
    { id: "media", label: t("library.media") },
    { id: "presentations", label: t("library.presentations") },
    { id: "themes", label: t("library.themes") },
  ]);

  let current = $state("songs");
  let quickText = $state("");
  let quickTextOpen = $state(false);

  async function previewQuickText() {
    if (!quickText.trim()) return;
    try {
      await showTextInPreview(t("library.quickText"), quickText.trim());
    } catch (error) {
      engine.showError(error);
    }
  }

  async function sendQuickTextLive() {
    if (!quickText.trim()) return;
    try {
      await showTextInPreview(t("library.quickText"), quickText.trim());
      engine.dispatch({ type: "goLive" });
      quickText = "";
      quickTextOpen = false;
    } catch (error) {
      engine.showError(error);
    }
  }

  /** The Quick Text button and its popover, for closing it on a click outside. */
  let quickTextTool: HTMLElement | undefined = $state();
  /** The Quick Text button, which gets the focus back when Esc closes the popover. */
  let quickTextButton: HTMLButtonElement | undefined = $state();
  /** The popover's text box, focused when it opens: it opens to be typed in. */
  let quickTextBox: HTMLTextAreaElement | undefined = $state();

  $effect(() => {
    if (quickTextOpen) quickTextBox?.focus();
  });

  /**
   * Closes the Quick Text popover on a click or tap anywhere else, as
   * `Popover` does. The path is fixed when the click happens, so a button
   * that removes itself still counts as a click inside.
   */
  function closeQuickTextOutside(event: PointerEvent) {
    if (quickTextOpen && quickTextTool && !event.composedPath().includes(quickTextTool)) {
      quickTextOpen = false;
    }
  }

  // The H key shows the newest reference heard: in the Scripture tab.
  $effect(() => {
    if (listen.request) current = "scripture";
  });
</script>

<svelte:window onpointerdown={closeQuickTextOutside} />

<section class="library" aria-label={t("library.title")}>
  <div class="tabs" role="tablist">
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        role="tab"
        aria-selected={current === tab.id}
        onclick={() => (current = tab.id)}
      >
        {tab.label}
        {#if tab.id === "scripture" && current !== "scripture" && listen.unseen > 0}
          <span class="badge" title={t("listen.newHeard")}>{listen.unseen}</span>
        {/if}
      </button>
    {/each}

    <span class="tab-spacer"></span>

    <div class="quick-text-tool" bind:this={quickTextTool}>
      <button
        type="button"
        class="quick-text-btn"
        bind:this={quickTextButton}
        class:open={quickTextOpen}
        onclick={() => (quickTextOpen = !quickTextOpen)}
        title={t("library.quickTextHint")}
      >
        + {t("library.quickText")}
      </button>

      {#if quickTextOpen}
        <div
          class="quick-text-popover"
          role="dialog"
          aria-label={t("library.quickText")}
          tabindex="-1"
          onkeydown={(event) => {
            if (event.key === "Escape") {
              // Closes the popover only, not whatever else listens for Esc.
              event.stopPropagation();
              quickTextOpen = false;
              // What had the focus is about to go; the keyboard carries on
              // from the button.
              quickTextButton?.focus();
            }
          }}
        >
          <textarea
            bind:this={quickTextBox}
            bind:value={quickText}
            placeholder={t("library.quickTextPlaceholder")}
            rows="2"
            onkeydown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                e.preventDefault();
                void sendQuickTextLive();
              }
            }}
          ></textarea>
          <div class="quick-actions">
            <button
              type="button"
              class="quick-action-btn"
              disabled={!quickText.trim()}
              onclick={previewQuickText}
            >
              {t("common.showInPreview")}
            </button>
            <button
              type="button"
              class="quick-action-btn primary"
              disabled={!quickText.trim()}
              onclick={sendQuickTextLive}
            >
              {t("pane.goLive")}
            </button>
            <button
              type="button"
              class="quick-action-btn close"
              onclick={() => (quickTextOpen = false)}
            >
              ✕
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <div class="panel" role="tabpanel">
    {#if current === "songs"}
      <SongsTab />
    {:else if current === "scripture"}
      <ScriptureTab />
    {:else if current === "text"}
      <TextTab />
    {:else if current === "media"}
      <MediaTab />
    {:else if current === "presentations"}
      <PresentationsTab />
    {:else if current === "themes"}
      <ThemesTab />
    {/if}
  </div>
</section>

<style>
  .library {
    grid-area: library;
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--sw-surface);
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    padding: var(--sw-space-1) var(--sw-space-2) 0;
    border-bottom: 1px solid var(--sw-border);
    position: relative;
  }

  .tab-spacer {
    flex: 1;
  }

  .quick-text-tool {
    position: relative;
    display: flex;
    align-items: center;
    margin-bottom: 2px;
  }

  .quick-text-btn {
    padding: var(--sw-space-1) var(--sw-space-2);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    color: var(--sw-text);
    font-size: var(--sw-font-size-small);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
  }

  .quick-text-btn:hover,
  .quick-text-btn.open {
    border-color: var(--sw-accent);
    color: var(--sw-accent);
  }

  .quick-text-popover {
    position: absolute;
    bottom: 100%;
    right: 0;
    margin-bottom: var(--sw-space-2);
    width: 320px;
    padding: var(--sw-space-2);
    background: var(--sw-surface-raised);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
  }

  .quick-text-popover textarea {
    width: 100%;
    resize: vertical;
    padding: var(--sw-space-1);
    background: var(--sw-surface);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    color: var(--sw-text);
    font-family: inherit;
    font-size: var(--sw-font-size-small);
  }

  .quick-actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--sw-space-1);
  }

  .quick-action-btn {
    padding: var(--sw-space-1) var(--sw-space-2);
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface);
    color: var(--sw-text);
    font-size: var(--sw-font-size-small);
    cursor: pointer;
  }

  .quick-action-btn.primary {
    background: var(--sw-live);
    border-color: var(--sw-live);
    color: #fff;
    font-weight: 600;
  }

  .quick-action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .quick-action-btn.close {
    padding: var(--sw-space-1) var(--sw-space-2);
  }

  [role="tab"] {
    padding: var(--sw-space-2) var(--sw-space-3);
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--sw-text-muted);
    cursor: pointer;
  }

  [role="tab"][aria-selected="true"] {
    color: var(--sw-text);
    border-bottom-color: var(--sw-accent);
  }

  [role="tab"]:disabled {
    color: var(--sw-text-faint);
    cursor: default;
  }

  .panel {
    flex: 1;
    min-height: 0;
  }

  /* References Listen heard while another tab was showing. */
  .badge {
    display: inline-block;
    min-width: 1.4em;
    margin-left: var(--sw-space-1);
    padding: 0 0.35em;
    border-radius: 999px;
    background: var(--sw-preview);
    color: #fff;
    font-size: var(--sw-font-size-small);
    line-height: 1.4;
    text-align: center;
  }
</style>
