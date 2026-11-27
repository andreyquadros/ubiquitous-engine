import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ActivityBlock, Category } from '../lib/types';

const categories: Category[] = [
  { id: 'cat-ifro', name: 'IFRO', color: '#2563EB', icon: 'graduation-cap', description: '', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 0, created_at: '2026-01-01T00:00:00Z' },
  { id: 'cat-incubadora', name: 'Incubadora', color: '#F97316', icon: 'rocket', description: '', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 1, created_at: '2026-01-01T00:00:00Z' },
];

const block = (over: Partial<ActivityBlock>): ActivityBlock => ({
  id: 'b1',
  started_at: '2026-09-17T11:00:00Z',
  ended_at: '2026-09-17T11:30:00Z',
  app_name: 'Visual Studio Code',
  app_id: 'com.microsoft.VSCode',
  title: 'store.ts',
  title_key: 'store.ts',
  url: null,
  domain: null,
  category_id: 'cat-ifro',
  confidence: 0.92,
  source: 'rule',
  description: null,
  screenshot_id: null,
  sample_count: 12,
  is_open: false,
  classify_attempts: 1,
  next_attempt_at: null,
  needs_review: false,
  ai_payload: null,
  ai_sent_at: null,
  is_manual: false,
  note: null,
  ...over,
});

const blocks: ActivityBlock[] = [
  block({ id: 'b1' }),
  block({ id: 'b2', started_at: '2026-09-17T12:00:00Z', ended_at: '2026-09-17T12:20:00Z', app_name: 'WhatsApp', app_id: 'net.whatsapp.WhatsApp', title: 'WhatsApp', title_key: 'whatsapp', category_id: null, confidence: 0.4, source: 'llm', needs_review: true }),
];

vi.mock('../lib/ipc', () => ({
  isTauri: () => false,
  onEngineEvent: async () => () => undefined,
  ipc: {
    getTimeline: vi.fn(async () => blocks),
    listCategories: vi.fn(async () => categories),
    reclassify: vi.fn(),
    acceptRuleSuggestion: vi.fn(),
    splitBlock: vi.fn(),
    addManualEntry: vi.fn(),
    restartApp: vi.fn(),
    getSettings: vi.fn(),
    getDashboard: vi.fn(),
    updateSettings: vi.fn(),
  },
}));

import { Timeline } from './Timeline';
import { setLocale } from '../i18n';
import { useAppStore } from '../lib/store';

const renderPage = () =>
  render(
    <MemoryRouter>
      <Timeline />
    </MemoryRouter>,
  );

describe('Timeline page', () => {
  beforeEach(() => {
    useAppStore.setState({ categories, date: '2026-09-17', settingsView: null });
  });

  it('lists the blocks of the day in Portuguese with a summary written from the numbers', async () => {
    renderPage();
    await waitFor(() => expect(screen.getByRole('list', { name: 'Blocos do dia' })).toBeInTheDocument());
    expect(screen.getAllByRole('article')).toHaveLength(2);
    expect(screen.getByRole('button', { name: 'Adicionar atividade manual' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Somente não classificados' })).toBeInTheDocument();
    expect(screen.getByText('2 de 2 blocos')).toBeInTheDocument();
    expect(screen.getByText(/em 2 blocos; 1 ainda espera uma categoria\./)).toBeInTheDocument();
    expect(screen.getByText('precisa de revisão')).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Todas as categorias' })).toBeInTheDocument();
  });

  it('switches every label to English when the locale is en', async () => {
    setLocale('en');
    renderPage();
    await waitFor(() => expect(screen.getByRole('list', { name: 'Blocks for the day' })).toBeInTheDocument());
    expect(screen.getByRole('button', { name: 'Add manual activity' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Unclassified only' })).toBeInTheDocument();
    expect(screen.getByText('2 of 2 blocks')).toBeInTheDocument();
    expect(screen.getByText(/across 2 blocks; 1 still needs a category\./)).toBeInTheDocument();
    expect(screen.getByText('needs review')).toBeInTheDocument();
    expect(screen.queryByText('precisa de revisão')).not.toBeInTheDocument();
  });
});
