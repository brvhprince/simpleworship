<!--
  @component
  Edits a theme, with a live preview on the left. Drag the text box on the
  preview to move or resize it, or type exact positions on the right.
-->
<script lang="ts">
  import type {
    HorizontalAlign,
    TextBox,
    TextStyle,
    ThemeRecord,
    VerticalAlign,
  } from "@simpleworship/protocol";
  import { ScaledCanvas, SlideView } from "@simpleworship/renderer";
  import { Button, Dialog } from "@simpleworship/ui";
  import { onMount, untrack } from "svelte";
  import { listFonts, saveTheme } from "../../lib/api.ts";
  import { overrideMenuItem } from "../../lib/menu.ts";
  import { sampleLyrics, sampleScripture } from "../../lib/samples.ts";
  import BackgroundEditor from "./BackgroundEditor.svelte";
  import ColorField from "./ColorField.svelte";
  import TextBoxEditor from "./TextBoxEditor.svelte";

  interface Props {
    record: ThemeRecord;
    onclose: () => void;
  }

  let { record, onclose }: Props = $props();

  // Work on a copy, so Cancel discards the changes.
  let draft: ThemeRecord = $state(untrack(() => structuredClone($state.snapshot(record))));
  let sample: "lyrics" | "scripture" = $state("lyrics");
  let fonts: string[] = $state([]);
  let saving = $state(false);
  let error: string | null = $state(null);

  const frame = $derived(
    sample === "lyrics" ? sampleLyrics(draft.theme) : sampleScripture(draft.theme),
  );

  /** The first family in the theme's font list, e.g. "Montserrat". */
  const currentFont = $derived(
    draft.theme.text.fontFamily
      .split(",")[0]
      ?.trim()
      .replace(/^["']|["']$/g, "") ?? "",
  );

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

  /** A font family list with sensible fallbacks for when a font is missing. */
  function fontFamily(name: string): string {
    const serif = /serif|merriweather|georgia|times|garamond/i.test(name) && !/sans/i.test(name);
    return `"${name}", ${serif ? "Georgia, serif" : '"Open Sans", Helvetica, Arial, sans-serif'}`;
  }

  async function save() {
    if (!draft.theme.name.trim()) {
      error = "Please give the theme a name.";
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

<Dialog title="Theme: {draft.theme.name || "Untitled"}" {onclose}>
  <div class="designer">
    <div class="preview-column">
      <div class="preview">
        <ScaledCanvas><SlideView {frame} /></ScaledCanvas>
        <TextBoxEditor box={draft.theme.textBox} onchange={(box) => (draft.theme.textBox = box)} />
      </div>
      <div class="sample" role="radiogroup" aria-label="Sample">
        <label><input type="radio" bind:group={sample} value="lyrics"> Song</label>
        <label><input type="radio" bind:group={sample} value="scripture"> Scripture</label>
        <span class="hint">Drag the dashed box to move the text; drag a corner to resize it.</span>
      </div>
    </div>

    <div class="settings">
      <section>
        <label class="row">
          Name
          <input bind:value={draft.theme.name}>
        </label>
      </section>

      <section>
        <h4>Background</h4>
        <BackgroundEditor
          value={draft.theme.background}
          onchange={(background) => (draft.theme.background = background)}
        />
      </section>

      <section>
        <h4>Text</h4>
        <div class="row">
          <select
            value={currentFont}
            aria-label="Font"
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
            Size
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
            aria-label="Weight"
            onchange={(e) => setText("fontWeight", Number(e.currentTarget.value))}
          >
            <option value="300">Light</option>
            <option value="400">Regular</option>
            <option value="600">Semibold</option>
            <option value="700">Bold</option>
            <option value="900">Black</option>
          </select>
        </div>
        <div class="row">
          <ColorField
            label="Text colour"
            value={draft.theme.text.color}
            onchange={(color) => setText("color", color)}
          />
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.text.italic}
              onchange={(e) => setText("italic", e.currentTarget.checked)}
            >
            Italic
          </label>
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.text.uppercase}
              onchange={(e) => setText("uppercase", e.currentTarget.checked)}
            >
            Capitals
          </label>
        </div>
        <div class="row">
          <div class="segmented" role="radiogroup" aria-label="Alignment">
            {#each [
              ["left", "Left"],
              ["center", "Centre"],
              ["right", "Right"],
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
            Line spacing
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
            Outline
          </label>
          {#if draft.theme.text.outline}
            {@const outline = draft.theme.text.outline}
            <ColorField
              label="Outline colour"
              value={outline.color}
              onchange={(color) => setText("outline", { ...outline, color })}
            />
            <input
              type="number"
              min="0.5"
              max="20"
              step="0.5"
              value={outline.width}
              aria-label="Outline width"
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
            Shadow
          </label>
          {#if draft.theme.text.shadow}
            {@const shadow = draft.theme.text.shadow}
            <ColorField
              label="Shadow colour"
              value={shadow.color}
              onchange={(color) => setText("shadow", { ...shadow, color })}
            />
            <label class="inline">
              Blur
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
        <h4>Text area</h4>
        <div class="row">
          {#each [
            ["top", "Top"],
            ["bottom", "Bottom"],
            ["left", "Left"],
            ["right", "Right"],
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
            aria-label="Vertical position"
            onchange={(e) => setBox("verticalAlign", e.currentTarget.value as VerticalAlign)}
          >
            <option value="top">Top of the area</option>
            <option value="middle">Middle of the area</option>
            <option value="bottom">Bottom of the area</option>
          </select>
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.textBox.autoFit}
              onchange={(e) => setBox("autoFit", e.currentTarget.checked)}
            >
            Shrink long text to fit
          </label>
        </div>
      </section>

      <section>
        <h4>Footer (copyright, CCLI)</h4>
        <div class="row">
          <label class="inline">
            <input
              type="checkbox"
              checked={draft.theme.footer.visible}
              onchange={(e) =>
                (draft.theme.footer = { ...draft.theme.footer, visible: e.currentTarget.checked })}
            >
            Show
          </label>
          <label class="inline">
            Size
            <input
              type="number"
              min="10"
              max="80"
              value={draft.theme.footer.fontSize}
              onchange={(e) =>
                (draft.theme.footer = {
                  ...draft.theme.footer,
                  fontSize: e.currentTarget.valueAsNumber || 28,
                })}
            >
          </label>
          <ColorField
            label="Footer colour"
            value={draft.theme.footer.color}
            onchange={(color) => (draft.theme.footer = { ...draft.theme.footer, color })}
          />
        </div>
      </section>
    </div>
  </div>

  {#snippet actions()}
    {#if error}
      <span class="error" role="alert">{error}</span>
    {/if}
    <Button onclick={onclose}>Cancel</Button>
    <Button variant="primary" disabled={saving} onclick={save}>Save Theme</Button>
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
