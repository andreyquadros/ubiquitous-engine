import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActivityBlock, Id, ScreenshotData } from '../../lib/types';

const shot: ScreenshotData = { mime: 'image/svg+xml', data_base64: btoa('<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400"/>'), width: 640, height: 400 };
const getScreenshot = vi.fn(async (blockId: Id): Promise<ScreenshotData | null> => (blockId === 'with-shot' ? shot : null));

vi.mock('../../lib/ipc', () => ({
  isTauri: () => false,
  onEngineEvent: async () => () => undefined,
  ipc: { getScreenshot: (...args: unknown[]) => getScreenshot(...(args as [Id])) },
}));

import { setLocale } from '../../i18n';
import { fmtTime } from '../../lib/format';
import { __clearScreenshotCache, BlockDetails } from './BlockDetails';

const block = (over: Partial<ActivityBlock> & { id: Id }): ActivityBlock => ({
  started_at: '2026-09-17T11:00:00Z',
  ended_at: '2026-09-17T11:30:00Z',
  app_name: 'Google Chrome',
  app_id: 'com.google.Chrome',
  title: 'Edital PROEX 2026 - Google Docs',
  title_key: 'edital proex 2026',
  url: 'https://docs.google.com/document/d/1XyZ/edit',
  domain: 'docs.google.com',
  category_id: 'cat-ifro',
  confidence: 0.74,
  source: 'llm',
  description: 'Leitura do edital PROEX',
  screenshot_id: null,
  sample_count: 360,
  is_open: false,
  classify_attempts: 1,
  next_attempt_at: null,
  needs_review: true,
  ai_payload: '{"app":"Google Chrome","title":"Edital PROEX 2026 - Google Docs","domain":"docs.google.com"}',
  ai_sent_at: '2026-09-17T11:32:00Z',
  is_manual: false,
  note: null,
  ...over,
});

describe('BlockDetails', () => {
  beforeEach(() => {
    getScreenshot.mockClear();
    __clearScreenshotCache();
  });

  it('shows a skeleton while the day is loading', () => {
    render(<BlockDetails blocks={[]} loading />);
    expect(screen.getByLabelText('Carregando os blocos do grupo')).toHaveAttribute('aria-busy', 'true');
  });

  it('lists the blocks oldest first with range, title, link, description and the collapsed AI payload', () => {
    render(<BlockDetails blocks={[block({ id: 'later', started_at: '2026-09-17T14:00:00Z', ended_at: '2026-09-17T14:10:00Z', title: 'Depois' }), block({ id: 'first' })]} />);
    expect(screen.getByText('2 blocos neste grupo')).toBeInTheDocument();
    const cards = screen.getAllByTestId('block-card');
    expect(cards.map((c) => c.getAttribute('aria-label'))).toEqual(['Edital PROEX 2026 - Google Docs', 'Depois']);
    const first = within(cards[0]!);
    expect(first.getByText(`${fmtTime('2026-09-17T11:00:00Z')} até ${fmtTime('2026-09-17T11:30:00Z')}, 30min`)).toBeInTheDocument();
    expect(first.getByText('https://docs.google.com/document/d/1XyZ/edit')).toHaveAttribute('title', 'https://docs.google.com/document/d/1XyZ/edit');
    expect(first.getByText('Leitura do edital PROEX')).toBeInTheDocument();
    expect(first.getByText('IA')).toBeInTheDocument();
    expect(first.getByTitle('Confiança: 74%')).toBeInTheDocument();

    const toggle = first.getByRole('button', { name: 'Dados enviados à IA' });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(first.queryByText(/"domain": "docs.google.com"/)).not.toBeInTheDocument();
    fireEvent.click(toggle);
    expect(first.getByRole('button', { name: 'Ocultar dados enviados à IA' })).toHaveAttribute('aria-expanded', 'true');
    expect(first.getByText(/"domain": "docs.google.com"/)).toBeInTheDocument();

    // no screenshot was ever taken: one short line plus the settings hint
    expect(first.getByText('Sem captura de tela para este bloco.')).toBeInTheDocument();
    expect(first.getByText(/Manter screenshots para revisão/)).toBeInTheDocument();
    expect(getScreenshot).not.toHaveBeenCalled();
  });

  it('fetches the screenshot lazily, caches it and opens it larger in a dialog', async () => {
    const b = block({ id: 'with-shot', screenshot_id: 'shot-1' });
    const { unmount } = render(<BlockDetails blocks={[b]} />);
    expect(screen.getByTestId('screenshot-skeleton')).toBeInTheDocument();
    const img = await screen.findByRole('img', { name: `Captura de Google Chrome às ${fmtTime(b.started_at)}` });
    expect(img).toHaveAttribute('src', `data:image/svg+xml;base64,${shot.data_base64}`);
    expect(getScreenshot).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole('button', { name: 'Ampliar captura' }));
    const dialog = await screen.findByRole('dialog', { name: 'Edital PROEX 2026 - Google Docs' });
    expect(within(dialog).getByRole('img')).toHaveAttribute('src', expect.stringMatching(/^data:image\/svg\+xml;base64,/));
    fireEvent.keyDown(window, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());

    // a second mount reuses the cache
    unmount();
    render(<BlockDetails blocks={[b]} />);
    await screen.findByRole('img');
    expect(getScreenshot).toHaveBeenCalledTimes(1);
  });

  it('says when the capture is gone and points at the setting', async () => {
    render(<BlockDetails blocks={[block({ id: 'gone', screenshot_id: 'shot-old' })]} />);
    expect(await screen.findByText('A captura foi apagada depois da classificação.')).toBeInTheDocument();
    expect(screen.getByText(/Manter screenshots para revisão/)).toBeInTheDocument();
  });

  it('speaks English', async () => {
    setLocale('en');
    render(<BlockDetails blocks={[block({ id: 'with-shot', screenshot_id: 'shot-1' })]} />);
    expect(screen.getByText('1 block in this group')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Data sent to the AI' })).toBeInTheDocument();
    expect(await screen.findByRole('button', { name: 'Enlarge screenshot' })).toBeInTheDocument();
  });
});
