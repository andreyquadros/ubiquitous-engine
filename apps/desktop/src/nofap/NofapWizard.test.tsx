import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { __nofapMock } from './mock';
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
    expect(step).toHaveTextContent('pergunta a IA do Ubi se há pornografia nela');
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

  it('offers the autostart switch of the host, on or off as the host says, and never forces it', async () => {
    const onChange = vi.fn();
    renderPage({ autostart: { value: true, onChange } });
    await goTo(2);
    const toggle = screen.getByRole('switch', { name: 'Abrir junto com o computador' });
    expect(toggle).toHaveAttribute('aria-checked', 'true');
    fireEvent.click(toggle);
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it('summarises what is stored, how to turn it off and how to remove it without the app', async () => {
    renderPage();
    await goTo(5);
    const step = screen.getByTestId('wizard-step-5');
    expect(step).toHaveTextContent('Camadas: Bloqueio no app, Lista do sistema, DNS filtrado, Regras dos navegadores, Verificação por IA.');
    expect(step).toHaveTextContent('Tempo de espera: 10 minutos.');
    expect(step).toHaveTextContent('Nenhum site visitado, nenhuma imagem.');
    expect(step).toHaveTextContent('Cancelar o pedido é sempre na hora.');
    expect(step).toHaveTextContent('ubiqx-shield BEGIN e END');
    expect(step).toHaveTextContent('Quem tem acesso à sua conta neste computador pode ver que o NoFap está instalado');
  });
});
