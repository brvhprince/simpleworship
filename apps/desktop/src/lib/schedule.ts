/**
 * Small helpers for showing schedule items. Mirror the Rust methods on
 * `ScheduleItem` in `crates/sw-core/src/schedule.rs`.
 */

import type { ScheduleItem } from "@simpleworship/protocol";
import { t } from "./i18n/i18n.svelte.ts";

/** The title shown in the schedule list. */
export function itemTitle(item: ScheduleItem): string {
  switch (item.content.type) {
    case "song":
      return item.content.song.title || t("kind.untitledSong");
    case "text":
      return item.content.title || t("kind.text");
    case "scripture": {
      const abbreviations = item.content.translations.map((t) => t.abbreviation).join(" · ");
      return `${item.content.reference} (${abbreviations})`;
    }
    case "media":
      return item.content.media.name;
    case "countdown":
      return item.content.title;
    case "slides":
    case "playlist":
      return item.content.name;
    case "deck":
      return item.content.deck.name || t("kind.presentation");
  }
}

/**
 * Whether pictures and slides can be dropped onto the item to add slides
 * to it (mirrors `ScheduleItem::as_deck` in Rust).
 */
export function canHoldSlides(item: ScheduleItem): boolean {
  switch (item.content.type) {
    case "deck":
    case "slides":
    case "text":
      return true;
    case "media":
      return item.content.media.kind !== "audio";
    default:
      return false;
  }
}

/** A short label for the kind of item, e.g. "Song". */
export function itemKind(item: ScheduleItem): string {
  switch (item.content.type) {
    case "song":
      return t("kind.song");
    case "text":
      return t("kind.text");
    case "scripture":
      return t("kind.bible");
    case "media":
      return item.content.media.kind === "video" ? t("kind.video") : t("kind.image");
    case "countdown":
      return t("kind.timer");
    case "slides":
      return t("kind.slides");
    case "playlist":
      return t("kind.music");
    case "deck":
      return t("kind.deckSlides", { count: item.content.deck.slides.length });
  }
}
