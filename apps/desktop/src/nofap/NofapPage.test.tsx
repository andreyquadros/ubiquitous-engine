import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CheckinCard } from './components/CheckinCard';
import { PendingBanner, RECHECK_MS } from './components/PendingBanner';
import { VisionCaps } from './components/VisionCaps';
import { nofapIpc } from './ipc';
import { ProgressCard } from './components/ProgressCard';
import { __nofapMock } from './mock';
import { setLocale, translate } from '../i18n';
import { waitingSwitches } from './labels';
import { useNofapStore } from './store';
import { calls, READS, renderPage, resetNofap } from './testing';
import type { NofapChange, NofapView } from './types';

const writes = () => calls().filter((c) => !READS.includes(c.cmd));
const openTab = async (name: string) => fireEvent.click(await screen.findByRole('tab', { name }));

/** Makes every waiting request ready, the way time would, and has the page read the state again. */
async function elapse() {
  for (const pending of __nofapMock.state().config.pending) pending.ready_at = new Date(Date.now() - 1000).toISOString();
  await act(async () => {
    await useNofapStore.getState().load();
  });
}

describe('the module page, switched on', () => {
  beforeEach(() => resetNofap('on'));

  it('shows "protected" only with the list of layers that are on, and one row per layer', async () => {
    renderPage();
    const status = await screen.findByTestId('nofap-status');
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Proteção ativa');
    expect(status).toHaveTextContent('Camadas ligadas: Bloqueio no app, Lista do sistema, DNS filtrado, Regras dos navegadores, Verificação por IA.');
    for (const layer of ['guard', 'hosts', 'resolver', 'policies', 'vision']) expect(screen.getByTestId(`tier-row-${layer}`)).toBeInTheDocument();
    expect(status).toHaveTextContent('Hoje, 2 páginas barradas.');
    // The status names layers and counts, never a site.
    expect(status.textContent).not.toMatch(/example\./);
  });

  it('says once, not twice, that the other app holds the protection', async () => {
    resetNofap('secondary');
    renderPage();
    const status = await screen.findByTestId('nofap-status');
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Ativa pelo outro app');
    expect(status.textContent?.split('A proteção está ativa pelo outro app.')).toHaveLength(2);
  });

  it('shows a tier this system cannot have as such: no switch, no choice, and no disclosure of what is not happening', async () => {
    resetNofap('gated');
    renderPage();
    const tab = await screen.findByTestId('tab-protection');
    const hosts = within(tab).getByTestId('protection-tier-hosts');
    expect(hosts).toHaveTextContent('Em verificação neste sistema. Por enquanto valem o bloqueio no app e a verificação por IA.');
    expect(within(hosts).queryByRole('switch')).toBeNull();
    const resolver = within(tab).getByTestId('protection-tier-resolver');
    expect(within(resolver).queryByRole('radio')).toBeNull();
    expect(within(tab).getByTestId('protection-tier-policies')).toHaveTextContent('Esta versão não traz essa camada.');
    // The stored choice still says "Cloudflare", but nothing goes there: the disclosure would be false.
    expect(__nofapMock.state().config.system.resolver).toBe('cloudflare_family');
    expect(tab).not.toHaveTextContent('passam pela Cloudflare');
    expect(tab).not.toHaveTextContent('Gerenciado pela sua organização');
  });

  it('offers a person to talk to one step from every tab', async () => {
    renderPage();
    for (const tab of ['Proteção', 'Apoio', 'Listas', 'Ajuda']) {
      await openTab(tab);
      fireEvent.click(screen.getByTestId('nofap-talk')); // 1
      const dialog = await screen.findByRole('dialog', { name: 'Falar com alguém' });
      await waitFor(() => expect(within(dialog).getByTestId('crisis-card')).toHaveTextContent('ligue 188. É o CVV: grátis, 24 horas.'));
      fireEvent.keyDown(window, { key: 'Escape' });
      await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Falar com alguém' })).toBeNull());
    }
    // Looking for help counts nothing and changes nothing.
    expect(writes()).toEqual([]);
  });

  it('says nothing about a missing contact list while the list is still on its way, and says so when it did not come', async () => {
    let answer: (value: never) => void = () => undefined;
    const held = vi.spyOn(nofapIpc, 'support').mockReturnValue(new Promise<never>((_, reject) => (answer = reject)));
    try {
      renderPage(undefined, '/nofap?tab=help');
      const tab = await screen.findByTestId('tab-help');
      expect(tab).not.toHaveTextContent('A lista de contatos de apoio não veio nesta versão do app.');
      await act(async () => {
        answer(new Error('unavailable') as never);
        await Promise.resolve();
      });
      await waitFor(() => expect(screen.getByTestId('tab-help')).toHaveTextContent('A lista de contatos de apoio não veio nesta versão do app.'));
    } finally {
      held.mockRestore();
    }
  });

  it('opens on the tab the address names', async () => {
    renderPage(undefined, '/nofap?tab=help');
    expect(await screen.findByTestId('tab-help')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Ajuda' })).toHaveAttribute('aria-selected', 'true');
  });
});

describe('turning protection off', () => {
  beforeEach(() => resetNofap('on'));

  it('waits out the delay: a request, a countdown, and a confirmation only once it is ready', async () => {
    renderPage();
    fireEvent.click(await screen.findByTestId('nofap-off-button'));
    const dialog = await screen.findByRole('dialog', { name: 'Desligar o NoFap?' });
    expect(dialog).toHaveTextContent('O pedido espera 10 minutos.');
    expect(dialog).toHaveTextContent('Você escreveu: "Quero estar presente com quem eu amo". Continua valendo?');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Pedir desligamento' }));

    const banner = await screen.findByTestId('pending-banner');
    expect(writes()).toEqual([{ cmd: 'request', args: { kind: 'disable' } }]);
    expect(banner).toHaveTextContent('Desligar o NoFap');
    expect(within(banner).getByTestId('pending-countdown')).toHaveTextContent('10:00');
    expect(banner).toHaveTextContent('Pedido anotado. Vale daqui a 10 minutos. Você pode cancelar quando quiser.');
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Desligamento pedido');
    // Still on, and nothing to confirm yet.
    expect(__nofapMock.state().config.enabled).toBe(true);
    expect(within(banner).queryByRole('button', { name: 'Desligar' })).toBeNull();
    await expect(useNofapStore.getState().confirmPending(__nofapMock.state().config.pending[0]!.id)).rejects.toThrow(/still waiting/);

    await elapse();
    expect(banner).toHaveTextContent('O tempo de espera terminou. Ainda quer desligar?');
    expect(within(banner).queryByTestId('pending-countdown')).toBeNull();
    // "Manter proteção" stays the primary button; turning off is the quiet one.
    expect(within(banner).getByRole('button', { name: 'Manter proteção' })).toBeEnabled();
    fireEvent.click(within(banner).getByRole('button', { name: 'Desligar' }));
    await screen.findByTestId('nofap-off');
    expect(__nofapMock.state().config.enabled).toBe(false);
    // One refused while it was waiting, one accepted once it was ready.
    expect(calls('confirm_pending')).toHaveLength(2);
  });

  it('puts the keyboard on "Manter proteção" when the dialog opens, not on the button that turns off', async () => {
    renderPage();
    fireEvent.click(await screen.findByTestId('nofap-off-button'));
    const dialog = await screen.findByRole('dialog', { name: 'Desligar o NoFap?' });
    const keep = within(dialog).getByRole('button', { name: 'Manter proteção' });
    await waitFor(() => expect(keep).toHaveFocus());
    // Enter on the focused button keeps everything as it is.
    fireEvent.click(keep);
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Desligar o NoFap?' })).toBeNull());
    expect(writes()).toEqual([]);
  });

  it('"Manter proteção" cancels the request in one tap', async () => {
    resetNofap('pending');
    renderPage();
    const banner = await screen.findByTestId('pending-banner');
    fireEvent.click(within(banner).getByRole('button', { name: 'Manter proteção' }));
    await waitFor(() => expect(screen.queryByTestId('pending-banner')).toBeNull());
    expect(writes().map((c) => c.cmd)).toEqual(['cancel_pending']);
    expect(__nofapMock.state().config.enabled).toBe(true);
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Proteção ativa');
  });

  it('with no waiting time, the dialog is the confirmation and "Manter proteção" its primary button', async () => {
    __nofapMock.state().config.delay_secs = 0;
    renderPage();
    fireEvent.click(await screen.findByTestId('nofap-off-button'));
    const dialog = await screen.findByRole('dialog', { name: 'Desligar o NoFap?' });
    expect(dialog).toHaveTextContent('Sem tempo de espera, o desligamento vale assim que você confirmar.');
    expect(within(dialog).getByRole('button', { name: 'Manter proteção' })).toBeEnabled();
    fireEvent.click(within(dialog).getByRole('button', { name: 'Desligar' }));
    await screen.findByTestId('nofap-off');
    expect(writes().map((c) => c.cmd)).toEqual(['request', 'confirm_pending']);
  });

  it('says what is left on the system after a declined prompt, and offers to remove it', async () => {
    resetNofap('left');
    renderPage();
    const off = await screen.findByTestId('nofap-off');
    expect(off).toHaveTextContent('A proteção no app foi desligada. A lista do sistema e o DNS continuam aplicados até você remover.');
    fireEvent.click(screen.getByTestId('nofap-remove-system'));
    const dialog = await screen.findByRole('dialog', { name: 'O que vai mudar no sistema' });
    await within(dialog).findByText('Tira o bloco marcado da lista do sistema.');
    fireEvent.click(within(dialog).getByTestId('nofap-apply-confirm'));
    await waitFor(() => expect(calls('system_remove')).toHaveLength(1));
    await waitFor(() => expect(screen.getByTestId('nofap-off')).toHaveTextContent('Nada do NoFap está aplicado no sistema.'));
  });

  it('lets a loosening change wait and a tightening one apply at once', async () => {
    renderPage();
    await screen.findByTestId('tab-protection');
    fireEvent.click(screen.getByRole('switch', { name: 'Lista do sistema' }));
    await screen.findByTestId('pending-banner');
    expect(screen.getByTestId('pending-banner')).toHaveTextContent('Desligar a lista do sistema');
    expect(__nofapMock.state().config.system.hosts).toBe(true);
    fireEvent.click(screen.getByRole('radio', { name: '30 minutos' }));
    await waitFor(() => expect(__nofapMock.state().config.delay_secs).toBe(1800));
  });
});

describe('a switch whose change is waiting (integrator decision 22)', () => {
  beforeEach(() => resetNofap('on'));

  const NOTE = 'Pedido em espera: isto continua ligado até você confirmar o pedido. Se você cancelar, fica como está.';
  /** Puts requests into the waiting list the way the engine would, each with the full delay ahead of it. */
  const wait = (...changes: NofapChange[]) => {
    const requested_at = new Date().toISOString();
    const ready_at = new Date(Date.now() + 600_000).toISOString();
    __nofapMock.state().config.pending = changes.map((change, n) => ({ id: `waiting-${n}`, change, requested_at, ready_at, observed_secs: 0, ready_since: null }));
  };

  it('is disabled and says so until the request is cancelled, and cannot be asked for twice', async () => {
    renderPage();
    const tab = await screen.findByTestId('tab-protection');
    const hosts = () => within(tab).getByRole('switch', { name: 'Lista do sistema' });
    expect(hosts()).toBeEnabled();
    expect(within(tab).queryByTestId('waiting-set_hosts')).toBeNull();

    fireEvent.click(hosts());
    const banner = await screen.findByTestId('pending-banner');
    await waitFor(() => expect(hosts()).toBeDisabled());
    // Still on: nothing changed yet, and the line under it says why it cannot be touched.
    expect(hosts()).toHaveAttribute('aria-checked', 'true');
    expect(within(tab).getByTestId('waiting-set_hosts')).toHaveTextContent(NOTE);
    // A click on it sends nothing.
    fireEvent.click(hosts());
    expect(writes()).toEqual([{ cmd: 'request', args: { kind: 'set_hosts', on: false } }]);
    // The switches that have nothing waiting are as they were.
    expect(within(tab).getByRole('switch', { name: 'Regras dos navegadores' })).toBeEnabled();
    expect(within(tab).getByRole('switch', { name: 'Verificação por IA' })).toBeEnabled();
    expect(screen.getByTestId('nofap-off-button')).toBeEnabled();

    fireEvent.click(within(banner).getByRole('button', { name: 'Manter proteção' }));
    await waitFor(() => expect(hosts()).toBeEnabled());
    expect(within(tab).queryByTestId('waiting-set_hosts')).toBeNull();
    // What the line said: cancelled, the switch is as it was, still on.
    expect(hosts()).toHaveAttribute('aria-checked', 'true');
    expect(__nofapMock.state().config.system.hosts).toBe(true);
  });

  it('covers every switch of the protection tab, and only the scope switch that is being turned off', async () => {
    const scope = { unknown_sites: true, media_apps: false, messaging_apps: false, private_windows: true };
    wait({ kind: 'set_policies', on: false }, { kind: 'set_resolver', resolver: null }, { kind: 'set_vision', on: false }, { kind: 'set_vision_scope', ...scope }, { kind: 'disable' });
    renderPage();
    const tab = await screen.findByTestId('tab-protection');

    expect(within(tab).getByRole('switch', { name: 'Regras dos navegadores' })).toBeDisabled();
    expect(within(tab).getByTestId('waiting-set_policies')).toHaveTextContent(NOTE);
    for (const radio of within(within(tab).getByTestId('protection-tier-resolver')).getAllByRole('radio')) expect(radio).toBeDisabled();
    expect(within(tab).getByTestId('waiting-set_resolver')).toHaveTextContent(NOTE);
    expect(within(tab).getByRole('switch', { name: 'Verificação por IA' })).toBeDisabled();
    expect(within(tab).getByTestId('waiting-set_vision')).toHaveTextContent(NOTE);
    // Nothing waits for the system list.
    expect(within(tab).getByRole('switch', { name: 'Lista do sistema' })).toBeEnabled();
    expect(within(tab).queryByTestId('waiting-set_hosts')).toBeNull();

    // The request narrows media apps (on now, off in the request). Messaging apps are off already, and the two
    // that stay on in the request can still be turned off.
    expect(__nofapMock.state().config.vision).toMatchObject({ unknown_sites: true, media_apps: true, messaging_apps: false, private_windows: true });
    expect(within(tab).getByRole('switch', { name: 'Apps de mídia' })).toBeDisabled();
    expect(within(tab).getByRole('switch', { name: 'Sites desconhecidos' })).toBeEnabled();
    expect(within(tab).getByRole('switch', { name: 'Apps de mensagens' })).toBeEnabled();
    expect(within(tab).getByRole('switch', { name: 'Janelas anônimas' })).toBeEnabled();
    expect(within(tab).getByTestId('waiting-set_vision_scope')).toHaveTextContent('Pedido em espera: o que está ligado aqui continua assim até você confirmar o pedido. Se você cancelar, fica como está.');

    // Turning everything off is waiting too: the button is disabled, next to the sentence the status uses.
    expect(screen.getByTestId('nofap-off-button')).toBeDisabled();
    expect(within(tab).getByTestId('waiting-disable')).toHaveTextContent('Tudo continua ligado até você confirmar.');
    // Nothing was sent by looking.
    expect(writes()).toEqual([]);
  });

  it('covers the switch of the typed sites on the lists tab', async () => {
    wait({ kind: 'set_hosts_user_sites', on: false });
    renderPage(undefined, '/nofap?tab=lists');
    const list = await screen.findByTestId('sites-blocked');
    expect(within(list).getByRole('switch', { name: 'Incluir meus sites na lista do sistema' })).toBeDisabled();
    expect(within(list).getByTestId('waiting-set_hosts_user_sites')).toHaveTextContent(NOTE);
  });

  it('says so in English too', () => {
    expect(translate('en', 'nofap.pending.switch_waiting')).toBe('Request waiting: this stays on until you confirm the request. If you cancel it, this stays as it is.');
    expect(translate('en', 'nofap.pending.scope_waiting')).toBe('Request waiting: what is on here stays on until you confirm the request. If you cancel it, this stays as it is.');
  });

  it('never calls a switch that is off waiting, nor one whose waiting request turns something on', () => {
    const config = __nofapMock.state().config;
    const nothing = { disable: false, hosts: false, resolver: false, policies: false, hostsUserSites: false, vision: false, scope: { unknown_sites: false, media_apps: false, messaging_apps: false, private_windows: false } };
    expect(waitingSwitches(config)).toEqual(nothing);

    wait({ kind: 'set_hosts', on: false }, { kind: 'set_policies', on: true }, { kind: 'set_delay', secs: 0 }, { kind: 'add_allow', host: 'example.com' });
    expect(waitingSwitches(__nofapMock.state().config)).toEqual({ ...nothing, hosts: true });

    // The same requests on a configuration where those things are off already: nothing is waiting to go off.
    const off = structuredClone(__nofapMock.state().config);
    off.system.hosts = false;
    off.system.resolver = null;
    off.vision.enabled = false;
    off.pending.push({ ...off.pending[0]!, id: 'waiting-r', change: { kind: 'set_resolver', resolver: null } }, { ...off.pending[0]!, id: 'waiting-v', change: { kind: 'set_vision', on: false } });
    expect(waitingSwitches(off)).toEqual(nothing);
  });

  it('the same change asked for again is the request that is already waiting', async () => {
    const first = await nofapIpc.request({ kind: 'set_vision_scope', unknown_sites: false, media_apps: true, messaging_apps: false, private_windows: true });
    // The same change with its fields in another order.
    const again = await nofapIpc.request({ kind: 'set_vision_scope', private_windows: true, messaging_apps: false, media_apps: true, unknown_sites: false });
    expect('pending' in first && 'pending' in again && again.pending.id === first.pending.id).toBe(true);
    expect(__nofapMock.state().config.pending).toHaveLength(1);
    // A different change is a request of its own.
    await nofapIpc.request({ kind: 'set_vision_scope', unknown_sites: true, media_apps: false, messaging_apps: false, private_windows: true });
    expect(__nofapMock.state().config.pending).toHaveLength(2);
  });
});

describe('with no waiting time', () => {
  beforeEach(() => {
    resetNofap('on');
    __nofapMock.state().config.delay_secs = 0;
  });

  /** "sem espera" is a label; inside a sentence about waiting it reads as nonsense. */
  const NONSENSE = /(espera|daqui a|em|waits|in) (sem espera|no wait)/i;

  it('never puts "sem espera" inside a sentence, on any tab, and asks for a confirmation in its place', async () => {
    renderPage();
    const protection = await screen.findByTestId('tab-protection');
    expect(protection).toHaveTextContent('Ligar uma camada vale na hora. Desligar pede uma confirmação.');
    expect(protection).toHaveTextContent('Aumentar um limite vale na hora. Baixar pede uma confirmação.');
    expect(document.body.textContent).not.toMatch(NONSENSE);
    await openTab('Listas');
    expect(await screen.findByTestId('sites-blocked')).toHaveTextContent('Adicionar vale na hora. Tirar pede uma confirmação.');
    expect(screen.getByTestId('sites-allowed')).toHaveTextContent('Liberar pede uma confirmação. Tirar daqui vale na hora.');
    expect(document.body.textContent).not.toMatch(NONSENSE);
    for (const tab of ['Apoio', 'Ajuda']) {
      await openTab(tab);
      expect(document.body.textContent).not.toMatch(NONSENSE);
    }
  });

  it('a loosening change is ready at once: the banner asks for the confirmation and no toast talks about waiting', async () => {
    renderPage();
    await screen.findByTestId('tab-protection');
    fireEvent.click(screen.getByRole('switch', { name: 'Lista do sistema' }));
    const banner = await screen.findByTestId('pending-banner');
    expect(banner).toHaveTextContent('O tempo de espera terminou. Confirme para aplicar.');
    expect(document.body.textContent).not.toMatch(NONSENSE);
    expect(document.body.textContent).not.toContain('Pedido anotado');
    // Still on until that confirmation.
    expect(__nofapMock.state().config.system.hosts).toBe(true);
    fireEvent.click(within(banner).getByRole('button', { name: 'Confirmar' }));
    await waitFor(() => expect(__nofapMock.state().config.system.hosts).toBe(false));
  });
});

describe('the pending countdown', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('ticks down locally and asks for a fresh state at zero', () => {
    const onElapsed = vi.fn();
    const base = { id: 'p', kind: 'disable' as const, requested_at: '2026-09-17T15:00:00Z', ready_at: '2026-09-17T15:10:00Z', ready: false, remaining_secs: 3, expires_at: null };
    const fake = { display_name: 'NoFap', config: { pending: [], values: '' }, status: { pending: [base] } } as unknown as NofapView;
    render(<PendingBanner view={fake} sites={null} onCancel={() => undefined} onConfirm={() => undefined} onSurf={() => undefined} onElapsed={onElapsed} />);
    expect(screen.getByTestId('pending-countdown')).toHaveTextContent('0:03');
    act(() => void vi.advanceTimersByTime(1000));
    expect(screen.getByTestId('pending-countdown')).toHaveTextContent('0:02');
    expect(onElapsed).not.toHaveBeenCalled();
    act(() => void vi.advanceTimersByTime(2000));
    expect(screen.getByTestId('pending-countdown')).toHaveTextContent('0:00');
    expect(onElapsed).toHaveBeenCalledTimes(1);
  });

  it('says the truth when the clock ran out before the engine called the request ready, and keeps asking', () => {
    const onElapsed = vi.fn();
    const waiting = { id: 'p', kind: 'disable' as const, requested_at: '2026-09-17T15:00:00Z', ready_at: '2026-09-17T15:10:00Z', ready: false, remaining_secs: 0, expires_at: null };
    const fake = { display_name: 'NoFap', config: { pending: [], values: '' }, status: { pending: [waiting] } } as unknown as NofapView;
    render(<PendingBanner view={fake} sites={null} onCancel={() => undefined} onConfirm={() => undefined} onSurf={() => undefined} onElapsed={onElapsed} />);
    const banner = screen.getByTestId('pending-banner');
    expect(banner).toHaveTextContent('O tempo passou. A espera só conta com o app aberto, então pode faltar um pouco.');
    expect(banner.textContent).not.toMatch(/sem espera/);
    // Not ready: nothing to confirm yet, and the way back is still there.
    expect(within(banner).queryByRole('button', { name: 'Desligar' })).toBeNull();
    expect(within(banner).getByRole('button', { name: 'Manter proteção' })).toBeEnabled();
    expect(onElapsed).not.toHaveBeenCalled();
    act(() => void vi.advanceTimersByTime(RECHECK_MS));
    expect(onElapsed).toHaveBeenCalledTimes(1);
    act(() => void vi.advanceTimersByTime(RECHECK_MS));
    expect(onElapsed).toHaveBeenCalledTimes(2);
  });
});

describe('the limits of the AI check', () => {
  beforeEach(() => resetNofap('on'));

  const field = (name: string) => within(screen.getByTestId('vision-caps')).getByLabelText(name);
  const save = () => fireEvent.click(within(screen.getByTestId('vision-caps')).getByRole('button', { name: 'Salvar limites' }));

  it('raising a limit applies at once', async () => {
    renderPage();
    await screen.findByTestId('vision-caps');
    expect(field('Verificações por dia')).toHaveValue('120');
    expect(field('Gasto por mês (US$)')).toHaveValue('2,00');
    expect(within(screen.getByTestId('vision-caps')).getByRole('button', { name: 'Salvar limites' })).toBeDisabled();
    fireEvent.change(field('Verificações por dia'), { target: { value: '200' } });
    fireEvent.change(field('Gasto por mês (US$)'), { target: { value: '3,5' } });
    save();
    await waitFor(() => expect(__nofapMock.state().config.vision.max_per_day).toBe(200));
    expect(calls('request')).toEqual([{ cmd: 'request', args: { kind: 'set_caps', max_per_hour: 20, max_per_day: 200, monthly_cap_usd: 3.5 } }]);
    expect(__nofapMock.state().config.pending).toEqual([]);
    await waitFor(() => expect(field('Gasto por mês (US$)')).toHaveValue('3,50'));
  });

  it('lowering one waits, and the fields go back to what is in force', async () => {
    renderPage();
    await screen.findByTestId('vision-caps');
    fireEvent.change(field('Verificações por hora'), { target: { value: '5' } });
    save();
    expect(await screen.findByTestId('pending-banner')).toHaveTextContent('Baixar os limites da verificação');
    expect(__nofapMock.state().config.vision.max_per_hour).toBe(20);
    expect(field('Verificações por hora')).toHaveValue('20');
  });

  it('sends nothing outside the ranges of the design', () => {
    const onSave = vi.fn();
    render(<VisionCaps caps={{ max_per_hour: 20, max_per_day: 120, monthly_cap_usd: 2 }} delaySecs={600} onSave={onSave} />);
    const button = screen.getByRole('button', { name: 'Salvar limites' });
    for (const [label, value] of [['Verificações por hora', '61'], ['Verificações por hora', '0'], ['Verificações por hora', '2.5'], ['Verificações por dia', '1001'], ['Gasto por mês (US$)', '0,1'], ['Gasto por mês (US$)', '21'], ['Gasto por mês (US$)', 'muito']] as const) {
      fireEvent.change(screen.getByLabelText(label), { target: { value } });
      expect(button, `${label} = ${value}`).toBeDisabled();
      expect(screen.getByLabelText(label)).toHaveAttribute('aria-invalid', 'true');
      expect(screen.getByTestId('vision-caps')).toHaveTextContent('Use números dentro das faixas indicadas.');
      fireEvent.change(screen.getByLabelText(label), { target: { value: label === 'Gasto por mês (US$)' ? '2,00' : label === 'Verificações por dia' ? '120' : '20' } });
    }
    fireEvent.change(screen.getByLabelText('Verificações por hora'), { target: { value: '60' } });
    fireEvent.click(button);
    expect(onSave.mock.calls).toEqual([[{ max_per_hour: 60, max_per_day: 120, monthly_cap_usd: 2 }]]);
  });
});

describe('exits that never wait (design 6.3): at most two clicks from the main page', () => {
  beforeEach(() => resetNofap('on'));

  it('"Restaurar minha rede": two clicks, no request, no delay', async () => {
    renderPage();
    fireEvent.click(await screen.findByTestId('exit-restore_network')); // 1
    const dialog = await screen.findByRole('dialog', { name: 'Restaurar minha rede' });
    expect(dialog).toHaveTextContent('Não tem espera.');
    fireEvent.click(within(dialog).getByTestId('nofap-restore-network-confirm')); // 2
    await waitFor(() => expect(screen.getByTestId('tier-row-resolver')).toHaveTextContent('Desligado'));
    expect(writes()).toEqual([{ cmd: 'restore_network' }]);
    expect(__nofapMock.state().config.pending).toEqual([]);
    // The other layers stay, and the way back is offered.
    expect(screen.getByTestId('tier-row-hosts')).toHaveTextContent('Aplicado');
    expect(screen.getByTestId('nofap-status')).toHaveTextContent('Você restaurou a rede. O DNS filtrado fica fora até você religar.');
    expect(screen.getByTestId('nofap-apply')).toHaveTextContent('Religar');
  });

  it('"Apagar meu histórico": two clicks, no request, the protection untouched', async () => {
    renderPage();
    fireEvent.click(await screen.findByTestId('exit-erase')); // 1
    const dialog = await screen.findByRole('dialog', { name: 'Apagar meu histórico do NoFap' });
    // The dialog says everything that goes: what the person wrote is erased too.
    expect(dialog).toHaveTextContent('Apaga as contagens, as pausas de regra, a sua frase e os seus planos. Suas listas e a proteção ficam como estão. Não tem espera.');
    expect(__nofapMock.state().config.values).not.toBe('');
    fireEvent.click(within(dialog).getByTestId('nofap-erase-confirm')); // 2
    await waitFor(() => expect(writes()).toEqual([{ cmd: 'erase_history' }]));
    await waitFor(() => expect(screen.getByTestId('nofap-status')).toHaveTextContent('Hoje, 0 páginas barradas.'));
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Proteção ativa');
    expect(__nofapMock.state().config.values).toBe('');
    expect(__nofapMock.state().config.plans).toEqual([]);
    expect(__nofapMock.state().config.system.hosts).toBe(true);
  });

  it('"Apagar meu histórico" names the sentence and the plans in both languages, and no longer says they stay', () => {
    const pt = translate('pt-BR', 'nofap.data.erase_body');
    const en = translate('en', 'nofap.data.erase_body');
    expect(pt).toBe('Apaga as contagens, as pausas de regra, a sua frase e os seus planos. Suas listas e a proteção ficam como estão. Não tem espera.');
    expect(en).toBe('Erases the counts, the rule snoozes, your sentence and your plans. Your lists and the protection stay as they are. There is no wait.');
    // The sentence that says what stays names neither.
    expect(pt.split('. ')[1]).not.toMatch(/frase|plano/);
    expect(en.split('. ')[1]).not.toMatch(/sentence|plan/);
  });

  it('removal without the app and quitting are explained one click away', async () => {
    renderPage();
    fireEvent.click(await screen.findByTestId('exit-info:remove')); // 1
    expect(await screen.findByTestId('info-remove')).toHaveTextContent('Quem tem a senha de administrador desfaz tudo à mão');
    fireEvent.click(within(screen.getByRole('dialog', { name: 'Como remover sem o app' })).getByText('Fechar'));
    fireEvent.click(screen.getByTestId('exit-info:quit')); // 1
    expect(await screen.findByTestId('info-quit')).toHaveTextContent('Sair é sempre possível');
  });

  it("stopping the AI check is two clicks away, in the host's own settings", async () => {
    const openSettings = vi.fn();
    renderPage({ openSettings });
    fireEvent.click(await screen.findByTestId('exit-info:ai')); // 1
    expect(await screen.findByTestId('info-ai')).toHaveTextContent('A verificação para na hora, sem espera, quando você remove a chave de IA nas configurações do app.');
    fireEvent.click(screen.getByTestId('nofap-open-settings')); // 2
    expect(openSettings).toHaveBeenCalledTimes(1);
  });

  it('cancelling a waiting request is one click', async () => {
    resetNofap('pending');
    renderPage();
    fireEvent.click(within(await screen.findByTestId('pending-banner')).getByRole('button', { name: 'Manter proteção' })); // 1
    await waitFor(() => expect(calls('cancel_pending')).toHaveLength(1));
  });
});

describe('support flows', () => {
  beforeEach(() => resetNofap('help'));

  it('lapse: only the user starts it; triggers are optional; it ends on the next small step', async () => {
    renderPage();
    await openTab('Apoio');
    fireEvent.click(await screen.findByTestId('nofap-lapse'));
    const flow = await screen.findByTestId('lapse-flow');
    expect(flow).toHaveTextContent('Aconteceu. Um deslize não apaga o que você já construiu.');
    expect(flow).toHaveTextContent('Fale com você como falaria com um amigo na mesma situação.');
    expect(calls('record')).toEqual([]);
    fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
    expect(screen.getByTestId('lapse-flow')).toHaveTextContent('Sem culpa, só curiosidade');
    fireEvent.click(screen.getByRole('button', { name: 'Cansaço' }));
    fireEvent.click(screen.getByRole('button', { name: 'Solidão' }));
    fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
    await waitFor(() => expect(calls('record')).toEqual([{ cmd: 'record', args: { event: 'lapse', triggers: ['tired', 'lonely'] } }]));
    expect(screen.getByTestId('lapse-flow')).toHaveTextContent('Quer ajustar um plano "se… então…"');
    fireEvent.click(screen.getByRole('button', { name: 'Pular' }));
    expect(screen.getByTestId('lapse-flow')).toHaveTextContent('O próximo passo é pequeno');
  });

  it('lapse: one tap counts, once, even when the flow is closed before the question about triggers', async () => {
    renderPage();
    await openTab('Apoio');
    fireEvent.click(await screen.findByTestId('nofap-lapse'));
    await screen.findByTestId('lapse-flow');
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => expect(calls('record')).toEqual([{ cmd: 'record', args: { event: 'lapse', triggers: [] } }]));
    // The sentence about the count follows from it.
    await waitFor(() => expect(screen.getByTestId('progress-card')).toHaveTextContent('A contagem recomeça, a prática não.'));

    // A second flow, walked to the end, counts once more and not twice.
    fireEvent.click(screen.getByTestId('nofap-lapse'));
    fireEvent.click(await screen.findByRole('button', { name: 'Continuar' }));
    fireEvent.click(screen.getByRole('button', { name: 'Tédio' }));
    fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
    fireEvent.click(screen.getByRole('button', { name: 'Pular' }));
    fireEvent.click(within(screen.getByTestId('lapse-flow')).getByRole('button', { name: 'Fechar' }));
    await waitFor(() => expect(screen.queryByTestId('lapse-flow')).toBeNull());
    expect(calls('record').map((c) => c.args)).toEqual([
      { event: 'lapse', triggers: [] },
      { event: 'lapse', triggers: ['bored'] },
    ]);
  });

  it('check-in: one tap; on a hard day the HALT chips; shown back afterwards', async () => {
    renderPage();
    await openTab('Apoio');
    const card = await screen.findByTestId('checkin-card');
    expect(card).toHaveTextContent('Como você está hoje? Um toque basta.');
    fireEvent.click(within(card).getByRole('button', { name: 'Difícil' }));
    expect(card).toHaveTextContent('Checagem rápida: fome, raiva, solidão ou cansaço?');
    fireEvent.click(within(card).getByRole('button', { name: 'Fome' }));
    fireEvent.click(within(card).getByRole('button', { name: 'Salvar' }));
    await waitFor(() => expect(calls('record')).toEqual([{ cmd: 'record', args: { event: 'checkin', mood: 'hard', triggers: ['hungry'] } }]));
    await waitFor(() => expect(screen.getByTestId('checkin-card')).toHaveTextContent('Hoje você marcou: Difícil.'));
  });

  it('check-in: a good day is recorded with one tap and no chips', () => {
    const onCheckin = vi.fn();
    render(<CheckinCard today={null} onCheckin={onCheckin} />);
    fireEvent.click(screen.getByRole('button', { name: 'Bem' }));
    expect(onCheckin).toHaveBeenCalledWith('ok', []);
  });

  it('the check-in card is there only when the user asked for it', async () => {
    resetNofap('on');
    renderPage();
    await openTab('Apoio');
    await screen.findByTestId('progress-card');
    expect(screen.queryByTestId('checkin-card')).toBeNull();
    fireEvent.click(screen.getByRole('switch', { name: 'Check-in diário' }));
    expect(await screen.findByTestId('checkin-card')).toBeInTheDocument();
  });

  it('values and plans: one sentence, up to three plans, saved at once', async () => {
    resetNofap('on');
    renderPage();
    await openTab('Apoio');
    const card = await screen.findByTestId('values-and-plans');
    expect(card).toHaveTextContent('Se eu sentir vontade quando estiver sem sono de madrugada, então eu vou levantar e beber água.');
    fireEvent.change(within(card).getByLabelText('Sua frase'), { target: { value: 'Quero dormir bem.' } });
    fireEvent.click(within(card).getByRole('button', { name: 'Salvar' }));
    await waitFor(() => expect(__nofapMock.state().config.values).toBe('Quero dormir bem.'));
    for (const [when, then] of [['estiver só em casa', 'ligar para alguém'], ['ficar no celular na cama', 'deixar o celular na sala']]) {
      fireEvent.change(within(card).getByLabelText('Se eu sentir vontade quando'), { target: { value: when } });
      fireEvent.change(within(card).getByLabelText('então eu vou'), { target: { value: then } });
      fireEvent.click(within(card).getByRole('button', { name: 'Adicionar plano' }));
      await waitFor(() => expect(screen.getByTestId('values-and-plans')).toHaveTextContent(then));
    }
    expect(__nofapMock.state().config.plans).toHaveLength(3);
    expect(screen.getByTestId('values-and-plans')).toHaveTextContent('Até três planos.');
    expect(within(screen.getByTestId('values-and-plans')).queryByRole('button', { name: 'Adicionar plano' })).toBeNull();
    // None of this waits.
    expect(__nofapMock.state().config.pending).toEqual([]);
  });

  it('progress: days in the window; the streak only when asked for; the count restarts after a lapse, the practice does not', () => {
    const { rerender } = render(<ProgressCard progress={{ window_days: 30, days: 26, streak: null, after_reset: false }} />);
    expect(screen.getByTestId('progress-card')).toHaveTextContent('Nos últimos 30 dias, 26 foram do jeito que você queria.');
    expect(screen.queryByTestId('progress-streak')).toBeNull();
    rerender(<ProgressCard progress={{ window_days: 30, days: 26, streak: 7, after_reset: false }} />);
    expect(screen.getByTestId('progress-streak')).toHaveTextContent('Sequência atual: 7 dias.');
    expect(screen.getByTestId('progress-card')).toHaveTextContent('Uma semana. Sem fogos, só um aceno: você está praticando.');
    rerender(<ProgressCard progress={{ window_days: 30, days: 19, streak: 0, after_reset: true }} />);
    expect(screen.getByTestId('progress-card')).toHaveTextContent('A contagem recomeça, a prática não. Nos últimos 30 dias, 19 foram do jeito que você queria.');
    expect(screen.getByTestId('progress-streak')).toHaveTextContent('Sequência atual: 0 dias.');
  });

  it('professional help: shown when due, says it is not a diagnosis, and lists where to go', async () => {
    renderPage();
    await openTab('Apoio');
    const card = await screen.findByTestId('help-card');
    expect(card).toHaveTextContent('Isso não é um diagnóstico.');
    fireEvent.click(within(card).getByRole('button', { name: 'Ver opções de ajuda' }));
    expect(within(card).getByTestId('resource-list')).toHaveTextContent('CVV');
    expect(card).toHaveTextContent('Eu sou um mascote, não um terapeuta.');
  });

  it('a person to talk to is one step from the page: the panic sheet and the help tab', async () => {
    renderPage();
    fireEvent.click(await screen.findByTestId('nofap-panic'));
    const sheet = await screen.findByTestId('panic-sheet');
    expect(sheet).toHaveTextContent('Estou aqui. Você fez bem em apertar.');
    await waitFor(() => expect(calls('record')).toEqual([{ cmd: 'record', args: { event: 'panic' } }]));
    fireEvent.click(within(sheet).getByRole('button', { name: 'Falar com alguém' }));
    expect(await screen.findByTestId('crisis-card')).toHaveTextContent('ligue 188. É o CVV: grátis, 24 horas. Também há chat em cvv.org.br/chat. Em emergência, ligue 192.');
  });
});

describe('lists', () => {
  beforeEach(() => resetNofap('on'));

  it('adds a site to stop at once, makes an allowance wait, and never shows a learned name', async () => {
    renderPage();
    await openTab('Listas');
    const blocked = await screen.findByTestId('sites-blocked');
    expect(blocked).toHaveTextContent('Sites barrados');
    fireEvent.change(within(blocked).getByLabelText('Site para barrar'), { target: { value: 'https://www.example.net/page' } });
    fireEvent.click(within(blocked).getByRole('button', { name: 'Barrar site' }));
    await waitFor(() => expect(screen.getByTestId('sites-blocked')).toHaveTextContent('example.net'));

    const allowed = screen.getByTestId('sites-allowed');
    fireEvent.change(within(allowed).getByLabelText('Site para liberar'), { target: { value: 'example.edu' } });
    fireEvent.click(within(allowed).getByRole('button', { name: 'Pedir liberação' }));
    const banner = await screen.findByTestId('pending-banner');
    expect(banner).toHaveTextContent('Liberar example.edu');
    expect(__nofapMock.state().sites.allowed.map((s) => s.host)).toEqual(['example.org']);

    expect(screen.getByTestId('sites-learned')).toHaveTextContent('3 sites foram aprendidos pela verificação por IA. Os nomes não aparecem em lugar nenhum.');
    // A site the user adds keeps its own row and its own id next to the ones that were there.
    const ids = __nofapMock.state().sites.blocked.map((s) => s.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(within(screen.getByTestId('sites-blocked')).getAllByRole('listitem')).toHaveLength(2);
  });

  it('says where the typed sites go, next to the list they belong to, and lets them stay in the app only', async () => {
    renderPage();
    await openTab('Listas');
    const blocked = await screen.findByTestId('sites-blocked');
    expect(blocked).toHaveTextContent('Os sites que você adiciona entram na lista do sistema, que outras contas deste computador conseguem ler.');
    fireEvent.click(within(blocked).getByRole('switch', { name: 'Incluir meus sites na lista do sistema' }));
    // Taking them out of the system list loosens the protection: it waits.
    expect(await screen.findByTestId('pending-banner')).toHaveTextContent('Tirar seus sites da lista do sistema');
    expect(__nofapMock.state().config.system.hosts_user_sites).toBe(true);
  });

  it('refuses something that is not an address without sending it', async () => {
    renderPage();
    await openTab('Listas');
    const blocked = await screen.findByTestId('sites-blocked');
    fireEvent.change(within(blocked).getByLabelText('Site para barrar'), { target: { value: 'not an address' } });
    fireEvent.click(within(blocked).getByRole('button', { name: 'Barrar site' }));
    expect(within(blocked).getByRole('alert')).toHaveTextContent('Digite um endereço como exemplo.com.');
    expect(calls('request')).toEqual([]);
  });
});

describe('the stored settings could not be read (integrator decision 33)', () => {
  beforeEach(() => resetNofap('unreadable'));

  const SENTENCE_PT = 'Não consegui ler as configurações salvas: o que já estava aplicado no sistema continua valendo, e o bloqueio no app e a verificação por IA ficam parados até você restaurar os padrões.';
  const SENTENCE_EN = 'I couldn\'t read the saved settings: what was already applied to the system still holds, and the in-app block and the AI check stay off until you restore the defaults.';

  it('says so in one calm sentence, with "Restaurar padrões" as its one action, and offers nothing that would be refused', async () => {
    renderPage();
    const card = await screen.findByTestId('nofap-unreadable');
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Proteção parcial');
    const issue = within(card).getByTestId('issue-config_unreadable');
    expect(issue).toHaveTextContent(SENTENCE_PT);
    expect(within(card).getAllByRole('button').map((b) => b.textContent)).toEqual(['Restaurar padrões']);
    // Not the wizard of a module nobody enabled, and not the page of a module that is on.
    expect(screen.queryByTestId('wizard-step-1')).toBeNull();
    expect(screen.queryByTestId('nofap-status')).toBeNull();
    // A person to talk to stays one step away.
    expect(screen.getByTestId('nofap-talk')).toBeInTheDocument();
    expect(writes()).toEqual([]);
  });

  it('says it in English too', async () => {
    setLocale('en');
    renderPage();
    const issue = await screen.findByTestId('issue-config_unreadable');
    expect(issue).toHaveTextContent(SENTENCE_EN);
    expect(within(issue).getByRole('button')).toHaveTextContent('Restore defaults');
    fireEvent.click(within(issue).getByRole('button'));
    const dialog = await screen.findByRole('dialog', { name: 'Restore defaults' });
    expect(dialog).toHaveTextContent('Starts everything over with NoFap off: erases the settings, your lists, your sentence, your plans and the counts. What is applied to the system stays as it is, and you set NoFap up again right after.');
  });

  it('"Restaurar padrões" says what it erases before the reset, and then the set-up starts again', async () => {
    renderPage();
    fireEvent.click(within(await screen.findByTestId('issue-config_unreadable')).getByRole('button', { name: 'Restaurar padrões' }));
    const dialog = await screen.findByRole('dialog', { name: 'Restaurar padrões' });
    expect(dialog).toHaveTextContent('Volta tudo ao começo, com o NoFap desligado: apaga as configurações, as suas listas, a sua frase, os seus planos e as contagens. O que está aplicado no sistema fica como está, e você configura o NoFap de novo em seguida.');
    // Nothing happens before the confirmation.
    expect(writes()).toEqual([]);
    fireEvent.click(within(dialog).getByTestId('nofap-restore-defaults-confirm'));
    await waitFor(() => expect(writes()).toEqual([{ cmd: 'reset' }]));
    expect(await screen.findByTestId('wizard-step-1')).toBeInTheDocument();
    expect(screen.queryByTestId('nofap-unreadable')).toBeNull();
    expect(__nofapMock.state().unreadable).toBe(false);
  });

  it('cancelling the dialog changes nothing', async () => {
    renderPage();
    fireEvent.click(within(await screen.findByTestId('issue-config_unreadable')).getByRole('button', { name: 'Restaurar padrões' }));
    const dialog = await screen.findByRole('dialog', { name: 'Restaurar padrões' });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Cancelar' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(writes()).toEqual([]);
    expect(screen.getByTestId('issue-config_unreadable')).toBeInTheDocument();
  });

  it('reads the settings again when the engine could read them after all, even with the same overall state', async () => {
    renderPage();
    await screen.findByTestId('nofap-unreadable');
    // The engine's verify pass read the stored row: a module that is on, with a tier not in place ("reduced" both
    // before and after). The status event alone carries no configuration, so the page has to ask for it.
    __nofapMock.reset('reduced');
    act(() => __nofapMock.setStatus(null));
    expect(await screen.findByTestId('nofap-status')).toBeInTheDocument();
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Proteção parcial');
    expect(screen.queryByTestId('nofap-unreadable')).toBeNull();
    expect(screen.queryByTestId('wizard-step-1')).toBeNull();
  });

  it('the mock refuses what the engine refuses while the settings cannot be read', async () => {
    await expect(nofapIpc.request({ kind: 'set_values', text: 'x' })).rejects.toThrow(/cannot be read/);
    await expect(nofapIpc.export()).rejects.toThrow(/cannot be read/);
    await expect(nofapIpc.systemApply()).rejects.toThrow(/cannot be read/);
    await expect(nofapIpc.consentVision({ provider: 'anthropic', managed: false })).rejects.toThrow(/cannot be read/);
  });
});

describe('the two products (integrator decision 32)', () => {
  it('says how the AI check stops at once in words true in both, and adds local-only mode only in ubiqX', async () => {
    resetNofap('on');
    const { unmount } = renderPage();
    await openTab('Ajuda');
    const ai = await screen.findByTestId('help-ai');
    expect(ai).toHaveTextContent('A verificação para na hora, sem espera, quando você remove a chave de IA nas configurações do app. O modo só local, também nas configurações, faz o mesmo.');
    unmount();

    resetNofap('on', 'shield');
    renderPage();
    await openTab('Ajuda');
    const standalone = await screen.findByTestId('help-ai');
    expect(standalone).toHaveTextContent('A verificação para na hora, sem espera, quando você remove a chave de IA nas configurações do app.');
    expect(standalone).not.toHaveTextContent('modo só local');
    fireEvent.click(screen.getByTestId('exit-info:ai'));
    expect(await screen.findByTestId('info-ai')).not.toHaveTextContent('modo só local');
  });

  it('shows the line about pausing ubiqX in the consent dialog of ubiqX only, in both languages', async () => {
    for (const [locale, line] of [
      ['pt-BR', 'Pausar o ubiqX ou usar o modo privado não pausa esta verificação.'],
      ['en', 'Pausing ubiqX or using private mode does not pause this check.'],
    ] as const) {
      for (const product of ['ubiqx', 'shield'] as const) {
        resetNofap('on', product);
        __nofapMock.setAi('openai');
        setLocale(locale);
        const { unmount } = renderPage();
        fireEvent.click(within(await screen.findByTestId('issue-list')).getByRole('button'));
        const consent = await screen.findByTestId('vision-consent');
        if (product === 'ubiqx') expect(consent, `${locale} ${product}`).toHaveTextContent(line);
        else expect(consent.textContent, `${locale} ${product}`).not.toMatch(/ubiqx/i);
        unmount();
      }
    }
  });
});
