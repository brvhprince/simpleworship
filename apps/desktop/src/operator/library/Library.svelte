<!--
  @component
  The library along the bottom of the operator window, one tab per kind of
  content. Tabs for content that is not built yet are shown but disabled,
  so the layout already matches what operators know from EasyWorship.
-->
<script lang="ts">
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

  // The H key shows the newest reference heard: in the Scripture tab.
  $effect(() => {
    if (listen.request) current = "scripture";
  });
</script>

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
    gap: var(--sw-space-1);
    padding: var(--sw-space-1) var(--sw-space-2) 0;
    border-bottom: 1px solid var(--sw-border);
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
