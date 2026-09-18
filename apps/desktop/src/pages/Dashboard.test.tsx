import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
vi.mock('@react-three/drei', () => ({ useGLTF: Object.assign(() => ({ scene: {} }), { preload: () => undefined }), Float: () => null, Center: () => null }));

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
    // the headline is a sentence written from the data
    expect(screen.getByRole('heading', { level: 2, name: /de foco/ })).toBeInTheDocument();
    expect(screen.getAllByText('Sem categoria').length).toBeGreaterThan(0);
    expect(screen.getAllByText('IFRO').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Incubadora').length).toBeGreaterThan(0);
    expect(screen.getByTestId('ubi-card')).toBeInTheDocument();
    // no PNG and no GLB in jsdom → the inline SVG mascot
    expect(screen.getByTestId('ubi-svg')).toBeInTheDocument();
    expect(screen.getByText(/Precisa de revisão/)).toBeInTheDocument();
    expect(screen.getByText('Apps mais usados')).toBeInTheDocument();
    expect(screen.getByText('Linha do tempo')).toBeInTheDocument();
  });
});
