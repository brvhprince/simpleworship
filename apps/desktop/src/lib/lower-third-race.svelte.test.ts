// @vitest-environment happy-dom
// Regression test: a lower third's timer reports the showing it was mounted
// for, even when the output hands its layer a replacement before it
// expires (as happens while the old one is still leaving).
import type { LowerThirdShown } from "@simpleworship/protocol";
import { LowerThirdLayer } from "@simpleworship/renderer";
import { flushSync, mount, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const showing = (showId: string, title: string, startedAt: number): LowerThirdShown => ({
  showId,
  startedAt,
  item: {
    id: `item-${showId}`,
    kind: "name",
    title,
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
    hideAfterSeconds: 5,
  },
});

describe("LowerThirdLayer's timer", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
  });
  afterEach(() => vi.useRealTimers());

  it("reports its own showing when a replacement arrives just before it expires", () => {
    const ended: string[] = [];
    const props = $state({
      shown: showing("a", "A", 1_000_000),
      onend: (id: string) => ended.push(id),
    });
    const layer = mount(LowerThirdLayer, { target: document.body, props });
    flushSync(); // runs onMount, which starts the timer

    // 4.9 s in, B replaces A while A's layer is still mounted (leaving).
    vi.advanceTimersByTime(4_900);
    props.shown = showing("b", "B", 1_004_900);
    flushSync();

    // A's five seconds are up: the timer must name A, never B.
    vi.advanceTimersByTime(200);
    expect(ended).toEqual(["a"]);
    unmount(layer);
  });

  it("does not report a showing without a time", () => {
    const ended: string[] = [];
    const untimed = showing("c", "C", 1_000_000);
    untimed.item.hideAfterSeconds = null;
    const layer = mount(LowerThirdLayer, {
      target: document.body,
      props: { shown: untimed, onend: (id: string) => ended.push(id) },
    });
    flushSync();
    vi.advanceTimersByTime(60_000);
    expect(ended).toEqual([]);
    unmount(layer);
  });
});
