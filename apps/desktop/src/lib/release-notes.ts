/**
 * Release notes, as shown in the update window.
 *
 * The notes are a CHANGELOG.md section, in Markdown. Only the little of
 * Markdown that release notes use is understood, and it is turned into
 * plain data ([`parseNotes`]) that the window draws with ordinary elements,
 * never as HTML, so nothing in a release text can run in the app:
 *
 * - `## Headings`;
 * - points (`- `, `* `, `+ ` or `1. `), and points under them (indented);
 * - paragraphs, with lines wrapped in the file joined back up;
 * - `**bold**`, `*italic*`, `` `code` `` and `[links](address)` (shown as
 *   their words).
 */

/** A run of text with one style. */
export interface Inline {
  text: string;
  bold?: boolean;
  italic?: boolean;
  code?: boolean;
}

/** A point, any points under it, and any words after those. */
export interface NoteItem {
  content: Inline[];
  children: NoteItem[];
  /** Words after its points (indented like the point, after a gap). */
  after?: Inline[];
}

/** A point being read: its lines, its points' lines and lines after. */
interface Reading {
  lines: string[];
  children: string[][];
  after: string[];
}

/** A heading, a paragraph or a list of points. */
export type NoteBlock =
  | { type: "heading"; level: number; content: Inline[] }
  | { type: "paragraph"; content: Inline[] }
  | { type: "list"; items: NoteItem[] };

/** A point's marker, with the indent before it and the text after. */
const POINT = /^(\s*)(?:[-*+]|\d+[.)])\s+(.*)$/;
const HEADING = /^(#{1,6})\s+(.*?)\s*#*\s*$/;

/** Reads release notes into blocks to draw. */
export function parseNotes(markdown: string): NoteBlock[] {
  const blocks: NoteBlock[] = [];
  /** The paragraph being read, as lines. */
  let paragraph: string[] = [];
  /** The list being read: its top-level points, with their lines. */
  let list: Reading[] | null = null;
  /** The indent of the list's top-level points. */
  let listIndent = 0;

  const endParagraph = () => {
    if (paragraph.length > 0) {
      blocks.push({ type: "paragraph", content: inline(paragraph.join(" ")) });
      paragraph = [];
    }
  };
  const endList = () => {
    if (list) {
      blocks.push({
        type: "list",
        items: list.map((point) => ({
          content: inline(point.lines.join(" ")),
          children: point.children.map((lines) => ({
            content: inline(lines.join(" ")),
            children: [],
          })),
          ...(point.after.length > 0 ? { after: inline(point.after.join(" ")) } : {}),
        })),
      });
      list = null;
    }
  };

  for (const raw of markdown.replace(/\r\n?/g, "\n").split("\n")) {
    const line = raw.trimEnd();
    if (!line.trim()) {
      // A blank line ends a paragraph; a list may go on after one.
      endParagraph();
      continue;
    }

    const heading = HEADING.exec(line);
    if (heading) {
      endParagraph();
      endList();
      blocks.push({
        type: "heading",
        level: heading[1]?.length ?? 2,
        content: inline(heading[2] ?? ""),
      });
      continue;
    }

    const point = POINT.exec(line);
    if (point) {
      endParagraph();
      const indent = point[1]?.length ?? 0;
      const text = point[2] ?? "";
      if (!list) {
        list = [];
        listIndent = indent;
      }
      const parent = list.at(-1);
      if (indent > listIndent && parent) {
        parent.children.push([text]);
      } else {
        list.push({ lines: [text], children: [], after: [] });
      }
      continue;
    }

    // An indented line carries on the point above it (wrapped in the
    // file): a sub-point if indented as far as its words, otherwise the
    // point itself, after its sub-points if it has any.
    const current = (list as Reading[] | null)?.at(-1);
    if (current && /^\s/.test(raw)) {
      const indent = raw.length - raw.trimStart().length;
      const child = current.children.at(-1);
      if (child && indent >= listIndent + 4) child.push(line.trim());
      else if (current.children.length > 0) current.after.push(line.trim());
      else current.lines.push(line.trim());
      continue;
    }

    endList();
    paragraph.push(line.trim());
  }
  endParagraph();
  endList();
  return blocks;
}

/** Splits text into runs of bold, italic, code and plain text. */
export function inline(text: string): Inline[] {
  const runs: Inline[] = [];
  // Links show as their words.
  const words = text.replace(/\[([^\]]+)\]\([^)]*\)/g, "$1");
  const pattern = /\*\*(.+?)\*\*|__(.+?)__|`([^`]+)`|\*([^*\s][^*]*?)\*|_([^_\s][^_]*?)_/g;
  let at = 0;
  for (const match of words.matchAll(pattern)) {
    const index = match.index ?? 0;
    if (index > at) runs.push({ text: words.slice(at, index) });
    const [, bold1, bold2, code, italic1, italic2] = match;
    if (bold1 !== undefined || bold2 !== undefined) {
      // Bold may hold italics or code, but the window shows it all bold.
      runs.push({ text: (bold1 ?? bold2 ?? "").replace(/[*_`]/g, ""), bold: true });
    } else if (code !== undefined) {
      runs.push({ text: code, code: true });
    } else {
      runs.push({ text: italic1 ?? italic2 ?? "", italic: true });
    }
    at = index + match[0].length;
  }
  if (at < words.length) runs.push({ text: words.slice(at) });
  return runs;
}
