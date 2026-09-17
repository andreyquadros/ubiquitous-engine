import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
vi.mock('@react-three/drei', () => ({ useGLTF: Object.assign(() => ({ scene: {} }), { preload: () => undefined }), Float: () => null, Center: () => null }));

import { __mock } from '../lib/mock';
import { useAppStore } from '../lib/store';
import { Onboarding } from './Onboarding';

describe('Onboarding (mock backend)', () => {
  beforeEach(async () => {
    __mock.reset();
    __mock.setOnboardingDone(false);
    useAppStore.setState({ settingsView: null });
    await useAppStore.getState().loadSettings();
  });

  it('step 2 offers the three providers and switching one swaps the key form and the models', async () => {
    render(
      <MemoryRouter>
        <Onboarding />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Oi, eu sou o UBI.' });
    expect(screen.getByText('O que vai para a IA escolhida (Anthropic, OpenAI ou xAI)')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Continuar/ }));
    await screen.findByRole('heading', { name: 'Escolha sua IA' });

    const cards = screen.getAllByRole('radio');
    expect(cards.map((c) => c.getAttribute('aria-label'))).toEqual(['Anthropic Claude', 'OpenAI', 'xAI Grok']);
    expect(screen.getByRole('radio', { name: 'Anthropic Claude' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getAllByText(/estimativa/).length).toBe(3);
    expect(screen.getByText(/Entrar com ChatGPT \(OAuth\) hoje só identifica o usuário/)).toBeInTheDocument();
    expect(screen.getByTestId('api-key-form-anthropic')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('sk-ant-…')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('radio', { name: 'OpenAI' }));
    await screen.findByTestId('api-key-form-openai');
    expect(screen.getByPlaceholderText('sk-…')).toBeInTheDocument();
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('openai'));
    expect(useAppStore.getState().settingsView?.settings.models).toEqual({ classify: 'gpt-5-mini', vision: 'gpt-5-mini', report: 'gpt-5' });
    expect(useAppStore.getState().settingsView?.ai_health).toEqual({ state: 'not_configured' });
    expect(screen.getByRole('radio', { name: 'OpenAI' })).toHaveAttribute('aria-checked', 'true');
  });

  it('opens on the step given by ?step= and the summary names the provider', async () => {
    window.history.replaceState({}, '', '/?onboarding=1&step=7');
    render(
      <MemoryRouter>
        <Onboarding />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Quase lá' });
    expect(screen.getByTestId('finish-ai-summary')).toHaveTextContent('IA: Anthropic Claude');
    expect(screen.getByTestId('finish-ai-summary')).toHaveTextContent('chave configurada …f3a9');
    window.history.replaceState({}, '', '/');
  });
});
