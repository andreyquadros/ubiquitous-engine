import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
vi.mock('@react-three/drei', () => ({ useGLTF: Object.assign(() => ({ scene: {} }), { preload: () => undefined }), Float: () => null, Center: () => null }));

import { Insights } from './Insights';

const renderPage = () =>
  render(
    <MemoryRouter>
      <Insights />
    </MemoryRouter>,
  );

describe('Insights (mock data)', () => {
  it('renders the week in Portuguese: stat tiles, charts, the advice card and the nudge history', async () => {
    renderPage();
    expect(screen.getByRole('heading', { level: 1, name: 'Insights' })).toBeInTheDocument();
    expect(screen.getByText(/Últimos 7 dias, de .+ a .+/)).toBeInTheDocument();
    expect(screen.getByText('Tempo produtivo')).toBeInTheDocument();
    expect(screen.getByText('Score médio')).toBeInTheDocument();
    expect(screen.getByText('Maior foco contínuo')).toBeInTheDocument();
    expect(screen.getByText('Trocas por hora')).toBeInTheDocument();
    expect(screen.getByText('Horas por categoria')).toBeInTheDocument();
    expect(screen.getByText('Score de foco')).toBeInTheDocument();

    // seven dashboards load from the mock backend, then the hints switch from "calculando" to real numbers
    await waitFor(() => expect(screen.getByText(/^\d+ dias? com atividade$/)).toBeInTheDocument(), { timeout: 4000 });
    expect(screen.queryByText('calculando')).not.toBeInTheDocument();

    // the nudge history lists the seeded nudges with the localized kind label and the unseen badge
    expect(screen.getByText('O que o UBI já disse')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText('Hora de uma pausa')).toBeInTheDocument());
    expect(screen.getByText('Pausa sugerida')).toBeInTheDocument();
    expect(screen.getByText('novo')).toBeInTheDocument();

    // advice: empty state, then Gerar writes the list and the model credit
    expect(screen.getByText('Recomendações do UBI')).toBeInTheDocument();
    expect(screen.getByText('Sem recomendações ainda')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Gerar' }));
    await waitFor(() => expect(screen.getByRole('list', { name: 'Recomendações' })).toBeInTheDocument());
    expect(screen.getByText(/^Escrito por /)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Gerar novamente' })).toBeInTheDocument();
  });

  it('speaks English when the locale is en', async () => {
    setLocale('en');
    renderPage();
    expect(screen.getByRole('heading', { level: 1, name: 'Insights' })).toBeInTheDocument();
    expect(screen.getByText(/Last 7 days, .+ to .+/)).toBeInTheDocument();
    expect(screen.getByText('Productive time')).toBeInTheDocument();
    expect(screen.getByText('Average score')).toBeInTheDocument();
    expect(screen.getByText('Longest focus streak')).toBeInTheDocument();
    expect(screen.getByText('Switches per hour')).toBeInTheDocument();
    expect(screen.getByText('Hours by category')).toBeInTheDocument();
    expect(screen.getByText("UBI's recommendations")).toBeInTheDocument();
    expect(screen.getByText('No recommendations yet')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Generate' })).toBeInTheDocument();
    expect(screen.getByText('What UBI has said')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText(/^\d+ active days?$/)).toBeInTheDocument(), { timeout: 4000 });
    await waitFor(() => expect(screen.getByText('Break suggested')).toBeInTheDocument());
    expect(screen.getByText('new')).toBeInTheDocument();
    expect(screen.queryByText('Tempo produtivo')).not.toBeInTheDocument();
    expect(screen.queryByText('novo')).not.toBeInTheDocument();
  });
});
