import { ptBR } from "./i18n/pt-br";

export type Language = "en" | "pt-BR";
export const languages: { value: Language; label: string }[] = [
  { value: "en", label: "English" },
  { value: "pt-BR", label: "Português (Brasil)" },
];

const dictionaries: Partial<Record<Language, Record<string, string>>> = {
  "pt-BR": ptBR,
};

const state = $state({ language: "en" as Language });

/** Maps a BCP 47 tag (e.g. `navigator.language`) to a supported language. */
export function detectLanguage(tag: string): Language {
  return /^pt\b/i.test(tag) ? "pt-BR" : "en";
}

export function currentLanguage(): Language {
  return state.language;
}

export function setLanguage(language: Language) {
  state.language = language;
  if (typeof document !== "undefined")
    document.documentElement.lang = language;
}

/**
 * Translates an interface string. The English text is the lookup key;
 * untranslated strings fall back to it. `{name}` placeholders are replaced
 * from `params` after lookup, so translations can reorder them.
 */
export function t(
  text: string,
  params?: Record<string, string | number>,
): string {
  let result = dictionaries[state.language]?.[text] ?? text;
  if (params)
    result = result.replace(/\{(\w+)\}/g, (match, key) =>
      key in params ? String(params[key]) : match,
    );
  return result;
}
