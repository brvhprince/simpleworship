import { describe, expect, it } from "vitest";
import { inline, parseNotes } from "./release-notes.ts";

describe("parseNotes", () => {
  it("reads a changelog section", () => {
    const notes = [
      "More than one computer.",
      "",
      "- **Join another computer:** a second computer can join",
      "  the one running the service:",
      "  - as a **second operator**;",
      "  - or as a mirror.",
      "",
      "  Pictures come from the main computer.",
      "- **Take over:** if the main computer stops answering.",
      "",
      "## Fixes",
      "",
      "Lines wrapped in the file",
      "join up again.",
    ].join("\n");

    expect(parseNotes(notes)).toEqual([
      { type: "paragraph", content: [{ text: "More than one computer." }] },
      {
        type: "list",
        items: [
          {
            content: [
              { text: "Join another computer:", bold: true },
              { text: " a second computer can join the one running the service:" },
            ],
            children: [
              {
                content: [
                  { text: "as a " },
                  { text: "second operator", bold: true },
                  { text: ";" },
                ],
                children: [],
              },
              { content: [{ text: "or as a mirror." }], children: [] },
            ],
            after: [{ text: "Pictures come from the main computer." }],
          },
          {
            content: [
              { text: "Take over:", bold: true },
              { text: " if the main computer stops answering." },
            ],
            children: [],
          },
        ],
      },
      { type: "heading", level: 2, content: [{ text: "Fixes" }] },
      { type: "paragraph", content: [{ text: "Lines wrapped in the file join up again." }] },
    ]);
  });

  it("keeps a sub-point's wrapped lines with it", () => {
    const blocks = parseNotes("- top\n  - a sub-point that\n    wraps\n");
    expect(blocks[0]).toEqual({
      type: "list",
      items: [
        {
          content: [{ text: "top" }],
          children: [{ content: [{ text: "a sub-point that wraps" }], children: [] }],
        },
      ],
    });
  });

  it("reads points written with stars and numbers", () => {
    const blocks = parseNotes("* one\n* two\n\n1. first");
    expect(blocks).toHaveLength(1);
    expect(blocks[0]).toMatchObject({ type: "list" });
    if (blocks[0]?.type === "list") expect(blocks[0].items).toHaveLength(3);
  });
});

describe("inline", () => {
  it("styles bold, italic and code, and shows links as words", () => {
    expect(
      inline("Press **H** or *double-click* `John 3:16`, see [the guide](https://x.org)."),
    ).toEqual([
      { text: "Press " },
      { text: "H", bold: true },
      { text: " or " },
      { text: "double-click", italic: true },
      { text: " " },
      { text: "John 3:16", code: true },
      { text: ", see the guide." },
    ]);
  });

  it("leaves a lone star alone", () => {
    expect(inline("5 * 3 = 15")).toEqual([{ text: "5 * 3 = 15" }]);
  });
});
