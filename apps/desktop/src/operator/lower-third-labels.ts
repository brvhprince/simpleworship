/**
 * The labels and examples for lower thirds, by kind and style, as message
 * keys (shared by the Lower Third panel and its editor).
 */

import type { LowerThirdKind, LowerThirdStyle } from "@simpleworship/protocol";
import type { MessageKey } from "../lib/i18n/i18n.svelte.ts";

export const KIND_LABELS: Record<LowerThirdKind, MessageKey> = {
  name: "lowerThird.kind.name",
  sermon: "lowerThird.kind.sermon",
  scripture: "lowerThird.kind.scripture",
  offering: "lowerThird.kind.offering",
  social: "lowerThird.kind.social",
  announcement: "lowerThird.kind.announcement",
};

/** Example first and second lines, shown as placeholders. */
export const KIND_EXAMPLES: Record<LowerThirdKind, [MessageKey, MessageKey]> = {
  name: ["lowerThird.example.nameTitle", "lowerThird.example.nameSubtitle"],
  sermon: ["lowerThird.example.sermonTitle", "lowerThird.example.sermonSubtitle"],
  scripture: ["lowerThird.example.scriptureTitle", "lowerThird.example.scriptureSubtitle"],
  offering: ["lowerThird.example.offeringTitle", "lowerThird.example.offeringSubtitle"],
  social: ["lowerThird.example.socialTitle", "lowerThird.example.socialSubtitle"],
  announcement: ["lowerThird.example.announcementTitle", "lowerThird.example.announcementSubtitle"],
};

export const STYLE_LABELS: Record<LowerThirdStyle, MessageKey> = {
  bar: "lowerThird.style.bar",
  wipe: "lowerThird.style.wipe",
  underline: "lowerThird.style.underline",
  rise: "lowerThird.style.rise",
  block: "lowerThird.style.block",
  strap: "lowerThird.style.strap",
};
