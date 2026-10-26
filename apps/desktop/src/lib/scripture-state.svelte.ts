import type { BookOutline, Passage, ReferenceFeedback, Verse } from "@simpleworship/protocol";

const SETTINGS_KEY = "simpleworship.scripture";

/** The default smallest font size before scripture flows on to another
 *  slide. Matches `DEFAULT_MIN_FONT_SIZE` in `crates/sw-core/src/scripture.rs`. */
export const DEFAULT_MIN_FONT_SIZE = 48;

/**
 * A usable minimum font size from what was typed or remembered: a whole
 * number from 16 to 120, or the default for an empty or odd value (an
 * emptied number box gives `null`, which the core cannot read).
 */
export function validMinFontSize(value: unknown): number {
  const size = Math.round(Number(value));
  if (!Number.isFinite(size) || size <= 0) return DEFAULT_MIN_FONT_SIZE;
  return Math.max(16, Math.min(120, size));
}

export interface ScriptureSettings {
  primaryId: string | null;
  secondaryId: string | null;
  maxVerses: number;
  autoFlow: boolean;
  minFontSize: number;
}

function loadSettings(): ScriptureSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) {
      return {
        primaryId: null,
        secondaryId: null,
        maxVerses: 2,
        autoFlow: true,
        minFontSize: DEFAULT_MIN_FONT_SIZE,
      };
    }
    const parsed = JSON.parse(raw);
    return {
      primaryId: typeof parsed.primaryId === "string" ? parsed.primaryId : null,
      secondaryId: typeof parsed.secondaryId === "string" ? parsed.secondaryId : null,
      maxVerses: Math.max(1, Math.min(10, Number(parsed.maxVerses) || 2)),
      autoFlow: parsed.autoFlow !== undefined ? Boolean(parsed.autoFlow) : true,
      minFontSize: validMinFontSize(parsed.minFontSize),
    };
  } catch {
    return {
      primaryId: null,
      secondaryId: null,
      maxVerses: 2,
      autoFlow: true,
      minFontSize: DEFAULT_MIN_FONT_SIZE,
    };
  }
}

class ScriptureStateStore {
  primaryId: string | null = $state(null);
  secondaryId: string | null = $state(null);
  maxVerses: number = $state(2);
  autoFlow: boolean = $state(true);
  minFontSize: number = $state(DEFAULT_MIN_FONT_SIZE);
  verseNumbers: boolean | null = $state(null);

  query: string = $state("");
  feedback: ReferenceFeedback | null = $state(null);
  passage: Passage | null = $state(null);
  outline: BookOutline[] = $state.raw([]);
  chapterVerses: Verse[] = $state.raw([]);
  searchResults: Verse[] = $state.raw([]);
  ideas: {
    about: string;
    list: { reference: string; why: string; passage: Passage }[];
  } | null = $state(null);

  constructor() {
    const saved = loadSettings();
    this.primaryId = saved.primaryId;
    this.secondaryId = saved.secondaryId;
    this.maxVerses = saved.maxVerses;
    this.autoFlow = saved.autoFlow;
    this.minFontSize = saved.minFontSize;
  }

  saveSettings() {
    try {
      localStorage.setItem(
        SETTINGS_KEY,
        JSON.stringify({
          primaryId: this.primaryId,
          secondaryId: this.secondaryId,
          maxVerses: this.maxVerses,
          autoFlow: this.autoFlow,
          minFontSize: this.minFontSize,
        }),
      );
    } catch {
      // Ignore quota/storage errors
    }
  }
}

export const scriptureState = new ScriptureStateStore();
