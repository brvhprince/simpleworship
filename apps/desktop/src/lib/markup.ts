/**
 * Bold, italic and underline in presentation slides: the editor's B / I / U
 * buttons and ⌘B / ⌘I / ⌘U. The markup itself is read in Rust (see
 * `crates/sw-core/src/markup.rs`): `**bold**`, `*italic*`, `__underline__`.
 */

/** The markers for each style. */
export const MARKERS = { bold: "**", italic: "*", underline: "__" } as const;

export type TextFormat = keyof typeof MARKERS;

/** Text and the selection in it, as a textarea has them. */
export interface Selection {
  text: string;
  start: number;
  end: number;
}

/**
 * Turns a style on or off for the selected text: wraps it in markers, or
 * removes them if it is already wrapped. With nothing selected, inserts an
 * empty pair with the cursor between, ready to type.
 */
export function toggleFormat({ text, start, end }: Selection, format: TextFormat): Selection {
  const marker = MARKERS[format];
  const size = marker.length;
  const before = text.slice(start - size, start);
  const after = text.slice(end, end + size);

  // Already wrapped: unwrap. (For italic, "*" must not be half of "**".)
  const isPartOfBold = format === "italic" && (text[start - 2] === "*" || text[end + 1] === "*");
  if (before === marker && after === marker && !isPartOfBold) {
    return {
      text: text.slice(0, start - size) + text.slice(start, end) + text.slice(end + size),
      start: start - size,
      end: end - size,
    };
  }

  return {
    text: text.slice(0, start) + marker + text.slice(start, end) + marker + text.slice(end),
    start: start + size,
    end: end + size,
  };
}
