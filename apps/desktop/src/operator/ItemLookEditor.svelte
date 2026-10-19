<!--
  @component
  The look of one schedule item: a theme for this item only, and optionally
  a background of its own (which replaces the theme's background). For a
  presentation or slideshow, also whether it moves to the next slide by
  itself.
-->
<script lang="ts">
  import type { Background, ItemLook, ScheduleItem } from "@simpleworship/protocol";
  import { engine } from "../lib/engine.svelte.ts";
  import { t } from "../lib/i18n/i18n.svelte.ts";
  import BackgroundEditor from "./themes/BackgroundEditor.svelte";

  interface Props {
    item: ScheduleItem;
  }

  let { item }: Props = $props();

  /** A plain black background, the starting point for "own background". */
  const BLACK: Background = { type: "color", color: "#000000" };

  const themes = $derived(engine.themes.themes);

  /** How long each slide can stay up when moving on by itself. */
  const ADVANCE_SECONDS = [3, 5, 8, 10, 15, 20, 30, 60];

  /** The item's presentation settings, when it is one. */
  const deck = $derived(item.content.type === "deck" ? item.content.deck : null);
  /** Pictures and imported pages can become a slideshow too. */
  const canAdvance = $derived(deck !== null || item.content.type === "slides");

  function setAdvance(seconds: number, loop: boolean) {
    engine.dispatch({
      type: "setItemAutoAdvance",
      id: item.id,
      autoAdvance: seconds > 0 ? { seconds, loop } : null,
    });
  }

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
    {t("common.theme")}
    <select value={item.look.theme?.id ?? ""} onchange={(e) => chooseTheme(e.currentTarget.value)}>
      <option value="">{t("look.automatic")}</option>
      {#each themes as theme (theme.id)}
        <option value={theme.id}>{theme.theme.name}</option>
      {/each}
      {#if item.look.theme && !themes.some((theme) => theme.id === item.look.theme?.id)}
        <!-- A theme from another computer, carried in the schedule file. -->
        <option value={item.look.theme.id}>
          {t("look.fromFile", { name: item.look.theme.theme.name })}
        </option>
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
    {t("look.ownBackground")}
  </label>
  {#if item.look.background}
    <BackgroundEditor
      value={item.look.background}
      onchange={(background) => setLook({ ...item.look, background })}
    />
  {/if}
  {#if canAdvance}
    {@const advance = deck?.autoAdvance ?? null}
    <label class="row">
      {t("advance.nextSlide")}
      <select
        value={advance?.seconds ?? 0}
        onchange={(e) => setAdvance(Number(e.currentTarget.value), advance?.loop ?? true)}
      >
        <option value={0}>{t("advance.manual")}</option>
        {#each ADVANCE_SECONDS as seconds (seconds)}
          <option value={seconds}>{t("advance.every", { seconds })}</option>
        {/each}
      </select>
    </label>
    {#if advance}
      <label class="row">
        <input
          type="checkbox"
          checked={advance.loop}
          onchange={(e) => setAdvance(advance.seconds, e.currentTarget.checked)}
        >
        {t("advance.loop")}
      </label>
    {/if}
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
