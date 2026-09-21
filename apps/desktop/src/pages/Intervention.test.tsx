import { render, screen, within } from '@testing-library/react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';
import { __mock } from '../lib/mock';
import { useAppStore } from '../lib/store';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
vi.mock('@react-three/drei', () => ({ useGLTF: Object.assign(() => ({ scene: {} }), { preload: () => undefined }), Float: () => null, Center: () => null }));

import { InterventionPage } from './Intervention';

const renderAt = (url: string) =>
  render(
    <MemoryRouter initialEntries={[url]}>
      <Routes>
        <Route path="/intervention" element={<InterventionPage />} />
      </Routes>
    </MemoryRouter>,
  );

describe('Intervention panel', () => {
  beforeEach(() => {
    __mock.reset();
    useAppStore.setState({ settingsView: null });
  });

  it('renders the sample message, the target chip and the button for ?id=test', async () => {
    renderAt('/intervention?id=test');
    const panel = await screen.findByRole('region', { name: 'Aviso do UBI' });
    expect(within(panel).getByTestId('intervention-message')).toHaveTextContent('Não! Foque na sua produtividade.');
    expect(within(panel).getByText('YouTube')).toBeInTheDocument();
    expect(within(panel).getByRole('button', { name: 'Ok, foco!' })).toBeInTheDocument();
    expect(within(panel).getByTestId('ubi')).toBeInTheDocument();
    // the panel is the drag region (no page chrome around it)
    expect(panel).toHaveAttribute('data-tauri-drag-region');
    expect(screen.queryByRole('navigation')).not.toBeInTheDocument();
  });

  it('loads a recorded intervention by id', async () => {
    const recorded = __mock.state().focus.interventions[1]!;
    renderAt(`/intervention?id=${recorded.id}`);
    expect(await screen.findByTestId('intervention-message')).toHaveTextContent(recorded.message);
    expect(screen.getByText(recorded.name)).toBeInTheDocument();
  });

  it('falls back to the sample for an unknown id and follows Settings.language like the shell', async () => {
    setLocale('en');
    __mock.state().settings.language = 'en';
    renderAt('/intervention?id=nope');
    expect(await screen.findByTestId('intervention-message')).toHaveTextContent('No! Focus on your productivity.');
    expect(screen.getByRole('button', { name: 'Ok, focus!' })).toBeInTheDocument();
  });
});
