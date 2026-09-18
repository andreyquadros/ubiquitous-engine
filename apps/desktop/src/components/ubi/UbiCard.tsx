import { AlertTriangle, Bell, BrainCircuit, Coffee, Eye, FileText, Hourglass, ThumbsUp, type LucideIcon } from 'lucide-react';
import { fmtRelative, fmtTime, fmtUsd } from '../../lib/format';
import { useAppStore } from '../../lib/store';
import type { AiHealth, DashboardData, Nudge, NudgeKind } from '../../lib/types';
import { Progress } from '../ui/misc';
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
      return { label: 'IA ativa', color: 'var(--signal)' };
    case 'not_configured':
      return { label: 'IA não configurada', color: 'var(--ink-4)', detail: 'Adicione sua chave em Configurações, na aba IA.' };
    case 'paused':
      return { label: 'IA pausada', color: 'var(--amber)', detail: h.reason };
    case 'degraded':
      return { label: 'IA instável', color: 'var(--rose)', detail: `${h.reason} até ${fmtTime(h.until)}` };
  }
}

/**
 * The assistant strip: UBI's latest nudge (or his tip for the mood), the AI status and this month's budget.
 * The mascot itself lives in the Hoje hero.
 */
export function UbiCard({ data, nudge }: { data: DashboardData; nudge: Nudge | null }) {
  const aiHealth = useAppStore((s) => s.aiHealth);
  const providerLabel = useAppStore((s) => {
    const v = s.settingsView;
    return v ? (v.providers.find((p) => p.id === v.settings.ai_provider)?.label ?? null) : null;
  });
  const status = aiStatus(aiHealth);
  const detail = status.detail ?? (aiHealth.state === 'ok' && providerLabel ? providerLabel : undefined);
  const NudgeIcon = nudge ? NUDGE_ICON[nudge.kind] : Bell;
  const usagePct = data.budget_usd > 0 ? data.usage_month.cost_usd / data.budget_usd : 0;
  const usageColor = usagePct > 0.9 ? 'var(--rose)' : usagePct > 0.7 ? 'var(--amber)' : 'var(--volt)';

  return (
    <section className="panel grid grid-cols-1 gap-5 p-5 min-[1100px]:grid-cols-[1.3fr_1fr_1fr]" data-testid="ubi-card" aria-label="Assistente">
      <div className="flex items-start gap-3">
        <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-control bg-ember-soft text-ember" aria-hidden>
          <NudgeIcon className="size-4" strokeWidth={1.75} />
        </span>
        <div className="min-w-0">
          <p className="text-sm font-medium">{nudge ? nudge.title : 'UBI diz'}</p>
          <p className="mt-0.5 text-xs leading-5 text-ink-2">{nudge ? nudge.message : MOOD_TIP[data.stats.mood]}</p>
          {nudge && <p className="mt-1 text-[11px] text-ink-3">{fmtRelative(nudge.at)}</p>}
        </div>
      </div>

      <div className="flex items-start gap-3 border-line min-[1100px]:border-l min-[1100px]:pl-5">
        <BrainCircuit className="mt-0.5 size-4 shrink-0 text-ink-3" strokeWidth={1.75} aria-hidden />
        <div className="min-w-0">
          <p className="flex items-center gap-2 text-sm font-medium">
            <span className="size-2 rounded-full" style={{ background: status.color, boxShadow: `0 0 0 3px color-mix(in oklab, ${status.color} 18%, transparent)` }} />
            {status.label}
          </p>
          {detail && <p className="mt-0.5 text-xs leading-5 text-ink-2">{detail}</p>}
        </div>
      </div>

      <div className="border-line min-[1100px]:border-l min-[1100px]:pl-5">
        <div className="mb-1.5 flex items-baseline justify-between gap-2 text-sm">
          <span className="font-medium">Uso da IA no mês</span>
          <span className="num text-xs text-ink-2">
            {fmtUsd(data.usage_month.cost_usd)} <span className="text-ink-3">de {fmtUsd(data.budget_usd)}</span>
          </span>
        </div>
        <Progress value={usagePct} color={usageColor} />
        <p className="num mt-1.5 text-[11px] text-ink-3">
          {data.usage_month.calls.toLocaleString('pt-BR')} chamadas, {(data.usage_month.input_tokens / 1000).toFixed(0)}k tokens de entrada
        </p>
      </div>
    </section>
  );
}
