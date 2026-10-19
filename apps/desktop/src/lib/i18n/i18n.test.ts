import { describe, expect, it } from "vitest";
import { pick, translate } from "./i18n.svelte.ts";

// Tests use keys that always exist in en.json.
describe("translate", () => {
  it("falls back to English for lines not yet translated", () => {
    expect(pick("", "tw", "Cancel")).toEqual(["Cancel", "en"]);
    expect(pick({ one: "", other: " " }, "tw", "Cancel")).toEqual(["Cancel", "en"]);
    expect(pick("Gyae", "tw", "Cancel")).toEqual(["Gyae", "tw"]);
  });

  it("fills in values and picks the form for the number", () => {
    expect(translate("en", "common.slides", { count: 1 })).toBe("1 slide");
    expect(translate("en", "common.slides", { count: 4 })).toBe("4 slides");
  });
});

describe("French", () => {
  it("is complete, with its own plural forms", () => {
    expect(translate("fr", "common.cancel")).toBe("Annuler");
    // French uses the singular for 0 and 1.
    expect(translate("fr", "common.slides", { count: 0 })).toBe("0 diapositive");
    expect(translate("fr", "common.slides", { count: 2 })).toBe("2 diapositives");
  });
});
