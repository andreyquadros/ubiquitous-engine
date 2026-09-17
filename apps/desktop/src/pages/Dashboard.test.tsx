import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
vi.mock('@react-three/drei', () => ({ useGLTF: Object.assign(() => ({ scene: {} }), { preload: () => undefined }), Float: () => null, Center: () => null }));

import { Dashboard } from './Dashboard';

describe('Dashboard (mock data)', () => {
  it('renders the stats, categories and UBI card from the mock backend', async () => {
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText('Tempo produtivo')).toBeInTheDocument(), { timeout: 4000 });
    expect(screen.getByText('Distrações')).toBeInTheDocument();
    expect(screen.getAllByText('Sem categoria').length).toBeGreaterThan(0);
    expect(screen.getAllByText('IFRO').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Incubadora').length).toBeGreaterThan(0);
    expect(screen.getByTestId('ubi-card')).toBeInTheDocument();
    expect(screen.getByTestId('ubi-svg')).toBeInTheDocument();
    expect(screen.getByText(/Precisa de revisão/)).toBeInTheDocument();
    expect(screen.getByText('Apps mais usados')).toBeInTheDocument();
  });
});
