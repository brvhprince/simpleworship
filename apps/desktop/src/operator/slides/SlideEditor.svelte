<!--
  @component
  The presentation editor: a slide canvas like EasyWorship's.

  - Top: the presentation's name, theme and auto-advance; buttons to add a
    text box, a shape, a picture or video, or scripture; undo and redo;
    zoom. Below them, the text toolbar.
  - Left: the slides. New Slide copies the current slide's layout
    (background, logo, shapes, empty text boxes in the same style).
  - Middle: the slide, exactly as the projector shows it. Drag, resize and
    double-click to type.
  - Right: the Inspector, with "Apply to All Slides".

  Keys (when not typing): Delete removes the selection, arrows nudge it
  (Shift for 10 pixels), ⌘C / ⌘V / ⌘D copy, paste and duplicate, ⌘Z and
  ⇧⌘Z undo and redo, Esc deselects.
-->
<script lang="ts">
  import type { ShapeKind, SlideDeck } from "@simpleworship/protocol";
  import { Button, Dialog } from "@simpleworship/ui";
  import { onMount, untrack } from "svelte";
  import { listFonts, savePresentation } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { media } from "../../lib/media.svelte.ts";
  import { overrideMenuItem } from "../../lib/menu.ts";
  import { withModifier } from "../../lib/platform.ts";
  import { prompts } from "../../lib/prompts.svelte.ts";
  import { SlideEditor } from "../../lib/slides/editor.svelte.ts";
  import { slideTheme } from "../../lib/slides/look.ts";
  import {
    addElement,
    blankSlide,
    duplicateSlide,
    newMedia,
    newShape,
    newTextBox,
    slideFromLayout,
    templateSlide,
  } from "../../lib/slides/slide-ops.ts";
  import PictureChooser from "../library/PictureChooser.svelte";
  import FormatBar from "./FormatBar.svelte";
  import Inspector from "./Inspector.svelte";
  import ScriptureInsert from "./ScriptureInsert.svelte";
  import SlideCanvas from "./SlideCanvas.svelte";
  import SlideList from "./SlideList.svelte";

  interface Props {
    /** The presentation to edit (a new one has never been saved). */
    deck: SlideDeck;
    onsave: (saved: SlideDeck) => void;
    oncancel: () => void;
  }

  let { deck, onsave, oncancel }: Props = $props();

  /** How long each slide can stay up when moving on by itself. */
  const ADVANCE_SECONDS = [3, 5, 8, 10, 15, 20, 30, 60];
  const ZOOMS = [1, 1.25, 1.5, 2];

  // Work on a copy, so Cancel really discards the changes.
  const original = untrack(() => JSON.stringify(deck));
  const editor = new SlideEditor(untrack(() => structuredClone($state.snapshot(deck))));

  let fonts: string[] = $state([]);
  let zoom = $state(1);
  /** The canvas area's inner size, for fitting the slide in it. */
  let areaWidth = $state(0);
  let areaHeight = $state(0);
  /** The slide's width at "Fit": as large as fits both ways. */
  const fitWidth = $derived(Math.max(200, Math.min(areaWidth - 32, ((areaHeight - 32) * 16) / 9)));
  let choosingPictures = $state(false);
  let insertingScripture = $state(false);
  let saving = $state(false);
  let error: string | null = $state(null);

  const themes = $derived(engine.themes.themes);
  /** The theme of the slide being edited: new text boxes start in its style. */
  const theme = $derived(
    editor.slide ? slideTheme(engine.themes, editor.deck, editor.slide) : null,
  );

  onMount(() => {
    listFonts().then((list) => (fonts = list));
    // A new presentation starts with one template slide (Title & Body placeholders).
    if (editor.deck.slides.length === 0) {
      const style = slideTheme(engine.themes, editor.deck, blankSlide())?.text;
      const first = templateSlide(style);
      editor.deck.slides.push(first);
    }
  });

  // ⌘S saves; Undo and Redo go to the editor (or to a text field being typed in).
  $effect(() => overrideMenuItem("schedule-save", save));
  $effect(() =>
    overrideMenuItem("undo", () => (inTextField() ? document.execCommand("undo") : editor.undo())),
  );
  $effect(() =>
    overrideMenuItem("redo", () => (inTextField() ? document.execCommand("redo") : editor.redo())),
  );

  function inTextField(): boolean {
    const focused = document.activeElement;
    return focused instanceof HTMLInputElement || focused instanceof HTMLTextAreaElement;
  }

  // ── Adding things ──────────────────────────────────────────────────────

  function addText() {
    const slide = editor.slide;
    if (!slide || !theme) return;
    const box = newTextBox(theme.text);
    editor.change(() => addElement(slide, box));
    editor.startEditing(box.id);
  }

  function addShape(shape: ShapeKind) {
    const slide = editor.slide;
    if (!slide) return;
    const added = newShape(shape);
    editor.change(() => addElement(slide, added));
    editor.select(added.id);
  }

  function addPictures(ids: string[]) {
    choosingPictures = false;
    const slide = editor.slide;
    if (!slide) return;
    const items = ids.flatMap((id) => media.list.filter((item) => item.id === id));
    editor.change(() => {
      items.forEach((item, index) => {
        const added = newMedia(item);
        // Fan them out a little, so several are not on top of each other.
        added.frame = {
          ...added.frame,
          x: added.frame.x + index * 40,
          y: added.frame.y + index * 40,
        };
        addElement(slide, added);
        editor.selectedId = added.id;
      });
    });
  }

  /** Puts verses into the selected text box (after its words), or a new one. */
  function addScripture(verses: string, reference: string | null) {
    insertingScripture = false;
    const slide = editor.slide;
    if (!slide || !theme) return;
    const text = reference ? `${verses}\n${reference}` : verses;
    const box = editor.textBox;
    editor.change(() => {
      if (box) {
        const existing = box.content.runs.map((run) => run.text).join("");
        const style = box.content.runs.at(-1) ?? { bold: false, italic: false, underline: false };
        box.content.runs = [
          ...box.content.runs,
          { ...style, text: existing.trim() ? `\n${text}` : text },
        ];
      } else {
        const added = newTextBox(theme.text, text);
        addElement(slide, added);
        editor.selectedId = added.id;
      }
    });
    editor.textVersion += 1;
  }

  // ── Slides ─────────────────────────────────────────────────────────────

  function insertSlide(slide: ReturnType<typeof blankSlide>) {
    const at = editor.deck.slides.length === 0 ? 0 : editor.slideIndex + 1;
    editor.change((current) => current.slides.splice(at, 0, slide));
    editor.showSlide(at);
  }

  function newSlide() {
    const current = editor.slide;
    if (current && current.elements.length > 0) {
      insertSlide(slideFromLayout($state.snapshot(current) as typeof current));
    } else {
      insertSlide(templateSlide(theme?.text));
    }
  }

  function deleteSlide() {
    if (!editor.slide) return;
    const index = editor.slideIndex;
    editor.change((current) => current.slides.splice(index, 1));
    editor.showSlide(Math.min(index, editor.deck.slides.length - 1));
  }

  // ── Keys ───────────────────────────────────────────────────────────────

  /** Whether a key press is for a form field or the text being typed. */
  function isTyping(target: EventTarget | null): boolean {
    return (
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement ||
      (target instanceof HTMLElement && target.isContentEditable)
    );
  }

  function handleKeys(event: KeyboardEvent) {
    if (choosingPictures || insertingScripture || isTyping(event.target)) return;
    const modifier = event.metaKey || event.ctrlKey;
    const key = event.key.toLowerCase();

    if (event.key === "Escape" && editor.selectedId) {
      // Deselect first; a second Esc closes the editor.
      event.preventDefault();
      editor.select(null);
    } else if ((event.key === "Delete" || event.key === "Backspace") && editor.selectedId) {
      event.preventDefault();
      editor.deleteSelected();
    } else if (event.key.startsWith("Arrow") && editor.selectedId) {
      event.preventDefault();
      const step = event.shiftKey ? 10 : 1;
      const dx = { ArrowLeft: -step, ArrowRight: step }[event.key] ?? 0;
      const dy = { ArrowUp: -step, ArrowDown: step }[event.key] ?? 0;
      editor.nudge(dx, dy);
    } else if (modifier && key === "d") {
      event.preventDefault();
      editor.duplicateSelected();
    } else if (event.key === "Enter" && editor.textBox) {
      event.preventDefault();
      editor.startEditing(editor.textBox.id);
    }
  }

  /** ⌘C / ⌘V reach the page as copy and paste events (from the Edit menu). */
  function handleCopy(event: ClipboardEvent) {
    if (isTyping(document.activeElement) || !editor.selected) return;
    event.preventDefault();
    editor.copy();
  }

  function handlePaste(event: ClipboardEvent) {
    if (isTyping(document.activeElement)) return;
    event.preventDefault();
    editor.paste();
  }

  // ── Saving ─────────────────────────────────────────────────────────────

  async function save() {
    if (!editor.deck.name.trim()) {
      error = t("editor.needsName");
      return;
    }
    saving = true;
    error = null;
    try {
      editor.stopEditing();
      onsave(await savePresentation($state.snapshot(editor.deck)));
    } catch (failure) {
      error = String(failure);
    } finally {
      saving = false;
    }
  }

  async function cancel() {
    if (editor.editingId) {
      editor.stopEditing();
      return;
    }
    if (JSON.stringify($state.snapshot(editor.deck)) === original) {
      oncancel();
      return;
    }
    const answer = await prompts.ask({
      title: t("editor.discardTitle"),
      message: t("editor.discardMessage", {
        name: editor.deck.name || t("editor.thisPresentation"),
      }),
      buttons: [
        { id: "keep", label: t("common.keepEditing") },
        { id: "discard", label: t("common.discard"), variant: "live" },
      ],
      cancel: "keep",
    });
    if (answer === "discard") oncancel();
  }
</script>

<svelte:window onkeydowncapture={handleKeys} />
<svelte:document oncopy={handleCopy} onpaste={handlePaste} />

<Dialog
  title={t("editor.title", { name: editor.deck.name || t("editor.untitled") })}
  size="full"
  onclose={cancel}
>
  <div class="top">
    <input
      class="name"
      bind:value={editor.deck.name}
      placeholder={t("editor.namePlaceholder")}
      aria-label={t("common.name")}
    >
    <select
      bind:value={editor.deck.themeId}
      aria-label={t("editor.theme")}
      title={t("common.theme")}
    >
      <option value={null}>{t("editor.defaultTheme")}</option>
      {#each themes as record (record.id)}
        <option value={record.id}>{record.theme.name}</option>
      {/each}
    </select>
    <select
      aria-label={t("editor.advance")}
      value={editor.deck.autoAdvance?.seconds ?? 0}
      onchange={(e) => {
        const seconds = Number(e.currentTarget.value);
        editor.deck.autoAdvance = seconds
          ? { seconds, loop: editor.deck.autoAdvance?.loop ?? true }
          : null;
      }}
    >
      <option value={0}>{t("editor.advanceManual")}</option>
      {#each ADVANCE_SECONDS as seconds (seconds)}
        <option value={seconds}>{t("editor.advanceEvery", { seconds })}</option>
      {/each}
    </select>
    {#if editor.deck.autoAdvance}
      <label class="check">
        <input type="checkbox" bind:checked={editor.deck.autoAdvance.loop}>
        {t("common.loop")}
      </label>
    {/if}

    <span class="divider"></span>
    <Button onclick={addText}>{t("editor.addText")}</Button>
    <select
      aria-label={t("editor.addShape")}
      value=""
      onchange={(e) => {
        const shape = e.currentTarget.value as ShapeKind;
        e.currentTarget.value = "";
        if (shape) addShape(shape);
      }}
    >
      <option value="" disabled>{t("editor.shapeMenu")}</option>
      <option value="rectangle">{t("editor.rectangle")}</option>
      <option value="ellipse">{t("editor.ellipse")}</option>
      <option value="line">{t("editor.line")}</option>
    </select>
    <Button onclick={() => (choosingPictures = true)}>{t("editor.addPicture")}</Button>
    <Button onclick={() => (insertingScripture = true)}>{t("editor.addScripture")}</Button>

    <span class="divider"></span>
    <Button
      disabled={!editor.canUndo}
      title={t("editor.undoHint", { key: withModifier("Z") })}
      onclick={() => editor.undo()}
      >{t("editor.undo")}</Button
    >
    <Button disabled={!editor.canRedo} title={t("editor.redoHint")} onclick={() => editor.redo()}
      >{t("editor.redo")}</Button
    >

    <span class="spacer"></span>
    <label class="check">
      {t("editor.zoom")}
      <select bind:value={zoom}>
        {#each ZOOMS as level (level)}
          <option value={level}>{level === 1 ? t("editor.fit") : `${level * 100}%`}</option>
        {/each}
      </select>
    </label>
  </div>

  <FormatBar {editor} {fonts} />

  <div class="body">
    <aside class="slides">
      <SlideList {editor} />
      <div class="slide-actions">
        <Button variant="primary" onclick={newSlide} title={t("editor.newSlideHint")}
          >{t("editor.newSlide")}</Button
        >
        <Button onclick={() => insertSlide(blankSlide())}>{t("editor.blank")}</Button>
        <Button
          disabled={!editor.slide}
          onclick={() =>
            editor.slide &&
            insertSlide(duplicateSlide($state.snapshot(editor.slide) as typeof editor.slide))}
        >
          {t("editor.duplicate")}
        </Button>
        <Button disabled={!editor.slide} onclick={deleteSlide}>{t("common.delete")}</Button>
      </div>
    </aside>

    <main class="canvas-area" bind:clientWidth={areaWidth} bind:clientHeight={areaHeight}>
      {#if editor.slide}
        <div class="zoomed" style:width="{fitWidth * zoom}px">
          <SlideCanvas {editor} />
        </div>
      {:else}
        <p class="empty">{t("editor.empty")}</p>
      {/if}
    </main>

    <aside class="inspector">
      <Inspector {editor} />
    </aside>
  </div>

  {#snippet actions()}
    {#if error}
      <span class="error" role="alert">{error}</span>
    {/if}
    <span class="count">
      {t("common.slides", { count: editor.deck.slides.length })}
    </span>
    <Button onclick={cancel}>{t("common.cancel")}</Button>
    <Button variant="primary" disabled={saving} shortcut={withModifier("S")} onclick={save}
      >{t("common.save")}</Button
    >
  {/snippet}
</Dialog>

{#if choosingPictures}
  <PictureChooser onchoose={addPictures} onclose={() => (choosingPictures = false)} />
{/if}
{#if insertingScripture}
  <ScriptureInsert oninsert={addScripture} onclose={() => (insertingScripture = false)} />
{/if}

<style>
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-2);
  }

  .name {
    width: 240px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--sw-space-1);
    color: var(--sw-text-muted);
  }

  .divider {
    width: 1px;
    height: 24px;
    background: var(--sw-border);
  }

  .spacer {
    flex: 1;
  }

  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 210px minmax(0, 1fr) 280px;
    gap: var(--sw-space-3);
    margin-top: var(--sw-space-2);
  }

  .slides {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
    min-height: 0;
  }

  .slide-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }

  .canvas-area {
    min-height: 0;
    overflow: auto;
    padding: var(--sw-space-3);
    background: var(--sw-surface-sunken, #111318);
    border-radius: var(--sw-radius-small);
    display: flex;
    align-items: flex-start;
    justify-content: center;
  }

  .zoomed {
    flex: none;
    max-width: none;
  }

  .inspector {
    min-height: 0;
    display: flex;
    flex-direction: column;
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
  }
</style>
