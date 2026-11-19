import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SURF_STEP_SECS, SURF_STEPS, SurfTimer } from './components/SurfTimer';
import { views } from './fixtures/golden';
import { cardSize } from './labels';
import { __nofapMock } from './mock';
import { calls, renderCard, resetNofap } from './testing';
import type { NofapCard, UserEvent } from './types';

/** Shows a fixture card in the window, the way the engine does: remembered under an id, opened by that id. */
async function open(kind: NofapCard['kind'], patch: Partial<NofapCard> = {}) {
  const card = { ...__nofapMock.sampleCard(kind), ...patch };
  __nofapMock.present(card);
  const utils = renderCard(card.id);
  await screen.findByTestId('card-message');
  return { card, ...utils };
}

const buttons = () => within(screen.getByTestId('card-actions')).getAllByRole('button').map((b) => b.textContent);

describe('the card window', () => {
  beforeEach(() => resetNofap('on'));

  it('discreet block: only "Página bloqueada pelo UBI." and its two buttons', async () => {
    await open('blocked');
    expect(screen.getByTestId('card-message')).toHaveTextContent(/^Página bloqueada pelo UBI\.$/);
    expect(buttons()).toEqual(['Abrir apoio', 'Ok']);
    // Nothing else is written in the window: no reason, no sentence, no site.
    expect(screen.getByTestId('page-nofap-card').textContent).toBe('Página bloqueada pelo UBI.Abrir apoioOk');
    expect(calls('support')).toEqual([]);
  });

  it('an unknown id gives the neutral card, never a sample', async () => {
    renderCard('no-such-card');
    expect(await screen.findByTestId('card-message')).toHaveTextContent(/^Página bloqueada pelo UBI\.$/);
    expect(buttons()).toEqual(['Abrir apoio', 'Ok']);
  });

  it('shows no host of the state in any card, in any view of it', async () => {
    const hosts = [...__nofapMock.state().sites.blocked, ...__nofapMock.state().sites.allowed].map((s) => s.host);
    expect(hosts).toEqual(['example.com', 'example.org']);
    for (const fixture of views.cards) {
      const { unmount } = await open(fixture.kind);
      const texts = [document.body.textContent ?? ''];
      // Every button that opens something inside the window is pressed once, and the result is read too.
      for (const label of ['Abrir apoio', 'Não era isso', 'Pedir exceção', 'Ver opções de ajuda', 'Como desligar']) {
        const button = screen.queryByRole('button', { name: label });
        if (!button) continue;
        fireEvent.click(button);
        await waitFor(() => expect(screen.queryByTestId('card-actions')).toBeNull());
        texts.push(document.body.textContent ?? '');
        break;
      }
      for (const text of texts) {
        for (const host of [...hosts, 'example.net']) expect(text.includes(host), `${fixture.kind}: "${host}"`).toBe(false);
        expect(text).not.toMatch(/nofap\.|\{\w+\}/);
      }
      unmount();
    }
  });

  it('gives every card of the fixtures its message, its buttons and one primary button', async () => {
    const expected: Record<NofapCard['kind'], [RegExp, string[]]> = {
      blocked: [/^Página bloqueada pelo UBI\.$/, ['Abrir apoio', 'Ok']],
      blocked_repeat: [/^Terceira tentativa em pouco tempo\./, ['Ok', 'Fazer a onda de 3 min']],
      vision_closed: [/^Fechei a aba, como você pediu que eu fizesse\./, ['Não era isso', 'Respirar 3 min', 'Adicionar à minha lista', 'Ok']],
      vision_ask: [/^Posso estar enganado/, ['Pedir exceção', 'Fechar a janela']],
      drift: [/^A proteção do sistema mudou\. Quer restaurar\?$/, ['Agora não', 'Restaurar']],
      pending_ready: [/^O tempo de espera terminou\. Ainda quer desligar\?$/, ['Desligar', 'Manter proteção']],
      resolver_unreachable: [/^Os nomes de sites não estão resolvendo\./, ['Agora não', 'Restaurar minha rede']],
      reminder: [/^O NoFap está ligado neste computador\. Veja como desligar\.$/, ['Como desligar', 'Ok']],
      help: [/^Tem sido difícil várias vezes nesta semana\./, ['Ok', 'Ver opções de ajuda']],
    };
    for (const fixture of views.cards) {
      const { unmount } = await open(fixture.kind);
      const [message, labels] = expected[fixture.kind];
      expect(screen.getByTestId('card-message')).toHaveTextContent(message);
      expect(buttons(), fixture.kind).toEqual(labels);
      expect(screen.getByTestId('card-actions').querySelectorAll('.bg-volt')).toHaveLength(1);
      unmount();
    }
    // The second line of the vision card.
    await open('vision_closed');
    expect(screen.getByTestId('card-note')).toHaveTextContent('Guardei esse site pra ele não te pegar de surpresa de novo.');
  });

  it('"Não era isso" tells the engine and says it was noted', async () => {
    const { card } = await open('vision_closed');
    fireEvent.click(screen.getByRole('button', { name: 'Não era isso' }));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent(/^Anotado\. Vou deixar passar por enquanto\.$/));
    expect(calls('card_action')).toEqual([{ cmd: 'card_action', args: { id: card.id, action: 'not_it' } }]);
    expect(__nofapMock.state().config.pending).toEqual([]);
  });

  it('"Não era isso" on a list hit becomes a request that waits, and the card says so without naming the site', async () => {
    await open('blocked', { discreet: false, message_key: 'block.default', actions: ['not_it', 'ok'], auto_close_secs: null });
    fireEvent.click(screen.getByRole('button', { name: 'Não era isso' }));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent(/^Esse site está na lista\. Pedi a exceção; ela vale daqui a 10 minutos\.$/));
    expect(__nofapMock.state().config.pending.map((p) => p.change.kind)).toEqual(['add_allow']);
  });

  it('"Pedir exceção" creates the delayed request', async () => {
    await open('vision_ask');
    fireEvent.click(screen.getByRole('button', { name: 'Pedir exceção' }));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent('Pedi a exceção; ela vale daqui a 10 minutos.'));
    expect(__nofapMock.state().config.pending).toHaveLength(1);
  });

  it('"Abrir apoio" opens the support card: a sentence, the user\'s own words, and three ways through', async () => {
    const { card } = await open('blocked');
    fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
    const support = await screen.findByTestId('card-support');
    expect(calls('card_action')[0]?.args).toEqual({ id: card.id, action: 'support' });
    expect(screen.getByTestId('card-message').textContent).toMatch(/^(Bloqueado, como combinamos\.|Você decidiu isso num momento de calma\.|A vontade veio\.)/);
    await waitFor(() => expect(screen.getByTestId('page-nofap-card')).toHaveTextContent('Quero estar presente com quem eu amo.'));
    expect(screen.getByTestId('page-nofap-card')).toHaveTextContent('Se eu sentir vontade quando estiver sem sono de madrugada, então eu vou levantar e beber água.');
    expect(within(support).getAllByRole('button').map((b) => b.textContent)).toEqual(['Respirar 3 min', 'Sair da tela por 10 min', 'Falar com alguém', 'Fechar']);
  });

  it('with detailed messages on, a block opens directly as the support card', async () => {
    await open('blocked', { discreet: false, message_key: 'block.breathe', auto_close_secs: null });
    expect(screen.getByTestId('card-message')).toHaveTextContent('Bloqueado, como combinamos. Respira comigo por um minuto?');
    expect(screen.getByTestId('card-support')).toBeInTheDocument();
  });

  it('the crisis card shows the phone number of the resources payload', async () => {
    __nofapMock.setSupport({
      resources: [
        { id: 'cvv', country: 'BR', name: 'Linha de Apoio', phone: '0800 000 0000', url: 'https://apoio.example/chat', note_pt: 'Grátis, 24 horas.', note_en: 'Free, 24 hours.' },
        { id: 'samu', country: 'BR', name: 'Emergência', phone: '0800 111 1111', url: null, note_pt: 'Emergência médica.', note_en: 'Medical emergency.' },
        { id: 'findahelpline', country: '', name: 'Diretório', phone: null, url: 'https://diretorio.example', note_pt: 'Outros países.', note_en: 'Other countries.' },
      ],
    });
    await open('blocked');
    fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Falar com alguém' }));
    const crisis = await screen.findByTestId('crisis-card');
    await waitFor(() => expect(crisis).toHaveTextContent('ligue 0800 000 0000. É o Linha de Apoio: grátis, 24 horas. Também há chat em apoio.example/chat. Em emergência, ligue 0800 111 1111.'));
    expect(within(crisis).getByTestId('resource-list')).toHaveTextContent('0800 000 0000');
  });

  it('says nothing half-filled when the crisis line is missing from the payload', async () => {
    __nofapMock.setSupport({ resources: [] });
    await open('blocked');
    fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Falar com alguém' }));
    const crisis = await screen.findByTestId('crisis-card');
    expect(crisis).toHaveTextContent('A lista de contatos de apoio não veio nesta versão do app.');
    expect(crisis.textContent).not.toMatch(/ligue/);
  });

  it('answers a ready request from the card: keep, or turn off', async () => {
    resetNofap('ready');
    await open('pending_ready');
    fireEvent.click(screen.getByRole('button', { name: 'Manter proteção' }));
    await screen.findByTestId('card-closed');
    expect(__nofapMock.state().config.pending).toEqual([]);
    expect(__nofapMock.state().config.enabled).toBe(true);
  });

  it('closes when the engine takes the card away, and shows the next one it sends', async () => {
    const { card } = await open('drift');
    act(() => void __nofapMock.present(__nofapMock.sampleCard('reminder')));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent('O NoFap está ligado neste computador.'));
    fireEvent.click(screen.getByRole('button', { name: 'Como desligar' }));
    expect(await screen.findByTestId('card-how-off')).toHaveTextContent('Em Proteção, peça o desligamento.');
    expect(card.kind).toBe('drift');
  });

  it('asks for the size of the design for each card', () => {
    const size = (kind: NofapCard['kind'], discreet = false, expanded = false) => cardSize({ kind, discreet }, expanded);
    expect(size('blocked', true)).toEqual({ width: 420, height: 132 });
    expect(size('blocked', true, true)).toEqual({ width: 420, height: 560 });
    expect(size('vision_closed')).toEqual({ width: 420, height: 220 });
    expect(size('vision_ask')).toEqual({ width: 420, height: 200 });
    expect(size('drift')).toEqual({ width: 420, height: 200 });
    expect(size('pending_ready')).toEqual({ width: 420, height: 220 });
    expect(size('resolver_unreachable')).toEqual({ width: 420, height: 220 });
    expect(size('reminder')).toEqual({ width: 420, height: 160 });
  });
});

describe('the card window closes by itself only while untouched', () => {
  beforeEach(() => {
    resetNofap('on');
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });
  afterEach(() => vi.useRealTimers());

  it('after the seconds the card names', async () => {
    await open('blocked');
    act(() => void vi.advanceTimersByTime(11_000));
    expect(screen.queryByTestId('card-closed')).toBeNull();
    act(() => void vi.advanceTimersByTime(1_500));
    expect(screen.getByTestId('card-closed')).toBeInTheDocument();
  });

  it('not once the user opened support', async () => {
    await open('blocked');
    fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
    await screen.findByTestId('card-support');
    act(() => void vi.advanceTimersByTime(60_000));
    expect(screen.queryByTestId('card-closed')).toBeNull();
  });
});

describe('urge surf', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('advances by itself every 30 seconds, shows the rating back and sends none', () => {
    const events: UserEvent[] = [];
    const onDone = vi.fn();
    render(<SurfTimer record={(e) => events.push(e)} onDone={onDone} />);
    const message = () => screen.getByTestId('surf-message').textContent;
    expect(events).toEqual([{ event: 'surf_started' }]);
    expect(message()).toBe('Vamos surfar essa vontade juntos. São 3 minutos. Você não precisa lutar contra ela, só observar.');
    act(() => void vi.advanceTimersByTime((SURF_STEP_SECS - 1) * 1000));
    expect(message()).toContain('Vamos surfar');
    act(() => void vi.advanceTimersByTime(1000));
    expect(message()).toBe('Onde você sente a vontade no corpo? Peito, barriga, mãos? Só repare.');
    act(() => void vi.advanceTimersByTime(SURF_STEP_SECS * 1000));
    expect(message()).toContain('Inspira contando até 4');
    act(() => void vi.advanceTimersByTime(SURF_STEP_SECS * 1000));
    expect(message()).toContain('Vontade é onda');
    act(() => void vi.advanceTimersByTime(SURF_STEP_SECS * 1000));
    expect(message()).toContain('Pensamento não é ordem.');
    act(() => void vi.advanceTimersByTime(SURF_STEP_SECS * 1000));
    expect(SURF_STEPS).toHaveLength(5);
    expect(message()).toBe('De 0 a 10, como está a vontade agora?');
    // The rating waits for the user: time alone does not move past it.
    act(() => void vi.advanceTimersByTime(120_000));
    expect(message()).toBe('De 0 a 10, como está a vontade agora?');

    fireEvent.click(screen.getByRole('radio', { name: '7' }));
    expect(screen.getByTestId('surf-timer')).toHaveTextContent('Você marcou 7. Fica só aqui, nesta tela.');
    fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
    expect(message()).toBe('Pronto. Você ficou com a vontade sem obedecer. Isso é treino, e conta.');
    fireEvent.click(screen.getByRole('button', { name: 'Concluir' }));
    expect(onDone).toHaveBeenCalledTimes(1);
    // Two counters, and no number anywhere in what left the component.
    expect(events).toEqual([{ event: 'surf_started' }, { event: 'surf_done' }]);
    expect(JSON.stringify(events)).not.toContain('7');
  });
});
