import clsx from 'clsx';
import { Bot, Brain, Eye, ListChecks, Send, User, type LucideIcon } from 'lucide-react';
import { appColor, appInitial, fmtPercent, intlLocale } from '../../lib/format';
import type { ClassificationSource, RuleSuggestion, Category } from '../../lib/types';
import { categoryById, categoryLabel } from '../../lib/categories';
import { useT } from '../../i18n';
import { Badge } from './Badge';

const SOURCE_ICON: Record<ClassificationSource, LucideIcon> = { rule: ListChecks, memory: Brain, llm: Bot, vision: Eye, user: User };
const SOURCE_TONE = { rule: 'neutral', memory: 'violet', llm: 'volt', vision: 'ember', user: 'signal' } as const;

export function SourceBadge({ source }: { source: ClassificationSource | null }) {
  const t = useT();
  if (!source) return <Badge tone="neutral">{t('ui.pending')}</Badge>;
  const Icon = SOURCE_ICON[source];
  const label = t(`common.source.${source}`);
  return (
    <Badge tone={SOURCE_TONE[source]} title={t('ui.classified_by', { source: label })}>
      <Icon className="size-3" strokeWidth={1.75} aria-hidden />
      {label}
    </Badge>
  );
}

export function AiSentBadge({ at }: { at: string | null }) {
  const t = useT();
  if (!at) return null;
  return (
    <Badge tone="neutral" title={t('ui.sent_to_ai_at', { at: new Date(at).toLocaleString(intlLocale()) })}>
      <Send className="size-3" strokeWidth={1.75} aria-hidden /> {t('ui.sent_to_ai')}
    </Badge>
  );
}

export function ConfidenceBar({ value, className, showLabel = true }: { value: number; className?: string; showLabel?: boolean }) {
  const t = useT();
  const pct = Math.max(0, Math.min(1, value));
  const color = pct >= 0.8 ? 'var(--signal)' : pct >= 0.6 ? 'var(--amber)' : 'var(--rose)';
  return (
    <div className={clsx('flex items-center gap-2', className)} title={t('ui.confidence', { value: fmtPercent(pct) })}>
      <div className="h-1.5 w-16 overflow-hidden rounded-pill bg-panel-3">
        <div className="h-full rounded-pill" style={{ width: `${pct * 100}%`, background: color }} />
      </div>
      {showLabel && <span className="num text-[11px] text-ink-3">{fmtPercent(pct)}</span>}
    </div>
  );
}

export function AppAvatar({ name, size = 'md', className }: { name: string; size?: 'sm' | 'md' | 'lg'; className?: string }) {
  const dim = size === 'sm' ? 'size-6 text-[11px] rounded-[7px]' : size === 'lg' ? 'size-10 text-base rounded-control' : 'size-8 text-sm rounded-[9px]';
  const color = appColor(name);
  return (
    <span
      className={clsx('inline-flex shrink-0 items-center justify-center border font-semibold', dim, className)}
      style={{ color, borderColor: `color-mix(in oklab, ${color} 35%, transparent)`, background: `color-mix(in oklab, ${color} 14%, transparent)` }}
      aria-hidden
    >
      {appInitial(name)}
    </span>
  );
}

export function SuggestionChips({ suggestions, categories, onAccept, accepted }: { suggestions: RuleSuggestion[]; categories: Category[]; onAccept: (s: RuleSuggestion) => void; accepted: Set<string> }) {
  const t = useT();
  if (!suggestions.length) return null;
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <span className="text-[11px] text-ink-3">{t('ui.create_rule')}</span>
      {suggestions.map((s) => {
        const key = `${s.matcher}:${s.pattern}`;
        const done = accepted.has(key);
        return (
          <button
            key={key}
            type="button"
            disabled={done}
            onClick={() => onAccept(s)}
            title={`${s.rationale} (${t(`common.matcher.${s.matcher}`)})`}
            className={clsx(
              'inline-flex h-6 items-center gap-1 rounded-pill border px-2 text-[11px] font-medium transition-colors duration-150',
              done ? 'border-signal/30 bg-signal/12 text-signal' : 'border-volt/30 bg-volt/12 text-volt hover:bg-volt/20',
            )}
          >
            {done ? t('ui.rule_created') : t('ui.always')}: <span className="font-mono">{s.pattern}</span> → {categoryLabel(categoryById(categories, s.category_id))}
          </button>
        );
      })}
    </div>
  );
}
