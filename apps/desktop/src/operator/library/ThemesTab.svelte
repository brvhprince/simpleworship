<!--
  @component
  The theme library: every theme as a preview, which theme songs, scripture
  and text use by default, and buttons to create, edit and delete themes.
-->
<script lang="ts">
  import type { ThemeDefaults, ThemeRecord } from "@simpleworship/protocol";
  import { ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { Button } from "@simpleworship/ui";
  import { deleteTheme, saveTheme, setThemeDefaults, uuidV7 } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { prompts } from "../../lib/prompts.svelte.ts";
  import { sampleLyrics } from "../../lib/samples.ts";
  import ThemeDesigner from "../themes/ThemeDesigner.svelte";

  /** The kinds of content with a default theme, and their labels. */
  const KINDS: [keyof ThemeDefaults, string][] = $derived([
    ["song", t("library.songs")],
    ["scripture", t("library.scripture")],
    ["text", t("library.text")],
    ["presentation", t("library.presentations")],
  ]);

  let selectedId: string | null = $state(null);
  /** The theme open in the designer, if any. */
  let editing: ThemeRecord | null = $state(null);

  const themes = $derived(engine.themes.themes);
  const defaults = $derived(engine.themes.defaults);
  const selected = $derived(themes.find((theme) => theme.id === selectedId) ?? null);

  /** A copy of `source` (or of the first theme) under a new name. */
  function copyOf(source: ThemeRecord | null, name: string): ThemeRecord | null {
    const base = source ?? themes[0];
    if (!base) return null;
    const theme = structuredClone($state.snapshot(base.theme));
    theme.name = name;
    return { id: uuidV7(), theme, builtin: false };
  }

  async function duplicate() {
    const copy = copyOf(selected, t("themes.copyName", { name: selected?.theme.name ?? "" }));
    if (!copy) return;
    try {
      await saveTheme(copy);
      selectedId = copy.id;
    } catch (error) {
      engine.showError(error);
    }
  }

  function create() {
    editing = copyOf(null, t("themes.newName"));
  }

  function editSelected() {
    if (selected) editing = selected;
  }

  async function remove(theme: ThemeRecord) {
    const answer = await prompts.ask({
      title: t("themes.deleteTitle"),
      message: t("themes.deleteMessage", { name: theme.theme.name }),
      buttons: [
        { id: "cancel", label: t("common.cancel") },
        { id: "delete", label: t("common.delete"), variant: "live" },
      ],
      cancel: "cancel",
    });
    if (answer !== "delete") return;
    try {
      await deleteTheme(theme.id);
      selectedId = null;
    } catch (error) {
      engine.showError(error);
    }
  }

  async function useFor(kind: keyof ThemeDefaults, theme: ThemeRecord) {
    try {
      await setThemeDefaults({ ...defaults, [kind]: theme.id });
    } catch (error) {
      engine.showError(error);
    }
  }

  /** Which kinds of content use `theme` by default, e.g. ["Songs"]. */
  function usedFor(theme: ThemeRecord): string[] {
    return KINDS.filter(([kind]) => defaults[kind] === theme.id).map(([, label]) => label);
  }
</script>

<div class="themes-tab">
  <div class="toolbar">
    <Button onclick={create}>{t("themes.new")}</Button>
    <Button disabled={!selected} onclick={duplicate}>{t("themes.duplicate")}</Button>
    <Button disabled={!selected} onclick={editSelected}>{t("common.edit")}</Button>
    <Button disabled={!selected} onclick={() => selected && remove(selected)}
      >{t("common.delete")}</Button
    >

    <span class="spacer"></span>

    {#if selected}
      <span class="label">{t("themes.useFor", { name: selected.theme.name })}</span>
      {#each KINDS as [kind, label] (kind)}
        <Button active={defaults[kind] === selected.id} onclick={() => useFor(kind, selected)}>
          {label}
        </Button>
      {/each}
    {/if}
  </div>

  <ul class="grid" aria-label={t("themes.list")}>
    {#each themes as theme (theme.id)}
      <li>
        <button
          type="button"
          class="card"
          class:selected={theme.id === selectedId}
          onclick={() => (selectedId = theme.id)}
          ondblclick={() => (editing = theme)}
        >
          <ScaledCanvas><SlideView frame={sampleLyrics(theme.theme)} /></ScaledCanvas>
          <span class="caption">
            <span class="title">{theme.theme.name}</span>
            {#each usedFor(theme) as label (label)}
              <span class="badge">{label}</span>
            {/each}
          </span>
        </button>
      </li>
    {/each}
  </ul>
</div>

{#if editing}
  <ThemeDesigner record={editing} onclose={() => (editing = null)} />
{/if}

<style>
  .themes-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--sw-space-2);
    padding: var(--sw-space-2) var(--sw-space-3);
  }

  .spacer {
    flex: 1;
  }

  .label {
    color: var(--sw-text-muted);
  }

  .grid {
    list-style: none;
    margin: 0;
    padding: var(--sw-space-3);
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    align-content: start;
    gap: var(--sw-space-3);
    overflow-y: auto;
    flex: 1;
    border-top: 1px solid var(--sw-border);
  }

  .card {
    width: 100%;
    padding: 0;
    border: 2px solid transparent;
    border-radius: var(--sw-radius);
    background: var(--sw-surface-raised);
    cursor: pointer;
    text-align: left;
    overflow: hidden;
  }

  .card:hover {
    border-color: var(--sw-surface-hover);
  }

  .card.selected {
    border-color: var(--sw-accent);
  }

  .caption {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    padding: var(--sw-space-1) var(--sw-space-2);
  }

  .title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    padding: 0 6px;
    border-radius: var(--sw-radius-small);
    background: var(--sw-accent);
    font-size: var(--sw-font-size-small);
  }
</style>
