import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';
import { __mock } from '../lib/mock';
import { useAppStore } from '../lib/store';
import { useToastStore } from '../lib/toast';

vi.mock('@react-three/fiber', () => ({ Canvas: () => null, useFrame: () => undefined }));
vi.mock('@react-three/drei', () => ({ useGLTF: Object.assign(() => ({ scene: {} }), { preload: () => undefined }), Float: () => null, Center: () => null }));

import { FocusPage } from './Focus';

const renderPage = () =>
  render(
    <MemoryRouter>
      <FocusPage />
    </MemoryRouter>,
  );

const resetStore = async () => {
  __mock.reset();
  useToastStore.setState({ toasts: [] });
  useAppStore.setState({ settingsView: null, focusStatus: null, focusTargets: [], interventions: [], installedApps: [], knownDomains: [], ubiSpeech: null });
  await useAppStore.getState().loadSettings();
};

describe('Focus page (mock backend)', () => {
  beforeEach(resetStore);

  it('renders the session card, the block list, the interventions and the options', async () => {
    renderPage();
    expect(await screen.findByRole('heading', { name: 'Foco' })).toBeInTheDocument();
    expect(screen.getByText('Bloqueios, sessões de foco e o que o UBI segurou por você.')).toBeInTheDocument();
    await screen.findByTestId('focus-targets');
    expect(screen.getByPlaceholderText('Ex.: terminar o relatório do IFRO')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Focar' })).toBeInTheDocument();
    // the three seeded targets, enabled first
    const rows = within(screen.getByTestId('focus-targets')).getAllByRole('listitem');
    expect(rows).toHaveLength(3);
    expect(rows[0]).toHaveTextContent('Instagram');
    expect(rows[1]).toHaveTextContent('YouTube');
    expect(rows[2]).toHaveTextContent('Discord');
    expect(screen.getByText('youtube.com')).toBeInTheDocument();
    expect(screen.getByText('7 bloqueios')).toBeInTheDocument();
    // four interventions today, with their action labels
    expect(within(screen.getByTestId('focus-interventions')).getAllByRole('listitem')).toHaveLength(4);
    expect(screen.getByText('4 hoje')).toBeInTheDocument();
    expect(screen.getAllByText('aba fechada').length).toBeGreaterThan(0);
    expect(screen.getByText('app fechado')).toBeInTheDocument();
    // options
    expect(screen.getByRole('switch', { name: 'Bloquear apps e sites da lista' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('button', { name: 'Testar aviso' })).toBeInTheDocument();
  });

  it('searches installed apps and known domains, offers "Bloquear o site" for a domain and adds targets', async () => {
    renderPage();
    const search = await screen.findByRole('combobox', { name: 'Buscar app ou site' });
    await waitFor(() => expect(useAppStore.getState().installedApps.length).toBe(6));

    fireEvent.change(search, { target: { value: 'sl' } });
    const list = await screen.findByRole('listbox', { name: 'Resultados' });
    expect(within(list).getByRole('option', { name: /Slack/ })).toBeInTheDocument();
    expect(within(list).getByText('com.tinyspeck.slackmacgap')).toBeInTheDocument();
    expect(within(list).queryByText(/Bloquear o site/)).not.toBeInTheDocument();

    // diacritic-insensitive: "xcóde" still finds Xcode
    fireEvent.change(search, { target: { value: 'xcóde' } });
    expect(await screen.findByRole('option', { name: /Xcode/ })).toBeInTheDocument();

    // a domain typed by hand gets the "block this site" entry (normalised: scheme, www and path stripped)
    fireEvent.change(search, { target: { value: 'https://www.Reddit.com/r/all' } });
    const custom = await screen.findByRole('option', { name: 'Bloquear o site reddit.com' });
    fireEvent.click(custom);
    await waitFor(() => expect(within(screen.getByTestId('focus-targets')).getByText('reddit.com')).toBeInTheDocument());
    expect(useAppStore.getState().focusTargets.some((x) => x.kind === 'site' && x.key === 'reddit.com' && x.enabled)).toBe(true);
    expect((search as HTMLInputElement).value).toBe('');

    // known domains come from the user's own blocks; Enter picks the first result
    fireEvent.change(search, { target: { value: 'github' } });
    expect(await screen.findByRole('option', { name: /github\.com/ })).toBeInTheDocument();
    fireEvent.keyDown(search, { key: 'Enter' });
    await waitFor(() => expect(useAppStore.getState().focusTargets.some((x) => x.key === 'github.com')).toBe(true));

    // an app: Slack lands in the list with its bundle id
    fireEvent.change(search, { target: { value: 'slack' } });
    fireEvent.keyDown(search, { key: 'Enter' });
    await waitFor(() => expect(within(screen.getByTestId('focus-targets')).getByText('Slack')).toBeInTheDocument());
    expect(useAppStore.getState().focusTargets.find((x) => x.name === 'Slack')?.key).toBe('com.tinyspeck.slackmacgap');
  });

  it('does not re-add a target that is already on the list; it says so instead', async () => {
    renderPage();
    const search = await screen.findByRole('combobox', { name: 'Buscar app ou site' });
    await waitFor(() => expect(useAppStore.getState().installedApps.length).toBe(6));
    const notices = () => useToastStore.getState().toasts.filter((x) => x.title === 'youtube.com já está na lista');

    // Enter on a listed result: a notice, no add
    fireEvent.change(search, { target: { value: 'youtube.com' } });
    const option = await screen.findByRole('option', { name: /youtube\.com/ });
    expect(within(option).getByText('na lista')).toBeInTheDocument();
    fireEvent.keyDown(search, { key: 'Enter' });
    await waitFor(() => expect(notices()).toHaveLength(1));
    expect(notices()[0]?.kind).toBe('info');
    expect(useToastStore.getState().toasts.some((x) => x.title === 'Bloqueio adicionado')).toBe(false);
    expect(__mock.state().focus.targets).toHaveLength(3);
    expect(useAppStore.getState().focusTargets).toHaveLength(3);
    expect((search as HTMLInputElement).value).toBe('');

    // clicking it does the same
    fireEvent.change(search, { target: { value: 'youtube.com' } });
    fireEvent.click(await screen.findByRole('option', { name: /youtube\.com/ }));
    await waitFor(() => expect(notices()).toHaveLength(2));
    expect(__mock.state().focus.targets).toHaveLength(3);
    expect((search as HTMLInputElement).value).toBe('');

    // a disabled target is not "listed": picking it re-enables it through the normal add
    fireEvent.change(search, { target: { value: 'discord' } });
    const discord = await screen.findByRole('option', { name: /Discord/ });
    expect(within(discord).queryByText('na lista')).not.toBeInTheDocument();
    fireEvent.keyDown(search, { key: 'Enter' });
    await waitFor(() => expect(__mock.state().focus.targets.find((x) => x.name === 'Discord')?.enabled).toBe(true));
    expect(__mock.state().focus.targets).toHaveLength(3);
    expect(useToastStore.getState().toasts.some((x) => x.title === 'Bloqueio adicionado')).toBe(true);
  });

  it('gives three warnings before disabling a target blocked in the last 15 minutes', async () => {
    renderPage();
    await screen.findByTestId('focus-targets');
    const toggle = screen.getByRole('switch', { name: 'Bloquear YouTube' });
    expect(toggle).toHaveAttribute('aria-checked', 'true');
    fireEvent.click(toggle);

    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).getByText('Espera. Você bloqueou isso por um motivo. Não desista dos seus sonhos.')).toBeInTheDocument();
    expect(within(dialog).getByText('Aviso 1 de 3')).toBeInTheDocument();
    expect(within(dialog).getByTestId('ubi')).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Manter o bloqueio' })).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Continuar' }));
    // still enabled after the first warning
    expect(screen.getByRole('switch', { name: 'Bloquear YouTube' })).toHaveAttribute('aria-checked', 'true');

    expect(await within(dialog).findByText('Segundo aviso: o que você quer conquistar está do outro lado desse foco.')).toBeInTheDocument();
    expect(within(dialog).queryByRole('button', { name: 'Desativar mesmo assim' })).not.toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Continuar' }));

    expect(await within(dialog).findByText('Último aviso. Se abrir agora, que seja por 5 minutos, e volte. Eu continuo aqui torcendo por você.')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Bloquear YouTube' })).toHaveAttribute('aria-checked', 'true');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Desativar mesmo assim' }));

    await waitFor(() => expect(screen.getByRole('switch', { name: 'Bloquear YouTube' })).toHaveAttribute('aria-checked', 'false'));
    await waitFor(() => expect(__mock.state().focus.targets.find((x) => x.name === 'YouTube')?.enabled).toBe(false));
  });

  it('"Manter o bloqueio" keeps the target on', async () => {
    renderPage();
    await screen.findByTestId('focus-targets');
    fireEvent.click(screen.getByRole('switch', { name: 'Bloquear YouTube' }));
    const dialog = await screen.findByRole('dialog');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Manter o bloqueio' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    expect(screen.getByRole('switch', { name: 'Bloquear YouTube' })).toHaveAttribute('aria-checked', 'true');
    expect(__mock.state().focus.targets.find((x) => x.name === 'YouTube')?.enabled).toBe(true);
  });

  it('asks once for a target that was not blocked recently, and once to remove one', async () => {
    renderPage();
    await screen.findByTestId('focus-targets');
    fireEvent.click(screen.getByRole('switch', { name: 'Bloquear Instagram' }));
    const dialog = await screen.findByRole('dialog', { name: 'Desativar o bloqueio de Instagram?' });
    expect(within(dialog).queryByText(/Não desista/)).not.toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Desativar' }));
    await waitFor(() => expect(screen.getByRole('switch', { name: 'Bloquear Instagram' })).toHaveAttribute('aria-checked', 'false'));

    fireEvent.click(screen.getByRole('button', { name: 'Remover Discord' }));
    const remove = await screen.findByRole('dialog', { name: 'Remover Discord da lista?' });
    fireEvent.click(within(remove).getByRole('button', { name: 'Remover' }));
    await waitFor(() => expect(within(screen.getByTestId('focus-targets')).queryByText('Discord')).not.toBeInTheDocument());
    expect(__mock.state().focus.targets.some((x) => x.name === 'Discord')).toBe(false);
  });

  it('starts a focus session from the card and counts down', async () => {
    renderPage();
    const input = await screen.findByPlaceholderText('Ex.: terminar o relatório do IFRO');
    // blank task: refused locally
    fireEvent.click(screen.getByRole('button', { name: 'Focar' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Escreva a tarefa antes de começar.');

    fireEvent.change(input, { target: { value: 'corrigir as provas' } });
    fireEvent.click(screen.getByRole('radio', { name: '25 min' }));
    fireEvent.click(screen.getByRole('button', { name: 'Focar' }));

    const countdown = await screen.findByTestId('focus-countdown');
    expect(countdown).toHaveTextContent(/^2[45]:\d\d$/);
    expect(screen.getByText('corrigir as provas')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Encerrar' })).toBeInTheDocument();
    expect(screen.getByText('0 distrações seguradas')).toBeInTheDocument();
    expect(useAppStore.getState().focusStatus?.session?.task).toBe('corrigir as provas');

    fireEvent.click(screen.getByRole('button', { name: 'Encerrar' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Focar' })).toBeInTheDocument());
    expect(useAppStore.getState().focusStatus?.session).toBeNull();
    // stopped within the first minutes: a plain "ended" toast, no praise
    await waitFor(() => expect(useToastStore.getState().toasts.some((x) => x.title === 'Sessão encerrada')).toBe(true));
    const ended = useToastStore.getState().toasts.find((x) => x.title === 'Sessão encerrada');
    expect(ended?.kind).toBe('info');
    expect(ended?.message).toBeUndefined();
  });

  it('thanks for a session stopped by hand after five minutes or more', async () => {
    __mock.setFocus({ session: 'active' }); // 45 min, started 12 min ago
    renderPage();
    await screen.findByTestId('focus-countdown');
    fireEvent.click(screen.getByRole('button', { name: 'Encerrar' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Focar' })).toBeInTheDocument());
    await waitFor(() => expect(useToastStore.getState().toasts.some((x) => x.title === 'Sessão encerrada')).toBe(true));
    const ended = useToastStore.getState().toasts.find((x) => x.title === 'Sessão encerrada');
    expect(ended?.kind).toBe('success');
    expect(ended?.message).toBe('12 min em "terminar o relatório do IFRO". Bom trabalho.');
  });

  it('shows the active session from the mock (?session=active) with its countdown', async () => {
    __mock.setFocus({ session: 'active' });
    renderPage();
    const countdown = await screen.findByTestId('focus-countdown');
    expect(countdown).toHaveTextContent(/^3[23]:\d\d$/);
    expect(screen.getByText('terminar o relatório do IFRO')).toBeInTheDocument();
    expect(screen.getByText('1 distração segurada')).toBeInTheDocument();
    expect(screen.getByText('Outras janelas escondidas')).toBeInTheDocument();
  });

  it('saves the options into settings.focus', async () => {
    renderPage();
    const guard = await screen.findByRole('switch', { name: 'Bloquear apps e sites da lista' });
    fireEvent.click(guard);
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.focus.guard_enabled).toBe(false));
    expect(await screen.findByText('O guarda está desligado: a lista não é aplicada. Ligue em Opções.')).toBeInTheDocument();
    expect(__mock.state().settings.focus.guard_enabled).toBe(false);

    fireEvent.click(screen.getByRole('switch', { name: 'Esconder as outras janelas ao começar' }));
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.focus.hide_others_on_start).toBe(false));

    const minutes = screen.getByLabelText('Duração padrão da sessão');
    fireEvent.change(minutes, { target: { value: '90' } });
    fireEvent.blur(minutes);
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.focus.session_minutes).toBe(90));
    // the default length is now preselected in the session card
    expect(screen.getByRole('radio', { name: '90 min' })).toHaveAttribute('aria-checked', 'true');

    const on = screen.getByLabelText('Atalho ao começar a sessão');
    fireEvent.change(on, { target: { value: 'Foco ligado' } });
    fireEvent.blur(on);
    await waitFor(() => expect(useAppStore.getState().settingsView?.settings.focus.macos_focus_shortcut_on).toBe('Foco ligado'));
    expect(__mock.state().settings.focus.macos_focus_shortcut_on).toBe('Foco ligado');
    // the other settings are untouched
    expect(__mock.state().settings.nudges.enabled).toBe(true);
  });

  it('speaks English when the locale is en', async () => {
    setLocale('en');
    renderPage();
    expect(await screen.findByRole('heading', { name: 'Focus' })).toBeInTheDocument();
    expect(screen.getByText('Blocks, focus sessions and what UBI held back for you.')).toBeInTheDocument();
    expect(await screen.findByRole('switch', { name: 'Block YouTube' })).toBeInTheDocument();
    expect(screen.getByPlaceholderText('E.g.: finish the IFRO report')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Test the warning' })).toBeInTheDocument();
    expect(screen.getByText('7 blocks')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('switch', { name: 'Block YouTube' }));
    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).getByText("Wait. You blocked this for a reason. Don't give up on your dreams.")).toBeInTheDocument();
    expect(within(dialog).getByRole('button', { name: 'Keep the block' })).toBeInTheDocument();
  });
});
