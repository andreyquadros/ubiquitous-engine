import clsx from 'clsx';
import { Bot, Brain, Eye, ListChecks, Send, User, type LucideIcon } from 'lucide-react';
import { appColor, appInitial, fmtPercent, SOURCE_LABEL } from '../../lib/format';
import type { ClassificationSource, RuleSuggestion, Category } from '../../lib/types';
import { categoryName } from '../../lib/categories';
import { Badge } from './Badge';

const SOURCE_ICON: Record<ClassificationSource, LucideIcon> = { rule: ListChecks, memory: Brain, llm: Bot, vision: Eye, user: User };
const SOURCE_TONE = { rule: 'neutral', memory: 'violet', llm: 'brand', vision: 'accent', user: 'success' } as const;

export function SourceBadge({ source }: { source: ClassificationSource | null }) {
  if (!source) return <Badge tone="neutral">pendente</Badge>;
  const Icon = SOURCE_ICON[source];
  return (
    <Badge tone={SOURCE_TONE[source]} title={`Classificado por: ${SOURCE_LABEL[source]}`}>
      <Icon className="size-3" />
      {SOURCE_LABEL[source]}
    </Badge>
  );
}

export function AiSentBadge({ at }: { at: string | null }) {
  if (!at) return null;
  return (
    <Badge tone="neutral" title={`Enviado à IA em ${new Date(at).toLocaleString('pt-BR')}`}>
      <Send className="size-3" /> enviado à IA
    </Badge>
  );
}

export function ConfidenceBar({ value, className, showLabel = true }: { value: number; className?: string; showLabel?: boolean }) {
  const pct = Math.max(0, Math.min(1, value));
  const color = pct >= 0.8 ? '#10b981' : pct >= 0.6 ? '#f59e0b' : '#ef4444';
  return (
    <div className={clsx('flex items-center gap-2', className)} title={`Confiança: ${fmtPercent(pct)}`}>
      <div className="h-1.5 w-16 overflow-hidden rounded-full bg-surface-3">
        <div className="h-full rounded-full" style={{ width: `${pct * 100}%`, background: color }} />
      </div>
      {showLabel && <span className="text-[11px] tabular-nums text-ink-3">{fmtPercent(pct)}</span>}
    </div>
  );
}

export function AppAvatar({ name, size = 'md', className }: { name: string; size?: 'sm' | 'md' | 'lg'; className?: string }) {
  const dim = size === 'sm' ? 'size-6 text-[11px]' : size === 'lg' ? 'size-10 text-base' : 'size-8 text-sm';
  return (
    <span className={clsx('inline-flex shrink-0 items-center justify-center rounded-lg font-semibold text-white', dim, className)} style={{ background: appColor(name) }} aria-hidden>
      {appInitial(name)}
    </span>
  );
}

const MATCHER_LABEL = { app: 'app', domain: 'domínio', title_contains: 'título contém', regex: 'regex' } as const;

export function SuggestionChips({ suggestions, categories, onAccept, accepted }: { suggestions: RuleSuggestion[]; categories: Category[]; onAccept: (s: RuleSuggestion) => void; accepted: Set<string> }) {
  if (!suggestions.length) return null;
  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <span className="text-[11px] text-ink-3">Criar regra:</span>
      {suggestions.map((s) => {
        const key = `${s.matcher}:${s.pattern}`;
        const done = accepted.has(key);
        return (
          <button
            key={key}
            type="button"
            disabled={done}
            onClick={() => onAccept(s)}
            title={`${s.rationale} (${MATCHER_LABEL[s.matcher]})`}
            className={clsx(
              'inline-flex h-6 items-center gap-1 rounded-full border px-2 text-[11px] font-medium transition-colors',
              done
                ? 'border-emerald-200 bg-emerald-50 text-emerald-700 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-300'
                : 'border-brand-200 bg-brand-50 text-brand-700 hover:bg-brand-100 dark:border-brand-900/60 dark:bg-brand-900/30 dark:text-brand-300',
            )}
          >
            {done ? 'Regra criada' : 'Sempre'}: <span className="font-mono">{s.pattern}</span> → {categoryName(categories, s.category_id)}
          </button>
        );
      })}
    </div>
  );
}
