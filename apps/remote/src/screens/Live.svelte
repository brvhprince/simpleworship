<!--
  @component
  What is live, and moving through it: the live slide as the room sees it,
  big Previous and Next buttons, the sections (V1, C, B…) and every slide
  to tap. Full control also has Black, Logo, Clear Text and Clear
  Background; the presenter has Black; the worship leader moves through
  songs only.
-->
<script lang="ts">
  import { t } from "@simpleworship/i18n";
  import { frameAt, ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { remote } from "../lib/remote.svelte.ts";

  const live = $derived(remote.live);
  const flags = $derived(remote.output?.flags);
  const frame = $derived(remote.output?.frame ?? null);
  const slides = $derived(live?.presentation.slides ?? []);
  const isSong = $derived(live?.presentation.source.type === "song");

  /** The worship leader may only move through songs. */
  const canMove = $derived(remote.role !== "worshipLeader" || isSong);
  const isFull = $derived(remote.role === "full");

  /** The first slide of each section, for the section buttons. */
  const sections = $derived(
    slides.flatMap((slide, index) =>
      slide.startsSection && slide.section
        ? [{ index, label: sectionShortcut(slide.section.name) }]
        : [],
    ),
  );

  /** "Verse 2" → "V2", "Chorus" → "C": short enough for a button. */
  function sectionShortcut(name: string): string {
    const [word = "", number = ""] = name.split(/\s+/);
    return `${word.charAt(0).toUpperCase()}${/^\d+$/.test(number) ? number : ""}`;
  }

  function select(index: number) {
    void remote.dispatch({ type: "selectLiveSlide", index });
  }

  function toggle(flag: "black" | "logo" | "hideText" | "hideBackground") {
    if (!flags) return;
    const on = !flags[flag];
    const command = {
      black: { type: "setBlack", on },
      logo: { type: "setLogo", on },
      hideText: { type: "setHideText", on },
      hideBackground: { type: "setHideBackground", on },
    } as const;
    void remote.dispatch(command[flag]);
  }
</script>

<section class="live">
  <header>
    <span class="badge">{t("pane.live")}</span>
    <span class="title">{live?.presentation.title ?? t("pane.nothingLive")}</span>
  </header>

  <div class="slide" class:dark={flags?.black}>
    {#if frame && !flags?.black}
      <ScaledCanvas><SlideView {frame} showText={!flags?.hideText} /></ScaledCanvas>
    {:else}
      <div class="blank">{flags?.black ? t("stageView.black") : t("pane.nothingLive")}</div>
    {/if}
    {#if flags?.logo}
      <span class="flag">{t("stageView.logo")}</span>
    {:else if flags?.hideText}
      <span class="flag">{t("stageView.textHidden")}</span>
    {/if}
  </div>

  {#if !canMove}
    <p class="hint">{t("remote.notASong")}</p>
  {:else}
    <div class="move">
      <button
        type="button"
        class="step"
        disabled={!live}
        onclick={() => void remote.dispatch({ type: "previousSlide" })}
      >
        ‹ {t("remote.previous")}
      </button>
      <button
        type="button"
        class="step next"
        disabled={!live}
        onclick={() => void remote.dispatch({ type: "nextSlide" })}
      >
        {t("remote.next")}
        ›
      </button>
    </div>

    {#if sections.length > 1}
      <fieldset class="sections" aria-label={t("remote.sections")}>
        {#each sections as section (section.index)}
          <button
            type="button"
            class:current={live?.selected === section.index}
            onclick={() => select(section.index)}
          >
            {section.label}
          </button>
        {/each}
      </fieldset>
    {/if}
  {/if}

  {#if isFull || remote.role === "presenter"}
    <div class="controls">
      <button type="button" class:on={flags?.black} onclick={() => toggle("black")}>
        {t("toolbar.black")}
      </button>
      {#if isFull}
        <button type="button" class:on={flags?.logo} onclick={() => toggle("logo")}>
          {t("toolbar.logo")}
        </button>
        <button type="button" class:on={flags?.hideText} onclick={() => toggle("hideText")}>
          {t("toolbar.clearText")}
        </button>
        <button
          type="button"
          class:on={flags?.hideBackground}
          onclick={() => toggle("hideBackground")}
        >
          {t("toolbar.clearBackground")}
        </button>
      {/if}
    </div>
  {/if}

  {#if live && canMove}
    <ol class="slides">
      {#each slides as slide, index (index)}
        {@const thumbnail = frameAt(live.presentation, index)}
        <li>
          <button
            type="button"
            class:current={index === live.selected}
            onclick={() => select(index)}
          >
            {#if thumbnail}
              <ScaledCanvas><SlideView frame={thumbnail} /></ScaledCanvas>
            {/if}
            <span class="number">
              {index + 1}{slide.startsSection && slide.section ? ` · ${slide.section.name}` : ""}
            </span>
          </button>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .live {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .badge {
    padding: 2px 8px;
    border-radius: 6px;
    background: var(--sw-live);
    color: #fff;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }

  .slide {
    position: relative;
    border: 2px solid var(--sw-live);
    border-radius: 8px;
    overflow: hidden;
    background: #000;
  }

  .blank {
    aspect-ratio: 16 / 9;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--sw-text-faint);
  }

  .flag {
    position: absolute;
    top: 8px;
    right: 8px;
    padding: 2px 8px;
    border-radius: 6px;
    background: rgba(0, 0, 0, 0.75);
    font-size: 13px;
    color: #fff; /* on the slide, in both appearances */
  }

  .move {
    display: grid;
    grid-template-columns: 1fr 1.6fr;
    gap: 10px;
  }

  .step {
    min-height: 72px;
    border: 1px solid var(--sw-border);
    border-radius: 12px;
    background: var(--sw-surface-raised);
    color: var(--sw-text);
    font-size: 20px;
    font-weight: 600;
  }

  .step.next {
    border-color: var(--sw-live);
    background: var(--sw-live);
    color: #fff;
  }

  .step:disabled {
    opacity: 0.4;
  }

  .sections {
    margin: 0;
    padding: 0;
    border: none;
  }

  .sections,
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .sections button,
  .controls button {
    min-width: 52px;
    min-height: 44px;
    padding: 0 12px;
    border: 1px solid var(--sw-border);
    border-radius: 10px;
    background: var(--sw-surface);
    color: var(--sw-text);
  }

  .sections button.current {
    border-color: var(--sw-accent);
    color: var(--sw-accent);
  }

  .controls button.on {
    border-color: var(--sw-live);
    background: var(--sw-live-soft);
  }

  .hint {
    margin: 0;
    color: var(--sw-text-muted);
    text-align: center;
  }

  .slides {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 8px;
  }

  .slides button {
    width: 100%;
    padding: 0;
    border: 2px solid transparent;
    border-radius: 8px;
    background: var(--sw-surface);
    overflow: hidden;
    color: var(--sw-text-muted);
    text-align: left;
  }

  .slides button.current {
    border-color: var(--sw-live);
  }

  .number {
    display: block;
    padding: 2px 6px;
    font-size: 12px;
  }
</style>
