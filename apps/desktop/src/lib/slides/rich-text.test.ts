// @vitest-environment happy-dom
import type { TextRun, TextStyle } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import {
  allHave,
  applyStyle,
  applyStyleToAll,
  normalize,
  plainText,
  runsFromDom,
  runsToHtml,
  styleAt,
} from "./rich-text.ts";

const plain = (text: string): TextRun => ({ text, bold: false, italic: false, underline: false });
const base = { fontSize: 80 } as TextStyle;

describe("applyStyle", () => {
  it("styles part of a run, splitting it", () => {
    const runs = applyStyle([plain("You are welcome to our Covenant Day")], 23, 35, {
      color: "gold",
    });
    expect(runs.map((run) => [run.text, run.color ?? null])).toEqual([
      ["You are welcome to our ", null],
      ["Covenant Day", "gold"],
    ]);
  });

  it("clears an override with null and merges what then matches", () => {
    const styled = applyStyle([plain("abcdef")], 2, 4, { color: "gold" });
    expect(applyStyle(styled, 0, 6, { color: null })).toEqual([plain("abcdef")]);
  });

  it("gives some words their own weight, and keeps it through the editable HTML", () => {
    const runs = applyStyle([plain("Call to Worship")], 8, 15, { fontWeight: 900, bold: false });
    expect(runs.map((run) => [run.text, run.fontWeight ?? null])).toEqual([
      ["Call to ", null],
      ["Worship", 900],
    ]);
    const root = document.createElement("div");
    root.innerHTML = runsToHtml(runs, base);
    expect(runsFromDom(root)).toEqual(runs);
  });

  it("keeps a styled empty run in an empty box, for typing", () => {
    expect(applyStyleToAll([plain("")], { bold: true })).toEqual([{ ...plain(""), bold: true }]);
  });
});

describe("styleAt and allHave", () => {
  const runs = [plain("ab"), { ...plain("cd"), bold: true }];

  it("reads the style before the caret", () => {
    expect(styleAt(runs, 3).bold).toBe(true);
    expect(styleAt(runs, 1).bold).toBe(false);
  });

  it("tells whether a whole range is bold", () => {
    expect(allHave(runs, 2, 4, "bold")).toBe(true);
    expect(allHave(runs, 1, 4, "bold")).toBe(false);
  });
});

describe("the editable HTML", () => {
  function roundTrip(html: string): TextRun[] {
    const root = document.createElement("div");
    root.innerHTML = html;
    return runsFromDom(root);
  }

  it("reads back what it wrote", () => {
    const runs = [plain("PRAYER "), { ...plain("1"), color: "gold", fontSize: 120 }];
    expect(roundTrip(runsToHtml(runs, base))).toEqual(runs);
  });

  it("reads line breaks from text, <br> and the <div>s browsers add", () => {
    expect(plainText(roundTrip("one<br>two"))).toBe("one\ntwo");
    expect(plainText(roundTrip("one<div>two</div>"))).toBe("one\ntwo");
    expect(plainText(roundTrip("one<div><br></div><div>three</div>"))).toBe("one\n\nthree");
    expect(plainText(roundTrip(runsToHtml([plain("end\n")], base)))).toBe("end\n");
  });

  it("styles typed text like the span it was typed into", () => {
    const runs = roundTrip(
      '<span data-run="{&quot;bold&quot;:true,&quot;italic&quot;:false,&quot;underline&quot;:false}">Hi</span> there',
    );
    expect(runs).toEqual([{ ...plain("Hi"), bold: true }, plain(" there")]);
  });
});

describe("normalize", () => {
  it("merges runs that look the same", () => {
    expect(normalize([plain("a"), plain(""), plain("b")])).toEqual([plain("ab")]);
  });
});
