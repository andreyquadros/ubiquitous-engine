// The standalone product's own state: its settings row (language, provider, model, updates, onboarding) and the
// theme. The module keeps its own store (`src/nofap/store.ts`); ubiqX's app store is never loaded here.

import { create } from 'zustand';
import { hasUrlLocaleOverride, setLocale } from '../i18n';
import { ipcErrorMessage } from '../lib/ipc';
import { applyTheme, initialTheme, type Theme } from '../lib/theme';
import { useNofapStore } from '../nofap/store';
import { hostIpc } from './hostIpc';
import { localeOf, type HostSettings, type SettingsPatch } from './types';

interface HostState {
  settings: HostSettings | null;
  error: string | null;
  theme: Theme;
  /** Reads the settings row and applies its language once the welcome is done (an address with `?lang=` wins, for previews). */
  load: () => Promise<void>;
  /** Writes the fields given and applies the language. The module is read again: its AI readiness may have changed. */
  save: (patch: SettingsPatch) => Promise<HostSettings>;
  toggleTheme: () => void;
}

/**
 * The stored language wins once the welcome is done. Before that the row holds only the backend's default (`pt-BR`),
 * which nobody chose: the page keeps the language it started in (the system's, or the one picked last on this
 * computer), and the welcome's last button stores it.
 */
const applyLanguage = (settings: HostSettings): void => {
  if (!hasUrlLocaleOverride() && settings.onboarding_done) setLocale(localeOf(settings.ui_language));
};

export const useHostStore = create<HostState>((set, get) => ({
  settings: null,
  error: null,
  theme: initialTheme(),
  load: async () => {
    try {
      const settings = await hostIpc.settings();
      applyLanguage(settings);
      set({ settings, error: null });
    } catch (e) {
      set({ error: ipcErrorMessage(e) });
    }
  },
  save: async (patch) => {
    const settings = await hostIpc.saveSettings(patch);
    applyLanguage(settings);
    set({ settings });
    void useNofapStore.getState().load();
    return settings;
  },
  toggleTheme: () => {
    const theme: Theme = get().theme === 'dark' ? 'light' : 'dark';
    applyTheme(theme);
    set({ theme });
  },
}));
