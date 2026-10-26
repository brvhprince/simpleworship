/**
 * The state of the slide editor: the presentation being edited (a copy,
 * so Cancel discards it), which slide and element are selected, the text
 * selection while typing in a box, undo and redo, and copy and paste.
 *
 * Components read and change it directly; every change that should be
 * undoable goes through {@link SlideEditor.change}.
 */

import type {
  DeckSlide,
  ElementContent,
  SlideDeck,
  SlideElement,
  TextRun,
} from "@simpleworship/protocol";
import {
  allHave,
  applyStyle,
  applyStyleToAll,
  type RunStyle,
  type StylePatch,
  styleAt,
} from "./rich-text.ts";
import { copyElement } from "./slide-ops.ts";

/** How many steps Undo can go back. */
const HISTORY_LIMIT = 100;

/** Changes of the same kind closer together than this are one undo step. */
const COALESCE_MS = 800;

type TextContent = Extract<ElementContent, { type: "text" }>;

/** Elements copied with ⌘C, kept for the whole session (any presentation). */
let clipboard: SlideElement[] = [];

export class SlideEditor {
  /** The presentation being edited. */
  deck: SlideDeck = $state() as SlideDeck;
  /** The slide shown on the canvas. */
  slideIndex = $state(0);
  /** The selected element, if any. */
  selectedId: string | null = $state(null);
  /** The text box being typed in, if any. */
  editingId: string | null = $state(null);
  /** The selected characters while typing in a box. */
  textSelection: { start: number; end: number } | null = $state(null);
  /**
   * Bumped when the text being typed in was restyled from outside (the
   * toolbar), so the editable element redraws it.
   */
  textVersion = $state(0);

  #past: string[] = $state([]);
  #future: string[] = $state([]);
  /** The last checkpoint's tag and time, so a run of small changes is one undo step. */
  #last: { tag: string; at: number } | null = null;

  constructor(deck: SlideDeck) {
    this.deck = deck;
  }

  get slide(): DeckSlide | null {
    return this.deck.slides[this.slideIndex] ?? null;
  }

  get selected(): SlideElement | null {
    return this.slide?.elements.find((item) => item.id === this.selectedId) ?? null;
  }

  /** The selected element when it is a text box. */
  get textBox(): (SlideElement & { content: TextContent }) | null {
    const element = this.selected;
    return element?.content.type === "text"
      ? (element as SlideElement & { content: TextContent })
      : null;
  }

  get canUndo(): boolean {
    return this.#past.length > 0;
  }

  get canRedo(): boolean {
    return this.#future.length > 0;
  }

  // ── Undo ───────────────────────────────────────────────────────────────

  /**
   * Remembers the current state, so the next change can be undone. Changes
   * with the same `tag` in quick succession (dragging a colour picker or a
   * slider) share one undo step.
   */
  checkpoint(tag?: string): void {
    const now = Date.now();
    const last = this.#last;
    this.#last = tag ? { tag, at: now } : null;
    if (tag && last?.tag === tag && now - last.at < COALESCE_MS) return;
    this.#past = [...this.#past.slice(-HISTORY_LIMIT + 1), this.#snapshot()];
    this.#future = [];
  }

  /** Makes an undoable change (see {@link SlideEditor.checkpoint} for `tag`). */
  change(update: (deck: SlideDeck) => void, tag?: string): void {
    this.checkpoint(tag);
    update(this.deck);
  }

  undo(): void {
    const previous = this.#past.at(-1);
    if (!previous) return;
    this.#future = [...this.#future, this.#snapshot()];
    this.#past = this.#past.slice(0, -1);
    this.#restore(previous);
  }

  redo(): void {
    const next = this.#future.at(-1);
    if (!next) return;
    this.#past = [...this.#past, this.#snapshot()];
    this.#future = this.#future.slice(0, -1);
    this.#restore(next);
  }

  #snapshot(): string {
    return JSON.stringify({ deck: $state.snapshot(this.deck), slide: this.slideIndex });
  }

  #restore(snapshot: string): void {
    const { deck, slide } = JSON.parse(snapshot) as { deck: SlideDeck; slide: number };
    this.stopEditing();
    this.deck = deck;
    this.slideIndex = Math.min(slide, Math.max(deck.slides.length - 1, 0));
    if (!this.selected) this.selectedId = null;
  }

  // ── Selection ──────────────────────────────────────────────────────────

  showSlide(index: number): void {
    this.stopEditing();
    this.slideIndex = Math.min(Math.max(index, 0), Math.max(this.deck.slides.length - 1, 0));
    this.selectedId = null;
  }

  select(id: string | null): void {
    if (this.editingId && this.editingId !== id) this.stopEditing();
    this.selectedId = id;
  }

  /** Starts typing in a text box. */
  startEditing(id: string): void {
    const element = this.slide?.elements.find((item) => item.id === id);
    if (element?.content.type !== "text") return;
    this.checkpoint();
    this.selectedId = id;
    this.editingId = id;
    this.textSelection = null;
  }

  /**
   * Stores what was typed in the box being edited. Not an undo step of its
   * own: starting to type was (see {@link SlideEditor.startEditing}).
   */
  setTypedText(runs: TextRun[]): void {
    const box = this.textBox;
    if (box && box.id === this.editingId) box.content.runs = runs;
  }

  stopEditing(): void {
    this.editingId = null;
    this.textSelection = null;
  }

  // ── Text style ─────────────────────────────────────────────────────────

  /** Whether a style change applies to selected words (rather than the whole box). */
  get stylingWords(): boolean {
    const range = this.textSelection;
    return this.editingId !== null && range !== null && range.end > range.start;
  }

  /** The style shown in the toolbar: of the selected words, or the box's. */
  get currentStyle():
    | (RunStyle & {
        boxFontSize: number;
        boxColor: string;
        boxFont: string;
        boxFontWeight: number;
      })
    | null {
    const box = this.textBox;
    if (!box) return null;
    const range = this.textSelection;
    const runs = box.content.runs;
    const style = this.stylingWords && range ? styleAt(runs, range.end) : styleAt(runs, 0);
    return {
      ...style,
      boxFontSize: box.content.style.fontSize,
      boxColor: box.content.style.color,
      boxFont: box.content.style.fontFamily,
      boxFontWeight: box.content.style.fontWeight,
    };
  }

  /**
   * Applies a text style to the selected words, or to the whole box when
   * no words are selected. For the whole box, colour, size, font and
   * weight go on the box itself and any word-level differences are cleared.
   */
  formatText(patch: StylePatch): void {
    const box = this.textBox;
    if (!box) return;
    this.checkpoint(`format:${Object.keys(patch).join()}`);
    const range = this.textSelection;
    if (this.stylingWords && range) {
      box.content.runs = applyStyle(box.content.runs, range.start, range.end, patch);
    } else {
      const content = box.content;
      const cleared: StylePatch = {};
      if (patch.color !== undefined) {
        if (patch.color) content.style = { ...content.style, color: patch.color };
        cleared.color = null;
      }
      if (patch.fontSize !== undefined) {
        if (patch.fontSize) content.style = { ...content.style, fontSize: patch.fontSize };
        cleared.fontSize = null;
      }
      if (patch.fontFamily !== undefined) {
        if (patch.fontFamily) content.style = { ...content.style, fontFamily: patch.fontFamily };
        cleared.fontFamily = null;
      }
      if (patch.fontWeight !== undefined) {
        if (patch.fontWeight) content.style = { ...content.style, fontWeight: patch.fontWeight };
        cleared.fontWeight = null;
      }
      for (const key of ["bold", "italic", "underline"] as const) {
        if (patch[key] !== undefined) cleared[key] = patch[key];
      }
      content.runs = applyStyleToAll(content.runs, cleared);
    }
    this.textVersion += 1;
  }

  /** Turns bold, italic or underline on or off for the selection or box. */
  toggleText(key: "bold" | "italic" | "underline"): void {
    const box = this.textBox;
    if (!box) return;
    const runs = box.content.runs;
    const range =
      this.stylingWords && this.textSelection ? this.textSelection : { start: 0, end: Infinity };
    const total = runs.reduce((sum, run) => sum + run.text.length, 0);
    const on = allHave(runs, range.start, Math.min(range.end, total), key);
    // Bold on or off replaces any weight chosen for those words.
    this.formatText(key === "bold" ? { bold: !on, fontWeight: null } : { [key]: !on });
  }

  /**
   * Sets the font weight (100–900) of the selected words, or of the whole
   * box when no words are selected. For words it replaces bold, so the
   * toolbar and the text agree.
   */
  setFontWeight(weight: number): void {
    this.formatText(
      this.stylingWords ? { fontWeight: weight, bold: false } : { fontWeight: weight },
    );
  }

  // ── Elements ───────────────────────────────────────────────────────────

  /** Removes the selected element. */
  deleteSelected(): void {
    const id = this.selectedId;
    const slide = this.slide;
    if (!id || !slide) return;
    this.change(() => {
      slide.elements = slide.elements.filter((item) => item.id !== id);
    });
    this.selectedId = null;
    this.stopEditing();
  }

  copy(): void {
    const element = this.selected;
    if (element) clipboard = [$state.snapshot(element) as SlideElement];
  }

  /** Pastes copied elements onto this slide, nudged so they do not hide the originals. */
  paste(): void {
    const slide = this.slide;
    if (!slide || clipboard.length === 0) return;
    this.change(() => {
      for (const source of clipboard) {
        const sameSlide = slide.elements.some((item) => item.id === source.id);
        const pasted = copyElement(source, false);
        if (sameSlide)
          pasted.frame = { ...pasted.frame, x: pasted.frame.x + 30, y: pasted.frame.y + 30 };
        slide.elements.push(pasted);
        this.selectedId = pasted.id;
      }
    });
  }

  duplicateSelected(): void {
    this.copy();
    this.paste();
  }

  /** Moves the selected element by a few canvas pixels (arrow keys). */
  nudge(dx: number, dy: number): void {
    const element = this.selected;
    if (!element || element.locked) return;
    this.change(() => {
      element.frame = { ...element.frame, x: element.frame.x + dx, y: element.frame.y + dy };
    });
  }
}
