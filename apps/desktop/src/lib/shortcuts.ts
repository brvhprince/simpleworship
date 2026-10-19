/**
 * Keyboard shortcuts for the operator window.
 *
 * | Key                              | Action                               |
 * | -------------------------------- | ------------------------------------ |
 * | → ↓ Space PageDown               | Next live slide                      |
 * | ← ↑ PageUp                       | Previous live slide                  |
 * | Enter                            | Send preview live                    |
 * | F6 / F7 / F8 / F9                | Black / Logo / Clear text / Clear bg |
 * | V P C B T I E, optionally a digit | Jump to a section (V2, C, B…)        |
 *
 * Section letters: Verse, Pre-chorus, Chorus, Bridge, Tag, Intro, Ending.
 *
 * Shortcuts are ignored while typing in a text field, and when Ctrl, Alt
 * or Cmd is held (those combinations belong to the system and the editor).
 */

import type { Command, DisplayFlags, PaneState, SectionKind } from "@simpleworship/protocol";

/** The letters that start a section jump. */
const SECTION_LETTERS = new Set(["V", "P", "C", "B", "T", "I", "E"]);

/** How long to wait for a section number after a letter, in milliseconds. */
export const SECTION_NUMBER_DELAY_MS = 600;

/**
 * The command for a key that acts immediately (everything except section
 * jumps), or `null` if the key has no shortcut.
 */
export function commandForKey(key: string, flags: DisplayFlags): Command | null {
  switch (key) {
    case "ArrowRight":
    case "ArrowDown":
    case " ":
    case "PageDown":
      return { type: "nextSlide" };
    case "ArrowLeft":
    case "ArrowUp":
    case "PageUp":
      return { type: "previousSlide" };
    case "Enter":
      return { type: "goLive" };
    case "F6":
      return { type: "setBlack", on: !flags.black };
    case "F7":
      return { type: "setLogo", on: !flags.logo };
    case "F8":
      return { type: "setHideText", on: !flags.hideText };
    case "F9":
      return { type: "setHideBackground", on: !flags.hideBackground };
    default:
      return null;
  }
}

/**
 * Whether the live presentation has a *numbered* section starting with
 * `letter` (e.g. "Verse 2" for "V"). Only then is it worth waiting for a digit.
 */
export function hasNumberedSection(live: PaneState | null, letter: string): boolean {
  const slides = live?.presentation.slides ?? [];
  return slides.some(
    (slide) => slide.section?.number != null && sectionLetter(slide.section.kind) === letter,
  );
}

/** The shortcut letter of a section kind (mirrors `SectionKind::shortcut_letter` in Rust). */
function sectionLetter(kind: SectionKind): string | null {
  const letters: Record<SectionKind, string | null> = {
    verse: "V",
    preChorus: "P",
    chorus: "C",
    bridge: "B",
    tag: "T",
    intro: "I",
    ending: "E",
    other: null,
  };
  return letters[kind];
}

/**
 * Turns typed section shortcuts ("V", then "2") into `jumpToSection` commands.
 *
 * A letter alone jumps straight away when there is nothing to disambiguate.
 * When the live song has numbered sections of that kind, it waits briefly
 * for a digit, so typing "V2" goes directly to verse 2 instead of flashing
 * verse 1 on the screen first.
 */
export class SectionJumper {
  #pending: string | null = null;
  #timer: ReturnType<typeof setTimeout> | undefined;

  constructor(
    private readonly dispatch: (command: Command) => void,
    private readonly needsNumber: (letter: string) => boolean,
    private readonly delayMs: number = SECTION_NUMBER_DELAY_MS,
  ) {}

  /** Handles a key press. Returns `true` if the key was a section shortcut. */
  handleKey(key: string): boolean {
    const upper = key.toUpperCase();

    if (this.#pending && /^[0-9]$/.test(key)) {
      this.#jump(this.#pending + key);
      return true;
    }

    if (!SECTION_LETTERS.has(upper)) {
      return false;
    }

    this.#cancel();
    if (this.needsNumber(upper)) {
      this.#pending = upper;
      this.#timer = setTimeout(() => this.#jump(upper), this.delayMs);
    } else {
      this.#jump(upper);
    }
    return true;
  }

  #jump(shortcut: string): void {
    this.#cancel();
    this.dispatch({ type: "jumpToSection", shortcut });
  }

  #cancel(): void {
    clearTimeout(this.#timer);
    this.#pending = null;
  }
}

/** Whether a key press happened while the operator was typing text. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.isContentEditable ||
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    target.closest("dialog") !== null
  );
}
