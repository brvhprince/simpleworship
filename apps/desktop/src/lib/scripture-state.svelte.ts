import type { BookOutline, Passage, ReferenceFeedback, Verse } from "@simpleworship/protocol";

const SETTINGS_KEY = "simpleworship.scripture";

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
      return { primaryId: null, secondaryId: null, maxVerses: 2, autoFlow: true, minFontSize: 36 };
    }
    const parsed = JSON.parse(raw);
    return {
      primaryId: typeof parsed.primaryId === "string" ? parsed.primaryId : null,
      secondaryId: typeof parsed.secondaryId === "string" ? parsed.secondaryId : null,
      maxVerses: Math.max(1, Math.min(10, Number(parsed.maxVerses) || 2)),
      autoFlow: parsed.autoFlow !== undefined ? Boolean(parsed.autoFlow) : true,
      minFontSize: Math.max(12, Math.min(120, Number(parsed.minFontSize) || 36)),
    };
  } catch {
    return { primaryId: null, secondaryId: null, maxVerses: 2, autoFlow: true, minFontSize: 36 };
  }
}

class ScriptureStateStore {
  primaryId: string | null = $state(null);
  secondaryId: string | null = $state(null);
  maxVerses: number = $state(2);
  autoFlow: boolean = $state(true);
  minFontSize: number = $state(36);
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
