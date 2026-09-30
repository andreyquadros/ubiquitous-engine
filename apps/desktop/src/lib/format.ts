// Formatting helpers. Locale-aware (pt-BR / en) through the i18n locale store; kept free of React so they can be unit tested.
import { addDays, format, parseISO, startOfDay, differenceInMinutes } from 'date-fns';
import { enUS, ptBR } from 'date-fns/locale';
import { getLocale, t, type Locale } from '../i18n';
import type { HHMM, IsoDate, IsoDateTime, Mood, TrackerState, ClassificationSource } from './types';

const DATE_FNS_LOCALE: Record<Locale, typeof ptBR> = { 'pt-BR': ptBR, en: enUS };
const INTL_LOCALE: Record<Locale, string> = { 'pt-BR': 'pt-BR', en: 'en-US' };

const PATTERNS: Record<Locale, { long: string; short: string; weekday: string; numeric: string; dateTime: string }> = {
  'pt-BR': { long: "EEEE, d 'de' MMMM", short: 'd MMM', weekday: 'EEEEEE', numeric: 'dd/MM/yyyy', dateTime: 'dd/MM HH:mm' },
  en: { long: 'EEEE, MMMM d', short: 'MMM d', weekday: 'EEE', numeric: 'MM/dd/yyyy', dateTime: 'MM/dd HH:mm' },
};

const dfLocale = () => DATE_FNS_LOCALE[getLocale()];
const pattern = () => PATTERNS[getLocale()];
/** BCP-47 tag for `toLocaleString` and friends ('pt-BR' / 'en-US'). */
export const intlLocale = (): string => INTL_LOCALE[getLocale()];

export const todayIso = (): IsoDate => format(new Date(), 'yyyy-MM-dd');

export const toIsoDate = (d: Date): IsoDate => format(d, 'yyyy-MM-dd');

export const fromIsoDate = (d: IsoDate): Date => parseISO(d + 'T00:00:00');

export const shiftDate = (d: IsoDate, days: number): IsoDate => toIsoDate(addDays(fromIsoDate(d), days));

export const isToday = (d: IsoDate): boolean => d === todayIso();

/** pt "quarta-feira, 17 de setembro" / en "Wednesday, September 17" */
export const fmtDateLong = (d: IsoDate): string => format(fromIsoDate(d), pattern().long, { locale: dfLocale() });

/** pt "17 set" / en "Sep 17" */
export const fmtDateShort = (d: IsoDate): string => format(fromIsoDate(d), pattern().short, { locale: dfLocale() });

/** pt "qua" / en "Wed" */
export const fmtWeekday = (d: IsoDate): string => format(fromIsoDate(d), pattern().weekday, { locale: dfLocale() });

/** pt "17/09/2026" / en "09/17/2026" */
export const fmtDateNumeric = (d: IsoDate): string => format(fromIsoDate(d), pattern().numeric);

/** "08:15" from an RFC3339 timestamp, in local time. */
export const fmtTime = (iso: IsoDateTime): string => format(new Date(iso), 'HH:mm');

/** pt "17/09 08:15" / en "09/17 08:15" */
export const fmtDateTime = (iso: IsoDateTime): string => format(new Date(iso), pattern().dateTime);

/** Relative label such as "há 5 min" / "5 min ago". */
export const fmtRelative = (iso: IsoDateTime, now = new Date()): string => {
  const mins = differenceInMinutes(now, new Date(iso));
  if (mins < 1) return t('common.relative.now');
  if (mins < 60) return t('common.relative.minutes', { count: mins });
  const h = Math.floor(mins / 60);
  if (h < 24) return t('common.relative.hours', { count: h });
  const d = Math.floor(h / 24);
  return d === 1 ? t('common.relative.yesterday') : t('common.relative.days', { count: d });
};

/** Seconds → "2h 15min" / "45min" / "30s" (same in both languages). */
export const fmtDuration = (secs: number, opts: { compact?: boolean } = {}): string => {
  const s = Math.max(0, Math.round(secs));
  if (s < 60) return `${s}s`;
  const h = Math.floor(s / 3600);
  const m = Math.round((s % 3600) / 60);
  if (h === 0) return `${m}min`;
  if (m === 0) return `${h}h`;
  return opts.compact ? `${h}h${String(m).padStart(2, '0')}` : `${h}h ${m}min`;
};

/** Seconds → pt "2,4 h" / en "2.4 h" */
export const fmtHours = (secs: number): string => `${(secs / 3600).toLocaleString(intlLocale(), { maximumFractionDigits: 1 })} h`;

export const fmtMinutes = (secs: number): string => `${Math.round(secs / 60)} min`;

export const fmtPercent = (v: number): string => `${Math.round(v * 100)}%`;

/** pt "US$ 12,30" / en "$12.30" */
export const fmtUsd = (v: number): string => {
  const n = v.toLocaleString(intlLocale(), { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  return getLocale() === 'en' ? `$${n}` : `US$ ${n}`;
};

export const fmtNumber = (v: number, digits = 0): string => v.toLocaleString(intlLocale(), { maximumFractionDigits: digits, minimumFractionDigits: digits });

/** "18:00:00" → "18:00" for <input type="time"> */
export const hhmmToInput = (v: HHMM | null | undefined): string => (v ? v.slice(0, 5) : '');

/** "18:00" → "18:00:00" (chrono NaiveTime). */
export const inputToHhmm = (v: string): HHMM => (v.length === 5 ? `${v}:00` : v);

/** Local ISO date + "HH:MM" → RFC3339 UTC timestamp. */
export const localTimeToIso = (date: IsoDate, hhmm: string): IsoDateTime => {
  const [h, m] = hhmm.split(':').map(Number);
  const d = startOfDay(fromIsoDate(date));
  d.setHours(h ?? 0, m ?? 0, 0, 0);
  return d.toISOString();
};

/** Fraction (0..1) of the local day elapsed at `iso`. */
export const dayFraction = (iso: IsoDateTime): number => {
  const d = new Date(iso);
  return (d.getHours() * 3600 + d.getMinutes() * 60 + d.getSeconds()) / 86400;
};

/**
 * Builds a label map whose values are read from i18n on every access, so `LABEL[key]` and `Object.keys(LABEL)`
 * keep working while following the current locale. Inside components prefer `const t = useT()` and
 * `t('common.mood.focused')` so the component re-renders when the language changes.
 */
const liveLabels = <K extends string>(prefix: string, keys: readonly K[]): Record<K, string> => {
  const o = {} as Record<K, string>;
  for (const k of keys) Object.defineProperty(o, k, { get: () => t(`${prefix}.${k}`), enumerable: true });
  return o;
};

export const MOODS: readonly Mood[] = ['sleeping', 'calm', 'focused', 'excited', 'worried'];
export const TRACKER_STATES: readonly TrackerState[] = ['running', 'paused', 'private', 'idle', 'blocked'];
export const SOURCES: readonly ClassificationSource[] = ['rule', 'memory', 'llm', 'vision', 'user'];
export const KINDS = ['desenvolvimento', 'reuniao', 'comunicacao', 'documentacao', 'ensino', 'pesquisa', 'extensao', 'gestao', 'outro'] as const;
export const NUDGE_KINDS = ['unproductive', 'distracted', 'break_suggested', 'praise', 'idle', 'report_ready', 'attention', 'focus_prompt'] as const;

/** Live (locale-following) label maps: keys → t('common.mood.*') etc. */
export const MOOD_LABEL: Record<Mood, string> = liveLabels('common.mood', MOODS);
export const TRACKER_LABEL: Record<TrackerState, string> = liveLabels('common.tracker', TRACKER_STATES);
export const SOURCE_LABEL: Record<ClassificationSource, string> = liveLabels('common.source', SOURCES);
export const KIND_LABEL: Record<string, string> = liveLabels('common.kind', KINDS);
export const NUDGE_KIND_LABEL: Record<string, string> = liveLabels('common.nudge_kind', NUDGE_KINDS);

export const moodLabel = (m: Mood): string => t(`common.mood.${m}`);
export const trackerLabel = (s: TrackerState): string => t(`common.tracker.${s}`);
export const sourceLabel = (s: ClassificationSource): string => t(`common.source.${s}`);
export const kindLabel = (k: string): string => t(`common.kind.${k}`);
export const nudgeKindLabel = (k: string): string => t(`common.nudge_kind.${k}`);

/** Semantic colour for a focus score (CSS variables from index.css so it follows the theme). */
export const focusColor = (score: number): string => {
  if (score >= 75) return 'var(--signal)';
  if (score >= 50) return 'var(--volt)';
  if (score >= 30) return 'var(--amber)';
  return 'var(--rose)';
};

/** Seconds → "mm:ss" (hours fold into the minutes: 5400 → "90:00"); used by the focus session countdown. */
export const fmtCountdown = (secs: number): string => {
  const s = Math.max(0, Math.round(secs));
  return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`;
};

/** Case- and diacritic-insensitive form of a string, for "contains" searches ("Calendário" → "calendario"). */
export const foldText = (s: string): string =>
  s
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .trim();

/** True when the text reads as a domain: has a dot, no spaces, no scheme junk left after trimming ("youtube.com", "https://www.x.com/y"). */
export const looksLikeDomain = (s: string): boolean => {
  const d = normalizeDomain(s);
  return d.length > 3 && d.includes('.') && !d.includes(' ') && /^[a-z0-9.-]+$/.test(d) && !d.startsWith('.') && !d.endsWith('.');
};

/** Lower-case domain with the scheme, path, query, port and a leading "www." stripped (mirrors the Rust normalisation of a site key). */
export const normalizeDomain = (s: string): string => {
  let d = s.trim().toLowerCase();
  d = d.replace(/^[a-z][a-z0-9+.-]*:\/\//, '');
  d = d.split(/[/?#]/)[0] ?? '';
  d = d.replace(/:\d+$/, '');
  d = d.replace(/^www\./, '');
  return d;
};

/** Initial letter used for app avatars. */
export const appInitial = (name: string): string => (name.trim()[0] ?? '?').toUpperCase();

/** Deterministic colour for an app name (used by avatars as a tint, not a fill). */
export const appColor = (name: string): string => {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) % 360;
  return `hsl(${h} 70% 62%)`;
};
