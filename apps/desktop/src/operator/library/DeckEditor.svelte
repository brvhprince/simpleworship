<!--
  @component
  The presentation editor: a small PowerPoint for announcements, sermon
  points and anything else typed up before the service.

  - Left: the slides. Click to edit one, drag to reorder; add text slides
    or pictures, duplicate, delete.
  - Middle: the slide as it will look, and its text. B / I / U (or
    ⌘B / ⌘I / ⌘U) make the selected words bold, italic or underlined.
  - Right: this slide's own theme, background, text size, colour and
    alignment. Anything left at "theme" follows the presentation's theme.
  - Top: the presentation's name and theme, and whether it moves to the next
    slide by itself.
-->
<script lang="ts">
  import type {
    Background,
    DeckSlide,
    HorizontalAlign,
    Presentation,
    SlideDeck,
  } from "@simpleworship/protocol";
  import { ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { Button, Dialog } from "@simpleworship/ui";
  import { untrack } from "svelte";
  import { deckPresentation, pictureSlides, savePresentation, uuidV7 } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { frameAt } from "../../lib/frames.ts";
  import { type TextFormat, toggleFormat } from "../../lib/markup.ts";
  import { overrideMenuItem } from "../../lib/menu.ts";
  import { withModifier } from "../../lib/platform.ts";
  import { prompts } from "../../lib/prompts.svelte.ts";
  import BackgroundEditor from "../themes/BackgroundEditor.svelte";
  import ColorField from "../themes/ColorField.svelte";
  import PictureChooser from "./PictureChooser.svelte";

  interface Props {
    /** The presentation to edit (a new one has never been saved). */
    deck: SlideDeck;
    onsave: (saved: SlideDeck) => void;
    oncancel: () => void;
  }

  let { deck, onsave, oncancel }: Props = $props();

  /** Wait this long after the last change before redrawing the slides. */
  const PREVIEW_DELAY_MS = 150;

  /** Payload type for a slide being reordered within the list. */
  const SLIDE_DRAG_TYPE = "application/x-simpleworship-deck-slide";

  /** How long each slide can stay up when moving on by itself. */
  const ADVANCE_SECONDS = [5, 8, 10, 15, 20, 30, 60];

  // Work on a copy, so Cancel really discards the changes.
  const original = untrack(() => JSON.stringify(deck));
  let draft: SlideDeck = $state(untrack(() => structuredClone($state.snapshot(deck))));
  let selected = $state(0);
  let presentation: Presentation | null = $state.raw(null);
  let choosingPictures = $state(false);
  let saving = $state(false);
  let error: string | null = $state(null);
  /** Where a slide being dragged would land, as an insertion index. */
  let dropAt: number | null = $state(null);

  let textArea: HTMLTextAreaElement | null = $state(null);

  const slide = $derived(draft.slides[selected] ?? null);
  const frame = $derived(presentation ? frameAt(presentation, selected) : null);
  const themes = $derived(engine.themes.themes);
  const isPicture = $derived(
    slide?.background?.type === "image" || slide?.background?.type === "video",
  );

  // Redraw the slides as the operator edits.
  $effect(() => {
    const snapshot = $state.snapshot(draft);
    const timer = setTimeout(async () => {
      try {
        presentation = await deckPresentation(snapshot);
      } catch {
        // The preview is a convenience; saving reports real problems.
      }
    }, PREVIEW_DELAY_MS);
    return () => clearTimeout(timer);
  });

  // ⌘/Ctrl+S saves the presentation while the editor is open.
  $effect(() => overrideMenuItem("schedule-save", save));

  // ── Slides ─────────────────────────────────────────────────────────────

  function blankSlide(): DeckSlide {
    return {
      id: uuidV7(),
      text: "",
      background: null,
      themeId: null,
      style: { fontSize: null, color: null, align: null },
    };
  }

  /** Inserts slides after the selected one and selects the first. */
  function insert(slides: DeckSlide[]) {
    const at = draft.slides.length === 0 ? 0 : selected + 1;
    draft.slides.splice(at, 0, ...slides);
    selected = at;
  }

  function addTextSlide() {
    insert([blankSlide()]);
    // Ready to type straight away.
    requestAnimationFrame(() => textArea?.focus());
  }

  async function addPictures(ids: string[]) {
    choosingPictures = false;
    try {
      insert(await pictureSlides(ids));
    } catch (failure) {
      engine.showError(failure);
    }
  }

  function duplicate() {
    if (!slide) return;
    insert([{ ...structuredClone($state.snapshot(slide)), id: uuidV7() }]);
  }

  function remove() {
    if (!slide) return;
    draft.slides.splice(selected, 1);
    selected = Math.min(selected, Math.max(draft.slides.length - 1, 0));
  }

  function move(from: number, insertion: number) {
    const to = insertion > from ? insertion - 1 : insertion;
    if (to === from) return;
    const [moved] = draft.slides.splice(from, 1);
    if (moved) draft.slides.splice(to, 0, moved);
    selected = to;
  }

  function handleListKeys(event: KeyboardEvent) {
    const step = { ArrowDown: 1, ArrowUp: -1 }[event.key];
    if (step !== undefined) {
      selected = Math.min(Math.max(selected + step, 0), draft.slides.length - 1);
      event.preventDefault();
    } else if (event.key === "Delete" || event.key === "Backspace") {
      remove();
      event.preventDefault();
    }
  }

  function dragOverSlide(event: DragEvent, index: number) {
    if (!event.dataTransfer?.types.includes(SLIDE_DRAG_TYPE)) return;
    event.preventDefault();
    const row = (event.currentTarget as HTMLElement).getBoundingClientRect();
    dropAt = event.clientY > row.top + row.height / 2 ? index + 1 : index;
  }

  function dropSlide(event: DragEvent) {
    const from = Number(event.dataTransfer?.getData(SLIDE_DRAG_TYPE));
    if (dropAt !== null && Number.isInteger(from)) {
      event.preventDefault();
      move(from, dropAt);
    }
    dropAt = null;
  }

  // ── Text ───────────────────────────────────────────────────────────────

  /** Makes the selected words bold, italic or underlined (or not). */
  function format(kind: TextFormat) {
    if (!slide || !textArea) return;
    const result = toggleFormat(
      { text: slide.text, start: textArea.selectionStart, end: textArea.selectionEnd },
      kind,
    );
    slide.text = result.text;
    const area = textArea;
    requestAnimationFrame(() => {
      area.focus();
      area.setSelectionRange(result.start, result.end);
    });
  }

  function handleTextKeys(event: KeyboardEvent) {
    if (!(event.metaKey || event.ctrlKey) || event.altKey) return;
    const kind = ({ b: "bold", i: "italic", u: "underline" } as const)[event.key.toLowerCase()];
    if (kind) {
      event.preventDefault();
      format(kind);
    }
  }

  // ── This slide's look ──────────────────────────────────────────────────

  /** Changes the selected slide, if there is one. */
  function update(change: (slide: DeckSlide) => void) {
    if (slide) change(slide);
  }

  function setBackground(own: boolean) {
    if (!slide) return;
    const black: Background = { type: "color", color: "#000000" };
    slide.background = own ? black : null;
  }

  function setAlign(value: string) {
    if (slide) slide.style.align = (value || null) as HorizontalAlign | null;
  }

  // ── Saving ─────────────────────────────────────────────────────────────

  async function save() {
    if (!draft.name.trim()) {
      error = "Please give the presentation a name.";
      return;
    }
    saving = true;
    error = null;
    try {
      onsave(await savePresentation($state.snapshot(draft)));
    } catch (failure) {
      error = String(failure);
    } finally {
      saving = false;
    }
  }

  async function cancel() {
    if (JSON.stringify($state.snapshot(draft)) === original) {
      oncancel();
      return;
    }
    const answer = await prompts.ask({
      title: "Discard changes?",
      message: `Your changes to “${draft.name || "this presentation"}” have not been saved.`,
      buttons: [
        { id: "keep", label: "Keep Editing" },
        { id: "discard", label: "Discard", variant: "live" },
      ],
      cancel: "keep",
    });
    if (answer === "discard") oncancel();
  }
</script>

<Dialog title="Presentation: {draft.name || "Untitled"}" onclose={cancel}>
  <div class="header">
    <label class="field name">
      Name
      <input bind:value={draft.name} placeholder="e.g. Announcements">
    </label>
    <label class="field">
      Theme
      <select bind:value={draft.themeId}>
        <option value={null}>Default for presentations</option>
        {#each themes as theme (theme.id)}
          <option value={theme.id}>{theme.theme.name}</option>
        {/each}
      </select>
    </label>
    <label class="field">
      Next slide by itself
      <select
        value={draft.autoAdvance?.seconds ?? 0}
        onchange={(e) => {
          const seconds = Number(e.currentTarget.value);
          draft.autoAdvance = seconds ? { seconds, loop: draft.autoAdvance?.loop ?? true } : null;
        }}
      >
        <option value={0}>Off</option>
        {#each ADVANCE_SECONDS as seconds (seconds)}
          <option value={seconds}>Every {seconds} seconds</option>
        {/each}
      </select>
    </label>
    {#if draft.autoAdvance}
      <label class="check">
        <input type="checkbox" bind:checked={draft.autoAdvance.loop}>
        Start again after the last
      </label>
    {/if}
  </div>

  <div class="editor">
    <!-- The slides -->
    <aside class="list-column">
      <div
        class="list"
        role="listbox"
        tabindex="0"
        aria-label="Slides"
        onkeydown={handleListKeys}
        ondrop={dropSlide}
        ondragleave={() => (dropAt = null)}
      >
        {#each draft.slides as item, index (item.id)}
          {@const thumbnail = presentation ? frameAt(presentation, index) : null}
          <!-- svelte-ignore a11y_click_events_have_key_events: keys are handled by the listbox (handleListKeys) -->
          <!-- biome-ignore lint/a11y/useKeyWithClickEvents: keys are handled by the listbox (handleListKeys) -->
          <div
            class="thumb"
            class:selected={index === selected}
            class:drop-before={dropAt === index}
            class:drop-after={dropAt === draft.slides.length && index === draft.slides.length - 1}
            role="option"
            tabindex="-1"
            aria-selected={index === selected}
            draggable="true"
            ondragstart={(e) => e.dataTransfer?.setData(SLIDE_DRAG_TYPE, String(index))}
            ondragover={(e) => dragOverSlide(e, index)}
            onclick={() => (selected = index)}
          >
            <span class="number">{index + 1}</span>
            <span class="picture">
              {#if thumbnail}
                <ScaledCanvas><SlideView frame={thumbnail} /></ScaledCanvas>
              {/if}
            </span>
          </div>
        {:else}
          <p class="empty">No slides yet. Add a text slide or pictures.</p>
        {/each}
      </div>
      <div class="list-actions">
        <Button onclick={addTextSlide}>+ Text</Button>
        <Button onclick={() => (choosingPictures = true)}>+ Pictures</Button>
        <Button disabled={!slide} onclick={duplicate}>Duplicate</Button>
        <Button disabled={!slide} onclick={remove}>Delete</Button>
      </div>
    </aside>

    <!-- The selected slide -->
    <section class="main">
      <div class="preview">
        {#if frame}
          <ScaledCanvas><SlideView {frame} /></ScaledCanvas>
        {/if}
      </div>
      {#if slide}
        <fieldset class="format" aria-label="Text style">
          <button
            type="button"
            class="bold"
            title="Bold ({withModifier("B")})"
            onclick={() => format("bold")}
          >
            B
          </button>
          <button
            type="button"
            class="italic"
            title="Italic ({withModifier("I")})"
            onclick={() => format("italic")}
          >
            I
          </button>
          <button
            type="button"
            class="underline"
            title="Underline ({withModifier("U")})"
            onclick={() => format("underline")}
          >
            U
          </button>
          <span class="hint">
            {isPicture ? "Text appears over the picture." : "Line breaks stay as typed."}
          </span>
        </fieldset>
        <textarea
          bind:this={textArea}
          bind:value={slide.text}
          onkeydown={handleTextKeys}
          placeholder={isPicture ? "Optional text over the picture" : "Type the slide text"}
          aria-label="Slide text"
        ></textarea>
      {/if}
    </section>

    <!-- This slide's look -->
    <aside class="look">
      {#if slide}
        <h4>This slide</h4>
        <label class="field">
          Theme
          <select bind:value={slide.themeId}>
            <option value={null}>The presentation's theme</option>
            {#each themes as theme (theme.id)}
              <option value={theme.id}>{theme.theme.name}</option>
            {/each}
          </select>
        </label>

        <label class="check">
          <input
            type="checkbox"
            checked={slide.background !== null}
            onchange={(e) => setBackground(e.currentTarget.checked)}
          >
          Its own background
        </label>
        {#if slide.background}
          <BackgroundEditor
            value={slide.background}
            onchange={(value) =>
              update((s) => {
                s.background = value;
              })}
          />
        {/if}

        <label class="check">
          <input
            type="checkbox"
            checked={slide.style.fontSize !== null}
            onchange={(e) => {
              const on = e.currentTarget.checked;
              update((s) => {
                s.style.fontSize = on ? (frame?.theme.text.fontSize ?? 84) : null;
              });
            }}
          >
          Text size
        </label>
        {#if slide.style.fontSize !== null}
          <input
            type="number"
            min="16"
            max="300"
            value={slide.style.fontSize}
            aria-label="Text size"
            onchange={(e) => {
              const size = e.currentTarget.valueAsNumber || 84;
              update((s) => {
                s.style.fontSize = size;
              });
            }}
          >
        {/if}

        <label class="check">
          <input
            type="checkbox"
            checked={slide.style.color !== null}
            onchange={(e) => {
              const on = e.currentTarget.checked;
              update((s) => {
                s.style.color = on ? (frame?.theme.text.color ?? "#ffffff") : null;
              });
            }}
          >
          Text colour
        </label>
        {#if slide.style.color !== null}
          <ColorField
            label="Text colour"
            value={slide.style.color}
            onchange={(color) =>
              update((s) => {
                s.style.color = color;
              })}
          />
        {/if}

        <label class="field">
          Alignment
          <select value={slide.style.align ?? ""} onchange={(e) => setAlign(e.currentTarget.value)}>
            <option value="">As the theme</option>
            <option value="left">Left</option>
            <option value="center">Centre</option>
            <option value="right">Right</option>
          </select>
        </label>
      {/if}
    </aside>
  </div>

  {#snippet actions()}
    {#if error}
      <span class="error" role="alert">{error}</span>
    {/if}
    <span class="count"
      >{draft.slides.length} {draft.slides.length === 1 ? "slide" : "slides"}</span
    >
    <Button onclick={cancel}>Cancel</Button>
    <Button variant="primary" disabled={saving} shortcut={withModifier("S")} onclick={save}>
      Save
    </Button>
  {/snippet}
</Dialog>

{#if choosingPictures}
  <PictureChooser onchoose={addPictures} onclose={() => (choosingPictures = false)} />
{/if}

<style>
  .header {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--sw-space-3);
    margin-bottom: var(--sw-space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .field input,
  .field select {
    color: var(--sw-text);
  }

  .name {
    flex: 1;
    min-width: 220px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .editor {
    display: grid;
    grid-template-columns: 190px minmax(0, 1fr) 250px;
    gap: var(--sw-space-3);
    height: min(64vh, 660px);
  }

  .list-column {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    min-height: 0;
  }

  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    padding: 2px;
  }

  .thumb {
    display: flex;
    gap: var(--sw-space-1);
    cursor: pointer;
    border-radius: var(--sw-radius-small);
  }

  .thumb .number {
    width: 1.4em;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
    text-align: right;
  }

  .thumb .picture {
    flex: 1;
    border: 2px solid transparent;
    border-radius: var(--sw-radius-small);
    overflow: hidden;
  }

  .thumb.selected .picture {
    border-color: var(--sw-accent);
  }

  .drop-before {
    box-shadow: 0 -2px 0 var(--sw-accent);
  }

  .drop-after {
    box-shadow: 0 2px 0 var(--sw-accent);
  }

  .list-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sw-space-1);
  }

  .main {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    min-height: 0;
  }

  .preview {
    border-radius: var(--sw-radius-small);
    overflow: hidden;
    background: #000;
    aspect-ratio: 16 / 9;
  }

  .format {
    margin: 0;
    padding: 0;
    border: none;
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
  }

  .format button {
    width: 30px;
    height: 28px;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    color: var(--sw-text);
    cursor: pointer;
  }

  .format .bold {
    font-weight: 800;
  }

  .format .italic {
    font-style: italic;
  }

  .format .underline {
    text-decoration: underline;
  }

  .hint {
    margin-left: var(--sw-space-2);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  textarea {
    flex: 1 1 auto;
    min-height: 80px;
    resize: none;
    font-size: 15px;
    line-height: 1.5;
  }

  .look {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    overflow-y: auto;
  }

  h4 {
    margin: 0;
    color: var(--sw-text-muted);
  }

  .look input[type="number"] {
    width: 90px;
  }

  .empty {
    color: var(--sw-text-faint);
  }

  .count {
    margin-right: auto;
    color: var(--sw-text-faint);
  }

  .error {
    color: var(--sw-live);
    margin-right: var(--sw-space-2);
  }
</style>
