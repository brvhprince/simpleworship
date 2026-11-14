// @vitest-environment happy-dom
// The stream's words play their entrance when they appear after nothing
// was showing or a different item goes live, not when a screen opens
// mid-song, and not from slide to slide.
import type { OutputState, RenderFrame } from "@simpleworship/protocol";
import { StreamView } from "@simpleworship/renderer";
import { flushSync, mount, unmount } from "svelte";
import { describe, expect, it } from "vitest";

/** A slide's words, as plainly as the renderer accepts: an empty canvas-editor slide. */
const frame = (text: string) =>
  ({
    text,
    elements: [],
    theme: { name: "Lower Third", background: { type: "color", color: "transparent" } },
  }) as unknown as RenderFrame;

function output(words: RenderFrame | null, entranceCue = 1): OutputState {
  return {
    frame: words,
    flags: { black: false, logo: false, hideText: false, hideBackground: false },
    transition: { kind: "cut", durationMs: 0 },
    video: null,
    logo: null,
    alert: null,
    lowerThird: null,
    stream: {
      frame: words,
      hideText: false,
      hideBackground: false,
      backdrop: "transparent",
      showAlerts: true,
      entrance: "rise",
      entranceCue,
    },
    revision: 0,
  } as unknown as OutputState;
}

const rising = () => document.querySelectorAll(".words.rise").length;

describe("the stream's entrance", () => {
  it("plays when words appear, not for the next slide", () => {
    const props = $state({ output: output(null) });
    const view = mount(StreamView, { target: document.body, props });
    flushSync();
    expect(rising()).toBe(0);

    props.output = output(frame("Amazing grace"));
    flushSync();
    expect(rising()).toBe(1);
    const first = document.querySelector(".words");

    props.output = output(frame("How sweet the sound"));
    flushSync();
    expect(document.querySelector(".words")).toBe(first); // the same run: no new entrance

    props.output = output(frame("Blessed assurance"), 2); // a different song goes live
    flushSync();
    expect(document.querySelector(".words")).not.toBe(first);
    expect(document.querySelector(".words:not(.rise)")).toBeNull();
    unmount(view);
  });

  it("shows words already up in place when the view opens", () => {
    const view = mount(StreamView, {
      target: document.body,
      props: { output: output(frame("Amazing grace")) },
    });
    flushSync();
    expect(rising()).toBe(0);
    unmount(view);
  });
});
