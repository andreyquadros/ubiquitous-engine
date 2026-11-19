// How ubiqX hosts the module: the two routes of App.tsx. The module itself (`NofapPage`, `NofapCardPage`) knows
// nothing about ubiqX's settings; what it needs from the host is lent here.

import { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAppStore } from '../lib/store';
import { NofapCardPage } from './NofapCard';
import { NofapPage } from './NofapPage';
import type { NofapHost } from './NofapWizard';

/** `/nofap` inside the app shell. */
export function UbiqxNofapPage() {
  const navigate = useNavigate();
  const launchAtLogin = useAppStore((s) => s.settingsView?.settings.launch_at_login);
  const saveSettings = useAppStore((s) => s.saveSettings);
  const host = useMemo<NofapHost>(
    () => ({
      autostart: launchAtLogin === undefined ? undefined : { value: launchAtLogin, onChange: (value) => void saveSettings({ launch_at_login: value }) },
      openSettings: () => navigate('/settings', { state: { section: 'ia' } }),
    }),
    [launchAtLogin, saveSettings, navigate],
  );
  return <NofapPage host={host} />;
}

/** `/nofap-card`, the floating window: outside the gate and the shell, like the intervention panel. */
export function UbiqxNofapCard() {
  const settingsView = useAppStore((s) => s.settingsView);
  const loadSettings = useAppStore((s) => s.loadSettings);
  // The window follows the app's language (Settings.language, applied by loadSettings) unless `?lang=` overrides it.
  useEffect(() => {
    if (!settingsView) void loadSettings();
  }, [settingsView, loadSettings]);
  return <NofapCardPage />;
}
