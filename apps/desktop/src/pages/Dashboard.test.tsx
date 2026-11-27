import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';
import { __mock } from '../lib/mock';
import { useAppStore } from '../lib/store';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));

import { Dashboard } from './Dashboard';

describe('Dashboard (mock data)', () => {
  it('renders the hero, the stats, categories and the assistant strip from the mock backend', async () => {
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText('Tempo produtivo')).toBeInTheDocument(), { timeout: 4000 });
    expect(screen.getByText('Distrações')).toBeInTheDocument();
    expect(screen.getByTestId('ubi-hero')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Resumo do dia' })).toBeInTheDocument();
    // the headline is a sentence written from the data
    expect(screen.getByRole('heading', { level: 2, name: /de foco/ })).toBeInTheDocument();
    expect(screen.getAllByText('Sem categoria').length).toBeGreaterThan(0);
    expect(screen.getAllByText('IFRO').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Incubadora').length).toBeGreaterThan(0);
    expect(screen.getByTestId('ubi-card')).toBeInTheDocument();
    // no PNG and no GLB in jsdom → the inline SVG mascot
    expect(screen.getByTestId('ubi-svg')).toBeInTheDocument();
    expect(screen.getByText(/Precisa de revisão: \d+ blocos?/)).toBeInTheDocument();
    expect(screen.getByText('Apps mais usados')).toBeInTheDocument();
    expect(screen.getByText('Linha do tempo')).toBeInTheDocument();
    expect(screen.getAllByRole('list', { name: 'Legenda' }).length).toBeGreaterThan(0);
  });

  it('speaks English when the locale is en', async () => {
    setLocale('en');
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText('Productive time')).toBeInTheDocument(), { timeout: 4000 });
    expect(screen.getByText('Distractions')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Day summary' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 2, name: /of focus/ })).toBeInTheDocument();
    expect(screen.getByText(/Needs review: \d+ blocks?/)).toBeInTheDocument();
    expect(screen.getByText('Top apps')).toBeInTheDocument();
    expect(screen.getByText('Time by category')).toBeInTheDocument();
    expect(screen.getAllByRole('list', { name: 'Legend' }).length).toBeGreaterThan(0);
    expect(screen.queryByText('Tempo produtivo')).not.toBeInTheDocument();
  });
});

describe('Dashboard · focus', () => {
  const fresh = () => {
    __mock.reset();
    useAppStore.setState({ dashboards: {}, latestNudge: null, unseenNudges: [], ubiSpeech: null, focusStatus: null });
  };

  it('turns the UBI bubble into a form for a focus_prompt nudge and starts the session from it', async () => {
    fresh();
    __mock.setFocus({ nudge: true });
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );
    const prompt = await screen.findByTestId('focus-prompt', {}, { timeout: 4000 });
    expect(prompt).toHaveTextContent(/Você tem alternado entre muitas janelas/);
    fireEvent.change(screen.getByPlaceholderText('Ex.: terminar o relatório do IFRO'), { target: { value: 'terminar o relatório do IFRO' } });
    fireEvent.click(screen.getByRole('button', { name: 'Me ajude a focar' }));

    await waitFor(() => expect(useAppStore.getState().focusStatus?.session?.task).toBe('terminar o relatório do IFRO'));
    // the form gives way to the session line and UBI's confirmation
    expect(await screen.findByTestId('dashboard-session-line')).toHaveTextContent('terminar o relatório do IFRO');
    expect(screen.getByTestId('dashboard-countdown')).toHaveTextContent(/^4[45]:\d\d$/);
    expect(screen.getByRole('button', { name: 'Encerrar' })).toBeInTheDocument();
    expect(screen.queryByTestId('focus-prompt')).not.toBeInTheDocument();
    expect(screen.getByText('Fechado. 45 min em "terminar o relatório do IFRO". Eu seguro as distrações.')).toBeInTheDocument();
  });

  it('shows the compact session line while a session is active', async () => {
    fresh();
    __mock.setFocus({ session: 'active' });
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );
    const line = await screen.findByTestId('dashboard-session-line', {}, { timeout: 4000 });
    expect(line).toHaveTextContent('terminar o relatório do IFRO');
    expect(screen.getByTestId('dashboard-countdown')).toHaveTextContent(/^3[23]:\d\d$/);
    fireEvent.click(screen.getByRole('button', { name: 'Encerrar' }));
    await waitFor(() => expect(screen.queryByTestId('dashboard-session-line')).not.toBeInTheDocument());
  });
});
