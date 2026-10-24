/**
 * Changes to presentation slides in the canvas editor, as plain functions
 * on the deck (easy to test, and easy to undo: the editor keeps copies).
 *
 * Elements that are "the same" on several slides share a `role`. New
 * slides copy the current slide's layout with the same roles, so "Apply
 * to All Slides" knows which box on every other slide is the title.
 */

import type {
  Background,
  DeckSlide,
  ElementContent,
  Frame,
  MediaItem,
  ShapeKind,
  SlideDeck,
  SlideElement,
  TextRun,
  TextStyle,
} from "@simpleworship/protocol";
import { CANVAS_HEIGHT, CANVAS_WIDTH } from "@simpleworship/renderer";
import { uuidV7 } from "../api.ts";
import { t } from "../i18n/i18n.svelte.ts";
import { plainText } from "./rich-text.ts";

type TextContent = Extract<ElementContent, { type: "text" }>;

/**
 * A deep copy. The editor's slides are reactive proxies, which
 * `structuredClone` refuses to copy; their JSON is plain data.
 */
function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

/** A frame centred on the canvas. */
export function centred(width: number, height: number): Frame {
  return { x: (CANVAS_WIDTH - width) / 2, y: (CANVAS_HEIGHT - height) / 2, width, height };
}

function element(frame: Frame, content: ElementContent): SlideElement {
  return { id: uuidV7(), role: uuidV7(), frame, opacity: 1, locked: false, content };
}

/** A new text box, in the theme's text style (white, centred, shrinks to fit). */
export function newTextBox(style: TextStyle, text = ""): SlideElement {
  return element(centred(1400, 300), {
    type: "text",
    runs: [{ text, bold: false, italic: false, underline: false }],
    style: { ...style },
    verticalAlign: "middle",
    autoFit: true,
    padding: 0,
    fill: null,
    cornerRadius: 0,
  });
}

/** A new shape: a white rectangle, a white ellipse, or a white line. */
export function newShape(shape: ShapeKind): SlideElement {
  const frame = shape === "line" ? centred(800, 40) : centred(500, 300);
  return element(frame, {
    type: "shape",
    shape,
    fill: shape === "line" ? null : "#ffffff",
    stroke: shape === "line" ? { color: "#ffffff", width: 6 } : null,
    cornerRadius: 0,
  });
}

/** A picture or video, at its own shape, as large as fits in 1200×800. */
export function newMedia(item: MediaItem): SlideElement {
  const ratio = item.width && item.height ? item.width / item.height : 16 / 9;
  const width = Math.min(1200, 800 * ratio);
  return element(centred(Math.round(width), Math.round(width / ratio)), {
    type: "media",
    media: { id: item.id, file: item.file },
    kind: item.kind === "video" ? "video" : "image",
    fit: "contain",
    cornerRadius: 0,
    playAudio: false,
  });
}

/**
 * Where a new element goes in the stacking order: pictures and videos at
 * the back (just above the background), text and shapes at the front.
 */
export function insertionIndex(slide: DeckSlide, added: SlideElement): number {
  return added.content.type === "media" ? 0 : slide.elements.length;
}

/** Puts an element on a slide, in its usual place in the stacking order. */
export function addElement(slide: DeckSlide, added: SlideElement): void {
  slide.elements.splice(insertionIndex(slide, added), 0, added);
}

/** A copy of an element with a fresh id. `sameRole` keeps it "the same" element. */
export function copyElement(source: SlideElement, sameRole: boolean): SlideElement {
  return {
    ...clone(source),
    id: uuidV7(),
    role: sameRole ? source.role : uuidV7(),
  };
}

/** A copy of a slide: fresh ids, the same roles. */
export function duplicateSlide(slide: DeckSlide): DeckSlide {
  return {
    ...clone(slide),
    id: uuidV7(),
    elements: slide.elements.map((item) => copyElement(item, true)),
  };
}

/**
 * A new slide with the layout of `slide`: the same background, theme,
 * pictures and shapes, and the same text boxes, empty (keeping the style
 * of their first words, so a yellow title stays yellow).
 */
export function slideFromLayout(slide: DeckSlide): DeckSlide {
  const copy = duplicateSlide(slide);
  for (const item of copy.elements) {
    if (item.content.type === "text") {
      const first = item.content.runs[0];
      item.content.runs = [
        { ...(first ?? { bold: false, italic: false, underline: false }), text: "" },
      ];
    }
  }
  return copy;
}

/**
 * Applies the look of one element to "the same" element (same role) on
 * every other slide: its position, size, opacity and style. Text boxes
 * keep their own words; pictures and shapes are copied whole. Slides
 * without it get a copy, at the same place in the stacking order.
 */
export function applyElementToAll(deck: SlideDeck, slideIndex: number, elementId: string): void {
  const source = deck.slides[slideIndex];
  const index = source?.elements.findIndex((item) => item.id === elementId) ?? -1;
  const model = source?.elements[index];
  if (!source || !model) return;

  deck.slides.forEach((slide, current) => {
    if (current === slideIndex) return;
    const twin = slide.elements.find((item) => item.role === model.role);
    if (!twin) {
      slide.elements.splice(Math.min(index, slide.elements.length), 0, copyElement(model, true));
      return;
    }
    twin.frame = { ...model.frame };
    twin.opacity = model.opacity;
    twin.locked = model.locked;
    if (model.content.type === "text" && twin.content.type === "text") {
      twin.content = {
        ...clone(model.content),
        runs: restyled(twin.content, model.content),
      };
    } else {
      twin.content = clone(model.content);
    }
  });
}

/**
 * A text box's words with the model box's styling: overrides that every
 * word of the model shares (all yellow, all bold) are given to every word;
 * the rest keep their own.
 */
function restyled(target: TextContent, model: TextContent): TextRun[] {
  const shared = sharedRunStyle(model.runs);
  return target.runs.map((run) => ({ ...run, ...shared }));
}

function sharedRunStyle(runs: TextRun[]): Partial<TextRun> {
  const words = runs.filter((run) => run.text.trim() !== "");
  const first = words[0];
  if (!first) return {};
  const shared: Partial<TextRun> = {};
  for (const key of ["bold", "italic", "underline", "color", "fontSize", "fontFamily"] as const) {
    if (words.every((run) => run[key] === first[key])) {
      (shared as Record<string, unknown>)[key] = first[key];
    }
  }
  return shared;
}

/** Gives every slide this slide's background and theme. */
export function applySlideLookToAll(deck: SlideDeck, slideIndex: number): void {
  const source = deck.slides[slideIndex];
  if (!source) return;
  for (const slide of deck.slides) {
    slide.background = source.background ? (clone(source.background) as Background) : null;
    slide.themeId = source.themeId;
  }
}

/** Moves an element in the stacking order. */
export function arrange(
  slide: DeckSlide,
  elementId: string,
  move: "front" | "back" | "forward" | "backward",
): void {
  const from = slide.elements.findIndex((item) => item.id === elementId);
  if (from === -1) return;
  const last = slide.elements.length - 1;
  const to = {
    front: last,
    back: 0,
    forward: Math.min(from + 1, last),
    backward: Math.max(from - 1, 0),
  }[move];
  const [moved] = slide.elements.splice(from, 1);
  if (moved) slide.elements.splice(to, 0, moved);
}

/** A name for a slide in the slide list: its first line of words. */
export function slideLabel(slide: DeckSlide): string {
  const boxes = slide.elements
    .filter((item) => item.content.type === "text")
    .sort((a, b) => a.frame.y - b.frame.y);
  for (const box of boxes) {
    if (box.content.type !== "text") continue;
    const line = plainText(box.content.runs)
      .split("\n")
      .find((text) => text.trim() !== "");
    if (line) return line.trim();
  }
  const legacy = slide.text.split("\n").find((text) => text.trim() !== "");
  if (legacy) return legacy.replace(/\*|__/g, "").trim();
  const hasPicture =
    slide.background?.type === "image" ||
    slide.elements.some((item) => item.content.type === "media");
  return hasPicture ? t("slides.picture") : t("slides.blank");
}

/** A blank slide: no elements, the presentation's theme and background. */
export function blankSlide(): DeckSlide {
  return {
    id: uuidV7(),
    background: null,
    themeId: null,
    elements: [],
    text: "",
    style: { fontSize: null, color: null, align: null },
  };
}

/** A default template slide with a Title text box and a Body text box. */
export function templateSlide(style?: TextStyle): DeckSlide {
  const slide = blankSlide();
  const baseStyle: TextStyle = style ?? {
    fontFamily: "Inter",
    fontSize: 60,
    fontWeight: 400,
    color: "#ffffff",
    align: "center",
    lineHeight: 1.2,
    italic: false,
    uppercase: false,
    shadow: null,
    outline: null,
  };

  // Title box
  const titleBox = newTextBox(
    { ...baseStyle, fontSize: Math.max(baseStyle.fontSize, 72), fontWeight: 700 },
    "",
  );
  titleBox.frame = { x: 260, y: 180, width: 1400, height: 200 };
  titleBox.role = "title";
  addElement(slide, titleBox);

  // Body box
  const bodyBox = newTextBox(
    { ...baseStyle, fontSize: Math.min(baseStyle.fontSize, 40), fontWeight: 400 },
    "",
  );
  bodyBox.frame = { x: 260, y: 440, width: 1400, height: 420 };
  bodyBox.role = "body";
  addElement(slide, bodyBox);

  return slide;
}
