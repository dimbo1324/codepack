// RU/EN switching without a restart (`docs/__arch__/ROADMAP.md` §3). `language` is a rune-backed
// reactive value: every component that calls `t(key)` re-renders the moment it
// changes, because Svelte 5 tracks the read of `language.current` inside `t`.
//
// `.svelte.ts` rather than `.ts`: runes (`$state`) are only usable in files the
// Svelte preprocessor compiles, and a plain module-level `let` would not be
// reactive across files.
import { en, type TranslationKey } from "./en";
import { ru } from "./ru";

const dictionaries = { en, ru } as const;

export type Language = keyof typeof dictionaries;

class LanguageState {
  current = $state<Language>("en");
}

export const language = new LanguageState();

if (import.meta.env.DEV) {
  const enKeys = Object.keys(en).sort();
  const ruKeys = Object.keys(ru).sort();
  const missingInRu = enKeys.filter((key) => !ruKeys.includes(key));
  const missingInEn = ruKeys.filter((key) => !enKeys.includes(key));
  if (missingInRu.length > 0 || missingInEn.length > 0) {
    throw new Error(
      `i18n dictionaries have drifted: missing in ru=[${missingInRu.join(", ")}], ` +
        `missing in en=[${missingInEn.join(", ")}]`,
    );
  }
}

/** Translates `key`, substituting any `{placeholder}` in `params`. */
export function t(key: TranslationKey, params?: Record<string, string | number>): string {
  const template = dictionaries[language.current][key];
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

export function setLanguage(next: Language): void {
  language.current = next;
  // Kept in sync with the document so assistive technology, hyphenation and the
  // browser's own text handling see the same language the user does.
  document.documentElement.lang = next;
}
