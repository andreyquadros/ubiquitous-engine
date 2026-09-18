import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getLocale, setLocale } from '../i18n';
import { __mock } from '../lib/mock';
import { providerPitch } from '../lib/providers';
import { useAppStore } from '../lib/store';
import { SettingsPage } from './Settings';

const renderPage = () =>
  render(
    <MemoryRouter>
      <SettingsPage />
    </MemoryRouter>,
  );

describe('Settings · IA section (mock backend)', () => {
  beforeEach(async () => {
    __mock.reset();
    useAppStore.setState({ settingsView: null });
    await useAppStore.getState().loadSettings();
  });

  it('picks the provider, lists the account models and restores the defaults', async () => {
    renderPage();
    const group = await screen.findByRole('radiogroup', { name: 'Provedor de IA' });
    expect(within(group).getByRole('radio', { name: /Anthropic Claude/ })).toHaveAttribute('aria-checked', 'true');
    // only anthropic has a key in the mock → one key icon
    expect(within(group).getAllByLabelText('chave configurada')).toHaveLength(1);
    expect(screen.getByText('Chave configurada')).toBeInTheDocument();
    expect(screen.getByText('…f3a9')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /Listar modelos da conta/ }));
    await screen.findByText(/3 modelos disponíveis na conta/);
    expect(document.querySelectorAll('datalist option')).toHaveLength(3);

    // free-text model field
    const report = screen.getByLabelText('Modelo de relatórios') as HTMLInputElement;
    fireEvent.change(report, { target: { value: 'claude-opus-5' } });
    expect(report.value).toBe('claude-opus-5');
    fireEvent.click(screen.getByRole('button', { name: /Padrões do provedor/ }));
    expect((screen.getByLabelText('Modelo de relatórios') as HTMLInputElement).value).toBe('claude-sonnet-5');

    // switching to xAI: no key yet → listing disabled, defaults adopted, key form for xAI
    fireEvent.click(within(group).getByRole('radio', { name: /xAI Grok/ }));
    expect(screen.getByRole('button', { name: /Listar modelos da conta/ })).toBeDisabled();
    expect((screen.getByLabelText('Modelo de classificação') as HTMLInputElement).value).toBe('grok-4-1-fast-non-reasoning');
    expect(screen.getByTestId('api-key-form-xai')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('xai-…')).toBeInTheDocument();
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('xai'), { timeout: 3000 });
    expect(useAppStore.getState().settingsView?.ai_health).toEqual({ state: 'not_configured' });

    // saving an xAI key enables the listing again
    fireEvent.change(screen.getByPlaceholderText('xai-…'), { target: { value: 'xai-abcdefghijklmnopqrstuvwxyz9f2c' } });
    fireEvent.click(screen.getByRole('button', { name: 'Validar e salvar' }));
    await screen.findByText('…9f2c');
    expect(screen.getByRole('button', { name: /Listar modelos da conta/ })).toBeEnabled();
    expect(screen.getByText(/Para a IA escolhida \(Anthropic, OpenAI ou xAI\) vão apenas/)).toBeInTheDocument();
  });
});

describe('Settings · language', () => {
  beforeEach(async () => {
    __mock.reset();
    useAppStore.setState({ settingsView: null });
    await useAppStore.getState().loadSettings();
  });

  it('renders the page in English when the locale is en', async () => {
    setLocale('en');
    renderPage();
    expect(await screen.findByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    expect(screen.getByRole('radiogroup', { name: 'AI provider' })).toBeInTheDocument();
    expect(screen.getByText('Key configured')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /List account models/ })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'macOS permissions' })).toBeInTheDocument();
    // the provider pitch follows the locale too
    expect(providerPitch('xai').cost).toBe('$0.50–2/mo');
    expect(providerPitch('openai').note).toMatch(/API key/);
  });

  it('switches the whole UI and persists Settings.language from the general section', async () => {
    renderPage();
    expect(await screen.findByRole('heading', { name: 'Configurações' })).toBeInTheDocument();
    const group = screen.getByRole('radiogroup', { name: 'Idioma' });
    expect(within(group).getByRole('radio', { name: 'Português (Brasil)' })).toHaveAttribute('aria-checked', 'true');

    fireEvent.click(within(group).getByRole('radio', { name: 'English' }));
    expect(await screen.findByRole('heading', { name: 'Settings' })).toBeInTheDocument();
    expect(getLocale()).toBe('en');
    expect(screen.getByRole('radiogroup', { name: 'Language' })).toBeInTheDocument();
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.language).toBe('en'));

    // and back
    fireEvent.click(screen.getByRole('radio', { name: 'Português (Brasil)' }));
    expect(await screen.findByRole('heading', { name: 'Configurações' })).toBeInTheDocument();
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.language).toBe('pt-BR'));
  });
});

describe('Settings · updates section (mock backend)', () => {
  beforeEach(async () => {
    __mock.reset();
    useAppStore.setState({ settingsView: null, updateStatus: null });
    await useAppStore.getState().loadSettings();
  });
  afterEach(() => vi.restoreAllMocks());

  it('shows the running build, the install commands and saves check_updates from the toggle', async () => {
    renderPage();
    expect(await screen.findByRole('heading', { name: 'Atualizações' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Atualizações' })).toBeInTheDocument(); // rail entry
    // "ubiqX 0.1.0" also shows in About: assert on the build rows this section adds
    expect(await screen.findByText('14c6e7f (main)')).toBeInTheDocument();
    expect(screen.getAllByText('ubiqX 0.1.0', { selector: 'dd' })).toHaveLength(2);
    expect(screen.getByText('Última verificação: nunca')).toBeInTheDocument();
    expect(screen.getByTestId('install-commands')).toHaveTextContent('xattr -dr com.apple.quarantine /Applications/ubiqX.app');
    expect(screen.getByTestId('install-commands')).toHaveTextContent('codesign --force --deep --options runtime --sign "ubiqX Dev" /Applications/ubiqX.app');
    expect(screen.queryByTestId('update-release')).not.toBeInTheDocument();

    const toggle = screen.getByRole('switch', { name: 'Verificar automaticamente' });
    expect(toggle).toHaveAttribute('aria-checked', 'true');
    fireEvent.click(toggle);
    expect(toggle).toHaveAttribute('aria-checked', 'false');
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.check_updates).toBe(false), { timeout: 3000 });
    expect(__mock.state().settings.check_updates).toBe(false);
  });

  it('"Verificar agora" reports the latest version, then shows the release served by __mock.setUpdate', async () => {
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    renderPage();
    const button = await screen.findByRole('button', { name: 'Verificar agora' });
    await waitFor(() => expect(button).toBeEnabled());
    fireEvent.click(button);
    expect(await screen.findByText('Você já está na versão mais recente.')).toBeInTheDocument();
    expect(screen.getByText(/Última verificação: \d\d\/\d\d \d\d:\d\d/)).toBeInTheDocument();

    act(() => __mock.setUpdate(true));
    fireEvent.click(screen.getByRole('button', { name: 'Verificar agora' }));
    const panel = await screen.findByTestId('update-release');
    expect(panel).toHaveTextContent('Nova versão disponível');
    expect(panel).toHaveTextContent('(a1b2c3d, nº 27)');
    expect(screen.getByTestId('release-notes')).toHaveTextContent('Faixa no topo e seção Atualizações em Configurações');
    expect(screen.queryByText('Você já está na versão mais recente.')).not.toBeInTheDocument();

    fireEvent.click(within(panel).getByRole('button', { name: 'Baixar (.dmg)' }));
    await waitFor(() => expect(open).toHaveBeenCalledWith(expect.stringMatching(/ubiqX-macos-aarch64\.dmg$/), '_blank', 'noopener'));
    fireEvent.click(within(panel).getByRole('button', { name: 'Página do release' }));
    await waitFor(() => expect(open).toHaveBeenCalledWith('https://github.com/andreyquadros/ubiquitous-engine/releases/tag/continuous', '_blank', 'noopener'));
  });

  it('scrolls to the updates section when opened from the banner and copies the install commands', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    const scroll = Element.prototype.scrollIntoView as unknown as ReturnType<typeof vi.fn>;
    scroll.mockClear();
    render(
      <MemoryRouter initialEntries={[{ pathname: '/settings', state: { section: 'atualizacoes' } }]}>
        <SettingsPage />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Atualizações' });
    await waitFor(() => expect(scroll).toHaveBeenCalled());
    expect(screen.getByRole('button', { name: 'Atualizações' })).toHaveAttribute('aria-current', 'true');

    fireEvent.click(screen.getByRole('button', { name: 'Copiar comandos' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith('xattr -dr com.apple.quarantine /Applications/ubiqX.app\ncodesign --force --deep --options runtime --sign "ubiqX Dev" /Applications/ubiqX.app'));
    expect(await screen.findByRole('button', { name: 'Copiado' })).toBeInTheDocument();
  });
});
