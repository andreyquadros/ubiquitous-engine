import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { nofapIpc } from './ipc';
import { __nofapMock } from './mock';
import { setLocale } from '../i18n';
import { calls, READS, renderPage, resetNofap } from './testing';

const next = () => fireEvent.click(screen.getByRole('button', { name: 'Continuar' }));
const tickSelfUse = () => fireEvent.click(screen.getByRole('checkbox', { name: 'É para mim, no meu computador' }));

/** Walks from step 1 to `step`, ticking what each step needs. */
async function goTo(step: number) {
  await screen.findByTestId('wizard-step-1');
  tickSelfUse();
  for (let s = 1; s < step; s++) {
    if (s === 3) for (const box of within(screen.getByTestId('wizard-step-3')).queryAllByRole('checkbox')) fireEvent.click(box);
    next();
  }
  await screen.findByTestId(`wizard-step-${step}`);
}

describe('the consent wizard', () => {
  beforeEach(() => resetNofap('off'));

  it('says what it is and is not, and waits for the self-use tick', async () => {
    renderPage();
    const step = await screen.findByTestId('wizard-step-1');
    expect(step).toHaveTextContent('O NoFap bloqueia sites de pornografia neste computador');
    expect(step).toHaveTextContent('Ele dificulta, não tranca.');
    expect(step).toHaveTextContent('Não serve para vigiar outra pessoa');
    expect(screen.getByRole('button', { name: 'Continuar' })).toBeDisabled();
    tickSelfUse();
    expect(screen.getByRole('button', { name: 'Continuar' })).toBeEnabled();
  });

  it('changes nothing before the last button, which sends exactly one enable request', async () => {
    renderPage();
    await goTo(4);
    fireEvent.change(screen.getByLabelText('Sua frase'), { target: { value: 'Quero estar presente.' } });
    fireEvent.click(screen.getByRole('radio', { name: '30 minutos' }));
    next();
    await screen.findByTestId('wizard-step-5');
    expect(calls().filter((c) => !READS.includes(c.cmd))).toEqual([]);
    expect(__nofapMock.state().config.enabled).toBe(false);

    fireEvent.click(screen.getByTestId('wizard-enable'));
    await screen.findByTestId('nofap-status');
    const writes = calls().filter((c) => !READS.includes(c.cmd));
    expect(writes).toHaveLength(1);
    expect(writes[0]).toEqual({
      cmd: 'enable',
      args: {
        self_use: true,
        hosts: true,
        resolver: 'cloudflare_family',
        policies: true,
        vision: true,
        vision_consent: { provider: 'anthropic', managed: false },
        vision_scope: { unknown_sites: true, media_apps: true, messaging_apps: false, private_windows: true },
        delay_secs: 1800,
        values: 'Quero estar presente.',
        discreet: true,
      },
    });
    expect(screen.getByTestId('nofap-overall')).toHaveTextContent('Proteção ativa');
  });

  it('shows the DNS and policy disclosures, the list of changes and the password notice', async () => {
    renderPage();
    await goTo(2);
    const step = screen.getByTestId('wizard-step-2');
    expect(step).toHaveTextContent('todos os nomes de sites que este computador consultar passam pela Cloudflare (1.1.1.3)');
    expect(step).toHaveTextContent('Os navegadores vão mostrar "Gerenciado pela sua organização".');
    expect(screen.getByTestId('wizard-changes')).toHaveTextContent('O sistema vai pedir sua senha de administrador para aplicar isso.');
    // "só a lista neste computador": the DNS disclosure goes away with the choice, and so does its change.
    fireEvent.click(screen.getByRole('radio', { name: 'Só a lista neste computador' }));
    expect(step).not.toHaveTextContent('passam pela Cloudflare');
    expect(screen.getByTestId('wizard-changes')).not.toHaveTextContent('Cloudflare for Families');
  });

  it('names the provider and does not let the AI check on without the tick', async () => {
    renderPage();
    await screen.findByTestId('wizard-step-1');
    tickSelfUse();
    next();
    next();
    const step = await screen.findByTestId('wizard-step-3');
    expect(step).toHaveTextContent('pergunta a Anthropic se há pornografia nela');
    expect(step).toHaveTextContent('Cada verificação custa cerca de US$ 0,001. Limite: 120 por dia e US$ 2,00 por mês.');
    expect(within(step).getByRole('checkbox', { name: 'Entendi o que é enviado' })).not.toBeChecked();
    expect(screen.getByRole('button', { name: 'Continuar' })).toBeDisabled();
    expect(step).toHaveTextContent('Marque a caixa para ligar a verificação.');
    // The managed tick belongs to "IA do Ubi" only.
    expect(within(step).queryByRole('checkbox', { name: /servidor do ubiqX/ })).toBeNull();

    // Switching the check off is the other way forward, and then no consent is sent.
    fireEvent.click(within(step).getByRole('switch', { name: 'Verificação por IA' }));
    expect(screen.getByRole('button', { name: 'Continuar' })).toBeEnabled();
    next();
    next();
    fireEvent.click(await screen.findByTestId('wizard-enable'));
    await waitFor(() => expect(calls('enable')).toHaveLength(1));
    expect(calls('enable')[0]?.args).toMatchObject({ vision: false, vision_consent: null });
  });

  it('asks for the second tick when the checks go through the managed route', async () => {
    __nofapMock.setAi('ubi');
    renderPage();
    await screen.findByTestId('wizard-step-1');
    tickSelfUse();
    next();
    next();
    const step = await screen.findByTestId('wizard-step-3');
    // The managed provider's name takes its article, and the rules and terms named are the vendor's.
    expect(step).toHaveTextContent('pergunta à IA do Ubi se há pornografia nela');
    expect(step).toHaveTextContent('A IA do Ubi envia a imagem à Anthropic, que aplica as próprias regras ao que recebe e guarda pelo prazo que os termos dela definem.');
    expect(step).not.toHaveTextContent('as regras dele');
    expect(step).toHaveTextContent('As imagens da verificação passam pelo servidor do ubiqX a caminho da Anthropic.');
    fireEvent.click(within(step).getByRole('checkbox', { name: 'Entendi o que é enviado' }));
    expect(screen.getByRole('button', { name: 'Continuar' })).toBeDisabled();
    fireEvent.click(within(step).getByRole('checkbox', { name: /servidor do ubiqX/ }));
    expect(screen.getByRole('button', { name: 'Continuar' })).toBeEnabled();
    next();
    next();
    fireEvent.click(await screen.findByTestId('wizard-enable'));
    await waitFor(() => expect(calls('enable')).toHaveLength(1));
    expect(calls('enable')[0]?.args.vision_consent).toEqual({ provider: 'ubi', managed: true });
  });

  it('the consent says what the private-windows switch under it says (frontend final review)', async () => {
    renderPage();
    await goTo(3);
    const step = screen.getByTestId('wizard-step-3');
    expect(within(step).getByTestId('vision-consent-text')).toHaveTextContent('Janelas anônimas são verificadas.');
    fireEvent.click(within(step).getByRole('switch', { name: 'Janelas anônimas' }));
    expect(within(step).getByTestId('vision-consent-text')).toHaveTextContent('Janelas anônimas não são verificadas.');
    expect(within(step).getByTestId('vision-consent-text')).not.toHaveTextContent('Janelas anônimas são verificadas');
  });

  it('keeps the AI check off, and says why, when no provider is usable', async () => {
    __nofapMock.setAi('anthropic', 'no_key');
    const openSettings = vi.fn();
    renderPage({ openSettings });
    await goTo(3);
    const step = screen.getByTestId('wizard-step-3');
    expect(step).toHaveTextContent('A verificação por IA fica desligada por enquanto. Falta a chave do provedor de IA.');
    expect(step).toHaveTextContent('Sem a verificação por IA, sites desconhecidos só são barrados pelo DNS filtrado e pelas listas.');
    expect(within(step).queryByRole('switch')).toBeNull();
    fireEvent.click(within(step).getByRole('button', { name: 'Abrir configurações' }));
    expect(openSettings).toHaveBeenCalledTimes(1);
  });

  it('shows a gated tier as such and never asks for it', async () => {
    __nofapMock.setCapabilities({ hosts: { state: 'gated' }, policies: { state: 'unsupported', reason: 'flatpak' } });
    renderPage();
    await goTo(2);
    const hosts = screen.getByTestId('wizard-tier-hosts');
    expect(hosts).toHaveTextContent('Em verificação neste sistema. Por enquanto valem o bloqueio no app e a verificação por IA.');
    expect(within(hosts).queryByRole('switch')).toBeNull();
    expect(screen.getByTestId('wizard-tier-policies')).toHaveTextContent('O navegador em Flatpak guarda as regras fora do alcance do app.');
    next();
    for (const box of within(await screen.findByTestId('wizard-step-3')).getAllByRole('checkbox')) fireEvent.click(box);
    next();
    next();
    fireEvent.click(await screen.findByTestId('wizard-enable'));
    await waitFor(() => expect(calls('enable')).toHaveLength(1));
    expect(calls('enable')[0]?.args).toMatchObject({ hosts: false, policies: false, resolver: 'cloudflare_family' });
  });

  it('says "Proteção parcial" when the OS prompt was declined, and offers to apply', async () => {
    __nofapMock.setPrompt('decline');
    renderPage();
    await goTo(5);
    fireEvent.click(screen.getByTestId('wizard-enable'));
    expect(await screen.findByTestId('nofap-overall')).toHaveTextContent('Proteção parcial');
    expect(screen.getByTestId('tier-row-hosts')).toHaveTextContent('Ainda não aplicado');
    expect(screen.getByTestId('nofap-apply')).toHaveTextContent('Aplicar');
  });

  it('shows the autostart switch on by default and tells the host only with the last button', async () => {
    const onChange = vi.fn();
    // The host has it off; the wizard still proposes it on, in plain sight.
    renderPage({ autostart: { value: false, onChange } });
    await goTo(2);
    expect(screen.getByRole('switch', { name: 'Abrir junto com o computador' })).toHaveAttribute('aria-checked', 'true');
    for (let s = 2; s < 5; s++) {
      if (s === 3) for (const box of within(screen.getByTestId('wizard-step-3')).getAllByRole('checkbox')) fireEvent.click(box);
      next();
    }
    expect(await screen.findByTestId('wizard-step-5')).toHaveTextContent('O app abre junto com o computador.');
    // Nothing is applied before the last button: not in the module, and not in the host either.
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.click(screen.getByTestId('wizard-enable'));
    await screen.findByTestId('nofap-status');
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it('never forces autostart: switched off in the wizard, the host is told so, or told nothing when it already agrees', async () => {
    const onChange = vi.fn();
    renderPage({ autostart: { value: true, onChange } });
    await goTo(2);
    fireEvent.click(screen.getByRole('switch', { name: 'Abrir junto com o computador' }));
    expect(onChange).not.toHaveBeenCalled();
    for (let s = 2; s < 5; s++) {
      if (s === 3) for (const box of within(screen.getByTestId('wizard-step-3')).getAllByRole('checkbox')) fireEvent.click(box);
      next();
    }
    expect(await screen.findByTestId('wizard-step-5')).toHaveTextContent('O app não abre junto com o computador.');
    fireEvent.click(screen.getByTestId('wizard-enable'));
    await screen.findByTestId('nofap-status');
    expect(onChange.mock.calls).toEqual([[false]]);
  });

  it('leaves the host alone when the switch was not touched and the host already has it on', async () => {
    const onChange = vi.fn();
    renderPage({ autostart: { value: true, onChange } });
    await goTo(5);
    fireEvent.click(screen.getByTestId('wizard-enable'));
    await screen.findByTestId('nofap-status');
    expect(onChange).not.toHaveBeenCalled();
  });

  it('does not touch the host setting when switching on failed', async () => {
    const onChange = vi.fn();
    renderPage({ autostart: { value: false, onChange } });
    await goTo(5);
    const refused = vi.spyOn(nofapIpc, 'enable').mockRejectedValue(new Error('invalid: refused for the test'));
    try {
      fireEvent.click(screen.getByTestId('wizard-enable'));
      expect(await screen.findByRole('alert')).toHaveTextContent('Não consegui ligar. invalid: refused for the test');
      expect(onChange).not.toHaveBeenCalled();
    } finally {
      refused.mockRestore();
    }
  });

  it('offers a person to talk to one step from the wizard', async () => {
    renderPage();
    await screen.findByTestId('wizard-step-1');
    fireEvent.click(screen.getByTestId('nofap-talk'));
    expect(await screen.findByTestId('crisis-card')).toHaveTextContent('ligue 188. É o CVV: grátis, 24 horas.');
    // Asking for help switches nothing on.
    expect(calls().filter((c) => !READS.includes(c.cmd))).toEqual([]);
  });

  it('summarises what is stored, how to turn it off and how to remove it without the app', async () => {
    renderPage();
    await goTo(5);
    const step = screen.getByTestId('wizard-step-5');
    expect(step).toHaveTextContent('Camadas: Bloqueio no app, Lista do sistema, DNS filtrado, Regras dos navegadores, Verificação por IA.');
    expect(step).toHaveTextContent('Tempo de espera: 10 minutos.');
    expect(step).toHaveTextContent('De um site aprendido pela verificação por IA fica só um código, sem o nome. Nenhum histórico de sites visitados, nenhuma imagem.');
    expect(step).toHaveTextContent('Cancelar o pedido é sempre na hora.');
    expect(step).toHaveTextContent('ubiqx-shield BEGIN e END');
    expect(step).toHaveTextContent('Quem tem acesso à sua conta neste computador pode ver que o NoFap está instalado');
  });
});

describe('the wizard in the two products (integrator decision 32)', () => {
  /** ubiqX by name, not the marker of the hosts block (`ubiqx-shield`), which is the same in both products. */
  const NAMES_UBIQX = /ubiqx(?!-shield)/i;

  /** The text of steps 2 and 3, where the disclosures and the consent are. */
  async function disclosures(): Promise<string> {
    await goTo(2);
    const two = screen.getByTestId('wizard-step-2').textContent ?? '';
    next();
    const three = (await screen.findByTestId('wizard-step-3')).textContent ?? '';
    return `${two}\n${three}`;
  }

  it('says in both products that the app receives nothing from the DNS choice', async () => {
    for (const product of ['ubiqx', 'shield'] as const) {
      resetNofap('off', product);
      const { unmount } = renderPage();
      const text = await disclosures();
      expect(text, product).toContain('É a Cloudflare que decide o que barrar: sites adultos e sites que ela considera perigosos. Este app não recebe nada.');
      unmount();
    }
  });

  it('adds the line about pausing ubiqX and its private mode in ubiqX only', async () => {
    resetNofap('off', 'ubiqx');
    const { unmount } = renderPage();
    const ubiqx = await disclosures();
    expect(ubiqx).toContain('Janelas anônimas são verificadas. Pausar o ubiqX ou usar o modo privado não pausa esta verificação.');
    unmount();

    resetNofap('off', 'shield');
    renderPage();
    const standalone = await disclosures();
    expect(standalone).toContain('Janelas anônimas são verificadas.');
    expect(standalone).not.toMatch(NAMES_UBIQX);
    expect(standalone).not.toMatch(/modo privado|modo só local/);
  });

  it('names nothing of ubiqX in the standalone wizard in English either', async () => {
    setLocale('en');
    resetNofap('off', 'shield');
    renderPage();
    await screen.findByTestId('wizard-step-1');
    fireEvent.click(screen.getByRole('checkbox', { name: 'It is for me, on my own computer' }));
    const text: string[] = [screen.getByTestId('wizard-step-1').textContent ?? ''];
    for (let s = 1; s < 5; s++) {
      if (s === 3) for (const box of within(screen.getByTestId('wizard-step-3')).queryAllByRole('checkbox')) fireEvent.click(box);
      fireEvent.click(screen.getByRole('button', { name: 'Continue' }));
      text.push((await screen.findByTestId(`wizard-step-${s + 1}`)).textContent ?? '');
    }
    const all = text.join('\n');
    expect(all).toContain('This app receives nothing.');
    expect(all).toContain('Private-browsing windows are checked.');
    expect(all).not.toMatch(NAMES_UBIQX);
    expect(all).not.toMatch(/private mode|local-only/i);
  });
});
