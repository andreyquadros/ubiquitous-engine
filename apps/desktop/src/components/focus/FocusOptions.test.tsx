import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { setLocale } from '../../i18n';
import { __mock } from '../../lib/mock';
import { useAppStore } from '../../lib/store';
import type { Platform } from '../../lib/types';
import { FocusOptions, shortcutCopy } from './FocusOptions';

/** Loads the mock settings on `platform`; the locale is set afterwards because loadSettings applies Settings.language (pt-BR). */
async function renderOn(platform: Platform, locale: 'pt-BR' | 'en' = 'pt-BR') {
  __mock.reset();
  __mock.setPlatform(platform);
  useAppStore.setState({ settingsView: null });
  await useAppStore.getState().loadSettings();
  setLocale(locale);
  const focus = useAppStore.getState().settingsView!.settings.focus;
  const onPatch = vi.fn();
  render(<FocusOptions focus={focus} onPatch={onPatch} onTest={() => Promise.resolve()} />);
  return onPatch;
}

describe('FocusOptions per platform', () => {
  beforeEach(() => setLocale('pt-BR'));

  it('on macOS the two hooks are Shortcut names and hiding windows mentions System Events', async () => {
    await renderOn('macos');
    expect(screen.getByLabelText('Atalho ao começar a sessão')).toHaveAttribute('placeholder', 'Nome do atalho');
    expect(screen.getByLabelText('Atalho ao encerrar a sessão')).toBeInTheDocument();
    expect(screen.getByText(/Crie no app Atalhos/)).toBeInTheDocument();
    expect(screen.getByText('Na primeira vez o macOS pede permissão para o app controlar o System Events.')).toBeInTheDocument();
    expect(shortcutCopy('macos').hint).toBe('options.shortcut_hint');
  });

  it('on Windows the same settings take a command line and the hint says the windows are minimised', async () => {
    const onPatch = await renderOn('windows');
    expect(screen.queryByLabelText('Atalho ao começar a sessão')).not.toBeInTheDocument();
    const on = screen.getByLabelText('Comando ao começar a sessão');
    expect(on).toHaveAttribute('placeholder', 'Linha de comando');
    expect(screen.getByLabelText('Comando ao encerrar a sessão')).toBeInTheDocument();
    expect(screen.getByText(/Qualquer linha de comando funciona \(cmd \/C no Windows, sh -c no Linux\)/)).toBeInTheDocument();
    expect(screen.getByText('No Windows e no Linux as outras janelas são minimizadas; nada pede permissão.')).toBeInTheDocument();
    // still stored in the same setting the engine runs
    fireEvent.change(on, { target: { value: 'powershell -c "Set-Focus on"' } });
    fireEvent.blur(on);
    expect(onPatch).toHaveBeenCalledWith({ macos_focus_shortcut_on: 'powershell -c "Set-Focus on"' });
  });

  it('on Linux, in English', async () => {
    await renderOn('linux', 'en');
    expect(screen.getByLabelText('Command when a session starts')).toBeInTheDocument();
    expect(screen.getByLabelText('Command when a session ends')).toBeInTheDocument();
    expect(screen.getByText('On Windows and Linux the other windows are minimised; nothing asks for permission.')).toBeInTheDocument();
    expect(shortcutCopy('linux')).toEqual({ on: 'options.command_on', off: 'options.command_off', placeholder: 'options.command_placeholder', hint: 'options.command_hint' });
  });
});
