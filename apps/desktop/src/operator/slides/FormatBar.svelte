<!--
  @component
  The text toolbar: font, size, colour, bold, italic, underline, and the
  box's alignment. While typing with words selected it styles those words
  (e.g. one line in yellow); otherwise it styles the whole box.
-->
<script lang="ts">
  import type { HorizontalAlign, VerticalAlign } from "@simpleworship/protocol";
  import { firstFamily, fontFamily } from "../../lib/fonts.ts";
  import { withModifier } from "../../lib/platform.ts";
  import type { SlideEditor } from "../../lib/slides/editor.svelte.ts";
  import ColorField from "../themes/ColorField.svelte";

  interface Props {
    editor: SlideEditor;
    fonts: string[];
  }

  let { editor, fonts }: Props = $props();

  const box = $derived(editor.textBox);
  const current = $derived(editor.currentStyle);
  const font = $derived(firstFamily(current?.fontFamily ?? current?.boxFont ?? ""));
  const size = $derived(Math.round(current?.fontSize ?? current?.boxFontSize ?? 0));
  const color = $derived(current?.color ?? current?.boxColor ?? "#ffffff");

  /** Keeps the caret in the text box when a toolbar button is pressed. */
  function keepFocus(event: MouseEvent) {
    event.preventDefault();
  }

  function setAlign(align: HorizontalAlign) {
    const target = box;
    if (!target) return;
    editor.change(() => {
      target.content.style = { ...target.content.style, align };
    });
  }

  function setVertical(verticalAlign: VerticalAlign) {
    const target = box;
    if (!target) return;
    editor.change(() => {
      target.content.verticalAlign = verticalAlign;
    });
  }

  function grow(factor: number) {
    if (size) editor.formatText({ fontSize: Math.max(8, Math.round(size * factor)) });
  }
</script>

<div class="bar" class:disabled={!box} role="toolbar" aria-label="Text style">
  <select
    value={font}
    disabled={!box}
    aria-label="Font"
    onchange={(e) => editor.formatText({ fontFamily: fontFamily(e.currentTarget.value) })}
  >
    {#if font && !fonts.includes(font)}
      <option value={font}>{font}</option>
    {/if}
    {#each fonts as family (family)}
      <option value={family}>{family}</option>
    {/each}
  </select>
  <input
    class="size"
    type="number"
    min="8"
    max="400"
    value={size || ""}
    disabled={!box}
    aria-label="Text size"
    title={box?.content.autoFit ? "The largest size; long text shrinks to fit" : "Text size"}
    onchange={(e) => editor.formatText({ fontSize: e.currentTarget.valueAsNumber || size })}
  >
  <button
    type="button"
    disabled={!box}
    title="Larger"
    onmousedown={keepFocus}
    onclick={() => grow(1.1)}
  >
    A+
  </button>
  <button
    type="button"
    disabled={!box}
    title="Smaller"
    onmousedown={keepFocus}
    onclick={() => grow(1 / 1.1)}
  >
    A−
  </button>
  {#if box}
    <ColorField
      label="Text colour"
      value={color}
      onchange={(value) => editor.formatText({ color: value })}
    />
  {/if}

  <span class="divider"></span>
  <button
    type="button"
    class="bold"
    class:on={current?.bold}
    disabled={!box}
    title="Bold ({withModifier("B")})"
    onmousedown={keepFocus}
    onclick={() => editor.toggleText("bold")}
  >
    B
  </button>
  <button
    type="button"
    class="italic"
    class:on={current?.italic}
    disabled={!box}
    title="Italic ({withModifier("I")})"
    onmousedown={keepFocus}
    onclick={() => editor.toggleText("italic")}
  >
    I
  </button>
  <button
    type="button"
    class="underline"
    class:on={current?.underline}
    disabled={!box}
    title="Underline ({withModifier("U")})"
    onmousedown={keepFocus}
    onclick={() => editor.toggleText("underline")}
  >
    U
  </button>

  <span class="divider"></span>
  {#each [
    ["left", "⇤", "Align left"],
    ["center", "↔", "Centre"],
    ["right", "⇥", "Align right"],
  ] as const as [value, icon, label] (value)}
    <button
      type="button"
      class:on={box?.content.style.align === value}
      disabled={!box}
      title={label}
      aria-label={label}
      onmousedown={keepFocus}
      onclick={() => setAlign(value)}
    >
      {icon}
    </button>
  {/each}
  <span class="divider"></span>
  {#each [
    ["top", "⤒", "Top of the box"],
    ["middle", "↕", "Middle of the box"],
    ["bottom", "⤓", "Bottom of the box"],
  ] as const as [value, icon, label] (value)}
    <button
      type="button"
      class:on={box?.content.verticalAlign === value}
      disabled={!box}
      title={label}
      aria-label={label}
      onmousedown={keepFocus}
      onclick={() => setVertical(value)}
    >
      {icon}
    </button>
  {/each}

  <span class="hint">
    {#if !box}
      Select a text box to style its text.
    {:else if editor.stylingWords}
      Styling the selected words
    {:else if editor.editingId}
      Select words to style just those, or style the whole box
    {:else}
      Styling the whole box · double-click to type
    {/if}
  </span>
</div>

<style>
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sw-space-1);
    padding: var(--sw-space-1) 0;
  }

  .bar.disabled {
    opacity: 0.6;
  }

  select {
    width: 170px;
  }

  .size {
    width: 64px;
  }

  button {
    min-width: 30px;
    height: 28px;
    padding: 0 6px;
    border: 1px solid var(--sw-border);
    border-radius: var(--sw-radius-small);
    background: var(--sw-surface-raised);
    color: var(--sw-text);
    cursor: pointer;
  }

  button:disabled {
    cursor: default;
  }

  button.on {
    border-color: var(--sw-accent);
    background: var(--sw-accent-soft, rgba(76, 110, 245, 0.25));
  }

  .bold {
    font-weight: 800;
  }

  .italic {
    font-style: italic;
  }

  .underline {
    text-decoration: underline;
  }

  .divider {
    width: 1px;
    height: 20px;
    margin: 0 var(--sw-space-1);
    background: var(--sw-border);
  }

  .hint {
    margin-left: var(--sw-space-2);
    color: var(--sw-text-faint);
    font-size: var(--sw-font-size-small);
  }
</style>
