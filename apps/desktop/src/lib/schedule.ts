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
  }
}
