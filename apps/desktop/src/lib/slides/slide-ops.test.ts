// biome-ignore-all lint/style/noNonNullAssertion: the tests index arrays they just built
import type { DeckSlide, SlideDeck, TextStyle } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import {
  addElement,
  applyElementToAll,
  applySlideLookToAll,
  arrange,
  blankSlide,
  newShape,
  newTextBox,
  slideFromLayout,
  slideLabel,
} from "./slide-ops.ts";

const style = { fontSize: 80, color: "#fff" } as TextStyle;

function deckOf(slides: DeckSlide[]): SlideDeck {
  return {
    id: "d",
    name: "D",
    themeId: null,
    slides,
    autoAdvance: null,
    createdAt: 0,
    updatedAt: 0,
  };
}

/** A prayer slide: two white bars and a title. */
function prayerSlide(): DeckSlide {
  const slide = blankSlide();
  addElement(slide, newShape("rectangle"));
  addElement(slide, newShape("rectangle"));
  const title = newTextBox(style, "PRAYER 1");
  if (title.content.type === "text")
    title.content.runs[0] = { ...title.content.runs[0]!, color: "gold" };
  addElement(slide, title);
  return slide;
}

describe("slideFromLayout", () => {
  it("keeps the shapes and empties the text, keeping its style and role", () => {
    const first = prayerSlide();
    const next = slideFromLayout(first);
    expect(next.elements).toHaveLength(3);
    const title = next.elements[2]!;
    expect(title.role).toBe(first.elements[2]!.role);
    expect(title.id).not.toBe(first.elements[2]!.id);
    if (title.content.type !== "text") throw new Error("a text box");
    expect(title.content.runs).toEqual([
      { text: "", bold: false, italic: false, underline: false, color: "gold" },
    ]);
  });
});

describe("applyElementToAll", () => {
  it("moves and restyles the same box on every slide but keeps its words", () => {
    const first = prayerSlide();
    const second = slideFromLayout(first);
    const secondTitle = second.elements[2]!;
    if (secondTitle.content.type === "text")
      secondTitle.content.runs = [
        { text: "PRAYER 2", bold: false, italic: false, underline: false },
      ];
    const deck = deckOf([first, second]);

    const title = first.elements[2]!;
    title.frame = { x: 10, y: 20, width: 800, height: 120 };
    if (title.content.type === "text") title.content.style = { ...style, fontSize: 140 };
    applyElementToAll(deck, 0, title.id);

    const moved = deck.slides[1]!.elements[2]!;
    expect(moved.frame).toEqual(title.frame);
    if (moved.content.type !== "text") throw new Error("a text box");
    expect(moved.content.style.fontSize).toBe(140);
    expect(moved.content.runs[0]!.text).toBe("PRAYER 2");
    expect(moved.content.runs[0]!.color).toBe("gold");
  });

  it("copies a logo onto slides that do not have it", () => {
    const withLogo = prayerSlide();
    const logo = newShape("ellipse");
    addElement(withLogo, logo);
    const deck = deckOf([withLogo, blankSlide()]);
    applyElementToAll(deck, 0, logo.id);
    expect(deck.slides[1]!.elements.map((item) => item.role)).toEqual([logo.role]);
  });
});

describe("applySlideLookToAll", () => {
  it("gives every slide the same background", () => {
    const first = blankSlide();
    first.background = { type: "color", color: "maroon" };
    const deck = deckOf([first, blankSlide()]);
    applySlideLookToAll(deck, 0);
    expect(deck.slides[1]!.background).toEqual({ type: "color", color: "maroon" });
  });
});

describe("arrange", () => {
  it("brings to the front and sends to the back", () => {
    const slide = prayerSlide();
    const [a, b, c] = slide.elements.map((item) => item.id);
    arrange(slide, a!, "front");
    expect(slide.elements.map((item) => item.id)).toEqual([b, c, a]);
    arrange(slide, a!, "back");
    arrange(slide, c!, "backward");
    expect(slide.elements.map((item) => item.id)).toEqual([a, c, b]);
  });
});

describe("slideLabel", () => {
  it("names a slide by its first line of words", () => {
    expect(slideLabel(prayerSlide())).toBe("PRAYER 1");
    expect(slideLabel(blankSlide())).toBe("Blank slide");
  });
});
