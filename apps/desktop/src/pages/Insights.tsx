import { Bell, Sparkles, TrendingUp } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { FocusTrend, WeeklyStacked, type WeekDay } from '../components/charts/Weekly';
import { NUDGE_ICON } from '../components/ubi/UbiCard';
import { Ubi } from '../components/ubi/Ubi';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { Card, CardHeader, StatTile } from '../components/ui/Card';
import { EmptyState, Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { fmtDateShort, fmtDateTime, fmtDuration, fmtNumber, NUDGE_KIND_LABEL, shiftDate, todayIso } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { Advice } from '../lib/types';
import { useAsync } from '../lib/useAsync';

export function Insights() {
  const categories = useAppStore((s) => s.categories);
  const dashboards = useAppStore((s) => s.dashboards);
  const loadDashboard = useAppStore((s) => s.loadDashboard);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const today = todayIso();
  const dates = useMemo(() => [...Array(7)].map((_, i) => shiftDate(today, i - 6)), [today]);

  useEffect(() => {
    for (const d of dates) void loadDashboard(d);
  }, [dates, loadDashboard, dataVersion]);

  const days: WeekDay[] = dates.map((date) => ({ date, data: dashboards[date] ?? null }));
  const loaded = days.filter((d) => d.data);
  const ready = loaded.length === days.length;
  const withData = loaded.filter((d) => (d.data?.stats.total_secs ?? 0) > 0);

  const totalProductive = withData.reduce((s, d) => s + (d.data?.stats.productive_secs ?? 0), 0);
  const avgScore = withData.length ? Math.round(withData.reduce((s, d) => s + (d.data?.stats.focus_score ?? 0), 0) / withData.length) : 0;
  const avgSwitches = withData.length ? withData.reduce((s, d) => s + (d.data?.stats.switches_per_hour ?? 0), 0) / withData.length : 0;
  const longest = withData.reduce((best, d) => ((d.data?.stats.longest_focus_secs ?? 0) > (best.secs ?? 0) ? { secs: d.data?.stats.longest_focus_secs ?? 0, date: d.date } : best), { secs: 0, date: today });

  return (
    <div data-testid="page-insights">
      <PageHeader title="Insights" subtitle={`Últimos 7 dias · ${fmtDateShort(dates[0] ?? today)} – ${fmtDateShort(today)}`} />

      <div className="mb-5 grid grid-cols-4 gap-4">
        <StatTile label="Tempo produtivo" value={ready ? fmtDuration(totalProductive) : '—'} accent="#10b981" hint="na semana" />
        <StatTile label="Score médio" value={ready ? avgScore : '—'} hint={`${withData.length} dia(s) com atividade`} />
        <StatTile label="Maior foco" value={ready ? fmtDuration(longest.secs, { compact: true }) : '—'} hint={ready && longest.secs ? fmtDateShort(longest.date) : 'contínuo'} />
        <StatTile label="Trocas/hora" value={ready ? fmtNumber(avgSwitches, 1) : '—'} hint="média" />
      </div>

      <div className="grid grid-cols-12 gap-5">
        <Card className="col-span-12 min-[1280px]:col-span-7">
          <CardHeader title="Horas por categoria" subtitle="Empilhado por dia" />
          {ready ? <WeeklyStacked days={days} categories={categories} /> : <Skeleton className="h-56" />}
        </Card>
        <Card className="col-span-12 min-[1280px]:col-span-5">
          <CardHeader title="Tendência do score de foco" subtitle="0–100 por dia" action={<TrendingUp className="size-4 text-ink-3" />} />
          {ready ? <FocusTrend days={days} /> : <Skeleton className="h-44" />}
        </Card>

        <div className="col-span-12 min-[1280px]:col-span-7">
          <AdviceCard />
        </div>
        <div className="col-span-12 min-[1280px]:col-span-5">
          <NudgeHistory />
        </div>
      </div>
    </div>
  );
}

function AdviceCard() {
  const [advice, setAdvice] = useState<Advice | null>(null);
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  const generate = async () => {
    setBusy(true);
    try {
      setAdvice(await ipc.getAdvice());
    } catch (e) {
      toast.error('Não foi possível gerar recomendações', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Card className="h-full">
      <CardHeader
        title="Recomendações do UBI"
        subtitle="Análise semanal gerada pela IA a partir dos seus padrões (custa uma chamada ao modelo de relatórios)."
        action={
          <Button size="sm" variant="primary" icon={<Sparkles className="size-3.5" />} loading={busy} onClick={() => void generate()}>
            {advice ? 'Gerar novamente' : 'Gerar'}
          </Button>
        }
      />
      <div className="flex gap-5">
        <div className="hidden shrink-0 min-[1100px]:block">
          <Ubi mood={busy ? 'focused' : advice ? 'excited' : 'calm'} size={110} speaking={busy ? 'Analisando sua semana…' : undefined} />
        </div>
        <div className="min-w-0 flex-1">
          {busy && !advice ? (
            <div className="flex flex-col gap-2">
              <Skeleton className="h-5 w-3/4" />
              <Skeleton className="h-4" />
              <Skeleton className="h-4" />
              <Skeleton className="h-4 w-5/6" />
            </div>
          ) : advice ? (
            <div>
              <p className="text-base font-semibold tracking-tight">{advice.headline}</p>
              <ol className="mt-3 flex flex-col gap-2.5">
                {advice.recommendations.map((r, i) => (
                  <li key={i} className="flex gap-3 text-sm leading-6 text-ink-2">
                    <span className="mt-1 flex size-5 shrink-0 items-center justify-center rounded-full bg-brand-50 text-[11px] font-semibold text-brand-700 dark:bg-brand-900/40 dark:text-brand-300">{i + 1}</span>
                    <span>{r}</span>
                  </li>
                ))}
              </ol>
              <p className="mt-3 text-[11px] text-ink-3">Modelo: {advice.model}</p>
            </div>
          ) : (
            <EmptyState className="py-6" title="Sem recomendações ainda" description="Clique em Gerar para o UBI analisar seus últimos dias e sugerir ajustes concretos de rotina." />
          )}
        </div>
      </div>
    </Card>
  );
}

function NudgeHistory() {
  const { data: nudges, loading } = useAsync(() => ipc.getNudges(30), []);
  return (
    <Card className="h-full" padded={false}>
      <div className="px-5 pt-5">
        <CardHeader title="Histórico de avisos" subtitle="O que o UBI te disse recentemente" action={<Bell className="size-4 text-ink-3" />} />
      </div>
      <ul className="scroll-thin max-h-[420px] divide-y divide-line overflow-y-auto">
        {loading && !nudges && (
          <li className="p-5">
            <Skeleton className="h-10" />
          </li>
        )}
        {nudges?.map((n) => {
          const Icon = NUDGE_ICON[n.kind];
          return (
            <li key={n.id} className="flex gap-3 px-5 py-3">
              <span className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg bg-surface-2 text-ink-2">
                <Icon className="size-3.5" />
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <p className="truncate text-sm font-medium">{n.title}</p>
                  {!n.seen && <Badge tone="accent">novo</Badge>}
                </div>
                <p className="mt-0.5 text-xs leading-5 text-ink-2">{n.message}</p>
                <p className="mt-1 text-[11px] text-ink-3">
                  {NUDGE_KIND_LABEL[n.kind]} · {fmtDateTime(n.at)}
                </p>
              </div>
            </li>
          );
        })}
        {nudges && !nudges.length && (
          <li>
            <EmptyState title="Nenhum aviso ainda" />
          </li>
        )}
      </ul>
    </Card>
  );
}
