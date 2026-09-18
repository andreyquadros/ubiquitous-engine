import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';

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
