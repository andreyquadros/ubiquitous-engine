import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';

import { setLocale } from '../../i18n';
import { __mock } from '../../lib/mock';
import { useAppStore } from '../../lib/store';
import type { FocusStatus, Platform } from '../../lib/types';
import { SessionCard } from './SessionCard';

const status = (ranShortcut: boolean): FocusStatus => ({
  session: {
    id: 'sess-1',
    task: 'Fechar o relatório',
    started_at: new Date(Date.now() - 12 * 60_000).toISOString(),
    ends_at: new Date(Date.now() + 33 * 60_000).toISOString(),
    ended_at: null,
    interventions: 2,
    hid_windows: false,
    ran_shortcut: ranShortcut,
  },
  remaining_secs: 33 * 60,
  targets_enabled: 3,
  interventions_today: 2,
  guard_enabled: true,
});

/** Loads the mock settings on `platform` (the card reads the OS from the settings view). */
async function renderOn(platform: Platform, ranShortcut = true, locale: 'pt-BR' | 'en' = 'pt-BR') {
  __mock.reset();
  __mock.setPlatform(platform);
  useAppStore.setState({ settingsView: null });
  await useAppStore.getState().loadSettings();
  setLocale(locale);
  render(<SessionCard status={status(ranShortcut)} defaultMinutes={45} onStart={() => Promise.resolve()} onStop={() => Promise.resolve()} />);
}

describe('SessionCard per platform', () => {
  beforeEach(() => setLocale('pt-BR'));

  it('names the macOS Shortcut on macOS', async () => {
    await renderOn('macos');
    expect(screen.getByText('Atalho do macOS executado')).toBeInTheDocument();
  });

  it('says a command ran on Windows and Linux', async () => {
    await renderOn('windows');
    expect(screen.getByText('Comando executado')).toBeInTheDocument();
    expect(screen.queryByText('Atalho do macOS executado')).not.toBeInTheDocument();
  });

  it('in English on Linux', async () => {
    await renderOn('linux', true, 'en');
    expect(screen.getByText('Command ran')).toBeInTheDocument();
  });

  it('shows nothing when no command ran', async () => {
    await renderOn('linux', false, 'en');
    expect(screen.queryByText('Command ran')).not.toBeInTheDocument();
    expect(screen.getByText('Fechar o relatório')).toBeInTheDocument();
  });
});
