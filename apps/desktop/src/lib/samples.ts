/**
 * Sample slides for previewing a theme: a lyric slide and a scripture
 * slide, drawn with whatever theme is being shown.
 */

import type { RenderFrame, Theme } from "@simpleworship/protocol";

/** A two-line lyric with a footer, like a typical song slide. */
export function sampleLyrics(theme: Theme): RenderFrame {
  return {
    text: "Amazing grace, how sweet the sound\nThat saved a wretch like me",
    scripture: null,
    caption: null,
    rich: null,
    elements: null,
    countdown: null,
    footer: "John Newton · Public Domain",
    theme,
  };
}

/** A verse with its number and reference, like a scripture slide. */
export function sampleScripture(theme: Theme): RenderFrame {
  const text =
    "For God so loved the world, that he gave his only begotten Son, that whosoever " +
    "believeth in him should not perish, but have everlasting life.";
  return {
    text: `16 ${text}`,
    scripture: {
      columns: [{ translation: "KJV", verses: [{ number: 16, text, continued: false }] }],
      // As the theme says, so the designer shows its verse-number setting.
      verseNumbers: null,
    },
    caption: "John 3:16 (KJV)",
    rich: null,
    elements: null,
    countdown: null,
    footer: "King James Version · Public Domain",
    theme,
  };
}

/** A sample slide for presentations (no song footer or scripture reference). */
export function samplePresentation(theme: Theme): RenderFrame {
  return {
    text: "Welcome to SimpleWorship\nLet everything that has breath praise the Lord",
    scripture: null,
    caption: null,
    rich: null,
    elements: null,
    countdown: null,
    footer: null,
    theme,
  };
}

/** A sample slide for plain text announcements. */
export function sampleText(theme: Theme): RenderFrame {
  return {
    text: "Sunday Worship Service\nEvery Sunday at 10:00 AM",
    scripture: null,
    caption: null,
    rich: null,
    elements: null,
    countdown: null,
    footer: null,
    theme,
  };
}
