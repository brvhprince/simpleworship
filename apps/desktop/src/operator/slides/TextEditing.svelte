<!--
  @component
  Typing in a text box, right on the canvas. It sits exactly where the box
  is and is drawn with the same styles as the projector, so what you type
  looks as it will on screen.

  The browser handles the typing; after each change the words are read
  back into runs (see `lib/slides/rich-text.ts`). Styling comes from the
  editor's toolbar, never from the browser, so ⌘B / ⌘I / ⌘U are handled
  here. Esc stops editing.

  Place it inside the `ScaledCanvas`, so positions are in canvas pixels.
-->
<script lang="ts">
  import type { ElementContent, SlideElement } from "@simpleworship/protocol";
  import {
    autofit,
    elementBoxStyle,
    frameStyle,
    textStyle,
    verticalJustify,
  } from "@simpleworship/renderer";
  import { onMount, untrack } from "svelte";
  import { t } from "../../lib/i18n/i18n.svelte.ts";
  import type { SlideEditor } from "../../lib/slides/editor.svelte.ts";
  import {
    runsFromDom,
    runsToHtml,
    selectionOffsets,
    setSelectionOffsets,
  } from "../../lib/slides/rich-text.ts";

  type TextContent = Extract<ElementContent, { type: "text" }>;

  interface Props {
    editor: SlideEditor;
    element: SlideElement & { content: TextContent };
    /** Where the operator double-clicked, to put the caret there. */
    clickAt: { x: number; y: number } | null;
  }

  let { editor, element, clickAt }: Props = $props();

  const placeholder = $derived(
    element.role === "title" ||
      element.frame.y < 350 ||
      element.content.style.fontSize >= 50
      ? t("editor.placeholderTitle")
      : t("editor.placeholderText"),
  );

  let editable: HTMLDivElement;

  /** Draws the words into the editable element, keeping the selection. */
  function draw(keepSelection: boolean) {
    const selection = keepSelection ? selectionOffsets(editable) : null;
    editable.innerHTML = runsToHtml(element.content.runs, element.content.style);
    const range = keepSelection ? (editor.textSelection ?? selection) : null;
    if (range) setSelectionOffsets(editable, range.start, range.end);
  }

  onMount(() => {
    draw(false);
    editable.focus();
    // Put the caret where the operator clicked, or at the end.
    const caret = clickAt ? document.caretRangeFromPoint?.(clickAt.x, clickAt.y) : null;
    const selection = window.getSelection();
    if (caret && editable.contains(caret.startContainer)) {
      selection?.removeAllRanges();
      selection?.addRange(caret);
    } else {
      const end = element.content.runs.reduce((sum, run) => sum + run.text.length, 0);
      setSelectionOffsets(editable, end, end);
    }

    // Keep the last selection while the toolbar has focus (choosing a font
    // or colour), so the change goes to the words that were selected.
    const follow = () => {
      const offsets = selectionOffsets(editable);
      if (offsets) editor.textSelection = offsets;
    };
    document.addEventListener("selectionchange", follow);
    return () => document.removeEventListener("selectionchange", follow);
  });

  // The toolbar restyled the words: draw them again.
  let drawnVersion = untrack(() => editor.textVersion);
  $effect(() => {
    const version = editor.textVersion;
    if (version === drawnVersion) return;
    drawnVersion = version;
    untrack(() => draw(true));
  });

  function handleInput() {
    editor.setTypedText(runsFromDom(editable));
  }

  function handleKeys(event: KeyboardEvent) {
    event.stopPropagation();
    if (event.key === "Escape") {
      event.preventDefault();
      editor.stopEditing();
      return;
    }
    if ((event.metaKey || event.ctrlKey) && !event.altKey) {
      const key = ({ b: "bold", i: "italic", u: "underline" } as const)[event.key.toLowerCase()];
      if (key) {
        event.preventDefault();
        editor.toggleText(key);
      }
    }
  }
</script>

<div class="element" style={frameStyle(element.frame, element.opacity)}>
  <div class="box" style={elementBoxStyle(element.content)}>
    <div
      class="fit"
      style:justify-content={verticalJustify(element.content)}
      use:autofit={{
        enabled: element.content.autoFit,
        maxSize: element.content.style.fontSize,
        content: [element.content.runs, element.content.style, element.frame],
      }}
    >
      <!-- A rich-text editor: a <textarea> cannot show styled words. -->
      <!-- biome-ignore lint/a11y/useSemanticElements: a textarea cannot show styled words -->
      <div
        bind:this={editable}
        class="text"
        style={textStyle(element.content.style)}
        data-placeholder={placeholder}
        contenteditable="plaintext-only"
        role="textbox"
        tabindex="0"
        aria-multiline="true"
        aria-label={t("textBox.label")}
        spellcheck="true"
        oninput={handleInput}
        onkeydown={handleKeys}
      ></div>
    </div>
  </div>
</div>

<style>
  .element {
    position: absolute;
    outline: 4px solid var(--sw-accent);
    outline-offset: 2px;
  }

  .box {
    position: absolute;
    inset: 0;
    box-sizing: border-box;
    overflow: hidden;
  }

  .fit {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .text {
    white-space: pre-wrap;
    overflow-wrap: normal;
    outline: none;
    cursor: text;
    min-height: 1em;
  }

  .text:empty::before {
    content: attr(data-placeholder);
    opacity: 0.5;
    pointer-events: none;
    font-style: italic;
  }
</style>
