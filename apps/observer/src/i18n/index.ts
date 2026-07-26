/**
 * Observer localisation.
 *
 * Locale is presentation only. Switching it renegotiates the observer locale with the
 * protocol handler and changes nothing about simulation state or its digests
 * (INV-006, INV-007). No label in these dictionaries carries simulation meaning.
 *
 * The English dictionary is the baseline: `Copy` is derived from it, so every other locale is
 * checked against it by the compiler, and an unrecognised tag resolves to it rather than
 * showing a key or an empty string.
 *
 * The chosen locale is remembered in `localStorage`; on a first run the observer reads the
 * browser's language preferences instead. Neither path can reach authoritative state.
 */

import type { ObserverLocale } from "../observer/format";
import { de } from "./de";
import type { Copy } from "./dictionary";
import { en } from "./en";
import { es } from "./es";
import { ru } from "./ru";
import { zhHans } from "./zh-Hans";

export type { Copy } from "./dictionary";

/** Display order in the language switcher. English leads because it is the fallback. */
export const LOCALES: readonly ObserverLocale[] = ["en-US", "ru-RU", "zh-Hans", "de-DE", "es-ES"];

export const FALLBACK_LOCALE: ObserverLocale = "en-US";

/**
 * How each locale names itself. A language switcher that labels languages in the language the
 * reader has not chosen yet is useless to the reader who needs it most, so these are endonyms
 * and are never translated.
 */
export const LOCALE_NAMES: Record<ObserverLocale, string> = {
  "en-US": "English",
  "ru-RU": "Русский",
  "zh-Hans": "简体中文",
  "de-DE": "Deutsch",
  "es-ES": "Español",
};

/** The two-letter mark shown on the compact switcher, mono-spaced and equal width. */
export const LOCALE_MARKS: Record<ObserverLocale, string> = {
  "en-US": "EN",
  "ru-RU": "RU",
  "zh-Hans": "中",
  "de-DE": "DE",
  "es-ES": "ES",
};

const dictionaries: Record<ObserverLocale, Copy> = {
  "en-US": en,
  "ru-RU": ru,
  "zh-Hans": zhHans,
  "de-DE": de,
  "es-ES": es,
};

const STORAGE_KEY = "causafera-observer-locale";

export function copyFor(locale: ObserverLocale): Copy {
  return dictionaries[locale] ?? dictionaries[FALLBACK_LOCALE];
}

export function isObserverLocale(value: string): value is ObserverLocale {
  return Object.prototype.hasOwnProperty.call(dictionaries, value);
}

/**
 * Resolve any BCP-47-ish tag onto a supported locale.
 *
 * Matching is by primary subtag, with the script subtag deciding for Chinese: `zh`, `zh-CN`
 * and `zh-Hans-CN` all resolve to `zh-Hans`, while a traditional-script tag does not — the
 * observer has no traditional dictionary and claiming otherwise would be a lie about coverage.
 */
export function normaliseLocale(tag: string): ObserverLocale | undefined {
  const clean = tag.trim().toLowerCase().replace(/_/g, "-");
  if (clean.length === 0) return undefined;
  if (isObserverLocale(tag)) return tag;
  const [primary, ...rest] = clean.split("-");
  switch (primary) {
    case "en":
      return "en-US";
    case "ru":
      return "ru-RU";
    case "de":
      return "de-DE";
    case "es":
      return "es-ES";
    case "zh":
      // Traditional script is not supported; only simplified tags resolve.
      if (rest.includes("hant") || rest.includes("tw") || rest.includes("hk") || rest.includes("mo")) {
        return undefined;
      }
      return "zh-Hans";
    default:
      return undefined;
  }
}

/**
 * The locale the observer opens in: a remembered choice first, then the browser's ordered
 * language preferences, then English. Storage access is guarded because a sandboxed or
 * private-mode window may refuse it, and a refused preference must not break the instrument.
 */
export function initialLocale(): ObserverLocale {
  if (typeof window === "undefined") return FALLBACK_LOCALE;
  const remembered = readStoredLocale();
  if (remembered !== undefined) return remembered;
  const preferences = window.navigator.languages ?? [window.navigator.language];
  for (const tag of preferences) {
    if (typeof tag !== "string") continue;
    const matched = normaliseLocale(tag);
    if (matched !== undefined) return matched;
  }
  return FALLBACK_LOCALE;
}

/** Remember an explicit choice. A storage failure is silent: the session still switches. */
export function rememberLocale(locale: ObserverLocale): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // Private mode or a storage quota. The choice still applies to this session.
  }
}

function readStoredLocale(): ObserverLocale | undefined {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    return stored === null ? undefined : normaliseLocale(stored);
  } catch {
    return undefined;
  }
}
