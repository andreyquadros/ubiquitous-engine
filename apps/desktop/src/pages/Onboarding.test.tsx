import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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

    // the two-card decision comes first; the own-key card is open by default and lists the three vendors
    const plans = screen.getByRole('radiogroup', { name: 'Como a IA será paga' });
    expect(within(plans).getAllByRole('radio').map((c) => c.getAttribute('aria-label'))).toEqual(['Deixar o Ubi cuidar da IA', 'Usar minha própria chave']);
    expect(within(plans).getByRole('radio', { name: 'Usar minha própria chave' })).toHaveAttribute('aria-checked', 'true');
    const cards = within(screen.getByRole('radiogroup', { name: 'Provedor de IA' })).getAllByRole('radio');
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

describe('Onboarding · plans (mock backend)', () => {
  beforeEach(async () => {
    __mock.reset();
    __mock.setOnboardingDone(false);
    useAppStore.setState({ settingsView: null, license: null });
    await useAppStore.getState().loadSettings();
    window.history.replaceState({}, '', '/?onboarding=1&step=2');
  });
  afterEach(() => window.history.replaceState({}, '', '/'));

  const renderStep2 = async () => {
    render(
      <MemoryRouter>
        <Onboarding />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Escolha sua IA' });
  };

  it('shows both cards with the prices, the optional annual key field and "Continuar sem licença por enquanto"', async () => {
    await renderStep2();
    expect(screen.getByTestId('plan-card-managed')).toHaveTextContent('R$ 49/mês');
    expect(screen.getByTestId('plan-card-own')).toHaveTextContent('R$ 197/ano ou 10x de R$ 25');
    expect(screen.getByTestId('own-panel')).toBeInTheDocument();
    expect(screen.queryByTestId('managed-panel')).not.toBeInTheDocument();
    // the annual license is optional on the own-key side
    const annual = screen.getByTestId('annual-license-panel');
    expect(within(annual).getByLabelText('Chave de licença (opcional)')).toBeInTheDocument();
    expect(within(annual).getByText('Assinar')).toBeInTheDocument();
    // soft enforcement: the shortcut is offered and no trial/discount copy shows up
    expect(screen.getByRole('button', { name: 'Continuar sem licença por enquanto' })).toBeEnabled();
    expect(screen.getByTestId('page-onboarding')).not.toHaveTextContent(/desconto|trial|teste grátis/i);
  });

  it('the managed card asks for a monthly key and switches to "IA do Ubi" once it validates', async () => {
    await renderStep2();
    fireEvent.click(screen.getByRole('radio', { name: 'Deixar o Ubi cuidar da IA' }));
    const panel = await screen.findByTestId('managed-panel');
    expect(panel).toHaveTextContent('Para usar a IA do Ubi, valide uma licença mensal.');
    expect(screen.queryByTestId('own-panel')).not.toBeInTheDocument();
    // nothing was persisted yet: the mock refuses the managed provider without a license
    expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('anthropic');

    fireEvent.change(within(panel).getByLabelText('Chave de licença'), { target: { value: __mock.sampleLicenseKeys.managed } });
    fireEvent.click(within(panel).getByRole('button', { name: 'Validar' }));
    await waitFor(() => expect(useAppStore.getState().license?.state).toBe('valid'), { timeout: 3000 });
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('ubi'), { timeout: 3000 });
    expect(useAppStore.getState().settingsView?.settings.models).toEqual({ classify: 'ubi-fast', vision: 'ubi-fast', report: 'ubi-smart' });
    expect(await screen.findByText(/IA do Ubi ativa: licença mensal válida até/)).toBeInTheDocument();
    expect(screen.getByTestId('managed-panel')).toHaveTextContent('ubi-fast');
    expect(screen.getByTestId('managed-panel')).toHaveTextContent('ubi-smart');
    expect(screen.getByRole('button', { name: 'Pular por enquanto' })).toBeInTheDocument();

    // back to the own-key card restores a vendor provider
    fireEvent.click(screen.getByRole('radio', { name: 'Usar minha própria chave' }));
    await screen.findByTestId('own-panel');
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('anthropic'), { timeout: 3000 });
  });

  it('an annual key on the managed card explains the plan mismatch; the finish step names the plan', async () => {
    act(() => {
      __mock.setLicense('annual');
    });
    await act(async () => {
      await useAppStore.getState().loadSettings();
    });
    await renderStep2();
    expect(screen.getByTestId('annual-license-panel')).toHaveTextContent('ubiqX Anual, com a sua IA.');
    expect(screen.getByTestId('annual-license-panel')).toHaveTextContent(/Licença válida até \d\d\/\d\d\/\d{4}\./);
    expect(screen.queryByRole('button', { name: 'Continuar sem licença por enquanto' })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('radio', { name: 'Deixar o Ubi cuidar da IA' }));
    expect(await screen.findByTestId('managed-panel')).toHaveTextContent('Esta chave é do plano anual.');
    expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('anthropic');
  });

  it('the finish summary shows the managed plan without a vendor key', async () => {
    act(() => {
      __mock.setLicense('managed');
    });
    await act(async () => {
      await useAppStore.getState().saveSettings({ ai_provider: 'ubi' });
    });
    window.history.replaceState({}, '', '/?onboarding=1&step=7');
    render(
      <MemoryRouter>
        <Onboarding />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Quase lá' });
    const summary = screen.getByTestId('finish-ai-summary');
    expect(summary).toHaveTextContent('IA: IA do Ubi');
    expect(summary).toHaveTextContent('IA do Ubi pelo plano mensal; sem chave de API.');
    expect(summary).toHaveTextContent('Plano: ubiqX Mensal, com a IA do Ubi');
    expect(summary).not.toHaveTextContent('sem chave: só regras');
  });
});
