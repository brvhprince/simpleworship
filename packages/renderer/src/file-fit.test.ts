import type { Background } from "@simpleworship/protocol";
import { describe, expect, it } from "vitest";
import { backgroundFit, scalingFit } from "./file-fit.ts";

const media = { id: "photo", file: "photo.jpg" };
const picture = (extra: Partial<Extract<Background, { type: "image" }>> = {}): Background => ({
  type: "image",
  media,
  fit: "cover",
  opacity: 1,
  ...extra,
});

describe("backgroundFit", () => {
  it("uses the theme's fit for a file on automatic", () => {
    expect(backgroundFit(picture(), () => null)).toBe("cover");
  });

  it("lets the file's own fit from the engine win", () => {
    expect(backgroundFit(picture({ fileFit: "contain" }), () => "stretch")).toBe("contain");
  });

  it("falls back to the library's fit outside the engine", () => {
    expect(backgroundFit(picture(), (id) => (id === "photo" ? "stretch" : null))).toBe("stretch");
  });

  it("always covers with a colour", () => {
    expect(backgroundFit({ type: "color", color: "#000" }, () => "contain")).toBe("cover");
  });
});

describe("scalingFit", () => {
  it("maps each scaling to a fit", () => {
    expect(scalingFit("automatic")).toBeNull();
    expect(scalingFit("stretch")).toBe("stretch");
    expect(scalingFit("maintain")).toBe("contain");
    expect(scalingFit("zoom")).toBe("cover");
  });
});
