import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { CategoryDonut } from '../components/charts/CategoryDonut';
import { DayBar } from '../components/charts/DayBar';
import { FocusRing } from '../components/charts/FocusRing';
import { HourlyFocus } from '../components/charts/HourlyFocus';
import { UbiCard } from '../components/ubi/UbiCard';
import { Card, CardHeader, StatTile } from '../components/ui/Card';
import { DayNav } from '../components/ui/DayNav';
import { AppAvatar } from '../components/ui/BlockBits';
import { Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { fmtDateLong, fmtDuration, fmtNumber, isToday } from '../lib/format';
import { useAppStore } from '../lib/store';

export function Dashboard() {
  const date = useAppStore((s) => s.date);
  const setDate = useAppStore((s) => s.setDate);
  const data = useAppStore((s) => s.dashboards[date]);
  const loadDashboard = useAppStore((s) => s.loadDashboard);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const latestNudge = useAppStore((s) => s.latestNudge);
  const navigate = useNavigate();

  useEffect(() => {
    void loadDashboard(date);
  }, [date, dataVersion, loadDashboard]);

  const title = isToday(date) ? 'Hoje' : 'Dia';

  return (
    <div data-testid="page-dashboard">
      <PageHeader title={title} subtitle={fmtDateLong(date)} actions={<DayNav date={date} onChange={setDate} />} />

      {!data ? (
        <div className="grid grid-cols-12 gap-5">
          <Skeleton className="col-span-8 h-40" />
          <Skeleton className="col-span-4 h-40" />
          <Skeleton className="col-span-8 h-64" />
          <Skeleton className="col-span-4 h-64" />
        </div>
      ) : (
        <div className="grid grid-cols-12 gap-5">
          <div className="col-span-12 flex flex-col gap-5 min-[1100px]:col-span-8">
            {/* score + tiles */}
            <div className="grid grid-cols-6 gap-4">
              <Card className="col-span-2 row-span-2 flex items-center justify-center gap-4">
                <FocusRing score={data.stats.focus_score} mood={data.stats.mood} />
                <div className="hidden min-[1280px]:block">
                  <p className="text-[11px] font-medium tracking-wide whitespace-nowrap text-ink-3 uppercase">Score de foco</p>
                  <p className="mt-1 max-w-32 text-xs leading-5 text-ink-2">
                    {fmtDuration(data.stats.total_secs)} registradas em {data.timeline.length} blocos.
                  </p>
                </div>
              </Card>
              <StatTile className="col-span-2" label="Tempo produtivo" value={fmtDuration(data.stats.productive_secs)} accent="#10b981" hint={`${Math.round((data.stats.productive_secs / Math.max(1, data.stats.total_secs)) * 100)}% do total`} />
              <StatTile className="col-span-2" label="Distrações" value={fmtDuration(data.stats.distraction_secs)} accent="#ef4444" hint={`${Math.round((data.stats.distraction_secs / Math.max(1, data.stats.total_secs)) * 100)}% do total`} />
              <StatTile className="col-span-2" label="Sem categoria" value={fmtDuration(data.stats.uncategorized_secs)} hint={data.needs_review ? `${data.needs_review} para revisar` : 'tudo classificado'} />
              <StatTile className="col-span-1" label="Maior foco" value={fmtDuration(data.stats.longest_focus_secs, { compact: true })} hint="contínuo" />
              <StatTile className="col-span-1" label="Trocas/h" value={fmtNumber(data.stats.switches_per_hour, 1)} hint={data.stats.switches_per_hour > 8 ? 'alto' : 'ok'} />
            </div>

            <div className="grid grid-cols-12 gap-5">
              <Card className="col-span-12 min-[1280px]:col-span-6">
                <CardHeader title="Tempo por categoria" subtitle="Horas registradas hoje" />
                <CategoryDonut totals={data.totals} categories={data.categories} height={150} />
              </Card>
              <Card className="col-span-12 min-[1280px]:col-span-6">
                <CardHeader title="Foco por hora" subtitle="% do tempo em categorias produtivas" />
                <HourlyFocus hourly={data.hourly_focus} height={170} />
              </Card>
            </div>

            <Card>
              <CardHeader title="Linha do tempo" subtitle="Clique em um bloco para abrir na Timeline" />
              <DayBar blocks={data.timeline} categories={data.categories} onSelect={(b) => navigate(`/timeline?block=${b.id}`)} />
              <ul className="mt-3 flex flex-wrap gap-x-4 gap-y-1" aria-label="Legenda">
                {data.categories
                  .filter((c) => data.totals.some((t) => t.category_id === c.id))
                  .map((c) => (
                    <li key={c.id} className="flex items-center gap-1.5 text-[11px] text-ink-2">
                      <span className="size-2 rounded-full" style={{ background: c.color }} />
                      {c.name}
                    </li>
                  ))}
                {data.totals.some((t) => !t.category_id) && (
                  <li className="flex items-center gap-1.5 text-[11px] text-ink-2">
                    <span className="size-2 rounded-full bg-slate-400/60" />
                    Sem categoria
                  </li>
                )}
              </ul>
            </Card>
          </div>

          <div className="col-span-12 flex flex-col gap-5 min-[1100px]:col-span-4">
            <UbiCard data={data} nudge={data.unseen_nudges[0] ?? latestNudge} />
            <Card>
              <CardHeader title="Apps mais usados" />
              <ul className="flex flex-col gap-2.5">
                {data.top_apps.slice(0, 6).map((a) => {
                  const pct = a.secs / Math.max(1, data.top_apps[0]?.secs ?? 1);
                  return (
                    <li key={a.app_id} className="flex items-center gap-3">
                      <AppAvatar name={a.app_name} size="sm" />
                      <div className="min-w-0 flex-1">
                        <div className="flex justify-between text-xs">
                          <span className="truncate font-medium">{a.app_name}</span>
                          <span className="tabular-nums text-ink-2">{fmtDuration(a.secs, { compact: true })}</span>
                        </div>
                        <div className="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-surface-3">
                          <div className="h-full rounded-full bg-brand-500/80" style={{ width: `${pct * 100}%` }} />
                        </div>
                      </div>
                    </li>
                  );
                })}
                {!data.top_apps.length && <li className="text-xs text-ink-3">Nenhum app registrado.</li>}
              </ul>
            </Card>
          </div>
        </div>
      )}
    </div>
  );
}
