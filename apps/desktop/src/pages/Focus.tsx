import { ShieldOff } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { DisableWarnings, type PendingChange } from '../components/focus/DisableWarnings';
import { FocusOptions } from '../components/focus/FocusOptions';
import { InterventionsList } from '../components/focus/InterventionsList';
import { SessionCard } from '../components/focus/SessionCard';
import { TargetList } from '../components/focus/TargetList';
import { TargetSearch } from '../components/focus/TargetSearch';
import { Badge } from '../components/ui/Badge';
import { Card, CardHeader } from '../components/ui/Card';
import { EmptyState, Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { useT } from '../i18n';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { FocusSettings, FocusTarget, FocusTargetKind } from '../lib/types';

const errorMessage = (e: unknown): string => (e instanceof Error ? e.message : String(e));

/** A session stopped by hand before this many whole minutes gets a plain "ended" toast, no praise (the engine's `PRAISE_MIN_MINUTES`). */
const PRAISE_MIN_MINUTES = 5;

/** Foco: the focus session, the block list with its search, the recent interventions and the guard options. */
export function FocusPage() {
  const t = useT();
  const toast = useToast();
  const settingsView = useAppStore((s) => s.settingsView);
  const saveSettings = useAppStore((s) => s.saveSettings);
  const focusStatus = useAppStore((s) => s.focusStatus);
  const targets = useAppStore((s) => s.focusTargets);
  const interventions = useAppStore((s) => s.interventions);
  const installedApps = useAppStore((s) => s.installedApps);
  const knownDomains = useAppStore((s) => s.knownDomains);
  const loadFocus = useAppStore((s) => s.loadFocus);
  const loadFocusStatus = useAppStore((s) => s.loadFocusStatus);
  const loadFocusCatalog = useAppStore((s) => s.loadFocusCatalog);
  const addFocusTarget = useAppStore((s) => s.addFocusTarget);
  const setFocusTargetEnabled = useAppStore((s) => s.setFocusTargetEnabled);
  const removeFocusTarget = useAppStore((s) => s.removeFocusTarget);
  const startFocusSession = useAppStore((s) => s.startFocusSession);
  const stopFocusSession = useAppStore((s) => s.stopFocusSession);
  const [loaded, setLoaded] = useState(false);
  const [pending, setPending] = useState<PendingChange | null>(null);

  useEffect(() => {
    let alive = true;
    void loadFocus()
      .catch((e) => toast.error(t('common.something_went_wrong'), errorMessage(e)))
      .finally(() => alive && setLoaded(true));
    void loadFocusCatalog();
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loadFocus, loadFocusCatalog]);

  const focus: FocusSettings | null = settingsView?.settings.focus ?? null;

  const patchFocus = useCallback(
    (patch: Partial<FocusSettings>) => {
      if (!focus) return;
      void saveSettings({ focus: { ...focus, ...patch } })
        .then(() => loadFocusStatus())
        .catch((e) => toast.error(t('focus.options.save_failed'), errorMessage(e)));
    },
    [focus, saveSettings, loadFocusStatus, toast, t],
  );

  const add = async (kind: FocusTargetKind, name: string, key: string) => {
    try {
      const target = await addFocusTarget(kind, name, key);
      toast.success(t('focus.targets.added'), t('focus.targets.added_body', { name: target.name }));
    } catch (e) {
      toast.error(t('focus.targets.add_failed'), errorMessage(e));
    }
  };

  const toggle = (target: FocusTarget, enabled: boolean) => {
    if (enabled) {
      void setFocusTargetEnabled(target.id, true).catch((e) => toast.error(t('focus.targets.update_failed'), errorMessage(e)));
      return;
    }
    setPending({ target, action: 'disable' });
  };

  const confirm = async (change: PendingChange) => {
    setPending(null);
    try {
      if (change.action === 'remove') await removeFocusTarget(change.target.id);
      else await setFocusTargetEnabled(change.target.id, false);
    } catch (e) {
      toast.error(t('focus.targets.update_failed'), errorMessage(e));
    }
  };

  const start = async (task: string, minutes: number) => {
    try {
      await startFocusSession(task, minutes);
    } catch (e) {
      toast.error(t('focus.session.start_failed'), errorMessage(e));
      throw e;
    }
  };

  const stop = async () => {
    try {
      const session = await stopFocusSession();
      if (session) {
        // whole minutes, truncated like the engine's `num_minutes`
        const minutes = Math.max(0, Math.floor((new Date(session.ended_at ?? Date.now()).getTime() - new Date(session.started_at).getTime()) / 60_000));
        if (minutes >= PRAISE_MIN_MINUTES) toast.success(t('focus.session.stopped'), t('focus.session.stopped_body', { minutes, task: session.task }));
        else toast.info(t('focus.session.stopped'));
      }
    } catch (e) {
      toast.error(t('focus.session.stop_failed'), errorMessage(e));
    }
  };

  const test = async () => {
    try {
      await ipc.testIntervention();
    } catch (e) {
      toast.error(t('focus.options.test_failed'), errorMessage(e));
    }
  };

  const enabledCount = targets.filter((x) => x.enabled).length;
  const todayCount = focusStatus?.interventions_today ?? 0;

  return (
    <div data-testid="page-focus">
      <PageHeader title={t('focus.title')} subtitle={t('focus.subtitle')} />
      {!loaded || !focus ? (
        <div className="grid grid-cols-12 gap-5">
          <div className="col-span-12 flex flex-col gap-5 min-[1100px]:col-span-8">
            <Skeleton className="h-40" />
            <Skeleton className="h-64" />
          </div>
          <Skeleton className="col-span-12 h-96 min-[1100px]:col-span-4" />
        </div>
      ) : (
        <div className="grid grid-cols-12 gap-5">
          <div className="col-span-12 flex flex-col gap-5 min-[1100px]:col-span-8">
            <SessionCard status={focusStatus} defaultMinutes={focus.session_minutes} onStart={start} onStop={stop} onElapsed={() => void loadFocus()} />

            <Card>
              <CardHeader
                title={t('focus.targets.title')}
                subtitle={t('focus.targets.subtitle')}
                action={enabledCount > 0 ? <Badge tone={focus.guard_enabled ? 'volt' : 'neutral'}>{t('focus.targets.enabled_count', { count: enabledCount })}</Badge> : undefined}
              />
              <TargetSearch installedApps={installedApps} knownDomains={knownDomains} targets={targets} onAdd={add} />
              {!focus.guard_enabled && (
                <p className="mt-3 flex items-center gap-2 rounded-control border border-amber/30 bg-amber/10 px-3 py-2 text-xs text-amber" role="status">
                  <ShieldOff className="size-3.5 shrink-0" strokeWidth={1.75} aria-hidden />
                  {t('focus.targets.guard_off')}
                </p>
              )}
              <div className="mt-3">
                {targets.length ? <TargetList targets={targets} onToggle={toggle} onRemove={(target) => setPending({ target, action: 'remove' })} /> : <EmptyState title={t('focus.targets.empty_title')} description={t('focus.targets.empty_description')} className="py-8" />}
              </div>
            </Card>

            <Card>
              <CardHeader title={t('focus.interventions.title')} subtitle={t('focus.interventions.subtitle')} action={todayCount > 0 ? <Badge tone="ember">{t('focus.interventions.today', { count: todayCount })}</Badge> : undefined} />
              <InterventionsList interventions={interventions} />
            </Card>
          </div>

          <div className="col-span-12 min-[1100px]:col-span-4">
            <FocusOptions focus={focus} onPatch={patchFocus} onTest={test} />
          </div>
        </div>
      )}

      <DisableWarnings pending={pending} onClose={() => setPending(null)} onConfirm={(change) => void confirm(change)} />
    </div>
  );
}
