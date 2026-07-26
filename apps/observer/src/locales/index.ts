import { en } from "./en";
import { ru } from "./ru";
import { zhHans } from "./zh-Hans";
import { de } from "./de";
import { es } from "./es";
import type { LocaleCopy } from "./types";

export type ObserverLocale = "en" | "ru" | "zh-Hans" | "de" | "es";

export const SUPPORTED_LOCALES: { id: ObserverLocale; nativeName: string }[] = [
  { id: "en", nativeName: "English" },
  { id: "ru", nativeName: "Русский" },
  { id: "zh-Hans", nativeName: "简体中文" },
  { id: "de", nativeName: "Deutsch" },
  { id: "es", nativeName: "Español" },
];

const dictionaries: Record<ObserverLocale, LocaleCopy> = {
  en,
  ru,
  "zh-Hans": zhHans,
  de,
  es,
};

const STORAGE_KEY = "causafera-observer-locale";

export function getInitialLocale(): ObserverLocale {
  if (typeof window !== "undefined") {
    const saved = window.localStorage.getItem(STORAGE_KEY);
    if (saved) {
      const normalized = normalizeLocale(saved);
      if (normalized !== undefined) return normalized;
    }
    const navLanguages = window.navigator.languages ?? [window.navigator.language];
    for (const lang of navLanguages) {
      if (typeof lang === "string") {
        const normalized = normalizeLocale(lang);
        if (normalized) return normalized;
      }
    }
  }
  return "en";
}

export function saveLocalePreference(locale: ObserverLocale): void {
  if (typeof window !== "undefined") {
    try {
      window.localStorage.setItem(STORAGE_KEY, locale);
    } catch {
      // Ignore localStorage write failures in sandboxed or private mode
    }
  }
}

export function copyFor(locale: string | undefined): LocaleCopy {
  if (locale && isSupportedLocale(locale)) {
    const normalized = normalizeLocale(locale);
    if (normalized && dictionaries[normalized]) {
      return dictionaries[normalized];
    }
  }
  return dictionaries.en;
}

function isSupportedLocale(input: string): boolean {
  return normalizeLocale(input) !== undefined;
}

export function normalizeLocale(input: string): ObserverLocale | undefined {
  const clean = input.trim().toLowerCase();
  if (clean === "en" || clean.startsWith("en-") || clean === "en-us" || clean === "en-gb") {
    return "en";
  }
  if (clean === "ru" || clean.startsWith("ru-") || clean === "ru-ru") {
    return "ru";
  }
  if (clean === "zh" || clean === "zh-hans" || clean.startsWith("zh-") || clean === "zh-cn" || clean === "zh-sg") {
    return "zh-Hans";
  }
  if (clean === "de" || clean.startsWith("de-") || clean === "de-de") {
    return "de";
  }
  if (clean === "es" || clean.startsWith("es-") || clean === "es-es" || clean === "es-mx") {
    return "es";
  }
  return undefined;
}
