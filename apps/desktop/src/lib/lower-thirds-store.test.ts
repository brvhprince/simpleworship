// The lower-thirds store: a list loaded more than once, or around a save,
// must end up as the newest one, whatever order the answers arrive in.
import type { LowerThird } from "@simpleworship/protocol";
import { beforeEach, describe, expect, it, vi } from "vitest";

/** Answers to getLowerThirds, released by the test in any order. */
const pending: { resolve: (list: LowerThird[]) => void }[] = [];

/** Failures for the same loads, to make one fail instead. */
const failing: { reject: (error: Error) => void }[] = [];
/** What the next save does: succeed, or be refused. */
const saves = { refuse: false };

vi.mock("./api.ts", () => ({
  getLowerThirds: () =>
    new Promise<LowerThird[]>((resolve, reject) => {
      pending.push({ resolve });
      failing.push({ reject });
    }),
  saveLowerThirds: () =>
    saves.refuse ? Promise.reject(new Error("changed elsewhere")) : Promise.resolve(),
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
    icon: "none",
    rounding: 0,
    size: "medium",
    speed: "normal",
  },
  target: "both",
  hideAfterSeconds: null,
});
const ids = (list: LowerThird[]) => list.map((one) => one.id);

/** Releases the answer to load number `n` (0 = the first load). */
function answer(n: number, list: LowerThird[]): void {
  const load = pending[n];
  if (!load) throw new Error(`load ${n} was never started`);
  load.resolve(list);
}

describe("lower thirds store", () => {
  beforeEach(() => {
    pending.length = 0;
    failing.length = 0;
    saves.refuse = false;
    vi.resetModules();
  });

  it("keeps the newest load when an older answer arrives last", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    const first = lowerThirds.load();
    const second = lowerThirds.load();
    answer(1, [item("a"), item("b")]); // the newer answer first
    await second;
    answer(0, [item("a")]); // then the stale one
    await first;
    expect(ids(lowerThirds.list)).toEqual(["a", "b"]);
  });

  it("does not let a load that began before a save undo it", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    const firstLoad = lowerThirds.load();
    answer(0, [item("a")]);
    await firstLoad;

    const slowLoad = lowerThirds.load(); // still waiting…
    lowerThirds.create();
    if (lowerThirds.editing) lowerThirds.editing.title = "New";
    await lowerThirds.saveEditing(); // …when this saves [a, new-id]
    answer(1, [item("a")]);
    await slowLoad;
    expect(ids(lowerThirds.list)).toEqual(["a", "new-id"]);
  });

  it("allows no changes before the list has loaded", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    lowerThirds.create();
    expect(lowerThirds.editing).toBeNull();
  });

  it("keeps what a pack import added, with saving locked until the list is reloaded", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    const firstLoad = lowerThirds.load();
    answer(0, [item("a")]);
    await firstLoad;

    let finishImport = () => {};
    const importing = lowerThirds.whileWrittenElsewhere(
      () => new Promise<void>((resolve) => (finishImport = resolve)),
    );
    // While the import runs, the old list cannot be saved over it.
    lowerThirds.create();
    expect(lowerThirds.editing).toBeNull();
    expect(lowerThirds.busy).toBe(true);

    finishImport(); // the pack added "b"
    await vi.waitFor(() => expect(pending).toHaveLength(2));
    expect(lowerThirds.busy).toBe(true); // still locked until the reload arrives
    answer(1, [item("a"), item("b")]);
    await importing;
    expect(ids(lowerThirds.list)).toEqual(["a", "b"]);
    expect(lowerThirds.busy).toBe(false);
  });

  it("keeps saving off after an import if the reload fails", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    const firstLoad = lowerThirds.load();
    answer(0, [item("a")]);
    await firstLoad;

    const importing = lowerThirds.whileWrittenElsewhere(() => Promise.resolve());
    await vi.waitFor(() => expect(failing).toHaveLength(2));
    failing[1]?.reject(new Error("no connection"));
    expect((await importing).reloadFailed).toBe(true); // the import says so

    // The old list must not be saved over the imported one.
    expect(lowerThirds.ready).toBe(false);
    lowerThirds.create();
    expect(lowerThirds.editing).toBeNull();
  });

  it("keeps saving off after an import until the load that overtook its reload arrives", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    const firstLoad = lowerThirds.load();
    answer(0, [item("a")]);
    await firstLoad;

    const importing = lowerThirds.whileWrittenElsewhere(() => Promise.resolve());
    await vi.waitFor(() => expect(pending).toHaveLength(2));
    const newer = lowerThirds.load(); // the panel opened meanwhile
    answer(1, [item("a"), item("b")]); // the overtaken reload: not used
    expect((await importing).reloadFailed).toBe(false); // overtaken, not failed
    expect(lowerThirds.ready).toBe(false);

    answer(2, [item("a"), item("b")]);
    await newer;
    expect(lowerThirds.ready).toBe(true);
    expect(ids(lowerThirds.list)).toEqual(["a", "b"]);
  });

  it("reloads the list when a save is refused because it changed elsewhere", async () => {
    const { lowerThirds } = await import("./lower-thirds.svelte.ts");
    const firstLoad = lowerThirds.load();
    answer(0, [item("a")]);
    await firstLoad;

    saves.refuse = true;
    lowerThirds.create();
    if (lowerThirds.editing) lowerThirds.editing.title = "New";
    expect(await lowerThirds.saveEditing()).toBeNull();
    expect(pending).toHaveLength(2); // fetching the list as it is now
    answer(1, [item("a"), item("from-a-pack")]);
    await vi.waitFor(() => expect(ids(lowerThirds.list)).toEqual(["a", "from-a-pack"]));
  });
});
