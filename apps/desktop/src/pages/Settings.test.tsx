import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { getLocale, setLocale } from '../i18n';
import { __mock } from '../lib/mock';
import { providerPitch } from '../lib/providers';
import { useAppStore } from '../lib/store';
import { IDLE_INSTALL } from '../lib/updater';
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

describe('Settings · Windows and Linux (platform from the settings view)', () => {
  beforeEach(() => {
    __mock.reset();
    useAppStore.setState({ settingsView: null, updateStatus: null, updateInstall: IDLE_INSTALL });
  });
  afterEach(() => vi.restoreAllMocks());

  it('on Windows hides the macOS-only bits and explains the installer', async () => {
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    __mock.setPlatform('windows');
    __mock.setUpdate(true);
    await useAppStore.getState().loadSettings();
    renderPage();
    await screen.findByRole('heading', { name: 'Atualizações' });
    // no permissions section, neither in the rail nor in the page
    expect(screen.queryByRole('heading', { name: 'Permissões do macOS' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Permissões do macOS' })).not.toBeInTheDocument();
    // copy that named the Mac, the Keychain or macOS notifications has its Windows twin
    expect(screen.getByText('Tudo fica no seu computador. Só sai o mínimo necessário para a IA classificar e escrever relatórios.')).toBeInTheDocument();
    expect(screen.getByText(/Gerenciador de Credenciais do Windows/)).toBeInTheDocument();
    expect(screen.getByText(/numa notificação do sistema\.$/)).toBeInTheDocument();
    expect(screen.getByText('Windows', { selector: 'dd' })).toBeInTheDocument();
    expect(screen.getByText('C:\\Users\\andrey\\AppData\\Roaming\\ai.ubiqx.app')).toBeInTheDocument();
    // install notes: the SmartScreen hint, no terminal commands
    const notes = screen.getByTestId('install-notes');
    expect(notes).toHaveTextContent('Abra o instalador e siga os passos; o Windows pode pedir confirmação do SmartScreen.');
    expect(notes).toHaveTextContent('Executar assim mesmo');
    expect(screen.queryByTestId('install-commands')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Copiar comandos' })).not.toBeInTheDocument();
    // the release panel downloads the setup .exe
    const panel = await screen.findByTestId('update-release');
    fireEvent.click(within(panel).getByRole('button', { name: 'Baixar (.exe)' }));
    await waitFor(() => expect(open).toHaveBeenCalledWith(expect.stringMatching(/ubiqX-windows-x86_64-setup\.exe$/), '_blank', 'noopener'));
  });

  it('on Linux shows the AppImage and .deb commands', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', { value: { writeText }, configurable: true });
    __mock.setPlatform('linux');
    await useAppStore.getState().loadSettings();
    setLocale('en');
    renderPage();
    await screen.findByRole('heading', { name: 'Updates' });
    expect(screen.queryByRole('heading', { name: 'macOS permissions' })).not.toBeInTheDocument();
    expect(screen.getByText('Linux', { selector: 'dd' })).toBeInTheDocument();
    expect(screen.getByText(/system keyring \(Secret Service\)/)).toBeInTheDocument();
    const notes = screen.getByTestId('install-notes');
    expect(notes).toHaveTextContent('Make the AppImage executable (chmod +x) and open it, or install the .deb:');
    expect(screen.getByTestId('install-commands')).toHaveTextContent('chmod +x ~/Downloads/ubiqX-linux-x86_64.AppImage');
    expect(screen.getByTestId('install-commands')).toHaveTextContent('sudo apt install ./ubiqX-linux-x86_64.deb');
    fireEvent.click(screen.getByRole('button', { name: 'Copy commands' }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith(expect.stringMatching(/^chmod \+x .*\nsudo apt install \.\/ubiqX-linux-x86_64\.deb$/)));
    setLocale('pt-BR');
  });
});

describe('Settings · updates section (mock backend)', () => {
  beforeEach(async () => {
    __mock.reset();
    useAppStore.setState({ settingsView: null, updateStatus: null, updateInstall: IDLE_INSTALL });
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

  it('offers the in-app update in the release panel and keeps the manual install as the fallback', async () => {
    __mock.setUpdate(true);
    __mock.setUpdateInstall({ totalBytes: 12_000_000 });
    renderPage();
    const panel = await screen.findByTestId('update-release');
    // The manual route is still documented, now named as the fallback it is.
    const notes = screen.getByTestId('install-notes');
    expect(notes).toHaveTextContent('Instalação manual (reserva)');
    expect(notes).toHaveTextContent('Use este caminho só se a atualização automática falhar');
    expect(within(panel).getByRole('button', { name: 'Baixar (.dmg)' })).toBeInTheDocument();

    fireEvent.click(within(panel).getByRole('button', { name: 'Atualizar agora' }));
    // macOS: the honest warning comes before anything is downloaded.
    expect(await within(panel).findByTestId('update-macos-warning')).toHaveTextContent('Gravação de Tela e Automação');
    fireEvent.click(within(panel).getByRole('button', { name: 'Atualizar mesmo assim' }));
    expect(await within(panel).findByRole('progressbar', { name: 'Progresso da atualização' })).toBeInTheDocument();
    await waitFor(() => expect(__mock.state().update.install.installed).toBe(1));
    await waitFor(() => expect(__mock.state().update.install.relaunched).toBe(1));
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

describe('Settings · Licença section (mock backend)', () => {
  beforeEach(async () => {
    __mock.reset();
    useAppStore.setState({ settingsView: null, license: null });
    await useAppStore.getState().loadSettings();
  });
  afterEach(() => vi.restoreAllMocks());

  const section = () => screen.getByRole('heading', { name: 'Licença' }).closest('[id="sec-licenca"]') as HTMLElement;

  it('sits between Geral and IA, starts unlicensed and disables "IA do Ubi" in the provider picker', async () => {
    renderPage();
    await screen.findByRole('heading', { name: 'Licença' });
    const rail = screen.getByRole('navigation', { name: 'Seções' });
    expect(within(rail).getAllByRole('button').map((b) => b.textContent)).toEqual(['Geral', 'Licença', 'IA', 'Rastreamento', 'Privacidade', 'Relatórios', 'UBI e notificações', 'Atualizações', 'Permissões do macOS', 'Sobre']);
    const sec = section();
    expect(within(sec).getByText('Sem licença')).toBeInTheDocument();
    expect(within(sec).getByTestId('license-plan')).toHaveTextContent('Nenhum');
    expect(sec).toHaveTextContent('Anual, com a sua IA: R$ 197/ano ou 10x de R$ 25. Mensal, com a IA do Ubi: R$ 49/mês.');
    expect(within(sec).getByLabelText('Chave de licença')).toHaveAttribute('placeholder', 'UBIQX-…');
    expect(within(sec).getByRole('button', { name: 'Validar' })).toBeDisabled();
    expect(within(sec).queryByRole('button', { name: 'Remover' })).not.toBeInTheDocument();
    expect(screen.queryByTestId('usage-bar')).not.toBeInTheDocument();

    const picker = screen.getByRole('radiogroup', { name: 'Provedor de IA' });
    const ubi = within(picker).getByRole('radio', { name: /IA do Ubi/ });
    expect(ubi).toBeDisabled();
    expect(ubi).toHaveAttribute('title', 'Precisa de uma licença mensal válida (seção Licença)');
    expect(screen.getByTestId('about-plan')).toHaveTextContent('Sem licença');
  });

  it('validates a managed key: chip, plan, usage bar, "IA do Ubi" selectable with read-only aliases, plan in About', async () => {
    renderPage();
    await screen.findByRole('heading', { name: 'Licença' });
    const sec = section();
    fireEvent.change(within(sec).getByLabelText('Chave de licença'), { target: { value: __mock.sampleLicenseKeys.managed } });
    fireEvent.click(within(sec).getByRole('button', { name: 'Validar' }));
    expect(await within(sec).findByText(/^Licença válida: ubiqX Mensal, com a IA do Ubi, até \d\d\/\d\d\/\d{4}\.$/)).toBeInTheDocument();
    await waitFor(() => expect(useAppStore.getState().license?.state).toBe('valid'));
    expect(within(sec).getByText(/^Válida até \d\d\/\d\d\/\d{4}$/)).toBeInTheDocument();
    expect(within(sec).getByTestId('license-plan')).toHaveTextContent('ubiqX Mensal, com a IA do Ubi');
    expect(within(sec).getByTestId('license-plan')).toHaveTextContent('R$ 49/mês');
    expect(within(sec).getByTestId('license-plan')).toHaveTextContent(/Validade\d\d\/\d\d\/\d{4} \(22 dias restantes\)/);
    expect(within(sec).getByText('Chave carregada')).toBeInTheDocument();
    expect(within(sec).getByText(`…${__mock.sampleLicenseKeys.managed.slice(-4)}`)).toBeInTheDocument();
    const bar = within(sec).getByTestId('usage-bar');
    expect(bar).toHaveTextContent('Uso da IA do Ubi neste mês');
    expect(bar).toHaveTextContent('US$ 2,35 de US$ 6,00');
    expect(within(bar).getByRole('progressbar')).toHaveAttribute('aria-valuenow', '39');
    expect(within(sec).getByTestId('license-managed-note')).toHaveTextContent('servidor do Ubi (api.ubiqx.ai)');
    expect(within(sec).getByTestId('license-managed-note')).toHaveTextContent('Selecione “IA do Ubi” na seção IA para usá-la.');
    expect(screen.getByTestId('about-plan')).toHaveTextContent('ubiqX Mensal, com a IA do Ubi');

    // the AI section: "IA do Ubi" is now selectable; the license key stands in for the API key; aliases are read-only
    const picker = screen.getByRole('radiogroup', { name: 'Provedor de IA' });
    const ubi = within(picker).getByRole('radio', { name: /IA do Ubi/ });
    expect(ubi).toBeEnabled();
    fireEvent.click(ubi);
    expect(ubi).toHaveAttribute('aria-checked', 'true');
    expect(await screen.findByTestId('api-key-form-ubi')).toHaveTextContent('A IA do Ubi não usa chave de provedor');
    expect(screen.getByTestId('model-fields-managed')).toHaveTextContent('Escolhidos pelo servidor do Ubi');
    const classify = screen.getByLabelText('Modelo de classificação') as HTMLInputElement;
    expect(classify.value).toBe('ubi-fast');
    expect(classify).toHaveAttribute('readonly');
    expect((screen.getByLabelText('Modelo de relatórios') as HTMLInputElement).value).toBe('ubi-smart');
    expect(screen.queryByRole('button', { name: /Listar modelos da conta/ })).not.toBeInTheDocument();
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('ubi'), { timeout: 3000 });
    expect(useAppStore.getState().settingsView?.ai_health).toEqual({ state: 'ok' });
    expect(within(sec).queryByText('Selecione “IA do Ubi” na seção IA para usá-la.')).not.toBeInTheDocument();

    // removing the key drops the status back; the managed provider stays selected but is no longer configured
    fireEvent.click(within(sec).getByRole('button', { name: 'Remover' }));
    expect(await within(sec).findByText('Chave de licença removida.')).toBeInTheDocument();
    await waitFor(() => expect(useAppStore.getState().license?.state).toBe('unlicensed'));
    await waitFor(() => expect(useAppStore.getState().settingsView?.ai_health).toEqual({ state: 'not_configured' }));
    expect(useAppStore.getState().settingsView?.settings.ai_provider).toBe('ubi');
    expect(within(picker).getByRole('radio', { name: /IA do Ubi/ })).toBeDisabled();
  });

  it('reports an expired key and an invalid key without losing the hint', async () => {
    renderPage();
    await screen.findByRole('heading', { name: 'Licença' });
    const sec = section();
    fireEvent.change(within(sec).getByLabelText('Chave de licença'), { target: { value: __mock.sampleLicenseKeys.expired } });
    fireEvent.click(within(sec).getByRole('button', { name: 'Validar' }));
    expect(await within(sec).findByText(/^Esta chave expirou em \d\d\/\d\d\/\d{4}\. Renove o plano para receber uma nova\.$/)).toBeInTheDocument();
    expect(within(sec).getByText('Expirada')).toBeInTheDocument();
    expect(within(sec).getByTestId('license-plan')).toHaveTextContent('ubiqX Anual, com a sua IA');
    expect(within(sec).getByTestId('license-plan')).toHaveTextContent(/Expirou em \d\d\/\d\d\/\d{4}/);
    expect(screen.getByTestId('about-plan')).toHaveTextContent('ubiqX Anual, com a sua IA (Expirada)');

    fireEvent.change(within(sec).getByLabelText('Trocar a chave de licença'), { target: { value: 'UBIQX-NOTAKEY-NOPE' } });
    fireEvent.click(within(sec).getByRole('button', { name: 'Validar' }));
    expect(await within(sec).findByText(/A chave não foi reconhecida/)).toBeInTheDocument();
    expect(within(sec).getByText('Inválida')).toBeInTheDocument();
    expect(within(sec).getByText('…NOPE')).toBeInTheDocument();
  });

  it('"Assinar" opens the plans page and the section is reachable from the banner deep link', async () => {
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    const scroll = Element.prototype.scrollIntoView as unknown as ReturnType<typeof vi.fn>;
    scroll.mockClear();
    render(
      <MemoryRouter initialEntries={[{ pathname: '/settings', state: { section: 'licenca' } }]}>
        <SettingsPage />
      </MemoryRouter>,
    );
    await screen.findByRole('heading', { name: 'Licença' });
    await waitFor(() => expect(scroll).toHaveBeenCalled());
    expect(screen.getByRole('button', { name: 'Licença' })).toHaveAttribute('aria-current', 'true');
    fireEvent.click(within(section()).getByRole('button', { name: 'Assinar' }));
    await waitFor(() => expect(open).toHaveBeenCalledWith('https://andreyquadros.github.io/ubiquitous-engine/#planos', '_blank', 'noopener'));
  });

  it('renders the license section in English', async () => {
    act(() => {
      __mock.setLicense('annual');
    });
    await act(async () => {
      await useAppStore.getState().loadSettings();
    });
    // after the load: Settings.language (pt-BR) is applied by the store on every get_settings
    setLocale('en');
    renderPage();
    expect(await screen.findByRole('heading', { name: 'License' })).toBeInTheDocument();
    expect(screen.getByText(/^Valid until \d\d\/\d\d\/\d{4}$/)).toBeInTheDocument();
    expect(screen.getByTestId('license-plan')).toHaveTextContent('ubiqX Yearly, with your AI');
    expect(screen.getByTestId('license-plan')).toHaveTextContent('R$ 197/year or 10x R$ 25');
    expect(screen.getByRole('button', { name: 'Subscribe' })).toBeInTheDocument();
    expect(screen.getByTestId('about-plan')).toHaveTextContent('ubiqX Yearly, with your AI');
    expect(within(screen.getByRole('radiogroup', { name: 'AI provider' })).getByRole('radio', { name: /Ubi AI/ })).toBeDisabled();
  });
});
