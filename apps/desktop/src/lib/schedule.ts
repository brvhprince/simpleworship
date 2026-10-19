/**
 * Small helpers for showing schedule items. Mirror the Rust methods on
 * `ScheduleItem` in `crates/sw-core/src/schedule.rs`.
 */

import type { ScheduleItem } from "@simpleworship/protocol";

/** The title shown in the schedule list. */
export function itemTitle(item: ScheduleItem): string {
  switch (item.content.type) {
    case "song":
      return item.content.song.title || "Untitled song";
    case "text":
      return item.content.title || "Text";
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
      return item.content.deck.name || "Presentation";
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
      return "Song";
    case "text":
      return "Text";
    case "scripture":
      return "Bible";
    case "media":
      return item.content.media.kind === "video" ? "Video" : "Image";
    case "countdown":
      return "Timer";
    case "slides":
      return "Slides";
    case "playlist":
      return "Music";
    case "deck":
      return `Slides · ${item.content.deck.slides.length}`;
  }
}
