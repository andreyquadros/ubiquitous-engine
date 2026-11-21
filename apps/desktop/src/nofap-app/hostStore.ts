// The standalone product's own state: its settings row (language, provider, model, updates, onboarding) and the
// theme. The module keeps its own store (`src/nofap/store.ts`); ubiqX's app store is never loaded here.

import { create } from 'zustand';
import { hasUrlLocaleOverride, setLocale } from '../i18n';
import { ipcErrorMessage } from '../lib/ipc';
import { applyTheme, initialTheme, type Theme } from '../lib/theme';
import { useNofapStore } from '../nofap/store';
import { hostIpc } from './hostIpc';
import type { HostSettings, HostSettingsPatch } from './types';

interface HostState {
  settings: HostSettings | null;
  error: string | null;
  theme: Theme;
  /** Reads the settings row and applies its language (an address with `?lang=` wins, for previews). */
  load: () => Promise<void>;
  /** Writes the fields given and applies the language. The module is read again: its AI readiness may have changed. */
  save: (patch: HostSettingsPatch) => Promise<HostSettings>;
  toggleTheme: () => void;
}

const applyLanguage = (settings: HostSettings): void => {
  if (!hasUrlLocaleOverride()) setLocale(settings.ui_language);
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
