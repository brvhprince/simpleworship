<!--
  @component
  The look of one schedule item: a theme for this item only, and optionally
  a background of its own (which replaces the theme's background).
-->
<script lang="ts">
  import type { Background, ItemLook, ScheduleItem } from "@simpleworship/protocol";
  import { engine } from "../lib/engine.svelte.ts";
  import BackgroundEditor from "./themes/BackgroundEditor.svelte";

  interface Props {
    item: ScheduleItem;
  }

  let { item }: Props = $props();

  /** A plain black background, the starting point for "own background". */
  const BLACK: Background = { type: "color", color: "#000000" };

  const themes = $derived(engine.themes.themes);

  function setLook(look: ItemLook) {
    engine.dispatch({ type: "setScheduleItemLook", id: item.id, look });
  }

  function chooseTheme(id: string) {
    const record = themes.find((theme) => theme.id === id) ?? null;
    setLook({ ...item.look, theme: record });
  }
</script>

<div class="look">
  <label class="row">
    Theme
    <select value={item.look.theme?.id ?? ""} onchange={(e) => chooseTheme(e.currentTarget.value)}>
      <option value="">Automatic</option>
      {#each themes as theme (theme.id)}
        <option value={theme.id}>{theme.theme.name}</option>
      {/each}
      {#if item.look.theme && !themes.some((theme) => theme.id === item.look.theme?.id)}
        <!-- A theme from another computer, carried in the schedule file. -->
        <option value={item.look.theme.id}>{item.look.theme.theme.name} (from file)</option>
      {/if}
    </select>
  </label>

  <label class="row">
    <input
      type="checkbox"
      checked={item.look.background !== null}
      onchange={(e) =>
        setLook({ ...item.look, background: e.currentTarget.checked ? BLACK : null })}
    >
    Own background
  </label>
  {#if item.look.background}
    <BackgroundEditor
      value={item.look.background}
      onchange={(background) => setLook({ ...item.look, background })}
    />
  {/if}
</div>

<style>
  .look {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    color: var(--sw-text-muted);
  }

  select {
    flex: 1;
    min-width: 0;
  }
</style>
