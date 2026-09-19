import { render, screen, waitFor, within } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Category, Rule } from '../lib/types';

const categories: Category[] = [
  { id: 'cat-ifro', name: 'IFRO', color: '#2563EB', icon: 'graduation-cap', description: 'Aulas e orientação', keywords: ['sei'], report_time: '08:00:00', report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 0, created_at: '2026-01-01T00:00:00Z' },
  { id: 'cat-incubadora', name: 'Incubadora', color: '#F97316', icon: 'rocket', description: 'Mentorias', keywords: [], report_time: null, report_template: null, is_productive: true, is_system: false, archived: false, sort_order: 1, created_at: '2026-01-01T00:00:00Z' },
  { id: 'sys-distraction', name: 'Distração', color: '#EF4444', icon: 'tv', description: '', keywords: [], report_time: null, report_template: null, is_productive: false, is_system: true, archived: false, sort_order: 101, created_at: '2026-01-01T00:00:00Z' },
];

const rules: Rule[] = [
  { id: 'r1', category_id: 'cat-ifro', matcher: 'domain', pattern: 'sei.ifro.edu.br', priority: 10, origin: 'user', enabled: true, created_at: '2026-01-01T00:00:00Z', hit_count: 4, miss_count: 0, last_contradicted_at: null },
  { id: 'r2', category_id: 'cat-incubadora', matcher: 'title_contains', pattern: 'incubadora', priority: 20, origin: 'learned', enabled: true, created_at: '2026-01-02T00:00:00Z', hit_count: 2, miss_count: 1, last_contradicted_at: null },
];

vi.mock('../lib/ipc', () => ({
  isTauri: () => false,
  onEngineEvent: async () => () => undefined,
  ipc: {
    listCategories: vi.fn(async () => categories),
    saveCategory: vi.fn(async (c: Category) => c),
    deleteCategory: vi.fn(async () => undefined),
    listRules: vi.fn(async () => rules),
    saveRule: vi.fn(async (r: Rule) => r),
    deleteRule: vi.fn(async () => undefined),
    getSettings: vi.fn(),
    getDashboard: vi.fn(),
    updateSettings: vi.fn(),
  },
}));

import { Categories } from './Categories';
import { setLocale } from '../i18n';
import { ipc } from '../lib/ipc';

const renderPage = () =>
  render(
    <MemoryRouter>
      <Categories />
    </MemoryRouter>,
  );

describe('Categories page', () => {
  beforeEach(() => {
    vi.mocked(ipc.listRules).mockResolvedValue(rules);
  });

  it('lists the categories, opens the first one in the form and names the icons in Portuguese', async () => {
    renderPage();
    expect(screen.getByRole('heading', { level: 1, name: 'Categorias' })).toBeInTheDocument();

    const yours = await screen.findByRole('region', { name: 'Suas categorias' });
    expect(within(yours).getByText('Produtiva, relatório às 08:00')).toBeInTheDocument();
    expect(within(yours).getByText('Produtiva')).toBeInTheDocument();
    expect(within(screen.getByRole('region', { name: 'Do sistema' })).getByText('Categoria do sistema')).toBeInTheDocument();

    // the first user category is selected: its icon shows with a Portuguese name, not a lucide id
    const icons = await screen.findByRole('radiogroup', { name: 'Ícone' });
    expect(within(icons).getByRole('radio', { name: 'Formatura' })).toHaveAttribute('aria-checked', 'true');
    expect(within(icons).queryByRole('radio', { name: 'graduation-cap' })).not.toBeInTheDocument();
    expect(screen.getByLabelText('Nome')).toHaveValue('IFRO');
    expect(screen.getByRole('button', { name: 'Salvar alterações' })).toBeInTheDocument();
    // once the form is open, only the header keeps the "new" action
    expect(screen.getByRole('button', { name: 'Nova categoria' })).toBeInTheDocument();

    // rules: pluralised subtitle and the origin badges
    await waitFor(() => expect(screen.getByText('2 regras, 1 aprendida das suas correções. Rodam antes da memória e da IA, sem custo.')).toBeInTheDocument());
    expect(screen.getByText('Aprendida')).toBeInTheDocument();
    expect(screen.getByRole('switch', { name: 'Ativar regra sei.ifro.edu.br' })).toBeInTheDocument();
  });

  it('shows the empty rules state', async () => {
    vi.mocked(ipc.listRules).mockResolvedValueOnce([]);
    renderPage();
    expect(await screen.findByText('Nenhuma regra ainda')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Criar regra' })).toBeInTheDocument();
  });

  it('speaks English when the locale is en', async () => {
    setLocale('en');
    renderPage();
    expect(screen.getByRole('heading', { level: 1, name: 'Categories' })).toBeInTheDocument();

    const yours = await screen.findByRole('region', { name: 'Your categories' });
    expect(within(yours).getByText('Productive, report at 08:00')).toBeInTheDocument();

    const icons = await screen.findByRole('radiogroup', { name: 'Icon' });
    expect(within(icons).getByRole('radio', { name: 'Graduation cap' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'New category' })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText('2 rules, 1 learned from your corrections. They run before memory and AI, at no cost.')).toBeInTheDocument());
    expect(screen.getByText('Learned')).toBeInTheDocument();
  });
});
