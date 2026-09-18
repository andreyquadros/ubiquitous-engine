// Formatting helpers (pt-BR). Kept free of React so they can be unit tested.
import { addDays, format, parseISO, startOfDay, differenceInMinutes } from 'date-fns';
import { ptBR } from 'date-fns/locale';
import type { HHMM, IsoDate, IsoDateTime, Mood, TrackerState, ClassificationSource } from './types';

export const todayIso = (): IsoDate => format(new Date(), 'yyyy-MM-dd');

export const toIsoDate = (d: Date): IsoDate => format(d, 'yyyy-MM-dd');

export const fromIsoDate = (d: IsoDate): Date => parseISO(d + 'T00:00:00');

export const shiftDate = (d: IsoDate, days: number): IsoDate => toIsoDate(addDays(fromIsoDate(d), days));

export const isToday = (d: IsoDate): boolean => d === todayIso();

/** "quarta-feira, 17 de setembro" */
export const fmtDateLong = (d: IsoDate): string => format(fromIsoDate(d), "EEEE, d 'de' MMMM", { locale: ptBR });

/** "17 set" */
export const fmtDateShort = (d: IsoDate): string => format(fromIsoDate(d), 'd MMM', { locale: ptBR });

/** "qua" */
export const fmtWeekday = (d: IsoDate): string => format(fromIsoDate(d), 'EEEEEE', { locale: ptBR });

/** "17/09/2026" */
export const fmtDateNumeric = (d: IsoDate): string => format(fromIsoDate(d), 'dd/MM/yyyy');

/** "08:15" from an RFC3339 timestamp, in local time. */
export const fmtTime = (iso: IsoDateTime): string => format(new Date(iso), 'HH:mm');

/** "17/09 08:15" */
export const fmtDateTime = (iso: IsoDateTime): string => format(new Date(iso), 'dd/MM HH:mm');

/** Relative label such as "há 5 min". */
export const fmtRelative = (iso: IsoDateTime, now = new Date()): string => {
  const mins = differenceInMinutes(now, new Date(iso));
  if (mins < 1) return 'agora';
  if (mins < 60) return `há ${mins} min`;
  const h = Math.floor(mins / 60);
  if (h < 24) return `há ${h} h`;
  const d = Math.floor(h / 24);
  return d === 1 ? 'ontem' : `há ${d} dias`;
};

/** Seconds → "2h 15min" / "45min" / "30s". */
export const fmtDuration = (secs: number, opts: { compact?: boolean } = {}): string => {
  const s = Math.max(0, Math.round(secs));
  if (s < 60) return `${s}s`;
  const h = Math.floor(s / 3600);
  const m = Math.round((s % 3600) / 60);
  if (h === 0) return `${m}min`;
  if (m === 0) return `${h}h`;
  return opts.compact ? `${h}h${String(m).padStart(2, '0')}` : `${h}h ${m}min`;
};

/** Seconds → "2,4 h" */
export const fmtHours = (secs: number): string => `${(secs / 3600).toLocaleString('pt-BR', { maximumFractionDigits: 1 })} h`;

export const fmtMinutes = (secs: number): string => `${Math.round(secs / 60)} min`;

export const fmtPercent = (v: number): string => `${Math.round(v * 100)}%`;

export const fmtUsd = (v: number): string => `US$ ${v.toLocaleString('pt-BR', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

export const fmtNumber = (v: number, digits = 0): string => v.toLocaleString('pt-BR', { maximumFractionDigits: digits, minimumFractionDigits: digits });

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

export const MOOD_LABEL: Record<Mood, string> = {
  sleeping: 'Descansando',
  calm: 'Tranquilo',
  focused: 'Focado',
  excited: 'Empolgado',
  worried: 'Preocupado',
};

export const TRACKER_LABEL: Record<TrackerState, string> = {
  running: 'Rastreando',
  paused: 'Pausado',
  private: 'Privado',
  idle: 'Ocioso',
  blocked: 'Bloqueado',
};

export const SOURCE_LABEL: Record<ClassificationSource, string> = {
  rule: 'regra',
  memory: 'memória',
  llm: 'IA',
  vision: 'visão',
  user: 'você',
};

export const KIND_LABEL: Record<string, string> = {
  desenvolvimento: 'Desenvolvimento',
  reuniao: 'Reunião',
  comunicacao: 'Comunicação',
  documentacao: 'Documentação',
  ensino: 'Ensino',
  pesquisa: 'Pesquisa',
  extensao: 'Extensão',
  gestao: 'Gestão',
  outro: 'Outro',
};

export const NUDGE_KIND_LABEL: Record<string, string> = {
  unproductive: 'Improdutivo',
  distracted: 'Distração',
  break_suggested: 'Pausa sugerida',
  praise: 'Elogio',
  idle: 'Ocioso',
  report_ready: 'Relatório pronto',
  attention: 'Atenção',
};

/** Semantic colour for a focus score (CSS variables from index.css so it follows the theme). */
export const focusColor = (score: number): string => {
  if (score >= 75) return 'var(--signal)';
  if (score >= 50) return 'var(--volt)';
  if (score >= 30) return 'var(--amber)';
  return 'var(--rose)';
};

/** Initial letter used for app avatars. */
export const appInitial = (name: string): string => (name.trim()[0] ?? '?').toUpperCase();

/** Deterministic colour for an app name (used by avatars as a tint, not a fill). */
export const appColor = (name: string): string => {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) % 360;
  return `hsl(${h} 70% 62%)`;
};
