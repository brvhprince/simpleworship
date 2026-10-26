import { describe, expect, it } from "vitest";
import { type MenuEntry, tidy } from "./context-menu.ts";

const item = (label: string): MenuEntry => ({ label, action: () => {} });
const labels = (entries: MenuEntry[]) =>
  entries.map((entry) => (entry === "separator" ? "—" : entry.label));

describe("tidy", () => {
  it("drops separators left at the ends or doubled when entries are left out", () => {
    const entries = [
      "separator",
      item("Edit"),
      "separator",
      "separator",
      item("Delete"),
      "separator",
    ];
    expect(labels(tidy(entries as MenuEntry[]))).toEqual(["Edit", "—", "Delete"]);
  });

  it("keeps a menu without separators as it is", () => {
    expect(labels(tidy([item("Play"), item("Add to Schedule")]))).toEqual([
      "Play",
      "Add to Schedule",
    ]);
  });
});
