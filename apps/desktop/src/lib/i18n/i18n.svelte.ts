/**
 * Translations of SimpleWorship's interface.
 *
 * Every piece of text the operator reads comes from `t("key")`, with the
 * words in `messages/<language>.json`. English (`en.json`) is complete;
 * other languages may be partly translated: a line that is missing or
 * empty there shows in English, so a half-done translation is still
 * usable. See `messages/README.md` for translators.
 *
 * - `t("schedule.items", { count: 3 })` fills in `{count}`, and picks the
 *   right form for the number when the message has `one` / `other` forms.
 * - The language follows the computer's unless chosen in Settings. It is
 *   remembered on this computer, and changing it updates every window at
 *   once (the text is reactive), including the menus.
 */

import en from "./messages/en.json";
import fr from "./messages/fr.json";
import tw from "./messages/tw.json";

/** A message: plain text, or different text for one and for many. */
type Message = string | { one?: string; other: string };

/** Every message key (English has them all). */
export type MessageKey = keyof typeof en;

/** Values to fill in, e.g. `{ count: 3, name: "Amazing Grace" }`. */
export type MessageValues = Record<string, string | number>;

/** The languages SimpleWorship has, with their names in that language. */
export const LANGUAGES = [
  { code: "en", name: "English" },
  { code: "fr", name: "Français" },
  { code: "tw", name: "Twi" },
] as const;

export type Language = (typeof LANGUAGES)[number]["code"];

const catalogs: Record<Language, Partial<Record<string, Message>>> = { en, fr, tw };

/**
 * The locale used for plural rules: Twi is a form of Akan, which the
 * system's plural rules know as "ak".
 */
const PLURAL_LOCALES: Record<Language, string> = { en: "en", fr: "fr", tw: "ak" };

/** Remembered on this computer (shared by all of its windows). */
const STORAGE_KEY = "simpleworship.language";

function isLanguage(code: string | null | undefined): code is Language {
  return LANGUAGES.some((language) => language.code === code);
}

/** The computer's language, if SimpleWorship has it. */
function systemLanguage(): Language {
  for (const preferred of navigator.languages ?? [navigator.language]) {
    const code = preferred.toLowerCase().split("-")[0];
    if (code === "ak" || code === "tw") return "tw";
    if (isLanguage(code)) return code;
  }
  return "en";
}

function storedLanguage(): Language | null {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return isLanguage(stored) ? stored : null;
  } catch {
    return null;
  }
}

class I18n {
  /** The language chosen in Settings, or `null` to follow the computer. */
  chosen: Language | null = $state(storedLanguage());

  /** The language the interface is in. */
  get language(): Language {
    return this.chosen ?? systemLanguage();
  }

  /** Chooses a language (`null` follows the computer), here and in every window. */
  choose(language: Language | null): void {
    this.chosen = language;
    try {
      if (language) localStorage.setItem(STORAGE_KEY, language);
      else localStorage.removeItem(STORAGE_KEY);
    } catch {
      // Not remembered, but used for now.
    }
    document.documentElement.lang = this.language;
  }

  /** Follows a choice made in another window. */
  listen(): () => void {
    const follow = (event: StorageEvent) => {
      if (event.key === STORAGE_KEY) this.chosen = storedLanguage();
    };
    window.addEventListener("storage", follow);
    document.documentElement.lang = this.language;
    return () => window.removeEventListener("storage", follow);
  }

  /** How much of a language is translated, from 0 to 1. */
  coverage(language: Language): number {
    const keys = Object.keys(en);
    const done = keys.filter((key) => hasText(catalogs[language][key])).length;
    return keys.length === 0 ? 1 : done / keys.length;
  }
}

export const i18n = new I18n();

function hasText(message: Message | undefined): message is Message {
  if (message === undefined) return false;
  return typeof message === "string" ? message.trim() !== "" : message.other.trim() !== "";
}

/** The text for `key` in the interface language (or English), with `values` filled in. */
export function t(key: MessageKey, values?: MessageValues): string {
  return translate(i18n.language, key, values);
}

/** Every menu label in the interface language, for the native menu bar. */
export function menuLabels(): Record<string, string> {
  return Object.fromEntries(
    Object.keys(en)
      .filter((key) => key.startsWith("menu."))
      .map((key) => [key, t(key as MessageKey)]),
  );
}

/** "3 songs, 1 Bible and 40 pictures": a list joined the way the language does. */
export function listOf(parts: string[]): string {
  return new Intl.ListFormat(i18n.language, { type: "conjunction" }).format(parts);
}

/** Like {@link t}, for a given language (used by tests and the menus). */
export function translate(language: Language, key: MessageKey, values?: MessageValues): string {
  const own = catalogs[language][key];
  const message: Message | undefined = hasText(own)
    ? own
    : (catalogs.en[key] as Message | undefined);
  if (message === undefined) return key;
  const usedLanguage = hasText(own) ? language : "en";
  return fill(choose(message, usedLanguage, values), values);
}

/** Picks the form of a message for the number in `values.count`. */
function choose(message: Message, language: Language, values?: MessageValues): string {
  if (typeof message === "string") return message;
  const count = Number(values?.count ?? 0);
  const form = new Intl.PluralRules(PLURAL_LOCALES[language]).select(count);
  return (form === "one" ? message.one : undefined) ?? message.other;
}

/** Replaces `{name}` with `values.name`. */
function fill(text: string, values?: MessageValues): string {
  if (!values) return text;
  return text.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in values ? String(values[name]) : whole,
  );
}
