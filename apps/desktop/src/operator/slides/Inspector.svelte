<!--
  @component
  The Inspector: settings for whatever is selected.

  - An element: exact position and size, opacity, locking, stacking order,
    and what kind of element it is (a text box's fill and effects, a
    shape's colours, a picture's fit).
  - Nothing selected: the slide's theme and background.

  Each has **Apply to All Slides**: a background goes onto every slide; an
  element's place and style go to "the same" element on every slide (each
  keeps its own words), and slides without it get a copy.
-->
<script lang="ts">
  import type { ElementContent, MediaFit, ShapeKind, SlideElement } from "@simpleworship/protocol";
  import { Button } from "@simpleworship/ui";
  import { engine } from "../../lib/engine.svelte.ts";
  import type { SlideEditor } from "../../lib/slides/editor.svelte.ts";
  import { applyElementToAll, applySlideLookToAll, arrange } from "../../lib/slides/slide-ops.ts";
  import BackgroundEditor from "../themes/BackgroundEditor.svelte";
  import ColorField from "../themes/ColorField.svelte";

  interface Props {
    editor: SlideEditor;
  }

  let { editor }: Props = $props();

  /** A short confirmation after "Apply to All Slides". */
  let applied: string | null = $state(null);
  let appliedTimer: ReturnType<typeof setTimeout> | undefined;

  const element = $derived(editor.selected);
  const slide = $derived(editor.slide);
  const themes = $derived(engine.themes.themes);
  const others = $derived(editor.deck.slides.length - 1);

  const KIND_NAMES = { text: "Text box", shape: "Shape", media: "Picture" } as const;

  function kindName(item: SlideElement): string {
    if (item.content.type === "media" && item.content.kind === "video") return "Video";
    return KIND_NAMES[item.content.type];
  }

  /** Changes the selected element (undoably; `tag` groups slider drags). */
  function update(change: (item: SlideElement) => void, tag?: string) {
    const target = element;
    if (target) editor.change(() => change(target), tag);
  }

  /** Changes the selected element's content, if it is of `type`. */
  function updateContent<T extends ElementContent["type"]>(
    type: T,
    change: (content: Extract<ElementContent, { type: T }>) => void,
    tag?: string,
  ) {
    update((item) => {
      if (item.content.type === type) change(item.content as Extract<ElementContent, { type: T }>);
    }, tag);
  }

  function setFrame(key: "x" | "y" | "width" | "height", value: number) {
    if (Number.isFinite(value))
      update((item) => (item.frame = { ...item.frame, [key]: value }), "frame");
  }

  function confirm(message: string) {
    applied = message;
    clearTimeout(appliedTimer);
    appliedTimer = setTimeout(() => (applied = null), 3000);
  }

  function applyElement() {
    const target = element;
    if (!target) return;
    editor.change((deck) => applyElementToAll(deck, editor.slideIndex, target.id));
    confirm(`Applied to ${others} other ${others === 1 ? "slide" : "slides"}.`);
  }

  function applySlide() {
    editor.change((deck) => applySlideLookToAll(deck, editor.slideIndex));
    confirm(`Background and theme applied to every slide.`);
  }
</script>

<div class="inspector">
  {#if element}
    <header>
      <h4>{kindName(element)}</h4>
      <label class="check">
        <input
          type="checkbox"
          checked={element.locked}
          onchange={(e) => {
            const locked = e.currentTarget.checked;
            update((item) => (item.locked = locked));
          }}
        >
        Lock
      </label>
    </header>

    <section>
      <div class="grid">
        {#each [
          ["x", "X"],
          ["y", "Y"],
          ["width", "Width"],
          ["height", "Height"],
        ] as const as [key, label] (key)}
          <label>
            {label}
            <input
              type="number"
              value={Math.round(element.frame[key])}
              onchange={(e) => setFrame(key, e.currentTarget.valueAsNumber)}
            >
          </label>
        {/each}
      </div>
      <label class="row">
        Opacity
        <input
          type="range"
          min="0.05"
          max="1"
          step="0.05"
          value={element.opacity}
          oninput={(e) => {
            const opacity = e.currentTarget.valueAsNumber;
            update((item) => (item.opacity = opacity), "opacity");
          }}
        >
        <span class="value">{Math.round(element.opacity * 100)}%</span>
      </label>
      <fieldset class="arrange" aria-label="Arrange">
        {#each [
          ["front", "To Front"],
          ["forward", "Forward"],
          ["backward", "Backward"],
          ["back", "To Back"],
        ] as const as [move, label] (move)}
          <Button onclick={() => slide && editor.change(() => arrange(slide, element.id, move))}>
            {label}
          </Button>
        {/each}
      </fieldset>
    </section>

    {#if element.content.type === "text"}
      {@const content = element.content}
      <section>
        <h5>Box</h5>
        <label class="check">
          <input
            type="checkbox"
            checked={content.autoFit}
            onchange={(e) => {
              const on = e.currentTarget.checked;
              updateContent("text", (text) => (text.autoFit = on));
            }}
          >
          Shrink long text to fit
        </label>
        <label class="check">
          <input
            type="checkbox"
            checked={content.fill !== null}
            onchange={(e) => {
              const on = e.currentTarget.checked;
              updateContent("text", (text) => (text.fill = on ? "rgba(0, 0, 0, 0.6)" : null));
            }}
          >
          Colour behind the text
        </label>
        {#if content.fill !== null}
          <ColorField
            label="Box colour"
            value={content.fill}
            onchange={(fill) => updateContent("text", (text) => (text.fill = fill), "fill")}
          />
        {/if}
        <div class="grid">
          <label>
            Padding
            <input
              type="number"
              min="0"
              max="200"
              value={content.padding}
              onchange={(e) => {
                const padding = e.currentTarget.valueAsNumber || 0;
                updateContent("text", (text) => (text.padding = padding));
              }}
            >
          </label>
          <label>
            Corners
            <input
              type="number"
              min="0"
              max="300"
              value={content.cornerRadius}
              onchange={(e) => {
                const radius = e.currentTarget.valueAsNumber || 0;
                updateContent("text", (text) => (text.cornerRadius = radius));
              }}
            >
          </label>
          <label>
            Line spacing
            <input
              type="number"
              min="0.7"
              max="3"
              step="0.05"
              value={content.style.lineHeight}
              onchange={(e) => {
                const lineHeight = e.currentTarget.valueAsNumber || 1.2;
                updateContent("text", (text) => (text.style = { ...text.style, lineHeight }));
              }}
            >
          </label>
        </div>
        <label class="check">
          <input
            type="checkbox"
            checked={content.style.uppercase}
            onchange={(e) => {
              const uppercase = e.currentTarget.checked;
              updateContent("text", (text) => (text.style = { ...text.style, uppercase }));
            }}
          >
          Capitals
        </label>
        <label class="check">
          <input
            type="checkbox"
            checked={content.style.outline !== null}
            onchange={(e) => {
              const on = e.currentTarget.checked;
              updateContent(
                "text",
                (text) =>
                  (text.style = {
                    ...text.style,
                    outline: on ? { color: "#000000", width: 2 } : null,
                  }),
              );
            }}
          >
          Outline
        </label>
        {#if content.style.outline}
          {@const outline = content.style.outline}
          <div class="row">
            <ColorField
              label="Outline colour"
              value={outline.color}
              onchange={(color) =>
                updateContent(
                  "text",
                  (text) => (text.style = { ...text.style, outline: { ...outline, color } }),
                  "outline",
                )}
            />
            <input
              type="number"
              min="0.5"
              max="20"
              step="0.5"
              value={outline.width}
              aria-label="Outline width"
              onchange={(e) => {
                const width = e.currentTarget.valueAsNumber || 1;
                updateContent(
                  "text",
                  (text) => (text.style = { ...text.style, outline: { ...outline, width } }),
                );
              }}
            >
          </div>
        {/if}
        <label class="check">
          <input
            type="checkbox"
            checked={content.style.shadow !== null}
            onchange={(e) => {
              const on = e.currentTarget.checked;
              updateContent(
                "text",
                (text) =>
                  (text.style = {
                    ...text.style,
                    shadow: on
                      ? { color: "rgba(0, 0, 0, 0.8)", offsetX: 3, offsetY: 3, blur: 6 }
                      : null,
                  }),
              );
            }}
          >
          Shadow
        </label>
      </section>
    {:else if element.content.type === "shape"}
      {@const content = element.content}
      <section>
        <h5>Shape</h5>
        <select
          value={content.shape}
          aria-label="Shape"
          onchange={(e) => {
            const shape = e.currentTarget.value as ShapeKind;
            updateContent("shape", (item) => (item.shape = shape));
          }}
        >
          <option value="rectangle">Rectangle</option>
          <option value="ellipse">Ellipse</option>
          <option value="line">Line</option>
        </select>
        {#if content.shape !== "line"}
          <label class="check">
            <input
              type="checkbox"
              checked={content.fill !== null}
              onchange={(e) => {
                const on = e.currentTarget.checked;
                updateContent("shape", (item) => (item.fill = on ? "#ffffff" : null));
              }}
            >
            Fill
          </label>
          {#if content.fill !== null}
            <ColorField
              label="Fill colour"
              value={content.fill}
              onchange={(fill) => updateContent("shape", (item) => (item.fill = fill), "fill")}
            />
          {/if}
        {/if}
        <label class="check">
          <input
            type="checkbox"
            checked={content.stroke !== null}
            onchange={(e) => {
              const on = e.currentTarget.checked;
              updateContent(
                "shape",
                (item) => (item.stroke = on ? { color: "#ffffff", width: 4 } : null),
              );
            }}
          >
          {content.shape === "line" ? "Line colour" : "Border"}
        </label>
        {#if content.stroke}
          {@const stroke = content.stroke}
          <div class="row">
            <ColorField
              label="Border colour"
              value={stroke.color}
              onchange={(color) =>
                updateContent("shape", (item) => (item.stroke = { ...stroke, color }), "stroke")}
            />
            <input
              type="number"
              min="1"
              max="60"
              value={stroke.width}
              aria-label="Border width"
              onchange={(e) => {
                const width = e.currentTarget.valueAsNumber || 1;
                updateContent("shape", (item) => (item.stroke = { ...stroke, width }));
              }}
            >
          </div>
        {/if}
        {#if content.shape === "rectangle"}
          <label class="row">
            Corners
            <input
              type="number"
              min="0"
              max="500"
              value={content.cornerRadius}
              onchange={(e) => {
                const radius = e.currentTarget.valueAsNumber || 0;
                updateContent("shape", (item) => (item.cornerRadius = radius));
              }}
            >
          </label>
        {/if}
      </section>
    {:else}
      {@const content = element.content}
      <section>
        <h5>{content.kind === "video" ? "Video" : "Picture"}</h5>
        <label class="row">
          Fit
          <select
            value={content.fit}
            onchange={(e) => {
              const fit = e.currentTarget.value as MediaFit;
              updateContent("media", (item) => (item.fit = fit));
            }}
          >
            <option value="contain">Show it all</option>
            <option value="cover">Fill the box (crop)</option>
            <option value="stretch">Stretch to the box</option>
          </select>
        </label>
        <label class="row">
          Corners
          <input
            type="number"
            min="0"
            max="500"
            value={content.cornerRadius}
            onchange={(e) => {
              const radius = e.currentTarget.valueAsNumber || 0;
              updateContent("media", (item) => (item.cornerRadius = radius));
            }}
          >
        </label>
        {#if content.kind === "video"}
          <label class="check">
            <input
              type="checkbox"
              checked={content.playAudio}
              onchange={(e) => {
                const on = e.currentTarget.checked;
                updateContent("media", (item) => (item.playAudio = on));
              }}
            >
            Play sound on the projector
          </label>
          <p class="hint">
            Videos on a slide loop. Transparent videos (and animated WebP or PNG) show the slide
            through.
          </p>
        {/if}
      </section>
    {/if}

    <section class="actions">
      <Button disabled={others === 0} onclick={applyElement}>Apply to All Slides</Button>
      <Button variant="live" onclick={() => editor.deleteSelected()}>Delete</Button>
    </section>
  {:else if slide}
    <header><h4>Slide {editor.slideIndex + 1}</h4></header>
    <section>
      <label class="column">
        Theme
        <select
          value={slide.themeId ?? ""}
          onchange={(e) => {
            const id = e.currentTarget.value || null;
            editor.change(() => (slide.themeId = id));
          }}
        >
          <option value="">The presentation's theme</option>
          {#each themes as theme (theme.id)}
            <option value={theme.id}>{theme.theme.name}</option>
          {/each}
        </select>
      </label>
      <label class="check">
        <input
          type="checkbox"
          checked={slide.background !== null}
          onchange={(e) => {
            const on = e.currentTarget.checked;
            editor.change(
              () => (slide.background = on ? { type: "color", color: "#000000" } : null),
            );
          }}
        >
        Its own background
      </label>
      {#if slide.background}
        <BackgroundEditor
          value={slide.background}
          onchange={(background) =>
            editor.change(() => (slide.background = background), "background")}
        />
      {/if}
    </section>
    <section class="actions">
      <Button disabled={others === 0} onclick={applySlide}>Apply to All Slides</Button>
    </section>
    <p class="hint">
      Click a text box, shape or picture to change it. Double-click a text box to type.
    </p>
  {/if}

  {#if applied}
    <p class="applied" role="status">{applied}</p>
  {/if}
</div>

<style>
  .inspector {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-3);
    overflow-y: auto;
    padding-right: var(--sw-space-1);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h4,
  h5 {
    margin: 0;
    color: var(--sw-text-muted);
  }

  h5 {
    font-size: var(--sw-font-size-small);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: var(--sw-space-2);
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--sw-space-2);
  }

  .grid label,
  .column {
    display: flex;
    flex-direction: column;
    gap: 2px;
    color: var(--sw-text-muted);
    font-size: var(--sw-font-size-small);
  }

  .grid input {
    width: 100%;
    box-sizing: border-box;
  }

  .row,
  .check {
    display: flex;
    align-items: center;
    gap: var(--sw-space-2);
    color: var(--sw-text-muted);
  }

  .row input[type="range"] {
    flex: 1;
  }

  .row input[type="number"] {
    width: 70px;
  }

  .value {
    min-width: 3em;
    font-variant-numeric: tabular-nums;
  }

  .arrange {
    margin: 0;
    padding: 0;
    border: none;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }

  .actions {
    flex-direction: row;
    flex-wrap: wrap;
  }

  .hint {
    margin: 0;
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }

  .applied {
    margin: 0;
    padding: var(--sw-space-2);
    border-radius: var(--sw-radius-small);
    background: rgba(64, 192, 87, 0.15);
    color: var(--sw-text);
  }
</style>
