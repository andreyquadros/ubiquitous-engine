import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));

import { getLocale, setLocale } from '../i18n';
import { __mock } from '../lib/mock';
import { useAppStore } from '../lib/store';
import { Onboarding, stepsFor } from './Onboarding';

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
    expect(screen.getByText('Passo 1 de 7')).toBeInTheDocument();
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

  it('skips the permissions step off macOS and speaks of the computer, not the Mac', async () => {
    __mock.setPlatform('windows');
    useAppStore.setState({ settingsView: null });
    await useAppStore.getState().loadSettings();
    render(
      <MemoryRouter>
        <Onboarding />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Oi, eu sou o UBI.' });
    expect(screen.getByText('Passo 1 de 6')).toBeInTheDocument();
    const rail = screen.getByRole('list', { name: 'Etapas' });
    expect(within(rail).getAllByRole('listitem').map((li) => li.textContent)).toEqual(['1Como funciona', '2Escolha sua IA', '3Categorias', '4Horários', '5Análise visual', '6Concluir']);
    expect(screen.getByText('Tudo fica no seu computador. Sem contas, sem telemetria.')).toBeInTheDocument();
    expect(screen.getByText('Nada mais sai do seu computador')).toBeInTheDocument();
    expect(screen.queryByText(/Keychain/)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Continuar/ }));
    await screen.findByRole('heading', { name: 'Escolha sua IA' });
    expect(screen.getByText(/guardada no cofre de senhas do sistema/)).toBeInTheDocument();
    expect(screen.getByText('Passo 2 de 6')).toBeInTheDocument();
    // "Pular por enquanto" on the AI step lands on the categories, not on macOS permissions
    fireEvent.click(screen.getByRole('button', { name: 'Pular por enquanto' }));
    await screen.findByRole('heading', { name: 'Suas categorias de trabalho' });
    expect(screen.getByText('Passo 3 de 6')).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Permissões do macOS' })).not.toBeInTheDocument();
  });

  it('?step= is clamped to the steps this OS has', async () => {
    __mock.setPlatform('linux');
    useAppStore.setState({ settingsView: null });
    await useAppStore.getState().loadSettings();
    window.history.replaceState({}, '', '/?onboarding=1&step=6');
    render(
      <MemoryRouter>
        <Onboarding />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Quase lá' });
    expect(screen.getByText('Passo 6 de 6')).toBeInTheDocument();
    expect(screen.getByText('O UBI vai morar na bandeja do sistema. Deixe-o iniciar com o sistema para não perder nenhum dia.')).toBeInTheDocument();
    expect(stepsFor('linux').map((s) => s.id)).toEqual(['intro', 'ai', 'cats', 'times', 'vision', 'finish']);
    expect(stepsFor('macos')).toHaveLength(7);
    window.history.replaceState({}, '', '/');
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

  it('switches to English from the first step, persists it and keeps the flow in English', async () => {
    render(
      <MemoryRouter>
        <Onboarding />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Oi, eu sou o UBI.' });
    expect(screen.getByRole('radiogroup', { name: 'Idioma' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('radio', { name: 'English' }));
    await screen.findByRole('heading', { name: "Hi, I'm UBI." });
    expect(screen.getByText('Step 1 of 7')).toBeInTheDocument();
    expect(screen.getByText('Everything stays on your Mac. No accounts, no telemetry.')).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: 'English' })).toHaveAttribute('aria-checked', 'true');
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.language).toBe('en'));
    expect(getLocale()).toBe('en');

    fireEvent.click(screen.getByRole('button', { name: /Continue/ }));
    await screen.findByRole('heading', { name: 'Pick your AI' });
    expect(screen.getAllByText('estimate at 8 h a day').length).toBe(3);
  });

  it('seeds the starter categories in the active language and picks icons with the shared IconPicker', async () => {
    // a first run has no user categories yet: the wizard proposes three starters in the active language
    const state = __mock.state();
    state.categories = state.categories.filter((c) => c.is_system);
    setLocale('en');
    window.history.replaceState({}, '', '/?onboarding=1&step=4');
    render(
      <MemoryRouter>
        <Onboarding />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Your work categories' });
    await waitFor(() => expect(screen.getAllByRole('textbox', { name: 'Category name' }).map((i) => (i as HTMLInputElement).value)).toEqual(['IFRO', 'Incubator', 'Smart Cities']));

    const pickers = screen.getAllByRole('radiogroup', { name: 'Icon' });
    expect(pickers).toHaveLength(3);
    const first = pickers[0]!;
    expect(first.querySelector('[role="radio"][aria-label="Graduation cap"]')).toHaveAttribute('aria-checked', 'true');
    fireEvent.click(first.querySelector('[role="radio"][aria-label="Coffee"]')!);
    expect(first.querySelector('[role="radio"][aria-label="Coffee"]')).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('button', { name: 'Remove IFRO' })).toBeInTheDocument();
    window.history.replaceState({}, '', '/');
  });
});
