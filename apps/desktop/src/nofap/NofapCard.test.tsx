import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter, useNavigate } from 'react-router-dom';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { SURF_STEP_SECS, SURF_STEPS, SurfTimer } from './components/SurfTimer';
import { views } from './fixtures/golden';
import { cardSize } from './labels';
import { nofapIpc } from './ipc';
import { __nofapMock } from './mock';
import { translate } from '../i18n';
import { CARD_DONE_CLOSE_MS, cardText, NofapCardPage, requestedText } from './NofapCard';
import { calls, renderCard, resetNofap } from './testing';
import type { NofapCard, SupportView, UserEvent } from './types';

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

  it('"Não era isso" that becomes a request that waits says so, without naming the site or a list', async () => {
    await open('blocked', { discreet: false, message_key: 'block.default', actions: ['not_it', 'ok'], auto_close_secs: null });
    fireEvent.click(screen.getByRole('button', { name: 'Não era isso' }));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent(/^Pedi a exceção; ela vale daqui a 10 minutos\.$/));
    expect(__nofapMock.state().config.pending.map((p) => p.change.kind)).toEqual(['add_allow']);
  });

  it('"Pedir exceção" creates the delayed request', async () => {
    await open('vision_ask');
    fireEvent.click(screen.getByRole('button', { name: 'Pedir exceção' }));
    // An unsure verdict on a site no list knows: the card does not say it is on one.
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent(/^Pedi a exceção; ela vale daqui a 10 minutos\.$/));
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

  it('answers a ready request from the card: "Manter proteção" cancels it', async () => {
    resetNofap('ready');
    await open('pending_ready');
    fireEvent.click(screen.getByRole('button', { name: 'Manter proteção' }));
    await screen.findByTestId('card-closed');
    expect(__nofapMock.state().config.pending).toEqual([]);
    expect(__nofapMock.state().config.enabled).toBe(true);
  });

  it('answers a ready request from the card: "Desligar" confirms it', async () => {
    resetNofap('ready');
    const { card } = await open('pending_ready');
    fireEvent.click(screen.getByRole('button', { name: 'Desligar' }));
    await screen.findByTestId('card-closed');
    expect(calls('card_action')).toEqual([{ cmd: 'card_action', args: { id: card.id, action: 'turn_off' } }]);
    expect(__nofapMock.state().config.enabled).toBe(false);
  });

  it('shows the next card the engine sends in the same window', async () => {
    await open('drift');
    act(() => void __nofapMock.present(__nofapMock.sampleCard('reminder')));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent('O NoFap está ligado neste computador.'));
    fireEvent.click(screen.getByRole('button', { name: 'Como desligar' }));
    expect(await screen.findByTestId('card-how-off')).toHaveTextContent('Em Proteção, peça o desligamento.');
  });

  it('closes when the engine takes its card away, and only then', async () => {
    const { card } = await open('drift');
    // Another card's end is not this window's business.
    act(() => __nofapMock.emit({ type: 'card_closed', id: 'some-other-card' }));
    expect(screen.getByTestId('card-message')).toBeInTheDocument();
    act(() => __nofapMock.emit({ type: 'card_closed', id: card.id }));
    expect(await screen.findByTestId('card-closed')).toBeInTheDocument();
    expect(calls('card_action')).toEqual([]);
  });

  it('Escape closes the window from any view and answers nothing', async () => {
    await open('blocked');
    fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Respirar 3 min' }));
    await screen.findByTestId('surf-timer');
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(await screen.findByTestId('card-closed')).toBeInTheDocument();
    expect(calls('record').map((c) => c.args.event)).toEqual(['surf_started']);
  });

  it('a late answer to the first request never resets a flow the person already opened', async () => {
    const card = __nofapMock.sampleCard('blocked');
    // The first `card` request is held back until the test lets it answer.
    let answer: (card: NofapCard) => void = () => undefined;
    const held = vi.spyOn(nofapIpc, 'card').mockReturnValue(new Promise<NofapCard>((resolve) => (answer = resolve)));
    try {
      renderCard(card.id);
      // The shell's event shows the card first (repeated until the window has subscribed).
      await waitFor(() => {
        act(() => void __nofapMock.present(card));
        expect(screen.getByRole('button', { name: 'Abrir apoio' })).toBeInTheDocument();
      });
      fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
      await screen.findByTestId('card-support');
      await act(async () => {
        answer(card);
        await Promise.resolve();
      });
      expect(screen.getByTestId('card-support')).toBeInTheDocument();
      expect(screen.queryByTestId('card-actions')).toBeNull();
    } finally {
      held.mockRestore();
    }
  });

  it('shows the card of a new id when the shell reuses the window', async () => {
    const first = __nofapMock.present(__nofapMock.sampleCard('drift'));
    const second = __nofapMock.present(__nofapMock.sampleCard('reminder'));
    function Go() {
      const navigate = useNavigate();
      return (
        <button type="button" onClick={() => navigate(`/nofap-card?id=${second.id}`)}>
          go
        </button>
      );
    }
    render(
      <MemoryRouter initialEntries={[`/nofap-card?id=${first.id}`]}>
        <NofapCardPage />
        <Go />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent('A proteção do sistema mudou.'));
    fireEvent.click(screen.getByRole('button', { name: 'go' }));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent('O NoFap está ligado neste computador.'));
  });

  it('keeps the way out and a person to talk to on top of every view a button opens', async () => {
    await open('blocked');
    // The card itself has only its two buttons.
    expect(screen.queryByTestId('card-close')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
    await screen.findByTestId('card-support');
    expect(screen.getByRole('button', { name: 'Fechar aviso' })).toBeInTheDocument();
    // Support already offers "Falar com alguém" among its own buttons: one such button, not two.
    expect(screen.getAllByRole('button', { name: 'Falar com alguém' })).toHaveLength(1);

    fireEvent.click(screen.getByRole('button', { name: 'Respirar 3 min' }));
    await screen.findByTestId('surf-timer');
    expect(screen.getByRole('button', { name: 'Fechar aviso' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Falar com alguém' })); // one step from the exercise
    expect(await screen.findByTestId('crisis-card')).toHaveTextContent('ligue 188.');
    expect(screen.queryByTestId('card-talk')).toBeNull();

    // Leaving an exercise half-way is not finishing it.
    fireEvent.click(screen.getByRole('button', { name: 'Fechar aviso' }));
    expect(await screen.findByTestId('card-closed')).toBeInTheDocument();
    expect(calls('record').map((c) => c.args.event)).toEqual(['surf_started']);
  });

  it('offers the way out and a person to talk to on the step-away, help and how-to-turn-off views too', async () => {
    const first = await open('blocked');
    fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
    fireEvent.click(await screen.findByRole('button', { name: 'Sair da tela por 10 min' }));
    await screen.findByTestId('step-away');
    expect(screen.getByTestId('card-close')).toBeInTheDocument();
    expect(screen.getByTestId('card-talk')).toBeInTheDocument();
    first.unmount();
    for (const [kind, label, view] of [['help', 'Ver opções de ajuda', 'card-help'], ['reminder', 'Como desligar', 'card-how-off']] as const) {
      const { unmount } = await open(kind);
      fireEvent.click(screen.getByRole('button', { name: label }));
      await screen.findByTestId(view);
      expect(screen.getByTestId('card-close'), kind).toBeInTheDocument();
      fireEvent.click(screen.getByTestId('card-talk'));
      expect(await screen.findByTestId('crisis-card'), kind).toBeInTheDocument();
      unmount();
    }
  });

  it('says nothing about a missing contact list while the list is still on its way', async () => {
    let answer: (value: SupportView) => void = () => undefined;
    const held = vi.spyOn(nofapIpc, 'support').mockReturnValue(new Promise<SupportView>((resolve) => (answer = resolve)));
    try {
      await open('help');
      fireEvent.click(screen.getByRole('button', { name: 'Ver opções de ajuda' }));
      const help = await screen.findByTestId('card-help');
      expect(help).not.toHaveTextContent('A lista de contatos de apoio não veio nesta versão do app.');
      expect(within(help).queryByTestId('resource-list')).toBeNull();
      await act(async () => {
        answer({ values: '', plans: [], progress: { window_days: 30, days: 0, streak: null, after_reset: false }, checkin_today: null, help_due: true, resources: [{ id: 'crp', country: 'BR', name: 'CRP', phone: null, url: null, note_pt: 'Procure um psicólogo com registro ativo no CRP.', note_en: 'Look for a psychologist with an active CRP registration.' }] });
        await Promise.resolve();
      });
      expect(await within(help).findByTestId('resource-list')).toHaveTextContent('Procure um psicólogo com registro ativo no CRP.');
    } finally {
      held.mockRestore();
    }
  });

  it('says that no contact list came only once the request has failed', async () => {
    const failed = vi.spyOn(nofapIpc, 'support').mockRejectedValue(new Error('unavailable'));
    try {
      await open('help');
      fireEvent.click(screen.getByRole('button', { name: 'Ver opções de ajuda' }));
      await waitFor(() => expect(screen.getByTestId('card-help')).toHaveTextContent('A lista de contatos de apoio não veio nesta versão do app.'));
    } finally {
      failed.mockRestore();
    }
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
    // The help card is a sentence and two buttons until its list opens.
    expect(size('help')).toEqual({ width: 420, height: 220 });
    expect(size('help', false, true)).toEqual({ width: 420, height: 560 });
    expect(size('blocked_repeat')).toEqual({ width: 420, height: 560 });
  });
});

describe('a card never shows a site, a window title or an application', () => {
  beforeEach(() => resetNofap('on'));

  const SECRETS = ['secret-site.example', 'Secret Window Title', 'SecretApp'];
  const expectNoSecret = () => {
    const text = document.body.textContent ?? '';
    for (const secret of SECRETS) expect(text.includes(secret), secret).toBe(false);
    expect(text).not.toMatch(/nofap\.|\{\w+\}/);
    for (const el of document.querySelectorAll('[title], [aria-label], [alt], [data-testid]')) {
      for (const attr of ['title', 'aria-label', 'alt']) for (const secret of SECRETS) expect((el.getAttribute(attr) ?? '').includes(secret), `${attr}: ${secret}`).toBe(false);
    }
  };

  it('ignores fields the contract does not have, whatever a newer or broken engine sends', async () => {
    const extra = { host: 'secret-site.example', url: 'https://secret-site.example/page', title: 'Secret Window Title', app: 'SecretApp', app_id: 'SecretApp', window_title: 'Secret Window Title' };
    for (const fixture of views.cards) {
      const { unmount } = await open(fixture.kind, extra as Partial<NofapCard>);
      expectNoSecret();
      for (const button of within(screen.getByTestId('card-actions')).getAllByRole('button')) {
        if (!['Abrir apoio', 'Ver opções de ajuda', 'Como desligar', 'Respirar 3 min', 'Fazer a onda de 3 min'].includes(button.textContent ?? '')) continue;
        fireEvent.click(button);
        await waitFor(() => expect(screen.queryByTestId('card-actions')).toBeNull());
        expectNoSecret();
        break;
      }
      unmount();
    }
  });

  it('shows the neutral sentence for a key that is not a sentence of this app, and no second line for such a note', async () => {
    for (const key of ['secret-site.example', 'Secret Window Title', '', 'block.no_such_key', '__proto__', 'constructor']) {
      const { unmount } = await open('vision_closed', { message_key: key, note_key: key });
      expect(screen.getByTestId('card-message').textContent, key).toBe('Página bloqueada pelo UBI.');
      expect(screen.queryByTestId('card-note'), key).toBeNull();
      expectNoSecret();
      unmount();
    }
  });

  it('never fills a sentence that has room for a site: such a key falls back to the neutral sentence', async () => {
    // Real keys of the namespace whose text has a placeholder a card cannot fill.
    for (const key of ['pending.allow_host', 'pending.remove_host', 'lists.remove_site', 'dns.previous', 'panic.reason', 'vision.today']) {
      const { unmount } = await open('vision_closed', { message_key: key, note_key: key });
      expect(screen.getByTestId('card-message').textContent, key).toBe('Página bloqueada pelo UBI.');
      expect(screen.queryByTestId('card-note'), key).toBeNull();
      unmount();
    }
  });

  it('a discreet block says only that a page was blocked, whatever key and note came with it', async () => {
    await open('blocked', { discreet: true, message_key: 'block.default', note_key: 'vision.added' });
    expect(screen.getByTestId('page-nofap-card').textContent).toBe('Página bloqueada pelo UBI.Abrir apoioOk');
  });

  it('fills the waiting time into a second line that names it', async () => {
    await open('vision_closed', { note_key: 'notit.request' });
    expect(screen.getByTestId('card-note')).toHaveTextContent('Esse site está na lista. Pedi a exceção; ela vale daqui a 10 minutos.');
  });

  it('with no waiting time, an exception still needs a confirmation, and the card says that instead of "daqui a sem espera"', async () => {
    __nofapMock.state().config.delay_secs = 0;
    await open('vision_ask');
    fireEvent.click(screen.getByRole('button', { name: 'Pedir exceção' }));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent(/^Pedi a exceção\. Falta você confirmar na página do app\.$/));
    expect(__nofapMock.state().config.pending.map((p) => p.change.kind)).toEqual(['add_allow']);
  });

  it('reads the waiting time when the button is pressed, not when the window opened', async () => {
    await open('vision_ask');
    __nofapMock.state().config.delay_secs = 1800;
    fireEvent.click(screen.getByRole('button', { name: 'Pedir exceção' }));
    await waitFor(() => expect(screen.getByTestId('card-message')).toHaveTextContent('ela vale daqui a 30 minutos.'));
  });

  it('writes no sentence with a hole where the product name goes when the view cannot be read', async () => {
    const failed = vi.spyOn(nofapIpc, 'view').mockRejectedValue(new Error('unavailable'));
    try {
      const card = __nofapMock.present(__nofapMock.sampleCard('reminder'));
      renderCard(card.id);
      const message = await screen.findByTestId('card-message');
      expect(message.textContent).toBe('Página bloqueada pelo UBI.');
      expect(document.body.textContent).not.toMatch(/O {2}está|\{name\}/);
    } finally {
      failed.mockRestore();
    }
  });

  it('names no waiting time it does not know', () => {
    const t = (key: string, vars?: Record<string, string | number>) => translate('pt-BR', key, vars);
    expect(requestedText(t, null)).toBe('Pedi a exceção. Ela espera o tempo que você escolheu.');
    expect(requestedText(t, 0)).toBe('Pedi a exceção. Falta você confirmar na página do app.');
    expect(requestedText(t, 600)).toBe('Pedi a exceção; ela vale daqui a 10 minutos.');
    expect(cardText('reminder.on', t, null, 600)).toBeNull();
    expect(cardText('reminder.on', t, 'Escudo', null)).toBe('O Escudo está ligado neste computador. Veja como desligar.');
    expect(cardText('notit.request', t, 'Escudo', 0)).toBeNull();
    expect(cardText(null, t, 'Escudo', 600)).toBeNull();
  });
});

describe('the card window closes by itself only while untouched', () => {
  // Fake timers all the way, so the outcome never depends on how long the machine takes: the modules the page
  // imports on demand are loaded beforehand, and `settle` lets its promises run without real time passing.
  beforeAll(async () => {
    await import('../lib/mock');
  });
  beforeEach(() => {
    resetNofap('on');
    vi.useFakeTimers();
  });
  afterEach(() => vi.useRealTimers());

  const advance = (ms: number) => act(async () => void (await vi.advanceTimersByTimeAsync(ms)));
  const settle = async () => {
    for (let i = 0; i < 10; i++) await advance(1);
  };

  it('after the seconds the card names', async () => {
    const card = __nofapMock.present(__nofapMock.sampleCard('blocked'));
    expect(card.auto_close_secs).toBe(12);
    renderCard(card.id);
    await settle();
    expect(screen.getByTestId('card-message')).toBeInTheDocument();
    await advance(11_000);
    expect(screen.queryByTestId('card-closed')).toBeNull();
    await advance(1_000);
    expect(screen.getByTestId('card-closed')).toBeInTheDocument();
  });

  it('not while a pressed button is still being answered', async () => {
    const card = __nofapMock.present(__nofapMock.sampleCard('blocked'));
    renderCard(card.id);
    await settle();
    // The engine takes its time with the button; the 12 seconds run out meanwhile.
    let answer: (value: null) => void = () => undefined;
    const held = vi.spyOn(nofapIpc, 'cardAction').mockReturnValue(new Promise<null>((resolve) => (answer = resolve)));
    try {
      fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
      await advance(20_000);
      expect(screen.queryByTestId('card-closed')).toBeNull();
      await act(async () => {
        answer(null);
        await Promise.resolve();
      });
      await settle();
      expect(screen.getByTestId('card-support')).toBeInTheDocument();
    } finally {
      held.mockRestore();
    }
  });

  it('an acknowledgement closes the window after a few seconds', async () => {
    const card = __nofapMock.present(__nofapMock.sampleCard('vision_closed'));
    renderCard(card.id);
    await settle();
    fireEvent.click(screen.getByRole('button', { name: 'Adicionar à minha lista' }));
    await settle();
    expect(screen.getByTestId('card-message')).toHaveTextContent('Adicionado à sua lista.');
    await advance(CARD_DONE_CLOSE_MS - 100);
    expect(screen.queryByTestId('card-closed')).toBeNull();
    await advance(200);
    expect(screen.getByTestId('card-closed')).toBeInTheDocument();
  });

  it('not once the user opened support', async () => {
    const card = __nofapMock.present(__nofapMock.sampleCard('blocked'));
    renderCard(card.id);
    await settle();
    fireEvent.click(screen.getByRole('button', { name: 'Abrir apoio' }));
    await settle();
    expect(screen.getByTestId('card-support')).toBeInTheDocument();
    await advance(600_000);
    expect(screen.queryByTestId('card-closed')).toBeNull();
    expect(screen.getByTestId('card-support')).toBeInTheDocument();
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
