<!--
  @component
  The library along the bottom of the operator window, one tab per kind of
  content. Tabs for content that is not built yet are shown but disabled,
  so the layout already matches what operators know from EasyWorship.
-->
<script lang="ts">
  import MediaTab from "./MediaTab.svelte";
  import ScriptureTab from "./ScriptureTab.svelte";
  import SongsTab from "./SongsTab.svelte";
  import TextTab from "./TextTab.svelte";
  import ThemesTab from "./ThemesTab.svelte";

  interface Tab {
    id: string;
    label: string;
    available: boolean;
  }

  const tabs: Tab[] = [
    { id: "songs", label: "Songs", available: true },
    { id: "text", label: "Text", available: true },
    { id: "scripture", label: "Scripture", available: true },
    { id: "media", label: "Media", available: true },
    { id: "presentations", label: "Presentations", available: false },
    { id: "themes", label: "Themes", available: true },
  ];

  let current = $state("songs");
</script>

<section class="library" aria-label="Library">
  <div class="tabs" role="tablist">
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        role="tab"
        aria-selected={current === tab.id}
        disabled={!tab.available}
        title={tab.available ? undefined : "Coming soon"}
        onclick={() => (current = tab.id)}
      >
        {tab.label}
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
</style>
