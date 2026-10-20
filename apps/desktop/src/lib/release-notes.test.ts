import { describe, expect, it } from "vitest";
import { plainText } from "./release-notes.ts";

describe("plainText", () => {
  it("removes Markdown but keeps the points", () => {
    const notes = [
      "Phones and cameras.",
      "",
      "- **Phone remote:** scan the `QR` code.",
      "- See [the guide](https://example.org/guide) for more.",
    ].join("\n");
    expect(plainText(notes)).toBe(
      [
        "Phones and cameras.",
        "",
        "- Phone remote: scan the QR code.",
        "- See the guide for more.",
      ].join("\n"),
    );
  });
});
