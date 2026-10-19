import type { Presentation, Slide } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { frameAt } from "./frames.ts";

function slide(overrides: Partial<Slide>): Slide {
  return {
    text: "Hello",
    section: null,
    startsSection: false,
    caption: null,
    scripture: null,
    background: null,
    countdown: null,
    rich: null,
    look: null,
    style: null,
    elements: null,
    ...overrides,
  };
}

function presentation(slides: Slide[]): Presentation {
  const base = {
    name: "Base",
    background: { type: "color" as const, color: "black" },
    text: { fontSize: 80, color: "white", align: "center" as const },
    scripture: { referenceOnEverySlide: false },
  };
  return {
    id: "p",
    source: { type: "text" },
    title: "T",
    slides,
    theme: base as never,
    themeId: null,
    backgroundOverride: null,
    footer: null,
    autoAdvance: null,
  };
}

describe("frameAt", () => {
  it("applies a slide's own background and text adjustments", () => {
    const frame = frameAt(
      presentation([
        slide({
          background: { type: "color", color: "red" },
          style: { fontSize: 120, color: "gold", align: null },
        }),
      ]),
      0,
    );
    expect(frame?.theme.background).toEqual({ type: "color", color: "red" });
    expect(frame?.theme.text.fontSize).toBe(120);
    expect(frame?.theme.text.color).toBe("gold");
    expect(frame?.theme.text.align).toBe("center");
  });

  it("shows the reference on the first scripture slide only when asked", () => {
    const scripture = { columns: [], verseNumbers: null };
    const slides = [
      slide({ caption: "Jn 3:16", scripture }),
      slide({ caption: "Jn 3:17", scripture }),
    ];
    expect(frameAt(presentation(slides), 0)?.caption).toBe("Jn 3:16");
    expect(frameAt(presentation(slides), 1)?.caption).toBeNull();
  });

  it("returns null past the end", () => {
    expect(frameAt(presentation([]), 0)).toBeNull();
  });
});
