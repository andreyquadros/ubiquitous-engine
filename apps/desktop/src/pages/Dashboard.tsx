import { ChevronRight, Sparkles } from 'lucide-react';
import { useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { CategoryDonut } from '../components/charts/CategoryDonut';
import { DayBar } from '../components/charts/DayBar';
import { FocusDial } from '../components/charts/FocusRing';
import { HourlyFocus } from '../components/charts/HourlyFocus';
import { Ubi } from '../components/ubi/Ubi';
import { UbiCard } from '../components/ubi/UbiCard';
import { MOOD_TIP } from '../components/ubi/moods';
import { Card, CardHeader } from '../components/ui/Card';
import { DayNav } from '../components/ui/DayNav';
import { AppAvatar } from '../components/ui/BlockBits';
import { Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { fmtDateLong, fmtDuration, fmtNumber, isToday } from '../lib/format';
import { useAppStore } from '../lib/store';
import type { DashboardData, IsoDate } from '../lib/types';

/** One sentence written from the numbers: what happened and what waits for the user. */
function headline(data: DashboardData, date: IsoDate): string {
  const { stats } = data;
  if (stats.total_secs === 0) return isToday(date) ? 'Nada registrado ainda. Quando você começar, eu registro.' : 'Nada foi registrado neste dia.';

  const today = isToday(date);
  const hour = new Date().getHours();
  const productive = new Set(data.categories.filter((c) => c.is_productive).map((c) => c.id));
  const noon = new Date();
  noon.setHours(12, 0, 0, 0);
  const sinceLunch = data.timeline
    .filter((b) => b.category_id && productive.has(b.category_id) && new Date(b.started_at) >= noon)
    .reduce((s, b) => s + Math.max(0, (new Date(b.ended_at).getTime() - new Date(b.started_at).getTime()) / 1000), 0);

  let first: string;
  if (today && hour >= 14 && sinceLunch >= 900) first = `${fmtDuration(sinceLunch, { compact: true })} de foco desde o almoço`;
  else if (today && hour >= 6 && hour < 12) first = `${fmtDuration(stats.productive_secs, { compact: true })} de foco nesta manhã`;
  else if (today) first = `${fmtDuration(stats.productive_secs, { compact: true })} de foco hoje`;
  else first = `${fmtDuration(stats.productive_secs, { compact: true })} de foco neste dia`;

  let second: string;
  if (data.needs_review > 0) second = data.needs_review === 1 ? '1 bloco espera sua revisão' : `${data.needs_review} blocos esperam sua revisão`;
  else if (stats.distraction_secs >= 300) second = `${fmtDuration(stats.distraction_secs, { compact: true })} de distração`;
  else if (stats.longest_focus_secs >= 1800) second = `maior sequência de ${fmtDuration(stats.longest_focus_secs, { compact: true })}`;
  else second = 'tudo classificado';
  return `${first}; ${second}.`;
}

function HeroStat({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div className="min-w-0">
      <p className="text-xs text-ink-3">{label}</p>
      <p className="display num mt-0.5 text-[20px] leading-6" style={color ? { color } : undefined}>
        {value}
      </p>
    </div>
  );
}

export function Dashboard() {
  const date = useAppStore((s) => s.date);
  const setDate = useAppStore((s) => s.setDate);
  const data = useAppStore((s) => s.dashboards[date]);
  const loadDashboard = useAppStore((s) => s.loadDashboard);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const latestNudge = useAppStore((s) => s.latestNudge);
  const ubiSpeech = useAppStore((s) => s.ubiSpeech);
  const navigate = useNavigate();

  useEffect(() => {
    void loadDashboard(date);
  }, [date, dataVersion, loadDashboard]);

  const title = isToday(date) ? 'Hoje' : 'Dia';
  const nudge = data ? (data.unseen_nudges[0] ?? latestNudge) : null;

  return (
    <div data-testid="page-dashboard">
      <PageHeader title={title} subtitle={fmtDateLong(date)} actions={<DayNav date={date} onChange={setDate} />} />

      {!data ? (
        <div className="flex flex-col gap-5">
          <Skeleton className="h-[264px] rounded-shell" />
          <Skeleton className="h-28" />
          <div className="grid grid-cols-12 gap-5">
            <Skeleton className="col-span-4 h-60" />
            <Skeleton className="col-span-4 h-60" />
            <Skeleton className="col-span-4 h-60" />
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-5">
          {/* Hero: the dial, the sentence, UBI. The only place with a glow behind it. */}
          <section className="relative overflow-hidden rounded-shell border border-line bg-panel" aria-label="Resumo do dia" data-testid="ubi-hero">
            <div className="pointer-events-none absolute inset-0" style={{ background: 'var(--hero-glow)' }} aria-hidden />
            <div className="relative grid grid-cols-1 items-center gap-6 p-6 min-[1000px]:grid-cols-[auto_1fr_auto] min-[1000px]:gap-8 min-[1000px]:px-8">
              <FocusDial score={data.stats.focus_score} mood={data.stats.mood} size={184} className="justify-self-center" />

              <div className="min-w-0">
                <h2 className="display max-w-[26ch] text-[24px] leading-8 text-ink min-[1280px]:text-[26px] min-[1280px]:leading-9">{headline(data, date)}</h2>
                <div className="mt-4 flex flex-wrap items-center gap-3">
                  {data.needs_review > 0 ? (
                    <Link
                      to="/review"
                      className="glow-ember group inline-flex h-10 items-center gap-2 rounded-control border border-ember/40 bg-ember-soft px-3.5 text-sm font-medium text-ember transition-colors duration-150 hover:bg-ember/20"
                    >
                      <Sparkles className="size-4" strokeWidth={1.75} aria-hidden />
                      <span>
                        Precisa de revisão: <span className="num">{data.needs_review}</span> {data.needs_review === 1 ? 'bloco' : 'blocos'}
                      </span>
                      <ChevronRight className="size-4 transition-transform duration-150 group-hover:translate-x-0.5" strokeWidth={1.75} aria-hidden />
                    </Link>
                  ) : (
                    <span className="inline-flex h-10 items-center gap-2 rounded-control border border-line px-3.5 text-sm text-ink-2">
                      <Sparkles className="size-4 text-signal" strokeWidth={1.75} aria-hidden />
                      Tudo classificado
                    </span>
                  )}
                  {data.stats.uncategorized_secs > 0 && (
                    <span className="text-xs text-ink-3">
                      <span className="num">{fmtDuration(data.stats.uncategorized_secs, { compact: true })}</span> sem categoria
                    </span>
                  )}
                </div>
                <div className="mt-5 grid grid-cols-2 gap-x-6 gap-y-3 min-[720px]:grid-cols-4">
                  <HeroStat label="Tempo produtivo" value={fmtDuration(data.stats.productive_secs, { compact: true })} color="var(--signal)" />
                  <HeroStat label="Distrações" value={fmtDuration(data.stats.distraction_secs, { compact: true })} color={data.stats.distraction_secs > 0 ? 'var(--rose)' : undefined} />
                  <HeroStat label="Maior foco contínuo" value={fmtDuration(data.stats.longest_focus_secs, { compact: true })} />
                  <HeroStat label="Trocas por hora" value={fmtNumber(data.stats.switches_per_hour, 1)} color={data.stats.switches_per_hour > 8 ? 'var(--amber)' : undefined} />
                </div>
              </div>

              <div className="justify-self-center min-[1000px]:justify-self-end">
                <Ubi mood={data.stats.mood} size={200} speaking={ubiSpeech ?? nudge?.message ?? MOOD_TIP[data.stats.mood]} />
              </div>
            </div>
          </section>

          <Card>
            <CardHeader title="Linha do tempo" subtitle="Clique em um bloco para abrir na Timeline" />
            <DayBar blocks={data.timeline} categories={data.categories} date={date} animate onSelect={(b) => navigate(`/timeline?block=${b.id}`)} />
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
                  <span className="size-2 rounded-full bg-ink-4/60" />
                  Sem categoria
                </li>
              )}
            </ul>
          </Card>

          <div className="grid grid-cols-12 gap-5">
            <Card className="col-span-12 min-[1100px]:col-span-5">
              <CardHeader title="Tempo por categoria" subtitle="Horas registradas no dia" />
              <CategoryDonut totals={data.totals} categories={data.categories} height={150} />
            </Card>
            <Card className="col-span-12 min-[1100px]:col-span-4">
              <CardHeader title="Foco por hora" subtitle="Parte do tempo em categorias produtivas" />
              <HourlyFocus hourly={data.hourly_focus} height={170} animate />
            </Card>
            <Card className="col-span-12 min-[1100px]:col-span-3">
              <CardHeader title="Apps mais usados" />
              <ul className="flex flex-col gap-2.5">
                {data.top_apps.slice(0, 6).map((a) => {
                  const pct = a.secs / Math.max(1, data.top_apps[0]?.secs ?? 1);
                  return (
                    <li key={a.app_id} className="flex items-center gap-2.5">
                      <AppAvatar name={a.app_name} size="sm" />
                      <div className="min-w-0 flex-1">
                        <div className="flex justify-between gap-2 text-xs">
                          <span className="truncate font-medium">{a.app_name}</span>
                          <span className="num text-ink-2">{fmtDuration(a.secs, { compact: true })}</span>
                        </div>
                        <div className="mt-1 h-1 w-full overflow-hidden rounded-pill bg-panel-3">
                          <div className="h-full rounded-pill bg-volt/80" style={{ width: `${pct * 100}%` }} />
                        </div>
                      </div>
                    </li>
                  );
                })}
                {!data.top_apps.length && <li className="text-xs text-ink-3">Nenhum app registrado ainda.</li>}
              </ul>
            </Card>
          </div>

          <UbiCard data={data} nudge={nudge} />
        </div>
      )}
    </div>
  );
}
