import { describe, expect, it } from "vitest";
import { uuidV7 } from "./api.ts";

describe("uuidV7", () => {
  it("is a well-formed version 7 UUID", () => {
    expect(uuidV7()).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
  });

  it("sorts by creation time", () => {
    const earlier = uuidV7(1_700_000_000_000);
    const later = uuidV7(1_700_000_000_001);
    expect(earlier < later).toBe(true);
  });

  it("encodes the timestamp in the first 48 bits", () => {
    expect(uuidV7(0x0123456789ab).startsWith("01234567-89ab-7")).toBe(true);
  });
});
