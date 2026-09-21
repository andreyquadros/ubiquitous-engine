import { AlertTriangle, Bell, BrainCircuit, Coffee, Eye, FileText, Hourglass, Shield, ThumbsUp, type LucideIcon } from 'lucide-react';
import { fmtRelative, fmtTime, fmtUsd, intlLocale } from '../../lib/format';
import { providerLabel } from '../../lib/providers';
import { useAppStore } from '../../lib/store';
import type { AiHealth, DashboardData, Nudge, NudgeKind } from '../../lib/types';
import { useT, type Vars } from '../../i18n';
import { Progress } from '../ui/misc';

export const NUDGE_ICON: Record<NudgeKind, LucideIcon> = {
  unproductive: Hourglass,
  distracted: Eye,
  break_suggested: Coffee,
  praise: ThumbsUp,
  idle: Hourglass,
  report_ready: FileText,
  attention: AlertTriangle,
  focus_prompt: Shield,
};

function aiStatus(h: AiHealth, t: (key: string, vars?: Vars) => string): { label: string; color: string; detail?: string } {
  switch (h.state) {
    case 'ok':
      return { label: t('ubi.ai.ok'), color: 'var(--signal)' };
    case 'not_configured':
      return { label: t('ubi.ai.not_configured'), color: 'var(--ink-4)', detail: t('ubi.ai.not_configured_hint') };
    case 'paused':
      return { label: t('ubi.ai.paused'), color: 'var(--amber)', detail: h.reason };
    case 'degraded':
      return { label: t('ubi.ai.degraded'), color: 'var(--rose)', detail: t('ubi.ai.degraded_until', { reason: h.reason, time: fmtTime(h.until) }) };
  }
}

/**
 * The assistant strip: UBI's latest nudge (or his tip for the mood), the AI status and this month's budget.
 * The mascot itself lives in the Hoje hero.
 */
export function UbiCard({ data, nudge }: { data: DashboardData; nudge: Nudge | null }) {
  const t = useT();
  const aiHealth = useAppStore((s) => s.aiHealth);
  const providerName = useAppStore((s) => {
    const v = s.settingsView;
    const info = v ? v.providers.find((p) => p.id === v.settings.ai_provider) : undefined;
    return info ? providerLabel(info, t) : null;
  });
  const status = aiStatus(aiHealth, t);
  const detail = status.detail ?? (aiHealth.state === 'ok' && providerName ? providerName : undefined);
  const NudgeIcon = nudge ? NUDGE_ICON[nudge.kind] : Bell;
  const usagePct = data.budget_usd > 0 ? data.usage_month.cost_usd / data.budget_usd : 0;
  const usageColor = usagePct > 0.9 ? 'var(--rose)' : usagePct > 0.7 ? 'var(--amber)' : 'var(--volt)';

  return (
    <section className="panel grid grid-cols-1 gap-5 p-5 min-[1100px]:grid-cols-[1.3fr_1fr_1fr]" data-testid="ubi-card" aria-label={t('ubi.assistant')}>
      <div className="flex items-start gap-3">
        <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-control bg-ember-soft text-ember" aria-hidden>
          <NudgeIcon className="size-4" strokeWidth={1.75} />
        </span>
        <div className="min-w-0">
          <p className="text-sm font-medium">{nudge ? nudge.title : t('ubi.says')}</p>
          <p className="mt-0.5 text-xs leading-5 text-ink-2">{nudge ? nudge.message : t(`ubi.tip.${data.stats.mood}`)}</p>
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
          <span className="font-medium">{t('ubi.usage_month')}</span>
          <span className="num text-xs text-ink-2">
            {fmtUsd(data.usage_month.cost_usd)} <span className="text-ink-3">{t('ubi.usage_of', { budget: fmtUsd(data.budget_usd) })}</span>
          </span>
        </div>
        <Progress value={usagePct} color={usageColor} />
        <p className="num mt-1.5 text-[11px] text-ink-3">
          {t('ubi.usage_detail', { calls: data.usage_month.calls.toLocaleString(intlLocale()), tokens: (data.usage_month.input_tokens / 1000).toFixed(0) })}
        </p>
      </div>
    </section>
  );
}
