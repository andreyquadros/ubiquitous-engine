// i18n runtime. Keys are '<namespace>.<key>'; '{name}' placeholders come from `vars`;
// `t('x', { count })` picks 'x_one' when count === 1, else 'x_other', when those keys exist.
import { useCallback } from 'react';
import type { Locale, Messages, NamespaceMessages, Vars } from './types';
import { getLocale, useLocale } from './locale';

import common from './messages/common';
import nav from './messages/nav';
import ui from './messages/ui';
import charts from './messages/charts';
import ubi from './messages/ubi';
import icons from './messages/icons';
import dashboard from './messages/dashboard';
import timeline from './messages/timeline';
import review from './messages/review';
import reports from './messages/reports';
import categories from './messages/categories';
import insights from './messages/insights';
import settings from './messages/settings';
import onboarding from './messages/onboarding';

export type { Locale, Messages, NamespaceMessages, Vars } from './types';
export { LOCALES, STORAGE_KEY, getLocale, setLocale, useLocale, useLocaleStore, isLocale, normaliseLocale, hasUrlLocaleOverride, initialLocale } from './locale';

/** Every namespace, keyed by its prefix. Exported for the parity test and tooling. */
export const NAMESPACES: Record<string, NamespaceMessages> = {
  common,
  nav,
  ui,
  charts,
  ubi,
  icons,
  dashboard,
  timeline,
  review,
  reports,
  categories,
  insights,
  settings,
  onboarding,
};

const REGISTRY: Record<Locale, Messages> = { 'pt-BR': {}, en: {} };
for (const [ns, messages] of Object.entries(NAMESPACES)) {
  for (const locale of Object.keys(messages) as Locale[]) {
    for (const [key, text] of Object.entries(messages[locale])) REGISTRY[locale][`${ns}.${key}`] = text;
  }
}

const warned = new Set<string>();
const isProd = (): boolean => {
  try {
    return Boolean(import.meta.env?.PROD);
  } catch {
    return false;
  }
};

const interpolate = (text: string, vars?: Vars): string => {
  if (!vars) return text;
  return text.replace(/\{(\w+)\}/g, (m, name: string) => (name in vars ? String(vars[name]) : m));
};

/** Looks a key up in one locale; `undefined` when missing. */
export const lookup = (locale: Locale, key: string): string | undefined => REGISTRY[locale]?.[key];

/** Translates `key` in `locale` (used by `t` and `useT`). */
export function translate(locale: Locale, key: string, vars?: Vars): string {
  const table = REGISTRY[locale] ?? REGISTRY['pt-BR'];
  let resolved = key;
  if (vars && typeof vars.count === 'number') {
    const form = vars.count === 1 ? `${key}_one` : `${key}_other`;
    if (form in table) resolved = form;
  }
  const text = table[resolved];
  if (text === undefined) {
    if (!isProd() && !warned.has(`${locale}:${key}`)) {
      warned.add(`${locale}:${key}`);
      console.warn(`[i18n] missing key "${key}" for locale "${locale}"`);
    }
    return key;
  }
  return interpolate(text, vars);
}

/** Non-React translation bound to the current locale (format.ts, toasts, module helpers). */
export const t = (key: string, vars?: Vars): string => translate(getLocale(), key, vars);

/** React hook: `const t = useT();` — re-renders the component when the locale changes. */
export function useT(): (key: string, vars?: Vars) => string {
  const locale = useLocale();
  return useCallback((key: string, vars?: Vars) => translate(locale, key, vars), [locale]);
}

/** True when the key exists in the given (or current) locale. */
export const hasKey = (key: string, locale: Locale = getLocale()): boolean => key in (REGISTRY[locale] ?? {});

/** Test hook: forget which keys already warned. */
export const __resetWarnings = (): void => warned.clear();
