import { describe, expect, it } from 'vitest';
import { setLocale } from '../i18n';
import { categoryDescription, categoryLabel, categoryName } from './categories';
import { SYSTEM_CATEGORIES, type Category } from './types';

const cat = (patch: Partial<Category>): Category => ({
  id: 'cat-x',
  name: 'IFRO',
  color: '#2563EB',
  icon: 'graduation-cap',
  description: 'Aulas e orientação.',
  keywords: [],
  report_time: null,
  report_template: null,
  is_productive: true,
  is_system: false,
  archived: false,
  sort_order: 0,
  created_at: '2026-09-18T10:00:00Z',
  ...patch,
});

// The DB migration seeds system categories in Portuguese; the display layer must localize them by id.
const distraction = cat({ id: SYSTEM_CATEGORIES.distraction, name: 'Distração', description: 'Redes sociais.', is_system: true });
const privateCat = cat({ id: SYSTEM_CATEGORIES.private, name: 'Privado', is_system: true });

describe('categoryLabel', () => {
  it('shows user categories by their own name in every language', () => {
    expect(categoryLabel(cat({}))).toBe('IFRO');
    setLocale('en');
    expect(categoryLabel(cat({}))).toBe('IFRO');
    expect(categoryDescription(cat({}))).toBe('Aulas e orientação.');
  });

  it('localizes system categories by id', () => {
    expect(categoryLabel(distraction)).toBe('Distração');
    expect(categoryLabel(privateCat)).toBe('Privado');
    expect(categoryDescription(distraction)).toBe('Redes sociais, entretenimento e outras atividades sem relação com o trabalho.');
    setLocale('en');
    expect(categoryLabel(distraction)).toBe('Distraction');
    expect(categoryLabel(privateCat)).toBe('Private');
    expect(categoryName([distraction], SYSTEM_CATEGORIES.distraction)).toBe('Distraction');
    expect(categoryDescription(distraction)).toBe('Social media, entertainment and anything else unrelated to work.');
  });

  it('falls back to the uncategorized label for unknown ids', () => {
    expect(categoryName([], null)).toBe('Sem categoria');
    setLocale('en');
    expect(categoryName([], 'missing')).toBe('Uncategorized');
  });
});
