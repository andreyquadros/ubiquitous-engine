import { ChevronRight, Shield, Sparkles } from 'lucide-react';
import { useEffect } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { CategoryDonut } from '../components/charts/CategoryDonut';
import { DayBar } from '../components/charts/DayBar';
import { FocusDial } from '../components/charts/FocusRing';
import { HourlyFocus } from '../components/charts/HourlyFocus';
import { FocusPrompt } from '../components/ubi/FocusPrompt';
import { useCountdown } from '../components/focus/useCountdown';
import { Ubi } from '../components/ubi/Ubi';
import { UbiCard } from '../components/ubi/UbiCard';
import { Button } from '../components/ui/Button';
import { Card, CardHeader } from '../components/ui/Card';
import { DayNav } from '../components/ui/DayNav';
import { AppAvatar } from '../components/ui/BlockBits';
import { Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { useT, type Vars } from '../i18n';
import { categoryLabel } from '../lib/categories';
import { fmtCountdown, fmtDateLong, fmtDuration, fmtNumber, isToday } from '../lib/format';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { DashboardData, FocusStatus, IsoDate } from '../lib/types';

type Translate = (key: string, vars?: Vars) => string;

/** One sentence written from the numbers: what happened and what waits for the user. */
function headline(t: Translate, data: DashboardData, date: IsoDate): string {
  const { stats } = data;
  if (stats.total_secs === 0) return isToday(date) ? t('dashboard.headline.empty_today') : t('dashboard.headline.empty_day');

  const today = isToday(date);
  const hour = new Date().getHours();
  const productive = new Set(data.categories.filter((c) => c.is_productive).map((c) => c.id));
  const noon = new Date();
  noon.setHours(12, 0, 0, 0);
  const sinceLunch = data.timeline
    .filter((b) => b.category_id && productive.has(b.category_id) && new Date(b.started_at) >= noon)
    .reduce((s, b) => s + Math.max(0, (new Date(b.ended_at).getTime() - new Date(b.started_at).getTime()) / 1000), 0);

  const productiveDuration = fmtDuration(stats.productive_secs, { compact: true });
  let first: string;
  if (today && hour >= 14 && sinceLunch >= 900) first = t('dashboard.headline.focus_since_lunch', { duration: fmtDuration(sinceLunch, { compact: true }) });
  else if (today && hour >= 6 && hour < 12) first = t('dashboard.headline.focus_morning', { duration: productiveDuration });
  else if (today) first = t('dashboard.headline.focus_today', { duration: productiveDuration });
  else first = t('dashboard.headline.focus_day', { duration: productiveDuration });

  let second: string;
  if (data.needs_review > 0) second = t('dashboard.headline.awaiting_review', { count: data.needs_review });
  else if (stats.distraction_secs >= 300) second = t('dashboard.headline.distraction', { duration: fmtDuration(stats.distraction_secs, { compact: true }) });
  else if (stats.longest_focus_secs >= 1800) second = t('dashboard.headline.longest_streak', { duration: fmtDuration(stats.longest_focus_secs, { compact: true }) });
  else second = t('dashboard.headline.all_sorted');
  return t('dashboard.headline.sentence', { first, second });
}

/** One line under the headline while a focus session runs: the task, mm:ss and "Encerrar". */
function SessionLine({ status }: { status: FocusStatus }) {
  const t = useT();
  const toast = useToast();
  const stopFocusSession = useAppStore((s) => s.stopFocusSession);
  const loadFocusStatus = useAppStore((s) => s.loadFocusStatus);
  const remaining = useCountdown(status.session, status.remaining_secs, () => void loadFocusStatus());
  if (!status.session) return null;
  const stop = async () => {
    try {
      await stopFocusSession();
    } catch (e) {
      toast.error(t('focus.session.stop_failed'), e instanceof Error ? e.message : String(e));
    }
  };
  return (
    <div className="mt-4 flex flex-wrap items-center gap-3 rounded-control border border-volt/30 bg-volt-soft px-3 py-2 text-sm" data-testid="dashboard-session-line">
      <Shield className="size-4 shrink-0 text-volt" strokeWidth={1.75} aria-hidden />
      <span className="min-w-0 flex-1 truncate font-medium text-ink">{status.session.task}</span>
      <span className="num text-volt" data-testid="dashboard-countdown">
        {fmtCountdown(remaining ?? status.remaining_secs ?? 0)}
      </span>
      <Button size="sm" variant="ghost" onClick={() => void stop()}>
        {t('focus.session.stop')}
      </Button>
    </div>
  );
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
  const t = useT();
  const date = useAppStore((s) => s.date);
  const setDate = useAppStore((s) => s.setDate);
  const data = useAppStore((s) => s.dashboards[date]);
  const loadDashboard = useAppStore((s) => s.loadDashboard);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const latestNudge = useAppStore((s) => s.latestNudge);
  const ubiSpeech = useAppStore((s) => s.ubiSpeech);
  const focusStatus = useAppStore((s) => s.focusStatus);
  const loadFocusStatus = useAppStore((s) => s.loadFocusStatus);
  const sessionMinutes = useAppStore((s) => s.settingsView?.settings.focus.session_minutes ?? 45);
  const navigate = useNavigate();

  useEffect(() => {
    void loadDashboard(date);
  }, [date, dataVersion, loadDashboard]);

  useEffect(() => {
    void loadFocusStatus();
  }, [loadFocusStatus]);

  const title = isToday(date) ? t('common.today') : t('dashboard.title_day');
  const nudge = data ? (data.unseen_nudges[0] ?? latestNudge) : null;
  const session = focusStatus?.session && !focusStatus.session.ended_at ? focusStatus.session : null;
  // "A lot of windows": the bubble becomes a mini form until the user answers it, dismisses it or a session runs.
  const prompt = nudge?.kind === 'focus_prompt' && !nudge.seen && !session && !ubiSpeech ? nudge : null;

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
          <section className="relative overflow-hidden rounded-shell border border-line bg-panel" aria-label={t('dashboard.hero_label')} data-testid="ubi-hero">
            <div className="pointer-events-none absolute inset-0" style={{ background: 'var(--hero-glow)' }} aria-hidden />
            <div className="relative grid grid-cols-1 items-center gap-6 p-6 min-[1000px]:grid-cols-[auto_1fr_auto] min-[1000px]:gap-8 min-[1000px]:px-8">
              <FocusDial score={data.stats.focus_score} mood={data.stats.mood} size={184} className="justify-self-center" />

              <div className="min-w-0">
                <h2 className="display max-w-[26ch] text-[24px] leading-8 text-ink min-[1280px]:text-[26px] min-[1280px]:leading-9">{headline(t, data, date)}</h2>
                <div className="mt-4 flex flex-wrap items-center gap-3">
                  {data.needs_review > 0 ? (
                    <Link
                      to="/review"
                      className="glow-ember group inline-flex h-10 items-center gap-2 rounded-control border border-ember/40 bg-ember-soft px-3.5 text-sm font-medium text-ember transition-colors duration-150 hover:bg-ember/20"
                    >
                      <Sparkles className="size-4" strokeWidth={1.75} aria-hidden />
                      <span className="num">{t('dashboard.review_cta', { count: data.needs_review })}</span>
                      <ChevronRight className="size-4 transition-transform duration-150 group-hover:translate-x-0.5" strokeWidth={1.75} aria-hidden />
                    </Link>
                  ) : (
                    <span className="inline-flex h-10 items-center gap-2 rounded-control border border-line px-3.5 text-sm text-ink-2">
                      <Sparkles className="size-4 text-signal" strokeWidth={1.75} aria-hidden />
                      {t('dashboard.all_sorted')}
                    </span>
                  )}
                  {data.stats.uncategorized_secs > 0 && (
                    <span className="num text-xs text-ink-3">{t('dashboard.uncategorized_time', { duration: fmtDuration(data.stats.uncategorized_secs, { compact: true }) })}</span>
                  )}
                </div>
                {session && focusStatus && <SessionLine status={focusStatus} />}
                <div className="mt-5 grid grid-cols-2 gap-x-6 gap-y-3 min-[720px]:grid-cols-4">
                  <HeroStat label={t('dashboard.stat.productive')} value={fmtDuration(data.stats.productive_secs, { compact: true })} color="var(--signal)" />
                  <HeroStat label={t('dashboard.stat.distractions')} value={fmtDuration(data.stats.distraction_secs, { compact: true })} color={data.stats.distraction_secs > 0 ? 'var(--rose)' : undefined} />
                  <HeroStat label={t('dashboard.stat.longest_focus')} value={fmtDuration(data.stats.longest_focus_secs, { compact: true })} />
                  <HeroStat label={t('dashboard.stat.switches_per_hour')} value={fmtNumber(data.stats.switches_per_hour, 1)} color={data.stats.switches_per_hour > 8 ? 'var(--amber)' : undefined} />
                </div>
              </div>

              <div className="justify-self-center min-[1000px]:justify-self-end">
                {prompt ? <FocusPrompt nudge={prompt} mood={data.stats.mood} size={200} defaultMinutes={sessionMinutes} /> : <Ubi mood={data.stats.mood} size={200} speaking={ubiSpeech ?? nudge?.message ?? t(`ubi.tip.${data.stats.mood}`)} />}
              </div>
            </div>
          </section>

          <Card>
            <CardHeader title={t('dashboard.timeline.title')} subtitle={t('dashboard.timeline.subtitle')} />
            <DayBar blocks={data.timeline} categories={data.categories} date={date} animate onSelect={(b) => navigate(`/timeline?block=${b.id}`)} />
            <ul className="mt-3 flex flex-wrap gap-x-4 gap-y-1" aria-label={t('charts.legend')}>
              {data.categories
                .filter((c) => data.totals.some((tot) => tot.category_id === c.id))
                .map((c) => (
                  <li key={c.id} className="flex items-center gap-1.5 text-[11px] text-ink-2">
                    <span className="size-2 rounded-full" style={{ background: c.color }} />
                    {categoryLabel(c)}
                  </li>
                ))}
              {data.totals.some((tot) => !tot.category_id) && (
                <li className="flex items-center gap-1.5 text-[11px] text-ink-2">
                  <span className="size-2 rounded-full bg-ink-4/60" />
                  {t('common.uncategorized')}
                </li>
              )}
            </ul>
          </Card>

          <div className="grid grid-cols-12 gap-5">
            <Card className="col-span-12 min-[1100px]:col-span-5">
              <CardHeader title={t('dashboard.categories.title')} subtitle={t('dashboard.categories.subtitle')} />
              <CategoryDonut totals={data.totals} categories={data.categories} height={150} />
            </Card>
            <Card className="col-span-12 min-[1100px]:col-span-4">
              <CardHeader title={t('dashboard.hourly.title')} subtitle={t('dashboard.hourly.subtitle')} />
              <HourlyFocus hourly={data.hourly_focus} height={170} animate />
            </Card>
            <Card className="col-span-12 min-[1100px]:col-span-3">
              <CardHeader title={t('dashboard.top_apps.title')} />
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
                {!data.top_apps.length && <li className="text-xs text-ink-3">{t('dashboard.top_apps.empty')}</li>}
              </ul>
            </Card>
          </div>

          <UbiCard data={data} nudge={nudge} />
        </div>
      )}
    </div>
  );
}
