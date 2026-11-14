import type { LowerThirdShown, LowerThirdTarget } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { lowerThirdFor } from "./lower-thirds.ts";

const shown = (target: LowerThirdTarget): LowerThirdShown => ({
  showId: "0192f0e4-0000-7000-8000-0000000000aa",
  item: {
    id: "0192f0e4-0000-7000-8000-000000000001",
    kind: "name",
    title: "Pastor Kwame",
    subtitle: "Senior Pastor",
    look: {
      style: "bar",
      accent: "#4c6ef5",
      background: "#000",
      color: "#fff",
      fontFamily: "",
      side: "left",
    },
    target,
    hideAfterSeconds: null,
  },
  startedAt: 0,
});

describe("lowerThirdFor", () => {
  it("shows each lower third only on the outputs it is for", () => {
    expect(lowerThirdFor(shown("both"), "screens")).not.toBeNull();
    expect(lowerThirdFor(shown("both"), "stream")).not.toBeNull();
    expect(lowerThirdFor(shown("screens"), "stream")).toBeNull();
    expect(lowerThirdFor(shown("stream"), "screens")).toBeNull();
    expect(lowerThirdFor(shown("stream"), "stream")).not.toBeNull();
  });

  it("shows every one in the operator's live pane, and nothing when none is up", () => {
    expect(lowerThirdFor(shown("stream"), "all")).not.toBeNull();
    expect(lowerThirdFor(null, "all")).toBeNull();
  });
});
