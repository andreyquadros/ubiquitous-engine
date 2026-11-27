import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLocale } from '../i18n';
import { fmtDateLong, fmtDateNumeric, fmtDateShort, fmtDuration, fmtHours, fmtNumber, fmtRelative, fmtUsd, fmtWeekday, KIND_LABEL, MOOD_LABEL, TRACKER_LABEL } from './format';

describe('format (locale-aware)', () => {
  beforeEach(() => setLocale('pt-BR'));
  afterEach(() => setLocale('pt-BR'));

  it('formats dates in Portuguese by default', () => {
    expect(fmtDateLong('2026-09-17')).toBe('quinta-feira, 17 de setembro');
    expect(fmtDateShort('2026-09-17')).toBe('17 set');
    expect(fmtWeekday('2026-09-17')).toBe('qui');
    expect(fmtDateNumeric('2026-09-17')).toBe('17/09/2026');
    expect(fmtHours(8640)).toBe('2,4 h');
    expect(fmtUsd(12.3)).toBe('US$ 12,30');
    expect(fmtNumber(1234.5, 1)).toBe('1.234,5');
  });

  it('formats dates in English when the locale is en', () => {
    setLocale('en');
    expect(fmtDateLong('2026-09-17')).toBe('Thursday, September 17');
    expect(fmtDateShort('2026-09-17')).toBe('Sep 17');
    expect(fmtWeekday('2026-09-17')).toBe('Thu');
    expect(fmtDateNumeric('2026-09-17')).toBe('09/17/2026');
    expect(fmtHours(8640)).toBe('2.4 h');
    expect(fmtUsd(12.3)).toBe('$12.30');
    expect(fmtNumber(1234.5, 1)).toBe('1,234.5');
  });

  it('keeps durations language-neutral', () => {
    expect(fmtDuration(30)).toBe('30s');
    expect(fmtDuration(45 * 60)).toBe('45min');
    expect(fmtDuration(2 * 3600 + 10 * 60)).toBe('2h 10min');
    expect(fmtDuration(2 * 3600 + 10 * 60, { compact: true })).toBe('2h10');
  });

  it('writes relative times per locale', () => {
    const now = new Date('2026-09-17T12:00:00Z');
    const ago = (mins: number) => new Date(now.getTime() - mins * 60_000).toISOString();
    expect(fmtRelative(ago(0), now)).toBe('agora');
    expect(fmtRelative(ago(5), now)).toBe('há 5 min');
    expect(fmtRelative(ago(3 * 60), now)).toBe('há 3 h');
    expect(fmtRelative(ago(26 * 60), now)).toBe('ontem');
    expect(fmtRelative(ago(3 * 24 * 60), now)).toBe('há 3 dias');
    setLocale('en');
    expect(fmtRelative(ago(0), now)).toBe('just now');
    expect(fmtRelative(ago(5), now)).toBe('5 min ago');
    expect(fmtRelative(ago(3 * 60), now)).toBe('3 h ago');
    expect(fmtRelative(ago(26 * 60), now)).toBe('yesterday');
    expect(fmtRelative(ago(3 * 24 * 60), now)).toBe('3 days ago');
  });

  it('exposes live label maps that follow the locale', () => {
    expect(MOOD_LABEL.focused).toBe('Focado');
    expect(TRACKER_LABEL.running).toBe('Rastreando');
    expect(Object.keys(KIND_LABEL)).toHaveLength(9);
    expect(KIND_LABEL.reuniao).toBe('Reunião');
    setLocale('en');
    expect(MOOD_LABEL.focused).toBe('Focused');
    expect(TRACKER_LABEL.running).toBe('Tracking');
    expect(KIND_LABEL.reuniao).toBe('Meeting');
  });
});
