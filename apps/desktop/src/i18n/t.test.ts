import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { __resetWarnings, getLocale, hasKey, LOCALES, normaliseLocale, setLocale, t, translate, useLocale, useLocaleStore, useT } from './index';
import { renderHook, act } from '@testing-library/react';

describe('t()', () => {
  beforeEach(() => {
    setLocale('pt-BR');
    __resetWarnings();
  });
  afterEach(() => {
    setLocale('pt-BR');
    vi.restoreAllMocks();
  });

  it('resolves namespaced keys in the current locale', () => {
    expect(t('nav.today')).toBe('Hoje');
    setLocale('en');
    expect(t('nav.today')).toBe('Today');
  });

  it('interpolates {placeholders} and leaves unknown ones alone', () => {
    expect(t('ui.remove_tag', { tag: 'IFRO' })).toBe('Remover IFRO');
    expect(translate('en', 'ui.pick_date', { date: '09/17/2026' })).toBe('Pick a date (09/17/2026)');
    expect(translate('en', 'ui.pick_date', {})).toBe('Pick a date ({date})');
  });

  it('picks the plural form from count', () => {
    expect(t('nav.to_review', { count: 1 })).toBe('1 bloco para revisar');
    expect(t('nav.to_review', { count: 8 })).toBe('8 blocos para revisar');
    expect(t('nav.to_review', { count: 0 })).toBe('0 blocos para revisar');
    expect(translate('en', 'common.blocks', { count: 1 })).toBe('1 block');
    expect(translate('en', 'common.blocks', { count: 2 })).toBe('2 blocks');
    // no plural forms for the key → the plain key is used and count is still interpolated
    expect(translate('en', 'ubi.usage_detail', { count: 3, calls: 3, tokens: 12 })).toBe('3 calls, 12k input tokens');
  });

  it('returns the key and warns once for a missing key', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    expect(t('nav.does_not_exist')).toBe('nav.does_not_exist');
    expect(t('nav.does_not_exist')).toBe('nav.does_not_exist');
    expect(warn).toHaveBeenCalledTimes(1);
    expect(hasKey('nav.does_not_exist')).toBe(false);
    expect(hasKey('nav.today')).toBe(true);
  });

  it('normalises language tags', () => {
    expect(normaliseLocale('pt')).toBe('pt-BR');
    expect(normaliseLocale('pt_BR')).toBe('pt-BR');
    expect(normaliseLocale('en-GB')).toBe('en');
    expect(normaliseLocale('EN')).toBe('en');
    expect(normaliseLocale('fr')).toBe('pt-BR');
    expect(normaliseLocale(undefined)).toBe('pt-BR');
  });

  it('lists both locales with their own-language labels', () => {
    expect(LOCALES.map((l) => l.id)).toEqual(['pt-BR', 'en']);
    expect(LOCALES.map((l) => l.label)).toEqual(['Português (Brasil)', 'English']);
  });

  it('setLocale updates <html lang>, localStorage and the store', () => {
    setLocale('en');
    expect(getLocale()).toBe('en');
    expect(document.documentElement.lang).toBe('en');
    expect(localStorage.getItem('ubiqx.locale')).toBe('en');
    expect(useLocaleStore.getState().locale).toBe('en');
  });

  it('useT re-renders with the locale', () => {
    const { result } = renderHook(() => ({ t: useT(), locale: useLocale() }));
    expect(result.current.locale).toBe('pt-BR');
    expect(result.current.t('common.save')).toBe('Salvar');
    act(() => setLocale('en'));
    expect(result.current.locale).toBe('en');
    expect(result.current.t('common.save')).toBe('Save');
  });
});
