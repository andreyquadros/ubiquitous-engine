import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { getLocale, setLocale } from '../i18n';
import { __nofapMock } from '../nofap/mock';
import { calls as nofapCalls, READS, resetNofap } from '../nofap/testing';
import type { Scenario } from '../nofap/mock';
import { AppRoutes } from './AppNofap';
import { __hostMock, handleHost, MOCK_BAD_KEY } from './hostMock';
import { useHostStore } from './hostStore';

interface Start {
  onboarded: boolean;
  /** Whether a key is stored (by default, one is once onboarded). */
  withKey?: boolean;
  scenario?: Scenario;
  entry?: string;
  setup?: () => void;
}

function start({ onboarded, withKey = onboarded, scenario = 'off', entry = '/', setup }: Start) {
  resetNofap(scenario);
  __hostMock.reset(onboarded, withKey);
  setup?.();
  useHostStore.setState({ settings: null, error: null });
  return render(
    <MemoryRouter initialEntries={[entry]}>
      <AppRoutes />
    </MemoryRouter>,
  );
}

const hostCalls = (cmd: string) => __hostMock.calls().filter((c) => c.cmd === cmd);
const writes = () => nofapCalls().filter((c) => !READS.includes(c.cmd));

describe('the welcome (first start)', () => {
  it('opens on a first start and finishes with onboarding_done, straight into the module wizard', async () => {
    start({ onboarded: false });
    const welcome = await screen.findByTestId('welcome-step-1');
    expect(welcome).toHaveTextContent('Ele não mede produtividade');
    fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
    await screen.findByTestId('welcome-step-2');
    fireEvent.click(screen.getByTestId('welcome-finish'));

    await screen.findByTestId('wizard-step-1');
    expect(hostCalls('settings_set').at(-1)).toEqual({ cmd: 'settings_set', args: { ui_language: 'pt-br', onboarding_done: true } });
    expect(__hostMock.state().settings.onboarding_done).toBe(true);
    // Nothing of the protection was switched on: that is the wizard's last button.
    expect(writes()).toEqual([]);
    expect(__nofapMock.state().config.enabled).toBe(false);
  });

  it('speaks the system\'s language on a first start, not the stored default, and stores it at the end', async () => {
    // An English system: the page starts in English, while the backend's row says Portuguese.
    setLocale('en');
    start({ onboarded: false });
    expect(__hostMock.state().settings.ui_language).toBe('pt-br');
    expect(await screen.findByTestId('welcome-step-1')).toHaveTextContent('It does not measure productivity');
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));
    fireEvent.click(await screen.findByTestId('welcome-finish'));
    await screen.findByTestId('wizard-step-1');
    expect(getLocale()).toBe('en');
    expect(__hostMock.state().settings.ui_language).toBe('en');
  });

  it('follows the stored language once the welcome is done', async () => {
    setLocale('en');
    start({ onboarded: true });
    await screen.findByTestId('wizard-step-1');
    expect(getLocale()).toBe('pt-BR');
  });

  it('does not come back once the onboarding is done', async () => {
    start({ onboarded: true, entry: '/welcome' });
    await screen.findByTestId('wizard-step-1');
    expect(screen.queryByTestId('welcome')).not.toBeInTheDocument();
  });

  it('switches the language at once and stores it', async () => {
    start({ onboarded: false });
    await screen.findByTestId('welcome-step-1');
    fireEvent.click(screen.getByRole('radio', { name: 'English' }));
    await screen.findByText(/It does not measure productivity/);
    expect(getLocale()).toBe('en');
    await waitFor(() => expect(__hostMock.state().settings.ui_language).toBe('en'));
  });

  it('offers the user\'s own key only: Anthropic, OpenAI and xAI, never the managed provider', async () => {
    start({ onboarded: false });
    fireEvent.click(await screen.findByRole('button', { name: 'Continuar' }));
    const picker = within(await screen.findByTestId('provider-key')).getByRole('radiogroup');
    expect(within(picker).getAllByRole('radio').map((r) => r.textContent)).toEqual(['Anthropic', 'OpenAI', 'xAI']);
    expect(screen.queryByText('IA do Ubi')).not.toBeInTheDocument();
  });

  it('stores a key checked by the vendor, and shows the vendor\'s refusal', async () => {
    start({ onboarded: false });
    fireEvent.click(await screen.findByRole('button', { name: 'Continuar' }));
    fireEvent.click(await screen.findByRole('radio', { name: 'OpenAI' }));
    await waitFor(() => expect(__hostMock.state().settings.ai_provider).toBe('openai'));

    const input = screen.getByLabelText('Chave de API da OpenAI');
    fireEvent.change(input, { target: { value: MOCK_BAD_KEY } });
    fireEvent.click(screen.getByTestId('key-save'));
    expect(await screen.findByRole('alert')).toHaveTextContent('refused this key');
    expect(__hostMock.state().keys.has('openai')).toBe(false);

    fireEvent.change(input, { target: { value: '  sk-test-ana  ' } });
    fireEvent.click(screen.getByTestId('key-save'));
    await screen.findByText('Há uma chave salva para a OpenAI.');
    expect(hostCalls('api_key_set').at(-1)).toEqual({ cmd: 'api_key_set', args: { provider: 'openai', key: '  sk-test-ana  ' } });
    expect(__hostMock.state().keys.has('openai')).toBe(true);
    expect(input).toHaveValue('');
  });
});

describe('the frame', () => {
  it('shows the module page with the display name, and nothing of the tracker', async () => {
    start({ onboarded: true, scenario: 'on' });
    await screen.findByTestId('nofap-status');
    const nav = screen.getByRole('complementary', { name: 'Navegação principal' });
    expect(within(nav).getAllByRole('link').map((a) => a.textContent)).toEqual(['Proteção', 'Configurações']);
    expect(nav).toHaveTextContent('NoFap');
    for (const tracker of ['Hoje', 'Timeline', 'Revisão', 'Relatórios', 'Categorias', 'Foco']) expect(within(nav).queryByText(tracker)).not.toBeInTheDocument();
  });

  it('opens the module tabs by name (the tray\'s breather opens the support tab)', async () => {
    start({ onboarded: true, scenario: 'on', entry: '/support' });
    expect(await screen.findByRole('tab', { name: 'Apoio', selected: true })).toBeInTheDocument();
  });

  it('lends the wizard the way to its own settings when the AI check has no key', async () => {
    start({ onboarded: true, withKey: false });
    await screen.findByTestId('wizard-step-1');
    fireEvent.click(screen.getByRole('checkbox', { name: 'É para mim, no meu computador' }));
    fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Continuar' }));
    const step = await screen.findByTestId('wizard-step-3');
    expect(step).toHaveTextContent('Falta a chave do provedor de IA.');
    fireEvent.click(within(step).getByRole('button', { name: 'Abrir configurações' }));
    await screen.findByTestId('page-settings');
    expect(screen.getByText('Nenhuma chave salva para a Anthropic.')).toBeInTheDocument();
  });
});

describe('the card window', () => {
  it('speaks the language of the app\'s settings', async () => {
    start({ onboarded: true, scenario: 'on', entry: '/nofap-card?id=card-x', setup: () => void handleHost({ cmd: 'settings_set', args: { ui_language: 'en' } }) });
    expect(await screen.findByText('Page blocked by UBI.')).toBeInTheDocument();
    expect(screen.queryByRole('complementary')).not.toBeInTheDocument();
  });
});

describe('quitting from the window (design 6.3)', () => {
  it('says that nothing keeps working once closed, keeps the app open by default and quits on demand', async () => {
    start({ onboarded: true });
    await screen.findByTestId('wizard-step-1');
    fireEvent.click(screen.getByTestId('nav-quit'));
    expect(await screen.findByTestId('quit-message')).toHaveTextContent('nada fica barrando sites neste computador');
    fireEvent.click(screen.getByTestId('quit-keep-open'));
    await waitFor(() => expect(screen.queryByTestId('quit-message')).not.toBeInTheDocument());
    expect(__hostMock.state().quitRequested).toBe(false);

    fireEvent.click(screen.getByTestId('nav-quit'));
    fireEvent.click(await screen.findByTestId('quit-confirm'));
    await waitFor(() => expect(__hostMock.state().quitRequested).toBe(true));
  });

  it('says what stays applied to the system while a tier is in place', async () => {
    start({ onboarded: true, scenario: 'on' });
    await screen.findByTestId('nofap-status');
    fireEvent.click(screen.getByTestId('nav-quit'));
    expect(await screen.findByTestId('quit-message')).toHaveTextContent('o que está aplicado no sistema continua valendo');
  });

  it('does not promise the system layer while the module is on but its tiers are closed on this system (R15)', async () => {
    start({ onboarded: true, scenario: 'gated' });
    await screen.findByTestId('nofap-status');
    expect(__nofapMock.state().config.enabled).toBe(true);
    fireEvent.click(screen.getByTestId('nav-quit'));
    const message = await screen.findByTestId('quit-message');
    expect(message).toHaveTextContent('nada fica barrando sites neste computador');
    expect(message).not.toHaveTextContent('continua valendo');
  });
});

describe('the settings', () => {
  it('removes the key at once, and the module sees the check is no longer ready', async () => {
    start({ onboarded: true, scenario: 'on', entry: '/settings' });
    await screen.findByText('Há uma chave salva para a Anthropic.');
    fireEvent.click(screen.getByTestId('key-clear'));
    await screen.findByText('Nenhuma chave salva para a Anthropic.');
    expect(hostCalls('api_key_clear')).toEqual([{ cmd: 'api_key_clear', args: { provider: 'anthropic' } }]);
    expect(__nofapMock.state().ai).toBe('no_key');
  });

  it('picks the model of the check from the vendor\'s list', async () => {
    start({ onboarded: true, scenario: 'on', entry: '/settings' });
    const select = await screen.findByTestId('model-select');
    fireEvent.change(select, { target: { value: 'claude-sonnet-4-5' } });
    await waitFor(() => expect(__hostMock.state().settings.vision_model).toBe('claude-sonnet-4-5'));
  });

  it('has no managed provider and no licence', async () => {
    start({ onboarded: true, entry: '/settings' });
    await screen.findByTestId('settings-ai');
    expect(screen.queryByText('IA do Ubi')).not.toBeInTheDocument();
    expect(screen.queryByText(/licen/i)).not.toBeInTheDocument();
  });

  it('shows the about box with the third-party notices', async () => {
    start({ onboarded: true, entry: '/settings' });
    const about = await screen.findByTestId('settings-about');
    await within(about).findByText('ai.ubiqx.shield');
    expect(screen.queryByTestId('notices')).not.toBeInTheDocument();
    fireEvent.click(within(about).getByTestId('notices-toggle'));
    expect(screen.getByTestId('notices')).toHaveTextContent('StevenBlack/hosts');
  });

  it('says in the about box what the AI check sends, instead of claiming nothing leaves the computer', async () => {
    start({ onboarded: true, entry: '/settings' });
    const about = await screen.findByTestId('settings-about');
    expect(about).toHaveTextContent('só a imagem reduzida da janela em dúvida vai ao provedor que você escolheu');
    expect(about).not.toHaveTextContent('não envia nada');
  });

  it('asks for a missing permission only where the system has it', async () => {
    start({ onboarded: true, entry: '/settings' });
    await screen.findByTestId('settings-general');
    expect(screen.queryByTestId('settings-permissions')).not.toBeInTheDocument();
  });

  it('asks for the screen recording permission where it is missing', async () => {
    start({ onboarded: true, entry: '/settings', setup: () => __hostMock.setPermissions({ screen_recording: 'denied', automation: 'granted' }) });
    const card = await screen.findByTestId('settings-permissions');
    expect(card).toHaveTextContent('Gravação de tela');
    expect(card).toHaveTextContent('Não concedida');
    fireEvent.click(within(card).getByRole('button', { name: 'Pedir permissão' }));
    await waitFor(() => expect(hostCalls('permission_request')).toEqual([{ cmd: 'permission_request', args: { kind: 'screen_recording' } }]));
    await waitFor(() => expect(within(card).queryByRole('button', { name: 'Pedir permissão' })).not.toBeInTheDocument());
  });

  it('checks for updates on demand', async () => {
    start({ onboarded: true, entry: '/settings' });
    fireEvent.click(await screen.findByTestId('update-check'));
    expect(await screen.findByTestId('update-line')).toHaveTextContent('Você está na versão mais recente.');
    expect(hostCalls('update_check')).toHaveLength(1);
  });
});
