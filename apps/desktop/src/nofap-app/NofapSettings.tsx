// The standalone product's settings: language and launch at login, the AI provider, key and model, the system
// permissions the AI check needs, updates, the user's data and the about box with the third-party notices.

import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '../components/ui/Button';
import { Card, CardHeader } from '../components/ui/Card';
import { Field, Select } from '../components/ui/Field';
import { LanguageSelect } from '../components/ui/LanguageSelect';
import { PageHeader } from '../components/ui/PageHeader';
import { Toggle } from '../components/ui/Toggle';
import { useLocale, useT, type Locale } from '../i18n';
import { ipcErrorMessage } from '../lib/ipc';
import { useToast } from '../lib/toast';
import type { PermissionKind, PermissionState } from '../lib/types';
import { useNofapStore } from '../nofap/store';
// The notices the bundle must carry (the blocklist's sources and their licences), shown as they are in the repository.
import notices from '../../../../THIRD-PARTY-NOTICES?raw';
import { useAutostart } from './autostart';
import { hostIpc } from './hostIpc';
import { useHostStore } from './hostStore';
import { ProviderKey } from './ProviderKey';
import type { HostAbout, PermissionStatus, UpdateStatus } from './types';

const PERMISSIONS: PermissionKind[] = ['screen_recording', 'automation'];

function GeneralCard() {
  const t = useT();
  const locale = useLocale();
  const save = useHostStore((s) => s.save);
  const [autostart, setAutostart] = useAutostart();
  const toast = useToast();
  const fail = (e: unknown) => toast.error(t('nofap.error.title'), ipcErrorMessage(e));
  return (
    <Card data-testid="settings-general">
      <CardHeader title={t('nofap_app.settings.general')} />
      <div className="flex flex-col gap-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <span className="text-[13px] font-medium text-ink">{t('nofap_app.settings.language')}</span>
          <LanguageSelect value={locale} onChange={(next: Locale) => void save({ ui_language: next }).catch(fail)} compact />
        </div>
        <Field label={t('nofap.system.autostart')} hint={t('nofap.system.autostart_hint')} inline>
          {(id) => <Toggle id={id} checked={autostart ?? false} disabled={autostart === undefined} onChange={(on) => void setAutostart(on).catch(fail)} />}
        </Field>
      </div>
    </Card>
  );
}

/** The model of the screen check: the vendor's list for the stored key, or the current one when it cannot be read. */
function ModelPicker() {
  const t = useT();
  const toast = useToast();
  const settings = useHostStore((s) => s.settings);
  const save = useHostStore((s) => s.save);
  const ready = useNofapStore((s) => s.view?.ai === 'ready');
  const provider = settings?.ai_provider;
  const [models, setModels] = useState<string[] | null>(null);
  useEffect(() => {
    setModels(null);
    if (!provider || !ready) return;
    let live = true;
    void hostIpc
      .models(provider)
      .then((list) => live && setModels(list))
      .catch(() => live && setModels([]));
    return () => {
      live = false;
    };
  }, [provider, ready]);
  if (!settings) return null;
  const current = settings.vision_model;
  const options = models?.length ? (models.includes(current) ? models : [current, ...models]) : null;
  return (
    <Field label={t('nofap_app.ai.model')} hint={t('nofap_app.ai.model_hint')}>
      {(id) =>
        options ? (
          <Select id={id} value={current} onChange={(e) => void save({ vision_model: e.target.value }).catch((err) => toast.error(t('nofap.error.title'), ipcErrorMessage(err)))} data-testid="model-select">
            {options.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </Select>
        ) : (
          <p id={id} className="num text-sm text-ink-2">
            {current}
          </p>
        )
      }
    </Field>
  );
}

function PermissionsCard() {
  const t = useT();
  const toast = useToast();
  const [status, setStatus] = useState<PermissionStatus | null>(null);
  const load = () => void hostIpc.permissions().then(setStatus).catch(() => undefined);
  useEffect(load, []);
  const shown = PERMISSIONS.filter((kind) => status && status[kind] !== 'not_applicable');
  if (!status || !shown.length) return null;
  const label = (state: PermissionState) => t(`nofap_app.permission.state_${state}`);
  return (
    <Card data-testid="settings-permissions">
      <CardHeader title={t('nofap_app.settings.permissions')} subtitle={t('nofap_app.settings.permissions_hint')} />
      <ul className="flex flex-col divide-y divide-line">
        {shown.map((kind) => (
          <li key={kind} className="flex flex-wrap items-center justify-between gap-3 py-2.5 first:pt-0 last:pb-0">
            <div className="min-w-0">
              <p className="text-[13px] font-medium text-ink">{t(`nofap_app.permission.${kind}`)}</p>
              <p className="text-xs text-ink-3">{label(status[kind])}</p>
            </div>
            {status[kind] !== 'granted' && (
              <Button size="sm" onClick={() => void hostIpc.requestPermission(kind).then(load).catch((e) => toast.error(t('nofap.error.title'), ipcErrorMessage(e)))}>
                {t('nofap_app.permission.request')}
              </Button>
            )}
          </li>
        ))}
      </ul>
    </Card>
  );
}

function UpdatesCard({ about }: { about: HostAbout | null }) {
  const t = useT();
  const toast = useToast();
  const settings = useHostStore((s) => s.settings);
  const save = useHostStore((s) => s.save);
  const [status, setStatus] = useState<UpdateStatus | null>(null);
  const [checking, setChecking] = useState(false);
  const [progress, setProgress] = useState<number | null | undefined>(undefined);

  const check = async () => {
    setChecking(true);
    try {
      setStatus(await hostIpc.checkUpdates());
    } catch (e) {
      toast.error(t('nofap.error.title'), ipcErrorMessage(e));
    } finally {
      setChecking(false);
    }
  };
  // A newer build found by the background check shows here at once.
  useEffect(() => {
    let off: (() => void) | null = null;
    let live = true;
    void hostIpc
      .onUpdate(() => void hostIpc.checkUpdates().then((next) => live && setStatus(next)).catch(() => undefined))
      .then((unsubscribe) => {
        if (live) off = unsubscribe;
        else unsubscribe();
      });
    return () => {
      live = false;
      off?.();
    };
  }, []);

  const install = async () => {
    setProgress(null);
    try {
      if (!(await hostIpc.installUpdate(setProgress)) && status?.available) await hostIpc.openUrl(status.available.download_url);
    } catch (e) {
      toast.error(t('nofap.error.title'), ipcErrorMessage(e));
    } finally {
      setProgress(undefined);
    }
  };

  const available = status?.available ?? null;
  let line: string | null = null;
  if (status?.last_error) line = t('nofap_app.update.failed', { error: status.last_error });
  else if (available) line = t('nofap_app.update.available', { version: available.version });
  else if (status?.last_check) line = t('nofap_app.update.none');

  return (
    <Card data-testid="settings-updates">
      <CardHeader title={t('nofap_app.settings.updates')} />
      <div className="flex flex-col gap-4">
        <Field label={t('nofap_app.update.automatic')} hint={t('nofap_app.update.automatic_hint')} inline>
          {(id) => <Toggle id={id} checked={settings?.check_updates ?? true} onChange={(on) => void save({ check_updates: on }).catch((e) => toast.error(t('nofap.error.title'), ipcErrorMessage(e)))} />}
        </Field>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="min-w-0 text-xs leading-5 text-ink-3">
            {about && <p className="num">{t('nofap_app.update.current', { version: about.build.version, build: about.build.sha })}</p>}
            {about && about.build.number === 0 && <p>{t('nofap_app.update.dev')}</p>}
            {line && (
              <p className="text-ink-2" role="status" data-testid="update-line">
                {line}
              </p>
            )}
          </div>
          <div className="flex flex-wrap gap-2">
            {available && (
              <Button variant="primary" size="sm" loading={progress !== undefined} onClick={() => void install()}>
                {progress !== undefined && progress !== null ? `${Math.round(progress * 100)}%` : t('nofap_app.update.install')}
              </Button>
            )}
            <Button size="sm" loading={checking} onClick={() => void check()} data-testid="update-check">
              {t('nofap_app.update.check')}
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
}

function AboutCard({ about }: { about: HostAbout | null }) {
  const t = useT();
  const [showNotices, setShowNotices] = useState(false);
  return (
    <Card data-testid="settings-about">
      <CardHeader title={t('nofap_app.settings.about')} />
      {about && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-[13px]">
          <dt className="text-ink-3">{t('nofap_app.about.name')}</dt>
          <dd className="text-ink">{about.display_name}</dd>
          <dt className="text-ink-3">{t('nofap_app.about.version')}</dt>
          <dd className="num text-ink">
            {about.build.version} ({about.build.sha})
          </dd>
          <dt className="text-ink-3">{t('nofap_app.about.identifier')}</dt>
          <dd className="num truncate text-ink">{about.identifier}</dd>
        </dl>
      )}
      <p className="mt-3 text-xs leading-5 text-ink-3">{t('nofap_app.about.no_tracker')}</p>
      <div className="mt-4 border-t border-line pt-4">
        <div className="flex items-center justify-between gap-3">
          <h3 className="text-[13px] font-medium text-ink">{t('nofap_app.about.notices')}</h3>
          <Button size="sm" variant="ghost" onClick={() => setShowNotices((v) => !v)} aria-expanded={showNotices} data-testid="notices-toggle">
            {t(showNotices ? 'nofap_app.about.notices_hide' : 'nofap_app.about.notices_show')}
          </Button>
        </div>
        {showNotices && (
          <pre className="scroll-thin panel-raised mt-3 max-h-72 overflow-auto p-3 text-[11px] leading-4 whitespace-pre-wrap text-ink-2" data-testid="notices">
            {notices}
          </pre>
        )}
      </div>
    </Card>
  );
}

export function NofapSettings() {
  const t = useT();
  const navigate = useNavigate();
  const [about, setAbout] = useState<HostAbout | null>(null);
  useEffect(() => {
    void hostIpc
      .about()
      .then(setAbout)
      .catch(() => undefined);
  }, []);
  return (
    <div data-testid="page-settings">
      <PageHeader title={t('nofap_app.settings.title')} subtitle={t('nofap_app.settings.subtitle')} />
      <div className="grid grid-cols-12 gap-5">
        <div className="col-span-12 flex min-w-0 flex-col gap-5 min-[1100px]:col-span-7">
          <GeneralCard />
          <Card data-testid="settings-ai">
            <CardHeader title={t('nofap_app.settings.ai')} subtitle={t('nofap_app.settings.ai_hint')} />
            <div className="flex flex-col gap-5">
              <ProviderKey />
              <div className="border-t border-line pt-4">
                <ModelPicker />
              </div>
            </div>
          </Card>
          <PermissionsCard />
        </div>
        <div className="col-span-12 flex min-w-0 flex-col gap-5 min-[1100px]:col-span-5">
          <UpdatesCard about={about} />
          <Card data-testid="settings-data">
            <CardHeader title={t('nofap_app.settings.data')} subtitle={t('nofap_app.settings.data_hint')} />
            <Button size="sm" onClick={() => navigate('/help')}>
              {t('nofap_app.settings.data_open')}
            </Button>
          </Card>
          <AboutCard about={about} />
        </div>
      </div>
    </div>
  );
}
