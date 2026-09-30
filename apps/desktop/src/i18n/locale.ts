// Locale store: a tiny zustand store so React re-renders on change while plain modules (format.ts) can read it synchronously.
import { create } from 'zustand';
import type { Locale } from './types';

export const STORAGE_KEY = 'ubiqx.locale';

export const LOCALES: readonly { id: Locale; label: string }[] = [
  { id: 'pt-BR', label: 'Português (Brasil)' },
  { id: 'en', label: 'English' },
];

export const isLocale = (v: unknown): v is Locale => v === 'pt-BR' || v === 'en';

/** Maps any BCP-47-ish tag ('pt', 'pt_BR', 'en-GB', …) to a supported locale; unknown values fall back to pt-BR. */
export const normaliseLocale = (v: string | null | undefined): Locale => {
  const s = (v ?? '').trim().toLowerCase().replace('_', '-');
  if (s.startsWith('en')) return 'en';
  return 'pt-BR';
};

const readUrlLocale = (): Locale | null => {
  try {
    if (typeof window === 'undefined') return null;
    const q = new URLSearchParams(window.location.search).get('lang');
    return q ? normaliseLocale(q) : null;
  } catch {
    return null;
  }
};

/** True when `?lang=` is present: a dev/screenshot override that wins over settings. */
export const hasUrlLocaleOverride = (): boolean => readUrlLocale() !== null;

/** URL `?lang=` > localStorage > navigator.language > pt-BR. Settings.language is applied later by the app store. */
export const initialLocale = (): Locale => {
  const fromUrl = readUrlLocale();
  if (fromUrl) return fromUrl;
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isLocale(stored)) return stored;
  } catch {
    /* ignore */
  }
  try {
    if (typeof navigator !== 'undefined' && navigator.language) return normaliseLocale(navigator.language);
  } catch {
    /* ignore */
  }
  return 'pt-BR';
};

interface LocaleState {
  locale: Locale;
}

export const useLocaleStore = create<LocaleState>(() => ({ locale: initialLocale() }));

export const getLocale = (): Locale => useLocaleStore.getState().locale;

/** Updates the store, `<html lang>` and the persisted preference. */
export const setLocale = (l: Locale): void => {
  const next = isLocale(l) ? l : normaliseLocale(l);
  if (useLocaleStore.getState().locale !== next) useLocaleStore.setState({ locale: next });
  if (typeof document !== 'undefined') document.documentElement.lang = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    /* ignore */
  }
};

/** React hook: the current locale; the component re-renders when it changes. */
export const useLocale = (): Locale => useLocaleStore((s) => s.locale);

if (typeof document !== 'undefined') document.documentElement.lang = getLocale();
