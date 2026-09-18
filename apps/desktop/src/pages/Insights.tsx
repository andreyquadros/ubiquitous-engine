import clsx from 'clsx';
import { Bell, Check, Sparkles } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { FocusTrend, WeeklyStacked, type WeekDay } from '../components/charts/Weekly';
import { NUDGE_ICON } from '../components/ubi/UbiCard';
import { Ubi } from '../components/ubi/Ubi';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { Card, CardHeader, StatTile } from '../components/ui/Card';
import { EmptyState, Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { useT } from '../i18n';
import { fmtDateShort, fmtDateTime, fmtDuration, fmtNumber, shiftDate, todayIso } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { Advice } from '../lib/types';
import { useAsync } from '../lib/useAsync';

export function Insights() {
  const t = useT();
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
  const activeDays = withData.length;

  return (
    <div data-testid="page-insights">
      <PageHeader title={t('insights.title')} subtitle={t('insights.subtitle', { from: fmtDateShort(dates[0] ?? today), to: fmtDateShort(today) })} />

      <div className="mb-5 grid grid-cols-2 gap-4 min-[1000px]:grid-cols-4">
        <StatTile label={t('insights.stat.productive')} value={ready ? fmtDuration(totalProductive, { compact: true }) : '—'} accent="var(--signal)" hint={t('insights.stat.productive_hint')} />
        <StatTile label={t('insights.stat.avg_score')} value={ready ? avgScore : '—'} hint={ready ? t('insights.stat.active_days', { count: activeDays }) : t('insights.stat.calculating')} />
        <StatTile
          label={t('insights.stat.longest_focus')}
          value={ready ? fmtDuration(longest.secs, { compact: true }) : '—'}
          hint={ready && longest.secs ? t('insights.stat.longest_focus_on', { date: fmtDateShort(longest.date) }) : t('insights.stat.no_streaks')}
        />
        <StatTile label={t('insights.stat.switches_per_hour')} value={ready ? fmtNumber(avgSwitches, 1) : '—'} accent={ready && avgSwitches > 8 ? 'var(--amber)' : undefined} hint={t('insights.stat.switches_hint')} />
      </div>

      <div className="grid grid-cols-12 gap-5">
        <Card className="col-span-12 min-[1280px]:col-span-7">
          <CardHeader title={t('insights.hours.title')} subtitle={t('insights.hours.subtitle')} />
          {ready ? <WeeklyStacked days={days} categories={categories} /> : <Skeleton className="h-56" />}
        </Card>
        <Card className="col-span-12 min-[1280px]:col-span-5">
          <CardHeader title={t('insights.score.title')} subtitle={t('insights.score.subtitle')} />
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
  const t = useT();
  const [advice, setAdvice] = useState<Advice | null>(null);
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  const generate = async () => {
    setBusy(true);
    try {
      setAdvice(await ipc.getAdvice());
    } catch (e) {
      toast.error(t('insights.advice.error'), e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const speech = busy ? t('insights.advice.analysing') : advice ? advice.headline : undefined;

  return (
    <Card className="h-full">
      <CardHeader
        title={t('insights.advice.title')}
        subtitle={t('insights.advice.subtitle')}
        action={
          <Button size="sm" variant={advice ? 'secondary' : 'primary'} icon={<Sparkles className="size-3.5" strokeWidth={1.75} />} loading={busy} onClick={() => void generate()}>
            {advice ? t('insights.advice.generate_again') : t('common.generate')}
          </Button>
        }
      />
      <div className="grid grid-cols-1 gap-5 min-[900px]:grid-cols-[auto_1fr]">
        <div className="justify-self-center min-[900px]:justify-self-start">
          <Ubi mood={busy ? 'focused' : advice ? 'excited' : 'calm'} size={140} speaking={speech} />
        </div>
        <div className="min-w-0 self-center">
          {busy && !advice ? (
            <div className="flex flex-col gap-2.5" aria-busy="true">
              <Skeleton className="h-4 w-11/12" />
              <Skeleton className="h-4 w-4/5" />
              <Skeleton className="h-4 w-5/6" />
            </div>
          ) : advice ? (
            <div>
              <ul className="flex flex-col divide-y divide-line" aria-label={t('insights.advice.list_label')}>
                {advice.recommendations.map((r, i) => (
                  <li key={i} className="flex gap-3 py-2.5 first:pt-0 last:pb-0">
                    <span className="mt-[3px] flex size-5 shrink-0 items-center justify-center rounded-full bg-volt-soft text-volt" aria-hidden>
                      <Check className="size-3" strokeWidth={2} />
                    </span>
                    <span className="text-sm leading-6 text-ink">{r}</span>
                  </li>
                ))}
              </ul>
              <p className="mt-3 text-[11px] text-ink-3">{t('insights.advice.written_by', { model: advice.model })}</p>
            </div>
          ) : (
            <EmptyState className="items-start py-4 text-left" title={t('insights.advice.empty_title')} description={t('insights.advice.empty_description')} />
          )}
        </div>
      </div>
    </Card>
  );
}

function NudgeHistory() {
  const t = useT();
  const { data: nudges, loading } = useAsync(() => ipc.getNudges(30), []);
  return (
    <Card className="h-full" padded={false}>
      <div className="px-5 pt-5">
        <CardHeader title={t('insights.nudges.title')} subtitle={t('insights.nudges.subtitle')} action={<Bell className="size-4 text-ink-4" strokeWidth={1.75} aria-hidden />} />
      </div>
      <ul className="scroll-thin max-h-[420px] divide-y divide-line overflow-y-auto border-t border-line">
        {loading && !nudges && (
          <li className="p-5">
            <Skeleton className="h-10" />
          </li>
        )}
        {nudges?.map((n) => {
          const Icon = NUDGE_ICON[n.kind];
          return (
            <li key={n.id} className="flex gap-3 px-5 py-3">
              <span className={clsx('mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-[8px]', n.seen ? 'bg-panel-2 text-ink-3' : 'bg-ember-soft text-ember')} aria-hidden>
                <Icon className="size-3.5" strokeWidth={1.75} />
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <p className="truncate text-sm font-medium">{n.title}</p>
                  {!n.seen && <Badge tone="ember">{t('insights.nudges.new')}</Badge>}
                </div>
                <p className="mt-0.5 text-xs leading-5 text-ink-2">{n.message}</p>
                <p className="mt-1 flex items-center gap-3 text-[11px] text-ink-3">
                  <span>{t(`common.nudge_kind.${n.kind}`)}</span>
                  <span className="num">{fmtDateTime(n.at)}</span>
                </p>
              </div>
            </li>
          );
        })}
        {nudges && !nudges.length && (
          <li>
            <EmptyState title={t('insights.nudges.empty_title')} description={t('insights.nudges.empty_description')} />
          </li>
        )}
      </ul>
    </Card>
  );
}
