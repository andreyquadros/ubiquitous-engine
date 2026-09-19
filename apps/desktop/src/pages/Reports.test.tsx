import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLocale } from '../i18n';
import type { Category, DailyReport } from '../lib/types';

const categories: Category[] = [
  { id: 'cat-ifro', name: 'IFRO', color: '#2563EB', icon: 'graduation-cap', description: '', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 0, created_at: '2026-01-01T00:00:00Z' },
  { id: 'cat-incubadora', name: 'Incubadora', color: '#F97316', icon: 'rocket', description: '', keywords: [], report_time: '17:30:00', report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 1, created_at: '2026-01-01T00:00:00Z' },
  { id: 'sys-distraction', name: 'Distração', color: '#EF4444', icon: 'tv', description: '', keywords: [], report_time: null, report_template: null, is_productive: false, is_system: true, archived: false, sort_order: 101, created_at: '2026-01-01T00:00:00Z' },
];

const report: DailyReport = {
  id: 'rep-1',
  date: '2026-09-17',
  category_id: 'cat-ifro',
  generated_at: '2026-09-17T21:05:00Z',
  summary_md: 'Manhã de aulas e correção de provas.',
  items: [
    { activity: 'Aula de Redes', kind: 'ensino', minutes: 95, evidence: ['Keynote · Redes 2026'], time_range: '08:00-09:35', continuation_of: null },
    { activity: 'Correção de provas', kind: 'documentacao', minutes: 40, evidence: [], time_range: '10:00-10:40', continuation_of: 'Aula de Redes' },
  ],
  highlights: ['95 min de aula'],
  total_secs: 8100,
  model: 'claude-sonnet-4-5',
  input_tokens: 12345,
  output_tokens: 678,
  stale: true,
  edited: false,
};

// Every real fetch deserializes fresh objects, so the double does too: a card that re-seeded its
// draft from object identity would throw away the user's unsaved edits on any refresh.
const getReports = vi.fn(async () => [structuredClone(report)]);

vi.mock('../lib/ipc', () => ({
  isTauri: () => false,
  onEngineEvent: async () => () => undefined,
  ipc: {
    getReports: (...args: unknown[]) => getReports(...(args as [])),
    generateReport: vi.fn(async () => report),
    updateReport: vi.fn(async (r: DailyReport) => r),
    getMonthlyReport: vi.fn(async () => '# Mensal'),
    listCategories: vi.fn(async () => categories),
    getSettings: vi.fn(),
    getDashboard: vi.fn(),
    updateSettings: vi.fn(),
  },
}));

import { Reports } from './Reports';
import { useAppStore } from '../lib/store';

const renderPage = () =>
  render(
    <MemoryRouter>
      <Reports />
    </MemoryRouter>,
  );

describe('Reports page', () => {
  beforeEach(() => {
    getReports.mockClear();
    useAppStore.setState({ categories, date: '2026-09-17', settingsView: null, reportsVersion: 0 });
  });

  it('renders the daily chrome in Portuguese: status line, badges, table and per-category empty state', async () => {
    renderPage();
    expect(screen.getByRole('heading', { level: 1, name: 'Relatórios' })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText(/1 de 2 relatórios prontos/)).toBeInTheDocument());

    // IFRO has a stale report: badge, meta line with formatted numbers, editable rows and the kind labels
    expect(screen.getByText('Desatualizado')).toBeInTheDocument();
    expect(screen.getByText(/tokens de entrada/)).toBeInTheDocument();
    expect(screen.getByText('12.345')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Atividades do relatório' })).toBeInTheDocument();
    expect(screen.getAllByRole('combobox', { name: 'Tipo' })).toHaveLength(2);
    expect(screen.getAllByRole('option', { name: 'Ensino' }).length).toBeGreaterThan(0);
    expect(screen.getByText('Continua de Aula de Redes')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Regenerar' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Copiar Markdown' })).toBeInTheDocument();

    // Incubadora has none: its own report time and the invitation to generate now
    expect(screen.getByText('Nenhum relatório de Incubadora para este dia')).toBeInTheDocument();
    expect(screen.getByText(/Ele será escrito às 17:30/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Gerar agora' })).toBeInTheDocument();

    // editing a row surfaces the save action
    fireEvent.change(screen.getAllByRole('spinbutton', { name: 'Minutos' })[0]!, { target: { value: '100' } });
    expect(await screen.findByRole('button', { name: 'Salvar alterações' })).toBeInTheDocument();
  });

  it('keeps an unsaved edit when the daily list is re-fetched', async () => {
    // The engine bumps `reportsVersion` whenever it finishes work, and every fetch brings fresh
    // objects: re-seeding the draft from those silently discarded what the user had just typed.
    renderPage();
    await waitFor(() => expect(screen.getByText(/1 de 2 relatórios prontos/)).toBeInTheDocument());
    const minutes = () => screen.getAllByRole('spinbutton', { name: 'Minutos' })[0]!;
    fireEvent.change(minutes(), { target: { value: '100' } });
    expect(screen.getByRole('button', { name: 'Salvar alterações' })).toBeInTheDocument();

    const before = getReports.mock.calls.length;
    act(() => useAppStore.setState({ reportsVersion: 1 }));
    await waitFor(() => expect(getReports.mock.calls.length).toBeGreaterThan(before));
    // the call alone proves nothing: let its result land and the effects run
    await act(async () => {});

    expect(minutes()).toHaveValue(100);
    expect(screen.getByRole('button', { name: 'Salvar alterações' })).toBeInTheDocument();
  });

  it('switches to the monthly tab and shows its empty state', async () => {
    renderPage();
    await waitFor(() => expect(screen.getByText(/relatórios prontos/)).toBeInTheDocument());
    fireEvent.click(screen.getByRole('tab', { name: 'Mensal' }));
    expect(screen.getByText('Nenhum mês gerado ainda')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Gerar relatório mensal' })).toBeInTheDocument();
    expect(screen.getByLabelText('Mês')).toHaveAttribute('type', 'month');
  });

  it('speaks English when the locale is en', async () => {
    setLocale('en');
    renderPage();
    expect(screen.getByRole('heading', { level: 1, name: 'Reports' })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText(/1 of 2 reports ready/)).toBeInTheDocument());
    expect(screen.getByText('Out of date')).toBeInTheDocument();
    expect(screen.getByText(/tokens in,/)).toBeInTheDocument();
    expect(screen.getByText('12,345')).toBeInTheDocument();
    expect(screen.getAllByRole('option', { name: 'Teaching' }).length).toBeGreaterThan(0);
    expect(screen.getByText('No Incubadora report for this day')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Generate now' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Regenerate' })).toBeInTheDocument();
    expect(screen.queryByText('Desatualizado')).not.toBeInTheDocument();
  });
});
