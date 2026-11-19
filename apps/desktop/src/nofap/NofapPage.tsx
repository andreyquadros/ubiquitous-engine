import { Wind } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { Dialog } from '../components/ui/Dialog';
import { Skeleton, Tabs } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { useT } from '../i18n';
import { ipcErrorMessage } from '../lib/ipc';
import { useToast } from '../lib/toast';
import { IssueList, type IssueAction } from './components/IssueList';
import { LapseFlow } from './components/LapseFlow';
import { PanicSheet } from './components/PanicSheet';
import { PendingBanner } from './components/PendingBanner';
import { SiteLists } from './components/SiteLists';
import { SurfTimer } from './components/SurfTimer';
import { SystemPreview } from './components/SystemPreview';
import { TierRow } from './components/TierRow';
import { consentComplete, VisionConsent } from './components/VisionConsent';
import { nofapIpc } from './ipc';
import { fmtDelay, LAYERS, layersOn, providerName, reasonText } from './labels';
import { NofapWizard, type NofapHost } from './NofapWizard';
import { useNofapEvents, useNofapStore } from './store';
import { HelpTab } from './tabs/HelpTab';
import { ProtectionTab } from './tabs/ProtectionTab';
import { SupportTab } from './tabs/SupportTab';
import type { ApplyOutcome, NofapChange, NofapView, Overall, PlannedChange, TierState, UserEvent } from './types';

export type NofapTab = 'protection' | 'support' | 'lists' | 'help';
const TABS: NofapTab[] = ['protection', 'support', 'lists', 'help'];
const isTab = (v: string | null): v is NofapTab => !!v && (TABS as string[]).includes(v);

/** The dialogs of the page. The three `info:*` ones only explain an exit that lives outside the module. */
export type NofapDialog = 'panic' | 'surf' | 'lapse' | 'apply' | 'remove' | 'restore_network' | 'erase' | 'reset' | 'off' | 'consent' | 'info:remove' | 'info:quit' | 'info:ai';

const OVERALL_COLOR: Record<Overall, string> = { off: 'var(--ink-4)', protected: 'var(--signal)', reduced: 'var(--amber)', pending_off: 'var(--violet)', secondary: 'var(--volt)' };

const needsApply = (tier: TierState): boolean => tier.state === 'not_applied' || tier.state === 'update_pending' || tier.state === 'changed' || tier.state === 'missing';
const drifted = (tier: TierState): boolean => tier.state === 'changed' || tier.state === 'missing';
const leftOnSystem = (tier: TierState): boolean => tier.state === 'applied' || tier.state === 'update_pending' || tier.state === 'changed';

/** The status header: the overall state with the layers that are on, one row per layer, and what needs attention. */
function StatusCard({ view, onIssue, onOpen }: { view: NofapView; onIssue: (action: IssueAction) => void; onOpen: (dialog: NofapDialog) => void }) {
  const t = useT();
  const status = view.status;
  const on = layersOn(status).map((layer) => t(`nofap.layer.${layer}`));
  const tiers = [status.hosts, status.resolver, status.policies];
  const sentence =
    status.overall === 'secondary'
      ? t('nofap.secondary.note')
      : status.overall === 'pending_off'
        ? t('nofap.overall.pending_hint')
        : `${status.overall === 'reduced' ? `${t('nofap.overall.reduced_hint')} ` : ''}${on.length ? t('nofap.overall.layers_on', { layers: on.join(', ') }) : t('nofap.overall.layers_none')}`;
  return (
    <Card data-testid="nofap-status">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h2 className="display flex items-center gap-2.5 text-lg" data-testid="nofap-overall">
            <span className="size-2.5 shrink-0 rounded-full" style={{ background: OVERALL_COLOR[status.overall] }} aria-hidden />
            {t(`nofap.overall.${status.overall}`)}
          </h2>
          <p className="mt-1 text-[13px] leading-5 text-ink-2">{sentence}</p>
        </div>
        <p className="num text-xs text-ink-3">{t('nofap.page.today', { count: view.today.blocks })}</p>
      </div>
      <ul className="mt-3 flex flex-col divide-y divide-line border-t border-line">
        {LAYERS.map((layer) => (
          <TierRow key={layer} layer={layer} status={status} capabilities={view.capabilities} />
        ))}
      </ul>
      {(tiers.some(needsApply) || view.config.system.network_restored) && (
        <div className="mt-1 flex flex-wrap items-center gap-3 border-t border-line pt-4">
          {view.config.system.network_restored && <p className="min-w-0 flex-1 basis-56 text-xs leading-5 text-ink-3">{t('nofap.network.restored_note')}</p>}
          <Button variant="primary" size="sm" onClick={() => onOpen('apply')} data-testid="nofap-apply">
            {t(view.config.system.network_restored ? 'nofap.btn.reapply' : tiers.some(drifted) ? 'nofap.btn.restore' : 'nofap.btn.apply')}
          </Button>
        </div>
      )}
      {status.issues.length > 0 && (
        <div className="mt-4">
          <IssueList issues={status.issues} onAction={onIssue} />
        </div>
      )}
    </Card>
  );
}

/** The exits that never wait (design 6.3), on every tab: each is at most two clicks from here. */
function ExitsCard({ name, onOpen }: { name: string; onOpen: (dialog: NofapDialog) => void }) {
  const t = useT();
  const exits: [NofapDialog, string][] = [
    ['restore_network', t('nofap.btn.restore_network')],
    ['erase', t('nofap.data.erase', { name })],
    ['info:remove', t('nofap.exits.remove')],
    ['info:quit', t('nofap.exits.quit')],
    ['info:ai', t('nofap.exits.ai')],
  ];
  return (
    <Card data-testid="nofap-exits">
      <h2 className="text-sm font-semibold text-ink">{t('nofap.exits.title')}</h2>
      <p className="mt-0.5 text-xs text-ink-3">{t('nofap.exits.subtitle')}</p>
      <ul className="mt-3 flex flex-col gap-1.5">
        {exits.map(([dialog, label]) => (
          <li key={dialog}>
            <Button className="w-full justify-start whitespace-normal text-left" size="sm" onClick={() => onOpen(dialog)} data-testid={`exit-${dialog}`}>
              {label}
            </Button>
          </li>
        ))}
      </ul>
    </Card>
  );
}

/**
 * The module's page: the consent wizard while it is off; once on, the status header with one row per layer, the
 * requests that are waiting, and the tabs "Proteção", "Apoio", "Listas" and "Ajuda".
 */
export function NofapPage({ host }: { host?: NofapHost }) {
  const t = useT();
  const toast = useToast();
  const view = useNofapStore((s) => s.view);
  const error = useNofapStore((s) => s.error);
  const sites = useNofapStore((s) => s.sites);
  const support = useNofapStore((s) => s.support);
  const load = useNofapStore((s) => s.load);
  const loadSites = useNofapStore((s) => s.loadSites);
  const loadSupport = useNofapStore((s) => s.loadSupport);
  const setView = useNofapStore((s) => s.setView);
  const request = useNofapStore((s) => s.request);
  const confirmPending = useNofapStore((s) => s.confirmPending);
  const cancelPending = useNofapStore((s) => s.cancelPending);
  const [params, setParams] = useSearchParams();
  const tab: NofapTab = isTab(params.get('tab')) ? (params.get('tab') as NofapTab) : 'protection';
  const [dialog, setDialog] = useState<NofapDialog | null>(null);
  const [preview, setPreview] = useState<PlannedChange[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [consent, setConsent] = useState(false);
  const [managed, setManaged] = useState(false);

  useNofapEvents();

  useEffect(() => {
    void load();
    void loadSites();
    void loadSupport();
  }, [load, loadSites, loadSupport]);

  const setTab = (next: NofapTab) => setParams(next === 'protection' ? {} : { tab: next }, { replace: true });
  const fail = useCallback((e: unknown) => toast.error(t('nofap.error.title'), ipcErrorMessage(e)), [toast, t]);
  const record = useCallback((event: UserEvent) => void nofapIpc.record(event).then(() => loadSupport()).catch(() => undefined), [loadSupport]);

  const open = (next: NofapDialog) => {
    setDialog(next);
    if (next === 'apply' || next === 'remove') {
      setPreview(null);
      void nofapIpc.systemPreview().then(setPreview).catch(() => setPreview([]));
    }
    if (next === 'consent') {
      setConsent(false);
      setManaged(false);
    }
  };
  const closeDialog = () => setDialog(null);

  /** Sends a change and says what became of it: applied now, or noted and waiting. */
  const send = useCallback(
    async (change: NofapChange) => {
      try {
        const outcome = await request(change);
        if ('pending' in outcome) {
          const delay = useNofapStore.getState().view?.config.delay_secs ?? 0;
          toast.info(t('nofap.pending.title'), t('nofap.pending.wait', { delay: fmtDelay(delay, t) }));
        }
        if (change.kind === 'set_values' || change.kind === 'set_plans') await loadSupport();
        return outcome;
      } catch (e) {
        fail(e);
        return null;
      }
    },
    [request, toast, t, fail, loadSupport],
  );

  const reportOutcome = (outcome: ApplyOutcome, done: string) => {
    switch (outcome.outcome) {
      case 'applied':
        toast.success(done);
        break;
      case 'partial':
        toast.info(
          t('nofap.apply.partial'),
          outcome.detail
            .filter((r) => !r.ok)
            .map((r) => t('nofap.apply.partial_tier', { layer: t(`nofap.layer.${r.tier}`), reason: reasonText(r.reason, t) }))
            .join(' '),
        );
        break;
      case 'declined':
        toast.info(t('nofap.apply.declined'));
        break;
      case 'unavailable':
        toast.error(t('nofap.error.title'), t('nofap.apply.unavailable', { reason: reasonText(outcome.detail, t) }));
        break;
    }
  };

  /** Runs one of the commands that raise the OS prompt, then reads the state again. */
  const system = async (run: () => Promise<ApplyOutcome>, done: string) => {
    setBusy(true);
    try {
      reportOutcome(await run(), done);
      await load();
      closeDialog();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const erase = async (run: () => Promise<null>, done: string) => {
    setBusy(true);
    try {
      await run();
      toast.success(done);
      await Promise.all([load(), loadSites(), loadSupport()]);
      closeDialog();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const requestOff = async () => {
    const outcome = await send({ kind: 'disable' });
    closeDialog();
    // With no waiting time the dialog itself was the fresh confirmation.
    if (outcome && 'pending' in outcome && outcome.pending.ready) await confirmPending(outcome.pending.id).catch(fail);
  };

  const confirmConsent = async () => {
    if (!view) return;
    setBusy(true);
    try {
      setView(await nofapIpc.consentVision({ provider: view.provider, managed: view.provider === 'ubi' && managed }));
      if (!view.config.vision.enabled) await send({ kind: 'set_vision', on: true });
      closeDialog();
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  };

  const onIssue = (action: IssueAction) => open(action === 'restore_network' ? 'restore_network' : action === 'apply' ? 'apply' : 'consent');

  if (!view) {
    return (
      <div data-testid="page-nofap">
        {error ? (
          <div className="panel mx-auto mt-10 max-w-md p-6 text-center" role="alert">
            <p className="text-sm font-medium text-ink">{t('nofap.page.load_failed')}</p>
            <p className="mt-1 text-xs text-ink-3">{error}</p>
            <div className="mt-4">
              <Button onClick={() => void load()}>{t('nofap.page.retry')}</Button>
            </div>
          </div>
        ) : (
          <div className="flex flex-col gap-5">
            <Skeleton className="h-16" />
            <Skeleton className="h-72" />
          </div>
        )}
      </div>
    );
  }

  const name = view.display_name;
  const status = view.status;
  const cancelButton = (
    <Button variant="ghost" onClick={closeDialog} disabled={busy}>
      {t('nofap.btn.cancel')}
    </Button>
  );
  const data = { onExport: () => void nofapIpc.export().then((r) => toast.success(t('nofap.data.exported', { path: r.path }))).catch(fail), onTestCard: () => void nofapIpc.testCard().catch(fail) };

  const dialogs = (
    <>
      <Dialog open={dialog === 'panic'} onClose={closeDialog} title={t('nofap.btn.panic')} width="sm">
        {dialog === 'panic' && <PanicSheet values={view.config.values} plans={view.config.plans} resources={support?.resources ?? []} record={record} onClose={closeDialog} />}
      </Dialog>
      <Dialog open={dialog === 'surf'} onClose={closeDialog} title={t('nofap.surf.title')} width="sm">
        {dialog === 'surf' && <SurfTimer record={record} onDone={closeDialog} />}
      </Dialog>
      <Dialog open={dialog === 'lapse'} onClose={closeDialog} title={t('nofap.lapse.button')} width="sm">
        {dialog === 'lapse' && (
          <LapseFlow
            onRecord={(triggers) => record({ event: 'lapse', triggers })}
            onAdjustPlan={() => {
              closeDialog();
              setTab('support');
            }}
            onClose={closeDialog}
          />
        )}
      </Dialog>
      <Dialog
        open={dialog === 'apply' || dialog === 'remove'}
        onClose={closeDialog}
        title={t('nofap.system.preview_title')}
        footer={
          <>
            {cancelButton}
            <Button
              variant="primary"
              loading={busy}
              disabled={!preview?.length}
              onClick={() => void (dialog === 'remove' ? system(nofapIpc.systemRemove, t('nofap.apply.removed')) : system(nofapIpc.systemApply, t('nofap.apply.applied')))}
              data-testid="nofap-apply-confirm"
            >
              {t(dialog === 'remove' ? 'nofap.btn.remove_system' : 'nofap.btn.apply')}
            </Button>
          </>
        }
      >
        <SystemPreview lines={preview} />
      </Dialog>
      <Dialog
        open={dialog === 'restore_network'}
        onClose={closeDialog}
        title={t('nofap.btn.restore_network')}
        width="sm"
        footer={
          <>
            {cancelButton}
            <Button variant="primary" loading={busy} onClick={() => void system(nofapIpc.restoreNetwork, t('nofap.network.restored'))} data-testid="nofap-restore-network-confirm">
              {t('nofap.btn.restore_network')}
            </Button>
          </>
        }
      >
        <p className="text-sm leading-6 text-ink-2">{t('nofap.network.restore_body')}</p>
        <p className="mt-2 text-xs text-ink-3">{t('nofap.system.prompt_next')}</p>
      </Dialog>
      <Dialog
        open={dialog === 'erase' || dialog === 'reset'}
        onClose={closeDialog}
        title={t(dialog === 'reset' ? 'nofap.data.reset' : 'nofap.data.erase', { name })}
        width="sm"
        footer={
          <>
            {cancelButton}
            <Button variant="danger" loading={busy} onClick={() => void (dialog === 'reset' ? erase(nofapIpc.reset, t('nofap.data.reset_done')) : erase(nofapIpc.eraseHistory, t('nofap.data.erased')))} data-testid="nofap-erase-confirm">
              {t(dialog === 'reset' ? 'nofap.data.reset' : 'nofap.data.erase', { name })}
            </Button>
          </>
        }
      >
        <p className="text-sm leading-6 text-ink-2">{t(dialog === 'reset' ? 'nofap.data.reset_body' : 'nofap.data.erase_body', { name })}</p>
      </Dialog>
      <Dialog
        open={dialog === 'off'}
        onClose={closeDialog}
        title={t('nofap.off.dialog_title', { name })}
        width="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => void requestOff()} data-testid="nofap-off-request">
              {t(view.config.delay_secs > 0 ? 'nofap.off.request' : 'nofap.btn.turn_off')}
            </Button>
            <Button variant="primary" onClick={closeDialog}>
              {t('nofap.btn.keep_protection')}
            </Button>
          </>
        }
      >
        <p className="text-sm leading-6 text-ink-2">{view.config.delay_secs > 0 ? t('nofap.off.dialog_body', { delay: fmtDelay(view.config.delay_secs, t) }) : t('nofap.off.dialog_body_now')}</p>
        {view.config.values && <p className="panel-raised mt-3 p-3 text-[13px] leading-5 text-ink-2">{t('nofap.panic.reason', { reason: view.config.values })}</p>}
      </Dialog>
      <Dialog
        open={dialog === 'consent'}
        onClose={closeDialog}
        title={t('nofap.vision.title')}
        footer={
          <>
            {cancelButton}
            <Button variant="primary" loading={busy} disabled={!consentComplete(view.provider, consent, managed)} onClick={() => void confirmConsent()} data-testid="nofap-consent-confirm">
              {t('nofap.vision.confirm_provider', { provider: providerName(view.provider, t) })}
            </Button>
          </>
        }
      >
        <VisionConsent provider={view.provider} name={name} consent={consent} onConsent={setConsent} managed={managed} onManaged={setManaged} />
      </Dialog>
      {(['remove', 'quit', 'ai'] as const).map((topic) => (
        <Dialog
          key={topic}
          open={dialog === `info:${topic}`}
          onClose={closeDialog}
          title={t(`nofap.helptab.${topic}.title`)}
          width="sm"
          footer={
            <>
              {topic === 'ai' && host?.openSettings && (
                <Button onClick={host.openSettings} data-testid="nofap-open-settings">
                  {t('nofap.btn.open_settings')}
                </Button>
              )}
              <Button variant="primary" onClick={closeDialog}>
                {t('nofap.btn.close')}
              </Button>
            </>
          }
        >
          <p className="text-sm leading-6 text-ink-2" data-testid={`info-${topic}`}>
            {t(`nofap.helptab.${topic}.body`)}
          </p>
        </Dialog>
      ))}
    </>
  );

  if (!view.config.enabled) {
    const left = [status.hosts, status.resolver, status.policies].some(leftOnSystem);
    return (
      <div data-testid="page-nofap">
        <PageHeader title={name} subtitle={t('nofap.page.subtitle')} />
        <div className="mx-auto flex w-full max-w-[720px] flex-col gap-5">
          {view.config.self_use_at && (
            <Card data-testid="nofap-off">
              <h2 className="display text-lg">{t('nofap.off.title', { name })}</h2>
              <p className="mt-1 text-[13px] leading-5 text-ink-2">{left ? t('nofap.off.system_left') : t('nofap.off.removed_hint', { name })}</p>
              <div className="mt-3 flex flex-wrap gap-2">
                {left && (
                  <Button variant="primary" size="sm" onClick={() => open('remove')} data-testid="nofap-remove-system">
                    {t('nofap.btn.remove_system')}
                  </Button>
                )}
                <Button size="sm" onClick={data.onExport}>
                  {t('nofap.data.export')}
                </Button>
                <Button size="sm" onClick={() => open('erase')}>
                  {t('nofap.data.erase', { name })}
                </Button>
                <Button size="sm" onClick={() => open('reset')}>
                  {t('nofap.data.reset')}
                </Button>
              </div>
            </Card>
          )}
          <NofapWizard
            view={view}
            host={host}
            onEnabled={(next) => {
              setView(next);
              void loadSupport();
            }}
          />
        </div>
        {dialogs}
      </div>
    );
  }

  return (
    <div data-testid="page-nofap">
      <PageHeader
        title={name}
        subtitle={t('nofap.page.subtitle')}
        actions={
          <Button icon={<Wind className="size-4" strokeWidth={1.75} aria-hidden />} onClick={() => open('panic')} data-testid="nofap-panic">
            {t('nofap.btn.panic')}
          </Button>
        }
      />
      <div className="grid grid-cols-12 gap-5">
        <div className="col-span-12 flex min-w-0 flex-col gap-5 min-[1100px]:col-span-8">
          <PendingBanner
            view={view}
            sites={sites}
            onCancel={(id) =>
              void cancelPending(id)
                .then(() => toast.info(t('nofap.pending.cancelled')))
                .catch(fail)
            }
            onConfirm={(id) => void confirmPending(id).catch(fail)}
            onSurf={() => open('surf')}
            onElapsed={() => void load()}
          />
          <StatusCard view={view} onIssue={onIssue} onOpen={open} />
          <Tabs<NofapTab> value={tab} onChange={setTab} items={TABS.map((value) => ({ value, label: t(`nofap.tab.${value}`) }))} className="self-start" />
          {tab === 'protection' && <ProtectionTab view={view} host={host} onRequest={(change) => void send(change)} onOpen={open} />}
          {tab === 'support' && <SupportTab view={view} support={support} record={record} onRequest={async (change) => void (await send(change))} onOpen={open} />}
          {tab === 'lists' && (sites ? <SiteLists view={view} sites={sites} onRequest={async (change) => void (await send(change))} /> : <Skeleton className="h-64" />)}
          {tab === 'help' && <HelpTab view={view} resources={support?.resources ?? []} host={host} onOpen={open} onExport={data.onExport} onTestCard={data.onTestCard} />}
        </div>
        <aside className="col-span-12 flex flex-col gap-5 min-[1100px]:col-span-4">
          <ExitsCard name={name} onOpen={open} />
        </aside>
      </div>
      {dialogs}
    </div>
  );
}
