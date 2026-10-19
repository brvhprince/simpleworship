/**
 * Rich text in slide text boxes: runs of text, each with its own style
 * (see `TextRun` in `crates/sw-core/src/markup.rs`).
 *
 * The editor types into a `contenteditable` element on the canvas. These
 * helpers turn runs into that element's HTML and read them back, apply a
 * style to a range of characters, and find and set the selection as
 * character offsets, so the model (not the browser) decides the styling.
 */

import type { TextRun, TextStyle } from "@simpleworship/protocol";
import { runStyle } from "@simpleworship/renderer";

/** A run's style: everything but its text. */
export type RunStyle = Omit<TextRun, "text">;

/** A change to apply to some text. `null` clears an override (follow the box). */
export type StylePatch = {
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  color?: string | null;
  fontSize?: number | null;
  fontFamily?: string | null;
};

/** No styling: plain text in the box's own style. */
export const PLAIN: RunStyle = { bold: false, italic: false, underline: false };

/** The text without any styling. */
export function plainText(runs: TextRun[]): string {
  return runs.map((run) => run.text).join("");
}

/** The style part of a run, with unset overrides left out. */
export function styleOf(run: RunStyle): RunStyle {
  const style: RunStyle = { bold: run.bold, italic: run.italic, underline: run.underline };
  if (run.color) style.color = run.color;
  if (run.fontSize) style.fontSize = run.fontSize;
  if (run.fontFamily) style.fontFamily = run.fontFamily;
  return style;
}

function sameStyle(a: RunStyle, b: RunStyle): boolean {
  return JSON.stringify(styleOf(a)) === JSON.stringify(styleOf(b));
}

/**
 * Merges neighbouring runs that look the same and drops empty ones. An
 * empty box keeps one empty run, so what is typed next keeps its style.
 */
export function normalize(runs: TextRun[]): TextRun[] {
  const merged: TextRun[] = [];
  for (const run of runs) {
    const last = merged.at(-1);
    if (run.text === "") continue;
    if (last && sameStyle(last, run)) last.text += run.text;
    else merged.push({ ...styleOf(run), text: run.text });
  }
  if (merged.length === 0) return [{ ...styleOf(runs[0] ?? PLAIN), text: "" }];
  return merged;
}

/** Applies a style change to the characters from `start` to `end`. */
export function applyStyle(
  runs: TextRun[],
  start: number,
  end: number,
  patch: StylePatch,
): TextRun[] {
  const result: TextRun[] = [];
  let position = 0;
  for (const run of runs) {
    const runStart = position;
    const runEnd = position + run.text.length;
    position = runEnd;
    // The part before, inside and after the range.
    const cuts = [
      [runStart, Math.max(runStart, Math.min(start, runEnd))],
      [Math.max(runStart, start), Math.min(runEnd, end)],
      [Math.min(runEnd, Math.max(end, runStart)), runEnd],
    ] as const;
    cuts.forEach(([from, to], part) => {
      if (to <= from) return;
      const text = run.text.slice(from - runStart, to - runStart);
      result.push(part === 1 ? patched({ ...run, text }, patch) : { ...run, text });
    });
  }
  return normalize(result);
}

/** Applies a style change to all of the text. */
export function applyStyleToAll(runs: TextRun[], patch: StylePatch): TextRun[] {
  const all = applyStyle(runs, 0, plainText(runs).length, patch);
  // An empty box: style its placeholder run, so typing picks it up.
  return plainText(runs) === "" ? [patched(all[0] ?? { ...PLAIN, text: "" }, patch)] : all;
}

function patched(run: TextRun, patch: StylePatch): TextRun {
  const next: TextRun = { ...run };
  for (const [key, value] of Object.entries(patch) as [keyof StylePatch, unknown][]) {
    if (value === null || value === undefined) delete next[key];
    else (next as Record<string, unknown>)[key] = value;
  }
  return next;
}

/** The style of the text at `offset` (the character before it), for the toolbar. */
export function styleAt(runs: TextRun[], offset: number): RunStyle {
  let position = 0;
  for (const run of runs) {
    position += run.text.length;
    if (offset <= position && run.text.length > 0) return styleOf(run);
  }
  return styleOf(runs.at(-1) ?? PLAIN);
}

/** Whether every character from `start` to `end` has `key` switched on. */
export function allHave(
  runs: TextRun[],
  start: number,
  end: number,
  key: "bold" | "italic" | "underline",
): boolean {
  let position = 0;
  let any = false;
  for (const run of runs) {
    const runStart = position;
    position += run.text.length;
    if (position <= start || runStart >= end) continue;
    any = true;
    if (!run[key]) return false;
  }
  return any;
}

// ── The editable element's HTML ──────────────────────────────────────────

const escapes: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };

function escapeHtml(text: string): string {
  return text.replace(/[&<>"]/g, (character) => escapes[character] ?? character);
}

/**
 * The HTML for the editable text: one span per run, carrying its style in
 * `data-run` (so it can be read back) and as CSS (so it looks right). A
 * trailing line break gets a placeholder `<br>` so the caret can go there.
 */
export function runsToHtml(runs: TextRun[], base: TextStyle): string {
  const spans = runs
    .map(
      (run) =>
        `<span data-run="${escapeHtml(JSON.stringify(styleOf(run)))}" style="${escapeHtml(runStyle(run, base))}">${escapeHtml(run.text)}</span>`,
    )
    .join("");
  return plainText(runs).endsWith("\n") || plainText(runs) === "" ? `${spans}<br>` : spans;
}

const BLOCKS = new Set(["DIV", "P", "LI"]);

/**
 * Reads the runs back from the editable element after the operator typed.
 * Styles come from the nearest `data-run` span; line breaks from text
 * `\n`, `<br>` and the `<div>`s some browsers add for new lines.
 */
export function runsFromDom(root: Node): TextRun[] {
  const runs: TextRun[] = [];
  let atLineStart = true;
  let lastStyle: RunStyle = PLAIN;

  const push = (text: string, style: RunStyle) => {
    runs.push({ ...style, text });
    atLineStart = text.endsWith("\n");
  };

  const walk = (node: Node, style: RunStyle) => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.textContent ?? "";
      if (text) push(text, style);
      lastStyle = style;
      return;
    }
    if (!(node instanceof Element)) {
      for (const child of Array.from(node.childNodes)) walk(child, style);
      return;
    }
    if (node.tagName === "BR") {
      // A <br> at the end of the text, or of a line with words on it, only
      // holds the line open. Alone in a block, it is an empty line.
      const isLast = !node.nextSibling;
      if (!isLast) push("\n", style);
      else if (node.parentNode !== root && atLineStart) push("\n", style);
      return;
    }
    const own =
      node instanceof HTMLElement && node.dataset.run ? safeParse(node.dataset.run) : null;
    const childStyle = own ?? style;
    const isBlock = BLOCKS.has(node.tagName) && node !== root;
    if (isBlock && !atLineStart) push("\n", lastStyle);
    for (const child of Array.from(node.childNodes)) walk(child, childStyle);
    if (isBlock && !atLineStart) push("\n", lastStyle);
  };

  walk(root, PLAIN);
  // A block at the very end adds a line break that was never typed.
  const last = runs.at(-1);
  if (
    last?.text === "\n" &&
    root instanceof Element &&
    BLOCKS.has(root.lastElementChild?.tagName ?? "")
  ) {
    runs.pop();
  }
  return normalize(runs);
}

function safeParse(json: string): RunStyle | null {
  try {
    return styleOf(JSON.parse(json));
  } catch {
    return null;
  }
}

// ── The selection, as character offsets ──────────────────────────────────

/** The selection inside `root` as character offsets, or `null` if it is elsewhere. */
export function selectionOffsets(root: HTMLElement): { start: number; end: number } | null {
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0) return null;
  const range = selection.getRangeAt(0);
  if (!root.contains(range.startContainer) || !root.contains(range.endContainer)) return null;
  const offsetOf = (container: Node, offset: number) => {
    const before = document.createRange();
    before.selectNodeContents(root);
    before.setEnd(container, offset);
    const wrapper = document.createElement("div");
    wrapper.append(before.cloneContents());
    return plainText(runsFromDom(wrapper)).length;
  };
  const start = offsetOf(range.startContainer, range.startOffset);
  const end = offsetOf(range.endContainer, range.endOffset);
  return { start: Math.min(start, end), end: Math.max(start, end) };
}

/** Selects the characters from `start` to `end` inside `root`. */
export function setSelectionOffsets(root: HTMLElement, start: number, end: number): void {
  const find = (target: number): [Node, number] => {
    let position = 0;
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let last: Text | null = null;
    for (
      let node = walker.nextNode() as Text | null;
      node;
      node = walker.nextNode() as Text | null
    ) {
      const length = node.data.length;
      if (target <= position + length) return [node, target - position];
      position += length;
      last = node;
    }
    return last ? [last, last.data.length] : [root, 0];
  };
  const selection = window.getSelection();
  if (!selection) return;
  const range = document.createRange();
  range.setStart(...find(start));
  range.setEnd(...find(end));
  selection.removeAllRanges();
  selection.addRange(range);
}
