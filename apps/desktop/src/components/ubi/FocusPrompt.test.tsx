import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '../../i18n';
import { __mock } from '../../lib/mock';
import { useAppStore } from '../../lib/store';
import type { Nudge } from '../../lib/types';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
vi.mock('@react-three/drei', () => ({ useGLTF: Object.assign(() => ({ scene: {} }), { preload: () => undefined }), Float: () => null, Center: () => null }));

import { FocusPrompt } from './FocusPrompt';

const nudge: Nudge = {
  id: 'ndg-focus',
  at: new Date().toISOString(),
  kind: 'focus_prompt',
  title: 'Muitas janelas',
  message: 'Você tem alternado entre muitas janelas, que tal focar mais? Que tarefa você precisa fazer agora e quer que eu te ajude com um foco maior?',
  seen: false,
};

describe('FocusPrompt (UBI bubble as a form)', () => {
  beforeEach(() => {
    __mock.reset();
    useAppStore.setState({ focusStatus: null, ubiSpeech: null, unseenNudges: [nudge], latestNudge: nudge });
  });

  it('shows the message, refuses an empty task and starts a session on submit', async () => {
    render(<FocusPrompt nudge={nudge} mood="worried" size={120} defaultMinutes={45} />);
    expect(screen.getByRole('status', { name: 'Muitas janelas' })).toHaveTextContent(/alternado entre muitas janelas/);
    const submit = screen.getByRole('button', { name: 'Me ajude a focar' });
    expect(submit).toBeDisabled();
    expect(screen.getByRole('radio', { name: '45 min' })).toHaveAttribute('aria-checked', 'true');

    fireEvent.change(screen.getByPlaceholderText('Ex.: terminar o relatório do IFRO'), { target: { value: 'fechar o edital' } });
    fireEvent.click(screen.getByRole('radio', { name: '25 min' }));
    fireEvent.click(submit);

    await waitFor(() => expect(useAppStore.getState().focusStatus?.session?.task).toBe('fechar o edital'));
    expect(useAppStore.getState().ubiSpeech).toBe('Fechado. 25 min em "fechar o edital". Eu seguro as distrações.');
    expect(useAppStore.getState().unseenNudges).toEqual([]);
    await waitFor(() => expect(__mock.state().nudges.every((n) => n.seen)).toBe(true));
    expect(__mock.state().focus.session?.task).toBe('fechar o edital');
  });

  it('"Agora não" marks the nudge seen and hands the bubble back to UBI', async () => {
    render(<FocusPrompt nudge={nudge} mood="calm" defaultMinutes={45} />);
    fireEvent.click(screen.getByRole('button', { name: 'Agora não' }));
    await waitFor(() => expect(useAppStore.getState().unseenNudges).toEqual([]));
    expect(useAppStore.getState().ubiSpeech).toBe('Dia tranquilo. Que tal um bloco de foco de 45 minutos?');
    expect(useAppStore.getState().focusStatus?.session ?? null).toBeNull();
  });

  it('speaks English', () => {
    setLocale('en');
    render(<FocusPrompt nudge={{ ...nudge, title: 'A lot of windows' }} mood="worried" defaultMinutes={45} />);
    expect(screen.getByRole('button', { name: 'Help me focus' })).toBeInTheDocument();
    expect(screen.getByPlaceholderText('E.g.: finish the IFRO report')).toBeInTheDocument();
  });
});
