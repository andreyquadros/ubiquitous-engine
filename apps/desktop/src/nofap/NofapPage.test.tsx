import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CheckinCard } from './components/CheckinCard';
import { PendingBanner } from './components/PendingBanner';
import { ProgressCard } from './components/ProgressCard';
import { __nofapMock } from './mock';
import { useNofapStore } from './store';
import { calls, READS, renderPage, resetNofap } from './testing';
import type { NofapView } from './types';

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
    fireEvent.click(within(dialog).getByTestId('nofap-erase-confirm')); // 2
    await waitFor(() => expect(writes()).toEqual([{ cmd: 'erase_history' }]));
    await waitFor(() => expect(screen.getByTestId('nofap-status')).toHaveTextContent('Hoje, 0 páginas barradas.'));
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Proteção ativa');
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
    expect(await screen.findByTestId('info-ai')).toHaveTextContent('param a verificação na hora, sem espera');
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
    expect(screen.getByTestId('sites-learned')).toHaveTextContent('outras contas deste computador conseguem ler');
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
