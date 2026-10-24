<!--
  @component
  Edits a theme, with a live preview on the left. Drag the text box, the
  scripture reference or the footer on the preview to move or resize it,
  or type exact positions on the right.
-->
<script lang="ts">
  import type {
    HorizontalAlign,
    TextBox,
    TextStyle,
    ThemeRecord,
    VerseNumberStyle,
    VerticalAlign,
  } from "@simpleworship/protocol";
  import { ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { Button, Dialog } from "@simpleworship/ui";
  import { onMount, untrack } from "svelte";
  import { listFonts, saveTheme } from "../../lib/api.ts";
  import { engine } from "../../lib/engine.svelte.ts";
  import { firstFamily, fontFamily } from "../../lib/fonts.ts";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import { FOOTER_SPACING, REFERENCE_SPACING } from "../../lib/label-placement.ts";
  import { overrideMenuItem } from "../../lib/menu.ts";
  import {
    sampleLyrics,
    samplePresentation,
    sampleScripture,
    sampleText,
  } from "../../lib/samples.ts";
  import BackgroundEditor from "./BackgroundEditor.svelte";
  import ColorField from "./ColorField.svelte";
  import LabelEditor from "./LabelEditor.svelte";
  import TextBoxEditor, { type EditableBox } from "./TextBoxEditor.svelte";

  interface Props {
    record: ThemeRecord;
    onclose: () => void;
  }

  let { record, onclose }: Props = $props();

  const initialCat = untrack(() => {
    if (record.theme.category) return record.theme.category;
    if (engine.themes.defaults.scripture === record.id) return "scripture";
    if (engine.themes.defaults.presentation === record.id) return "presentation";
    if (engine.themes.defaults.text === record.id) return "text";
    return "song";
  });

  // Work on a copy, so Cancel discards the changes.
  let draft: ThemeRecord = $state(untrack(() => {
    const copy = structuredClone($state.snapshot(record));
    if (!copy.theme.category) copy.theme.category = initialCat;
    return copy;
  }));
  let sample: "lyrics" | "scripture" = $state(initialCat === "scripture" ? "scripture" : "lyrics");
  let fonts: string[] = $state([]);
  let saving = $state(false);
  let error: string | null = $state(null);

  function onCategoryChange(cat: "song" | "scripture" | "presentation" | "text") {
    draft.theme.category = cat;
    if (cat === "scripture") {
      sample = "scripture";
    } else {
      sample = "lyrics";
    }
  }

  const frame = $derived.by(() => {
    const cat = draft.theme.category;
    if (cat === "scripture") return sampleScripture(draft.theme);
    if (cat === "presentation") return samplePresentation(draft.theme);
    if (cat === "text") return sampleText(draft.theme);
    if (cat === "song") return sampleLyrics(draft.theme);
    return sample === "scripture" ? sampleScripture(draft.theme) : sampleLyrics(draft.theme);
  });

  /** The first family in the theme's font list, e.g. "Montserrat". */
  const currentFont = $derived(firstFamily(draft.theme.text.fontFamily));

  /** The box being dragged on the preview. */
  let activeBox = $state("text");

  /** The boxes that can be dragged on the preview: those that are shown. */
  const boxes: EditableBox[] = $derived.by(() => {
    const list: EditableBox[] = [
      { id: "text", label: t("designer.boxText"), box: draft.theme.textBox },
    ];
    const { caption, footer, category } = draft.theme;
    const isScripture = category === "scripture" || (!category && sample === "scripture");
    const isSong = category === "song" || (!category && sample === "lyrics");

    if (isScripture && caption.visible && caption.placement === "box") {
      list.push({ id: "caption", label: t("designer.boxReference"), box: caption.textBox });
    }
    if (isSong && footer.visible && footer.placement === "box") {
      list.push({ id: "footer", label: t("designer.boxFooter"), box: footer.textBox });
    }
    return list;
  });

  function moveBox(id: string, box: TextBox) {
    if (id === "caption") draft.theme.caption = { ...draft.theme.caption, textBox: box };
    else if (id === "footer") draft.theme.footer = { ...draft.theme.footer, textBox: box };
    else draft.theme.textBox = box;
  }

  function setNumbers<K extends keyof VerseNumberStyle>(key: K, value: VerseNumberStyle[K]) {
    const scripture = draft.theme.scripture;
    draft.theme.scripture = {
      ...scripture,
      verseNumbers: { ...scripture.verseNumbers, [key]: value },
    };
  }

  onMount(() => {
    listFonts().then((list) => (fonts = list));
  });

  // ⌘/Ctrl+S saves the theme while the designer is open.
  $effect(() => overrideMenuItem("schedule-save", save));

  function setText<K extends keyof TextStyle>(key: K, value: TextStyle[K]) {
    draft.theme.text = { ...draft.theme.text, [key]: value };
  }

  function setBox<K extends keyof TextBox>(key: K, value: TextBox[K]) {
    draft.theme.textBox = { ...draft.theme.textBox, [key]: value };
  }

  async function save() {
    if (!draft.theme.name.trim()) {
      error = t("designer.needsName");
      return;
    }
    saving = true;
    error = null;
    try {
      await saveTheme($state.snapshot(draft));
      onclose();
    } catch (failure) {
      error = String(failure);
    } finally {
      saving = false;
    }
  }
</script>

<Dialog title={t("designer.title", { name: draft.theme.name || t("editor.untitled") })} {onclose}>
  <div class="designer">
    <div class="preview-column">
      <div class="preview">
        <ScaledCanvas><SlideView {frame} /></ScaledCanvas>
        <TextBoxEditor
          {boxes}
          active={boxes.some((item) => item.id === activeBox) ? activeBox : "text"}
          onselect={(id) => (activeBox = id)}
          onchange={moveBox}
        />
      </div>
      {#if !draft.theme.category}
        <div class="sample" role="radiogroup" aria-label={t("designer.sample")}>
          <label
            ><input type="radio" bind:group={sample} value="lyrics">
            {t("designer.song")}</label
          >
          <label
            ><input type="radio" bind:group={sample} value="scripture">
            {t("designer.scripture")}</label
          >
          <span class="hint">{t("designer.dragHint")}</span>
        </div>
      {:else}
        <div class="sample">
          <span class="hint">{t("designer.dragHint")}</span>
        </div>
      {/if}
    </div>

    <div class="settings">
      <section>
        <div class="row">
          <label class="row" style="flex: 2;">
            {t("common.name")}
            <input bind:value={draft.theme.name}>
          </label>
          <label class="row" style="flex: 1;">
            {t("designer.category")}
            <select
              value={draft.theme.category ?? "song"}
              onchange={(e) => onCategoryChange(e.currentTarget.value as any)}
            >
              <option value="song">{t("designer.categorySong")}</option>
              <option value="scripture">{t("designer.categoryScripture")}</option>
              <option value="presentation">{t("designer.categoryPresentation")}</option>
              <option value="text">{t("designer.categoryText")}</option>
            </select>
          </label>
        </div>
      </section>

      <section>
        <h4>{t("designer.background")}</h4>
        <BackgroundEditor
          value={draft.theme.background}
          onchange={(background) => (draft.theme.background = background)}
        />
      </section>

      <section>
        <h4>{t("designer.text")}</h4>
        <div class="row">
          <select
            value={currentFont}
            aria-label={t("designer.font")}
            onchange={(e) => setText("fontFamily", fontFamily(e.currentTarget.value))}
          >
            {#if !fonts.includes(currentFont)}
              <option value={currentFont}>{currentFont}</option>
            {/if}
            {#each fonts as font (font)}
              <option value={font}>{font}</option>
            {/each}
          </select>
          <label class="inline">
            {t("designer.size")}
            <input
              type="number"
              min="12"
              max="300"
              value={draft.theme.text.fontSize}
              onchange={(e) => setText("fontSize", e.currentTarget.valueAsNumber || 72)}
            >
          </label>
          <select
            value={String(draft.theme.text.fontWeight)}
            aria-label={t("designer.weight")}
            onchange={(e) => setText("fontWeight", Number(e.currentTarget.value))}
          >
            <option value="300">{t("designer.light")}</option>
            <option value="400">{t("designer.regular")}</option>
            <option value="600">{t("designer.semibold")}</option>
            <option value="700">{t("designer.bold")}</option>
            <option value="900">{t("designer.black")}</option>
          </select>
        </div>
        <div class="row">
          <ColorField
            label={t("designer.textColour")}
            value={draft.theme.text.color}
            onchange={(color) => setText("color", color)}
          />
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.text.italic}
              onchange={(e) => setText("italic", e.currentTarget.checked)}
            >
            {t("designer.italic")}
          </label>
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.text.uppercase}
              onchange={(e) => setText("uppercase", e.currentTarget.checked)}
            >
            {t("designer.capitals")}
          </label>
        </div>
        <div class="row">
          <div class="segmented" role="radiogroup" aria-label={t("designer.alignment")}>
            {#each [
              ["left", t("designer.left")],
              ["center", t("designer.centre")],
              ["right", t("designer.right")],
            ] as [value, label] (value)}
              <label>
                <input
                  type="radio"
                  name="align"
                  checked={draft.theme.text.align === value}
                  onchange={() => setText("align", value as HorizontalAlign)}
                >
                {label}
              </label>
            {/each}
          </div>
          <label class="inline">
            {t("designer.lineSpacing")}
            <input
              type="number"
              min="0.8"
              max="3"
              step="0.05"
              value={draft.theme.text.lineHeight}
              onchange={(e) => setText("lineHeight", e.currentTarget.valueAsNumber || 1.2)}
            >
          </label>
        </div>
        <div class="row">
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.text.outline !== null}
              onchange={(e) =>
                setText("outline", e.currentTarget.checked ? { color: "#000000", width: 2 } : null)}
            >
            {t("designer.outline")}
          </label>
          {#if draft.theme.text.outline}
            {@const outline = draft.theme.text.outline}
            <ColorField
              label={t("designer.outlineColour")}
              value={outline.color}
              onchange={(color) => setText("outline", { ...outline, color })}
            />
            <input
              type="number"
              min="0.5"
              max="20"
              step="0.5"
              value={outline.width}
              aria-label={t("designer.outlineWidth")}
              onchange={(e) =>
                setText("outline", { ...outline, width: e.currentTarget.valueAsNumber || 1 })}
            >
          {/if}
        </div>
        <div class="row">
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.text.shadow !== null}
              onchange={(e) =>
                setText(
                  "shadow",
                  e.currentTarget.checked
                    ? { color: "rgba(0, 0, 0, 0.8)", offsetX: 3, offsetY: 3, blur: 6 }
                    : null,
                )}
            >
            {t("designer.shadow")}
          </label>
          {#if draft.theme.text.shadow}
            {@const shadow = draft.theme.text.shadow}
            <ColorField
              label={t("designer.shadowColour")}
              value={shadow.color}
              onchange={(color) => setText("shadow", { ...shadow, color })}
            />
            <label class="inline">
              {t("designer.blur")}
              <input
                type="number"
                min="0"
                max="60"
                value={shadow.blur}
                onchange={(e) =>
                  setText("shadow", { ...shadow, blur: e.currentTarget.valueAsNumber || 0 })}
              >
            </label>
          {/if}
        </div>
      </section>

      <section>
        <h4>{t("designer.textArea")}</h4>
        <div class="row">
          {#each [
            ["top", t("designer.top")],
            ["bottom", t("designer.bottom")],
            ["left", t("designer.left")],
            ["right", t("designer.right")],
          ] as const as [key, label] (key)}
            <label class="inline">
              {label}
              <input
                type="number"
                min="0"
                max="1900"
                value={draft.theme.textBox[key]}
                onchange={(e) => setBox(key, e.currentTarget.valueAsNumber || 0)}
              >
            </label>
          {/each}
        </div>
        <div class="row">
          <select
            value={draft.theme.textBox.verticalAlign}
            aria-label={t("designer.verticalPosition")}
            onchange={(e) => setBox("verticalAlign", e.currentTarget.value as VerticalAlign)}
          >
            <option value="top">{t("designer.areaTop")}</option>
            <option value="middle">{t("designer.areaMiddle")}</option>
            <option value="bottom">{t("designer.areaBottom")}</option>
          </select>
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.textBox.autoFit}
              onchange={(e) => setBox("autoFit", e.currentTarget.checked)}
            >
            {t("designer.shrink")}
          </label>
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.textBox.scaleUp ?? false}
              onchange={(e) => setBox("scaleUp", e.currentTarget.checked)}
            >
            {t("designer.scaleUp")}
          </label>
        </div>
      </section>

      {#if draft.theme.category === "scripture" || (!draft.theme.category && sample === "scripture")}
        <section>
          <h4>{t("designer.scriptureSection")}</h4>
          <div class="row">
            <label class="inline">
              <input
                type="checkbox"
                checked={draft.theme.scripture.verseNumbers.visible}
                onchange={(e) => {
                  sample = "scripture";
                  setNumbers("visible", e.currentTarget.checked);
                }}
              >
              {t("designer.verseNumbers")}
            </label>
            <label class="inline">
              {t("designer.size")}
              <select
                value={draft.theme.scripture.verseNumbers.sizePercent}
                onchange={(e) => setNumbers("sizePercent", Number(e.currentTarget.value))}
              >
                {#each [40, 55, 70, 85, 100] as percent (percent)}
                  <option value={percent}>{percent}%</option>
                {/each}
              </select>
            </label>
            <label class="inline">
              <input
                type="checkbox"
                checked={draft.theme.scripture.verseNumbers.superscript}
                onchange={(e) => setNumbers("superscript", e.currentTarget.checked)}
              >
              {t("designer.raised")}
            </label>
            <label class="inline">
              <input
                type="checkbox"
                checked={draft.theme.scripture.verseNumbers.bold}
                onchange={(e) => setNumbers("bold", e.currentTarget.checked)}
              >
              {t("designer.bold")}
            </label>
          </div>
          <div class="row">
            <label class="inline">
              <input
                type="checkbox"
                checked={draft.theme.scripture.verseNumbers.color === ""}
                onchange={(e) => setNumbers("color", e.currentTarget.checked ? "" : "#f5c542")}
              >
              {t("designer.numbersTextColour")}
            </label>
            {#if draft.theme.scripture.verseNumbers.color !== ""}
              <ColorField
                label={t("designer.numberColour")}
                value={draft.theme.scripture.verseNumbers.color}
                onchange={(color) => setNumbers("color", color)}
              />
            {/if}
          </div>
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.scripture.referenceOnEverySlide}
              onchange={(e) =>
                (draft.theme.scripture = {
                  ...draft.theme.scripture,
                  referenceOnEverySlide: e.currentTarget.checked,
                })}
            >
            {t("designer.everySlide")}
          </label>
        </section>

        <section>
          <h4>{t("designer.reference")}</h4>
          <LabelEditor
            name={t("designer.boxReference")}
            value={draft.theme.caption}
            spacing={REFERENCE_SPACING}
            {fonts}
            onchange={(caption) => {
              sample = "scripture";
              activeBox = "caption";
              draft.theme.caption = caption;
            }}
          />
        </section>
      {/if}

      {#if draft.theme.category === "song" || (!draft.theme.category && sample === "lyrics")}
        <section>
          <h4>{t("designer.footer")}</h4>
          <LabelEditor
            name={t("designer.boxFooter")}
            value={draft.theme.footer}
            spacing={FOOTER_SPACING}
            {fonts}
            onchange={(footer) => {
              activeBox = "footer";
              draft.theme.footer = footer;
            }}
          />
        </section>
      {/if}
    </div>
  </div>

  {#snippet actions()}
    {#if error}
      <span class="error" role="alert">{error}</span>
    {/if}
    <Button onclick={onclose}>{t("common.cancel")}</Button>
    <Button variant="primary" disabled={saving} onclick={save}>{t("designer.save")}</Button>
  {/snippet}
</Dialog>

<style>
  .designer {
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(320px, 1fr);
    gap: var(--sw-space-4);
    min-height: min(68vh, 680px);
  }

  .preview {
    position: relative;
    border-radius: var(--sw-radius-small);
    overflow: hidden;
  }

  .sample {
    display: flex;
    align-items: center;
    gap: var(--sw-space-3);
    margin-top: var(--sw-space-2);
    color: var(--sw-text-muted);
  }

  .hint {
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .settings {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-3);
    overflow-y: auto;
    max-height: 70vh;
    padding-right: var(--sw-space-1);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
  }

  h4 {
    margin: 0;
    font-size: var(--sw-font-size);
    color: var(--sw-text-muted);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-2);
    color: var(--sw-text-muted);
  }

  .row input:not([type]) {
    flex: 1;
  }

  .inline {
    display: inline-flex;
    align-items: center;
    gap: var(--sw-space-1);
  }

  input[type="number"] {
    width: 70px;
  }

  .segmented {
    display: inline-flex;
    gap: var(--sw-space-2);
  }

  .error {
    color: var(--sw-live);
    margin-right: auto;
  }
</style>
