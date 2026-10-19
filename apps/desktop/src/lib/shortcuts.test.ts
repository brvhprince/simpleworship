import type { Command, DisplayFlags } from "@simpleworship/protocol";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { commandForKey, SectionJumper } from "./shortcuts.ts";

const noFlags: DisplayFlags = { black: false, logo: false, hideText: false, hideBackground: false };

describe("commandForKey", () => {
  it("maps navigation keys", () => {
    expect(commandForKey("ArrowRight", noFlags)).toEqual({ type: "nextSlide" });
    expect(commandForKey(" ", noFlags)).toEqual({ type: "nextSlide" });
    expect(commandForKey("PageUp", noFlags)).toEqual({ type: "previousSlide" });
    expect(commandForKey("Enter", noFlags)).toEqual({ type: "goLive" });
  });

  it("toggles display flags relative to their current value", () => {
    expect(commandForKey("F6", noFlags)).toEqual({ type: "setBlack", on: true });
    expect(commandForKey("F6", { ...noFlags, black: true })).toEqual({
      type: "setBlack",
      on: false,
    });
  });

  it("ignores other keys", () => {
    expect(commandForKey("x", noFlags)).toBeNull();
  });
});

describe("SectionJumper", () => {
  let sent: Command[];

  beforeEach(() => {
    vi.useFakeTimers();
    sent = [];
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const jumper = (numbered: boolean) =>
    new SectionJumper(
      (command) => sent.push(command),
      () => numbered,
      500,
    );

  it("jumps immediately when the song has no numbered sections of that kind", () => {
    expect(jumper(false).handleKey("c")).toBe(true);
    expect(sent).toEqual([{ type: "jumpToSection", shortcut: "C" }]);
  });

  it("waits for a digit when numbered sections exist", () => {
    const j = jumper(true);
    j.handleKey("v");
    expect(sent).toEqual([]);

    j.handleKey("2");
    expect(sent).toEqual([{ type: "jumpToSection", shortcut: "V2" }]);

    vi.advanceTimersByTime(1000);
    expect(sent).toHaveLength(1);
  });

  it("falls back to the bare letter when no digit follows", () => {
    jumper(true).handleKey("v");
    vi.advanceTimersByTime(500);
    expect(sent).toEqual([{ type: "jumpToSection", shortcut: "V" }]);
  });

  it("ignores digits on their own and unrelated letters", () => {
    const j = jumper(true);
    expect(j.handleKey("3")).toBe(false);
    expect(j.handleKey("x")).toBe(false);
    expect(sent).toEqual([]);
  });
});
