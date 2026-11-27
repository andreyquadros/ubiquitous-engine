import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActivityBlock, BlockGroup, Category, DashboardData, Id, LicenseStatus } from '../lib/types';

const categories: Category[] = [
  { id: 'cat-ifro', name: 'IFRO', color: '#2563EB', icon: 'graduation-cap', description: '', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 0, created_at: '2026-01-01T00:00:00Z' },
  { id: 'cat-incubadora', name: 'Incubadora', color: '#F97316', icon: 'rocket', description: '', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 1, created_at: '2026-01-01T00:00:00Z' },
  { id: 'cat-pessoal', name: 'Compromissos Pessoais (Igreja, Estudos, Cursos)', color: '#10B981', icon: 'heart', description: '', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 2, created_at: '2026-01-01T00:00:00Z' },
  { id: 'sys-distraction', name: 'Distração', color: '#EF4444', icon: 'tv', description: '', keywords: [], report_time: null, report_template: null, is_productive: false, is_system: true, archived: false, sort_order: 101, created_at: '2026-01-01T00:00:00Z' },
];

const seedGroups: BlockGroup[] = [
  { key: 'net.whatsapp.WhatsApp|whatsapp', app_id: 'net.whatsapp.WhatsApp', app_name: 'WhatsApp', domain: null, title: 'WhatsApp', total_secs: 540, block_ids: ['b1', 'b2'], category_id: null, min_confidence: 0.4, source: 'llm', needs_review: true, description: 'Mensagens de alunos', first_started_at: '2026-09-17T11:00:00Z' },
  { key: 'com.google.Chrome|mail.google.com', app_id: 'com.google.Chrome', app_name: 'Google Chrome', domain: 'mail.google.com', title: 'Caixa de entrada - Gmail', total_secs: 1800, block_ids: ['b3'], category_id: 'cat-ifro', min_confidence: 0.62, source: 'llm', needs_review: true, description: null, first_started_at: '2026-09-17T11:10:00Z' },
  { key: 'com.apple.iCal|calendario', app_id: 'com.apple.iCal', app_name: 'Calendário', domain: null, title: 'Calendário', total_secs: 180, block_ids: ['b4'], category_id: 'cat-pessoal', min_confidence: 0.7, source: 'llm', needs_review: false, description: null, first_started_at: '2026-09-17T12:00:00Z' },
];

const block = (over: Partial<ActivityBlock> & { id: Id }): ActivityBlock => ({
  started_at: '2026-09-17T11:00:00Z',
  ended_at: '2026-09-17T11:04:00Z',
  app_name: 'WhatsApp',
  app_id: 'net.whatsapp.WhatsApp',
  title: 'WhatsApp',
  title_key: 'whatsapp',
  url: null,
  domain: null,
  category_id: null,
  confidence: 0.4,
  source: 'llm',
  description: null,
  screenshot_id: null,
  sample_count: 48,
  is_open: false,
  classify_attempts: 1,
  next_attempt_at: null,
  needs_review: true,
  ai_payload: null,
  ai_sent_at: null,
  is_manual: false,
  note: null,
  ...over,
});

const blocks: ActivityBlock[] = [
  block({ id: 'b1', description: 'Mensagens de alunos sobre a entrega do projeto', screenshot_id: 'shot-1', ai_payload: '{"app":"WhatsApp","title":"WhatsApp"}', ai_sent_at: '2026-09-17T11:06:00Z' }),
  block({ id: 'b2', started_at: '2026-09-17T11:20:00Z', ended_at: '2026-09-17T11:25:00Z', title: 'WhatsApp (2)', description: 'Conversa com a equipe da AgroTech' }),
  block({ id: 'b3', app_name: 'Google Chrome', app_id: 'com.google.Chrome', title: 'Caixa de entrada - Gmail', title_key: 'gmail', url: 'https://mail.google.com/mail/u/0/#inbox', domain: 'mail.google.com', category_id: 'cat-ifro', confidence: 0.62, started_at: '2026-09-17T11:10:00Z', ended_at: '2026-09-17T11:40:00Z' }),
  block({ id: 'b4', app_name: 'Calendário', app_id: 'com.apple.iCal', title: 'Calendário', title_key: 'calendario', category_id: 'cat-pessoal', confidence: 0.7, needs_review: false, started_at: '2026-09-17T12:00:00Z', ended_at: '2026-09-17T12:03:00Z' }),
];

// a stateful stand-in for the backend: assigning a group turns its source into 'user' on the next fetch
let overrides: Record<string, Partial<BlockGroup>> = {};
const reclassifyGroup = vi.fn(async (_date: string, key: string, categoryId: Id) => {
  overrides[key] = { category_id: categoryId, source: 'user', needs_review: false, min_confidence: 1 };
  return { block_ids: ['b3'], backfilled: 1, suggestions: [{ category_id: categoryId, matcher: 'domain', pattern: 'mail.google.com', support: 3, rationale: 'x', auto_apply_safe: true }], auto_rules: [], disabled_rules: [] };
});
const confirmGroups = vi.fn(async (_date: string, keys: string[]) => {
  for (const key of keys) overrides[key] = { source: 'user', min_confidence: 1, needs_review: false };
  return { block_ids: keys.map((k) => `blocks-of-${k}`), backfilled: 0, suggestions: [], auto_rules: [], disabled_rules: [] };
});
const getScreenshot = vi.fn(async (blockId: Id) => {
  const b = blocks.find((x) => x.id === blockId);
  return b?.screenshot_id ? placeholderScreenshot(b.app_name, b.title) : null;
});

vi.mock('../lib/ipc', () => ({
  isTauri: () => false,
  onEngineEvent: async () => () => undefined,
  ipc: {
    getReviewGroups: vi.fn(async () => seedGroups.map((g) => ({ ...g, ...overrides[g.key] }))),
    getTimeline: vi.fn(async () => blocks),
    getScreenshot: (...args: unknown[]) => getScreenshot(...(args as [Id])),
    listCategories: vi.fn(async () => categories),
    reclassifyGroup: (...args: unknown[]) => reclassifyGroup(...(args as [string, string, Id])),
    confirmGroups: (...args: unknown[]) => confirmGroups(...(args as [string, string[]])),
    acceptRuleSuggestion: vi.fn(async () => ({})),
    classifyNow: vi.fn(async () => ({ local: 0, remote: 0, vision: 0, needs_review: 0, skipped_remote: false })),
    getSettings: vi.fn(),
    getDashboard: vi.fn(),
    updateSettings: vi.fn(),
  },
}));

import { Review } from './Review';
import { setLocale } from '../i18n';
import { __clearScreenshotCache } from '../components/ui/BlockDetails';
import { fmtTime } from '../lib/format';
import { ipc } from '../lib/ipc';
import { placeholderScreenshot } from '../lib/mock';
import { useAppStore } from '../lib/store';

const renderPage = () =>
  render(
    <MemoryRouter>
      <Review />
    </MemoryRouter>,
  );

const pendingRows = () => screen.queryAllByTestId('review-row').filter((el) => !el.closest('[data-testid="reviewed-list"]'));

describe('Review page', () => {
  beforeEach(() => {
    overrides = {};
    reclassifyGroup.mockClear();
    confirmGroups.mockClear();
    getScreenshot.mockClear();
    __clearScreenshotCache();
    useAppStore.setState({ categories, date: '2026-09-17', dataVersion: 0, license: null, dashboards: {} });
  });

  // The queue asks only about what the chain could not settle. Calendário was classified with
  // enough confidence, so it is an answer, not a question, and never shows up here.
  it('queues only what the classifier could not settle and assigns the nth category with the keyboard', async () => {
    renderPage();
    await waitFor(() => expect(pendingRows()).toHaveLength(2));
    expect(screen.getByText(/2 grupos esperam sua decisão/)).toBeInTheDocument();
    // rising confidence: WhatsApp (0.4, uncategorized), Gmail (0.62, flagged)
    expect(pendingRows().map((el) => el.getAttribute('data-key'))).toEqual(['net.whatsapp.WhatsApp|whatsapp', 'com.google.Chrome|mail.google.com']);

    // move to the second group and press "2" → Incubadora
    fireEvent.keyDown(window, { key: 'ArrowDown' });
    fireEvent.keyDown(window, { key: '2' });

    await waitFor(() => expect(reclassifyGroup).toHaveBeenCalledTimes(1));
    expect(reclassifyGroup).toHaveBeenCalledWith('2026-09-17', 'com.google.Chrome|mail.google.com', 'cat-incubadora');

    // the rule suggestion chip shows up after the correction
    await waitFor(() => expect(screen.getByText(/mail\.google\.com/, { selector: 'button span' })).toBeInTheDocument());
  });

  it('clears the queue as the user decides and collects the answered groups at the bottom', async () => {
    renderPage();
    await waitFor(() => expect(pendingRows()).toHaveLength(2));

    // WhatsApp is selected first; "1" → IFRO
    fireEvent.keyDown(window, { key: '1' });
    await waitFor(() => expect(reclassifyGroup).toHaveBeenCalledWith('2026-09-17', 'net.whatsapp.WhatsApp|whatsapp', 'cat-ifro'));

    // the group leaves the pending list and the selection moves on to the next one
    await waitFor(() => expect(pendingRows()).toHaveLength(1));
    expect(pendingRows()[0]).toHaveAttribute('aria-current', 'true');
    expect(pendingRows()[0]).toHaveAttribute('data-key', 'com.google.Chrome|mail.google.com');
    expect(screen.getByText(/1 grupo espera sua decisão/)).toBeInTheDocument();

    // it joins Calendário under "Classificados neste dia (2)", the Ubi's answer first
    const toggle = screen.getByRole('button', { name: 'Mostrar os grupos classificados' });
    expect(toggle).toHaveTextContent('Classificados neste dia (2)');
    fireEvent.click(toggle);
    const rows = within(screen.getByTestId('reviewed-list')).getAllByTestId('review-row');
    expect(rows.map((el) => el.getAttribute('data-key'))).toEqual(['com.apple.iCal|calendario', 'net.whatsapp.WhatsApp|whatsapp']);
    const mine = rows[1]!;
    expect(within(mine).getByText('IFRO')).toBeInTheDocument();

    // and can be reopened and reassigned from there
    fireEvent.click(mine);
    fireEvent.keyDown(window, { key: '2' });
    await waitFor(() => expect(reclassifyGroup).toHaveBeenLastCalledWith('2026-09-17', 'net.whatsapp.WhatsApp|whatsapp', 'cat-incubadora'));
    await waitFor(() => expect(within(screen.getByTestId('reviewed-list')).getByText('Incubadora')).toBeInTheDocument());
    expect(pendingRows()).toHaveLength(1);
  });

  // The queue no longer forces a decision per group, so the classifier's own answers would never
  // become user labels -- and user labels are the only thing the memory classifier learns from.
  // This button is what closes that gap, and it only exists inside the expanded list.
  it('confirms the Ubi own answers in one click, and offers nothing to confirm once they are yours', async () => {
    renderPage();
    await waitFor(() => expect(pendingRows()).toHaveLength(2));
    expect(screen.queryByTestId('confirm-all')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Mostrar os grupos classificados' }));
    expect(screen.getByText('1 grupo foi o Ubi que decidiu.')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('confirm-all'));

    await waitFor(() => expect(confirmGroups).toHaveBeenCalledWith('2026-09-17', ['com.apple.iCal|calendario']));
    // nothing left that is not the user's word, so the button goes away
    await waitFor(() => expect(screen.queryByTestId('confirm-all')).not.toBeInTheDocument());
    expect(pendingRows()).toHaveLength(2);
    expect(reclassifyGroup).not.toHaveBeenCalled();
  });

  // The day is clean when nothing is in doubt -- not when the user has pressed a key on everything.
  it('celebrates a queue with nothing in doubt, counting what the Ubi settled', async () => {
    overrides = {
      'net.whatsapp.WhatsApp|whatsapp': { category_id: 'cat-ifro', needs_review: false, min_confidence: 0.9 },
      'com.google.Chrome|mail.google.com': { needs_review: false, min_confidence: 0.8 },
    };
    renderPage();
    await waitFor(() => expect(screen.getByTestId('review-done')).toBeInTheDocument());
    expect(screen.getByText('Nada esperando por você')).toBeInTheDocument();
    expect(screen.getByText(/O Ubi resolveu 3 grupos neste dia/)).toBeInTheDocument();
    expect(screen.getByText(/a fila está vazia/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Mostrar os grupos classificados' })).toHaveTextContent('Classificados neste dia (3)');
    expect(pendingRows()).toHaveLength(0);
  });

  // Every review surface is scoped to one day. A day that is clean still has to say so when other
  // days are not, or the flagged blocks there are reachable only by guessing the date.
  it('points a cleared day at the most recent day that still has flagged blocks', async () => {
    overrides = {
      'net.whatsapp.WhatsApp|whatsapp': { category_id: 'cat-ifro', needs_review: false },
      'com.google.Chrome|mail.google.com': { needs_review: false },
    };
    useAppStore.setState({ dashboards: { '2026-09-17': { review_backlog: { count: 2, date: '2026-09-15' } } as DashboardData } });
    renderPage();
    await waitFor(() => expect(screen.getByTestId('review-done')).toBeInTheDocument());
    const jump = screen.getByTestId('review-backlog');
    expect(jump).toHaveTextContent('Ver 2 blocos sinalizados em 15 set');
    fireEvent.click(jump);
    expect(useAppStore.getState().date).toBe('2026-09-15');
  });

  it('offers no jump when every other day is clean', async () => {
    overrides = {
      'net.whatsapp.WhatsApp|whatsapp': { category_id: 'cat-ifro', needs_review: false },
      'com.google.Chrome|mail.google.com': { needs_review: false },
    };
    useAppStore.setState({ dashboards: { '2026-09-17': { review_backlog: null } as DashboardData } });
    renderPage();
    await waitFor(() => expect(screen.getByTestId('review-done')).toBeInTheDocument());
    expect(screen.queryByTestId('review-backlog')).not.toBeInTheDocument();
  });

  it('opens a group to show its blocks, the AI payload and the stored screenshot', async () => {
    renderPage();
    await waitFor(() => expect(pendingRows()).toHaveLength(2));
    expect(screen.queryByTestId('block-details')).not.toBeInTheDocument();

    // Enter opens the selected (first) group
    fireEvent.keyDown(window, { key: 'Enter' });
    const details = await screen.findByTestId('block-details');
    expect(within(details).getByText('2 blocos neste grupo')).toBeInTheDocument();
    const cards = within(details).getAllByTestId('block-card');
    expect(cards).toHaveLength(2);
    expect(within(cards[0]!).getByText('WhatsApp')).toBeInTheDocument();
    expect(within(cards[0]!).getByText(`${fmtTime('2026-09-17T11:00:00Z')} até ${fmtTime('2026-09-17T11:04:00Z')}, 4min`)).toBeInTheDocument();
    expect(within(cards[0]!).getByText('Mensagens de alunos sobre a entrega do projeto')).toBeInTheDocument();
    expect(within(cards[1]!).getByText('WhatsApp (2)')).toBeInTheDocument();
    expect(within(cards[1]!).getByText('Conversa com a equipe da AgroTech')).toBeInTheDocument();

    // the payload is collapsed until asked for
    expect(within(cards[0]!).queryByText(/"app": "WhatsApp"/)).not.toBeInTheDocument();
    fireEvent.click(within(cards[0]!).getByRole('button', { name: 'Dados enviados à IA' }));
    expect(within(cards[0]!).getByText(/"app": "WhatsApp"/)).toBeInTheDocument();
    expect(within(cards[1]!).getByText('Nada foi enviado à IA para este bloco.')).toBeInTheDocument();

    // the screenshot is fetched lazily, only for the block that still has one
    const img = await within(cards[0]!).findByRole('img', { name: `Captura de WhatsApp às ${fmtTime('2026-09-17T11:00:00Z')}` });
    expect(img).toHaveAttribute('src', expect.stringMatching(/^data:image\/svg\+xml;base64,/));
    expect(getScreenshot).toHaveBeenCalledTimes(1);
    expect(getScreenshot).toHaveBeenCalledWith('b1');
    expect(within(cards[1]!).getByText('Sem captura de tela para este bloco.')).toBeInTheDocument();
    expect(within(cards[1]!).getByText(/Manter screenshots para revisão/)).toBeInTheDocument();

    // clicking the thumbnail enlarges it in a dialog; details stay open while assigning
    fireEvent.click(within(cards[0]!).getByRole('button', { name: 'Ampliar captura' }));
    expect(await screen.findByRole('dialog', { name: 'WhatsApp' })).toBeInTheDocument();
    fireEvent.keyDown(window, { key: '1' });
    expect(reclassifyGroup).not.toHaveBeenCalled();
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());

    // the chevron closes it again
    fireEvent.click(within(pendingRows()[0]!).getByRole('button', { name: 'Ocultar detalhes do grupo' }));
    expect(screen.queryByTestId('block-details')).not.toBeInTheDocument();
  });

  it('truncates a long category name in the row chip and in the assign list, keeping the full name as a tooltip', async () => {
    renderPage();
    await waitFor(() => expect(pendingRows()).toHaveLength(2));
    fireEvent.click(screen.getByRole('button', { name: 'Mostrar os grupos classificados' }));
    const long = 'Compromissos Pessoais (Igreja, Estudos, Cursos)';
    const chip = within(within(screen.getByTestId('reviewed-list')).getAllByTestId('review-row')[0]!).getByTestId('group-category');
    expect(chip).toHaveAttribute('title', long);
    expect(chip.className).toContain('max-w-full');
    const label = within(chip).getByText(long);
    expect(label.className).toContain('truncate');

    const option = screen.getByRole('option', { name: /Compromissos Pessoais/ });
    expect(option).toHaveAttribute('title', long);
    expect(within(option).getByText(long).className).toContain('truncate');
  });

  it('ignores shortcuts while typing in an input', async () => {
    renderPage();
    await waitFor(() => expect(pendingRows()).toHaveLength(2));
    const input = document.createElement('input');
    document.body.appendChild(input);
    fireEvent.keyDown(input, { key: '1' });
    expect(reclassifyGroup).not.toHaveBeenCalled();
    input.remove();
  });

  it('shows the classic empty state when the day has no groups at all', async () => {
    vi.mocked(ipc.getReviewGroups).mockResolvedValueOnce([]);
    renderPage();
    await waitFor(() => expect(screen.getByText('Nada para revisar')).toBeInTheDocument());
    expect(screen.getByTestId('ubi')).toBeInTheDocument();
    expect(screen.getByText(/a fila está vazia/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Ver a Timeline' })).toHaveAttribute('href', '/timeline');
    expect(screen.queryAllByTestId('review-row')).toHaveLength(0);
  });

  it('under hard enforcement "Classificar agora" opens the license dialog instead of calling the AI', async () => {
    const hard: LicenseStatus = { state: 'unlicensed', plan: null, expires_at: null, days_left: null, key_hint: null, enforcement: 'hard', managed_usage: null };
    useAppStore.setState({ license: hard });
    vi.mocked(ipc.classifyNow).mockClear();
    renderPage();
    await waitFor(() => expect(pendingRows()).toHaveLength(2));

    fireEvent.click(screen.getByRole('button', { name: 'Classificar agora' }));
    expect(await screen.findByText('Licença necessária')).toBeInTheDocument();
    expect(ipc.classifyNow).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Já tenho uma chave' })).toBeInTheDocument();

    // the same button runs under soft enforcement, which is what this build ships
    act(() => useAppStore.setState({ license: { ...hard, enforcement: 'soft' } }));
    fireEvent.click(screen.getByRole('button', { name: 'Classificar agora' }));
    await waitFor(() => expect(ipc.classifyNow).toHaveBeenCalled());
  });

  it('speaks English when the locale is en and keeps the number shortcuts', async () => {
    setLocale('en');
    renderPage();
    await waitFor(() => expect(pendingRows()).toHaveLength(2));
    expect(screen.getByRole('heading', { level: 1, name: 'Review' })).toBeInTheDocument();
    expect(screen.getByText(/2 groups await your decision, 9 min still uncategorized/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Classify now' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Show classified groups' })).toHaveTextContent('Classified on this day (1)');
    // the 1–9 hint is one translated sentence with the keys rendered as <kbd>
    expect(screen.getByText((_, el) => el?.tagName === 'P' && el.textContent === 'Press 1 to 9 or pick a category on the right; your choice applies to all 2 blocks and teaches the classifier.')).toBeInTheDocument();
    expect(screen.getByText('2 blocks')).toBeInTheDocument();

    fireEvent.keyDown(window, { key: 'Enter' });
    const details = await screen.findByTestId('block-details');
    expect(within(details).getByText('2 blocks in this group')).toBeInTheDocument();
    expect(within(details).getByRole('button', { name: 'Data sent to the AI' })).toBeInTheDocument();
    expect(within(details).getByText('No screenshot for this block.')).toBeInTheDocument();

    fireEvent.keyDown(window, { key: '1' });
    await waitFor(() => expect(reclassifyGroup).toHaveBeenCalledWith('2026-09-17', 'net.whatsapp.WhatsApp|whatsapp', 'cat-ifro'));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Show classified groups' })).toHaveTextContent('Classified on this day (2)'));
  });
});
