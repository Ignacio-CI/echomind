import { en, type TranslationKey } from "./en.js";
import { es } from "./es.js";

type Locale = "en" | "es";

const translations: Record<Locale, Record<TranslationKey, string>> = { en, es };

function getInitialLocale(): Locale {
  if (typeof localStorage !== "undefined") {
    const stored = localStorage.getItem("locale");
    if (stored === "en" || stored === "es") return stored;
  }
  if (typeof navigator !== "undefined" && navigator.language.startsWith("es")) return "es";
  return "en";
}

let locale = $state<Locale>(getInitialLocale());

export function getLocale() {
  return locale;
}

export function setLocale(next: Locale) {
  locale = next;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("locale", next);
  }
}

export function t(key: TranslationKey): string {
  return translations[locale][key];
}
