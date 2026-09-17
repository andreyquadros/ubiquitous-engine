export type Theme = 'light' | 'dark';

const KEY = 'ubiqx-theme';

export const systemTheme = (): Theme =>
  typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';

export const initialTheme = (): Theme => {
  try {
    const q = new URLSearchParams(window.location.search).get('theme');
    if (q === 'dark' || q === 'light') return q;
    const stored = localStorage.getItem(KEY);
    if (stored === 'dark' || stored === 'light') return stored;
  } catch {
    /* ignore */
  }
  return systemTheme();
};

export const applyTheme = (t: Theme): void => {
  document.documentElement.classList.toggle('dark', t === 'dark');
  try {
    localStorage.setItem(KEY, t);
  } catch {
    /* ignore */
  }
};

/** Follows OS changes while the user has not chosen explicitly. */
export const watchSystemTheme = (onChange: (t: Theme) => void): (() => void) => {
  if (typeof window === 'undefined' || !window.matchMedia) return () => {};
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  const handler = (e: MediaQueryListEvent) => {
    let stored: string | null = null;
    try {
      stored = localStorage.getItem(KEY);
    } catch {
      /* ignore */
    }
    if (!stored) onChange(e.matches ? 'dark' : 'light');
  };
  mq.addEventListener?.('change', handler);
  return () => mq.removeEventListener?.('change', handler);
};
