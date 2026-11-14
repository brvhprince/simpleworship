// The lower-thirds store: a list loaded more than once, or around a save,
// must end up as the newest one, whatever order the answers arrive in.
import type { LowerThird } from "@simpleworship/protocol";
import { beforeEach, describe, expect, it, vi } from "vitest";

/** Answers to getLowerThirds, released by the test in any order. */
const pending: { resolve: (list: LowerThird[]) => void }[] = [];

vi.mock("./api.ts", () => ({
  getLowerThirds: () => new Promise<LowerThird[]>((resolve) => pending.push({ resolve })),
  saveLowerThirds: () => Promise.resolve(),
  uuidV7: () => "new-id",
}));
vi.mock("./engine.svelte.ts", () => ({ engine: { showError: vi.fn(), dispatch: vi.fn() } }));

const item = (id: string): LowerThird => ({
  id,
  kind: "name",
  title: id,
  subtitle: "",
  look: {
    style: "bar",
    accent: "#4c6ef5",
    background: "#000",
    color: "#fff",
    fontFamily: "",
    side: "left",
  },
  target: "both",
  hideAfterSeconds: null,
});
const ids = (list: LowerThird[]) => list.map((one) => one.id);

describe("lower thirds store", () => {
  beforeEach(() => {
    pending.length = 0;
    vi.resetModules();
  });

  it("keeps the newest load when an older answer arrives last", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    const first = lowerThirds.load();
    const second = lowerThirds.load();
    pending[1].resolve([item("a"), item("b")]); // the newer answer first
    await second;
    pending[0].resolve([item("a")]); // then the stale one
    await first;
    expect(ids(lowerThirds.list)).toEqual(["a", "b"]);
  });

  it("does not let a load that began before a save undo it", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    const firstLoad = lowerThirds.load();
    pending[0].resolve([item("a")]);
    await firstLoad;

    const slowLoad = lowerThirds.load(); // still waiting…
    lowerThirds.create();
    if (lowerThirds.editing) lowerThirds.editing.title = "New";
    await lowerThirds.saveEditing(); // …when this saves [a, new-id]
    pending[1].resolve([item("a")]);
    await slowLoad;
    expect(ids(lowerThirds.list)).toEqual(["a", "new-id"]);
  });

  it("allows no changes before the list has loaded", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    lowerThirds.create();
    expect(lowerThirds.editing).toBeNull();
  });
});
