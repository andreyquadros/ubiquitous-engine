import { AlertTriangle, Bell, BrainCircuit, ChevronRight, Coffee, Eye, FileText, Hourglass, Sparkles, ThumbsUp, type LucideIcon } from 'lucide-react';
import { Link } from 'react-router-dom';
import { fmtRelative, fmtTime, fmtUsd } from '../../lib/format';
import { useAppStore } from '../../lib/store';
import type { AiHealth, DashboardData, Nudge, NudgeKind } from '../../lib/types';
import { Card } from '../ui/Card';
import { Progress } from '../ui/misc';
import { Ubi } from './Ubi';
import { MOOD_TIP } from './moods';

export const NUDGE_ICON: Record<NudgeKind, LucideIcon> = {
  unproductive: Hourglass,
  distracted: Eye,
  break_suggested: Coffee,
  praise: ThumbsUp,
  idle: Hourglass,
  report_ready: FileText,
  attention: AlertTriangle,
};

function aiStatus(h: AiHealth): { label: string; color: string; detail?: string } {
  switch (h.state) {
    case 'ok':
      return { label: 'IA ativa', color: '#10b981' };
    case 'not_configured':
      return { label: 'IA não configurada', color: '#94a3b8', detail: 'Adicione sua chave em Configurações → IA.' };
    case 'paused':
      return { label: 'IA pausada', color: '#f59e0b', detail: h.reason };
    case 'degraded':
      return { label: 'IA instável', color: '#ef4444', detail: `${h.reason} · até ${fmtTime(h.until)}` };
  }
}

export function UbiCard({ data, nudge }: { data: DashboardData; nudge: Nudge | null }) {
  const speech = useAppStore((s) => s.ubiSpeech);
  const aiHealth = useAppStore((s) => s.aiHealth);
  const mood = data.stats.mood;
  const status = aiStatus(aiHealth);
  const NudgeIcon = nudge ? NUDGE_ICON[nudge.kind] : Bell;
  const usagePct = data.budget_usd > 0 ? data.usage_month.cost_usd / data.budget_usd : 0;

  return (
    <Card className="flex flex-col gap-4 overflow-hidden" data-testid="ubi-card">
      <div className="flex flex-col items-center pt-1">
        <Ubi mood={mood} size={150} speaking={speech ?? nudge?.message ?? MOOD_TIP[mood]} />
      </div>

      {nudge && (
        <div className="flex items-start gap-2.5 rounded-xl bg-surface-2 p-3">
          <span className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-md bg-accent-100 text-accent-600 dark:bg-accent-900/30 dark:text-accent-300">
            <NudgeIcon className="size-3.5" />
          </span>
          <div className="min-w-0">
            <p className="text-sm font-medium">{nudge.title}</p>
            <p className="text-[11px] text-ink-3">{fmtRelative(nudge.at)}</p>
          </div>
        </div>
      )}

      <Link to="/review" className="group flex items-center justify-between rounded-xl border border-line px-3 py-2.5 text-sm transition-colors hover:bg-surface-2">
        <span className="flex items-center gap-2">
          <Sparkles className="size-4 text-accent-500" />
          Precisa de revisão: <strong className="tabular-nums">{data.needs_review}</strong> {data.needs_review === 1 ? 'bloco' : 'blocos'}
        </span>
        <ChevronRight className="size-4 text-ink-3 transition-transform group-hover:translate-x-0.5" />
      </Link>

      <div className="flex flex-col gap-2 text-xs">
        <div className="flex items-center gap-2">
          <BrainCircuit className="size-4 text-ink-3" />
          <span className="size-2 rounded-full" style={{ background: status.color }} />
          <span className="font-medium">{status.label}</span>
          {status.detail && <span className="truncate text-ink-3">· {status.detail}</span>}
        </div>
        <div>
          <div className="mb-1 flex justify-between text-ink-2">
            <span>Uso da IA no mês</span>
            <span className="tabular-nums">
              {fmtUsd(data.usage_month.cost_usd)} <span className="text-ink-3">de {fmtUsd(data.budget_usd)}</span>
            </span>
          </div>
          <Progress value={usagePct} color={usagePct > 0.9 ? '#ef4444' : usagePct > 0.7 ? '#f59e0b' : undefined} />
          <p className="mt-1 text-[11px] text-ink-3">{data.usage_month.calls.toLocaleString('pt-BR')} chamadas · {(data.usage_month.input_tokens / 1000).toFixed(0)}k tokens de entrada</p>
        </div>
      </div>
    </Card>
  );
}
