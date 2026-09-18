import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BlockGroup, Category } from '../lib/types';

const categories: Category[] = [
  { id: 'cat-ifro', name: 'IFRO', color: '#2563EB', icon: 'graduation-cap', description: '', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 0, created_at: '2026-01-01T00:00:00Z' },
  { id: 'cat-incubadora', name: 'Incubadora', color: '#F97316', icon: 'rocket', description: '', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 1, created_at: '2026-01-01T00:00:00Z' },
  { id: 'sys-distraction', name: 'Distração', color: '#EF4444', icon: 'tv', description: '', keywords: [], report_time: null, report_template: null, is_productive: false, is_system: true, archived: false, sort_order: 101, created_at: '2026-01-01T00:00:00Z' },
];

const groups: BlockGroup[] = [
  { key: 'net.whatsapp.WhatsApp|whatsapp', app_id: 'net.whatsapp.WhatsApp', app_name: 'WhatsApp', domain: null, title: 'WhatsApp', total_secs: 540, block_ids: ['b1', 'b2'], category_id: null, min_confidence: 0.4, source: 'llm', needs_review: true, description: 'Mensagens de alunos', first_started_at: '2026-09-17T11:00:00Z' },
  { key: 'com.google.Chrome|mail.google.com', app_id: 'com.google.Chrome', app_name: 'Google Chrome', domain: 'mail.google.com', title: 'Caixa de entrada - Gmail', total_secs: 1800, block_ids: ['b3'], category_id: 'cat-ifro', min_confidence: 0.62, source: 'llm', needs_review: true, description: null, first_started_at: '2026-09-17T11:10:00Z' },
];

const reclassifyGroup = vi.fn(async () => ({ block_ids: ['b3'], backfilled: 1, suggestions: [{ category_id: 'cat-ifro', matcher: 'domain', pattern: 'mail.google.com', support: 3, rationale: 'x', auto_apply_safe: true }], auto_rules: [], disabled_rules: [] }));

vi.mock('../lib/ipc', () => ({
  isTauri: () => false,
  onEngineEvent: async () => () => undefined,
  ipc: {
    getReviewGroups: vi.fn(async () => groups),
    listCategories: vi.fn(async () => categories),
    reclassifyGroup: (...args: unknown[]) => reclassifyGroup(...(args as [])),
    acceptRuleSuggestion: vi.fn(async () => ({})),
    classifyNow: vi.fn(async () => ({ local: 0, remote: 0, vision: 0, needs_review: 0, skipped_remote: false })),
    getSettings: vi.fn(),
    getDashboard: vi.fn(),
    updateSettings: vi.fn(),
  },
}));

import { Review } from './Review';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';

describe('Review page', () => {
  beforeEach(() => {
    reclassifyGroup.mockClear();
    useAppStore.setState({ categories, date: '2026-09-17' });
  });

  it('lists groups and assigns the nth category with the keyboard', async () => {
    render(
      <MemoryRouter>
        <Review />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getAllByTestId('review-row')).toHaveLength(2));
    expect(screen.getByText(/2 grupos/)).toBeInTheDocument();

    // move to the second group and press "2" → Incubadora
    fireEvent.keyDown(window, { key: 'ArrowDown' });
    fireEvent.keyDown(window, { key: '2' });

    await waitFor(() => expect(reclassifyGroup).toHaveBeenCalledTimes(1));
    expect(reclassifyGroup).toHaveBeenCalledWith('2026-09-17', 'com.google.Chrome|mail.google.com', 'cat-incubadora');

    // the rule suggestion chip shows up after the correction
    await waitFor(() => expect(screen.getByText(/mail\.google\.com/, { selector: 'button span' })).toBeInTheDocument());
  });

  it('celebrates an empty queue with UBI and a way forward', async () => {
    vi.mocked(ipc.getReviewGroups).mockResolvedValueOnce([]);
    render(
      <MemoryRouter>
        <Review />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText('Nada para revisar')).toBeInTheDocument());
    expect(screen.getByTestId('ubi')).toBeInTheDocument();
    expect(screen.getByText(/a fila está vazia/)).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Ver a Timeline' })).toHaveAttribute('href', '/timeline');
    expect(screen.queryAllByTestId('review-row')).toHaveLength(0);
  });

  it('ignores shortcuts while typing in an input', async () => {
    render(
      <MemoryRouter>
        <Review />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getAllByTestId('review-row')).toHaveLength(2));
    const input = document.createElement('input');
    document.body.appendChild(input);
    fireEvent.keyDown(input, { key: '1' });
    expect(reclassifyGroup).not.toHaveBeenCalled();
    input.remove();
  });
});
