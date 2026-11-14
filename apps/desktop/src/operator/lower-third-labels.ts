/**
 * The labels and examples for lower thirds, by kind and style, as message
 * keys (shared by the Lower Third panel and its editor).
 */

import type {
  LowerThirdIcon,
  LowerThirdKind,
  LowerThirdSize,
  LowerThirdSpeed,
  LowerThirdStyle,
} from "@simpleworship/protocol";
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

/** The icons, in the order the editor offers them. */
export const ICON_LABELS: Record<LowerThirdIcon, MessageKey> = {
  none: "lowerThird.icon.none",
  logo: "lowerThird.icon.logo",
  facebook: "lowerThird.icon.facebook",
  youtube: "lowerThird.icon.youtube",
  instagram: "lowerThird.icon.instagram",
  whatsapp: "lowerThird.icon.whatsapp",
  phone: "lowerThird.icon.phone",
  web: "lowerThird.icon.web",
  email: "lowerThird.icon.email",
  location: "lowerThird.icon.location",
  bible: "lowerThird.icon.bible",
  cross: "lowerThird.icon.cross",
};

export const SIZE_LABELS: Record<LowerThirdSize, MessageKey> = {
  small: "lowerThird.size.small",
  medium: "lowerThird.size.medium",
  large: "lowerThird.size.large",
};

export const SPEED_LABELS: Record<LowerThirdSpeed, MessageKey> = {
  slow: "lowerThird.speed.slow",
  normal: "lowerThird.speed.normal",
  fast: "lowerThird.speed.fast",
};
