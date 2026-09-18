import clsx from 'clsx';
import { AlertTriangle, Bell, BrainCircuit, Camera, Check, Download, ExternalLink, EyeOff, Info, KeyRound, ListRestart, Loader2, RefreshCw, Shield, ShieldCheck, SlidersHorizontal, Trash2, type LucideIcon } from 'lucide-react';
import { Fragment, useCallback, useEffect, useId, useMemo, useRef, useState } from 'react';
import { Badge, StatusPill } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { Dialog } from '../components/ui/Dialog';
import { Field, Input, Textarea } from '../components/ui/Field';
import { LanguageSelect } from '../components/ui/LanguageSelect';
import { EmptyState } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { TagInput } from '../components/ui/TagInput';
import { Toggle } from '../components/ui/Toggle';
import { useLocale, useT, type Locale } from '../i18n';
import { fmtDateNumeric, fmtDateTime, fmtTime, hhmmToInput, inputToHhmm } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import { keyStatus, providerInfo, providerPitch, reconcileModels, sameModels } from '../lib/providers';
import type { AiModels, AiProvider, PermissionKind, PermissionState, PrivateModeDuration, Settings, SettingsView, VisionPolicy } from '../lib/types';
import { useAsync } from '../lib/useAsync';

export const REPO_URL = 'https://github.com/andreyquadros/ubiquitous-engine';

/** Section ids are stable (anchors, tests); labels come from t('settings.section.<key>'). */
const SECTIONS: { id: string; key: string; Icon: LucideIcon }[] = [
  { id: 'geral', key: 'general', Icon: SlidersHorizontal },
  { id: 'ia', key: 'ai', Icon: BrainCircuit },
  { id: 'rastreamento', key: 'tracking', Icon: RefreshCw },
  { id: 'privacidade', key: 'privacy', Icon: Shield },
  { id: 'relatorios', key: 'reports', Icon: Info },
  { id: 'ubi', key: 'ubi', Icon: Bell },
  { id: 'permissoes', key: 'permissions', Icon: ShieldCheck },
  { id: 'sobre', key: 'about', Icon: Info },
];

const errorMessage = (e: unknown): string => (e instanceof Error ? e.message : String(e));

/** Local draft of the settings with debounced persistence. */
function useSettingsDraft() {
  const settingsView = useAppStore((s) => s.settingsView);
  const saveSettings = useAppStore((s) => s.saveSettings);
  const [draft, setDraft] = useState<Settings | null>(settingsView?.settings ?? null);
  const [status, setStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pending = useRef<Partial<Settings>>({});
  const toast = useToast();
  const t = useT();

  useEffect(() => {
    if (settingsView && !draft) setDraft(settingsView.settings);
  }, [settingsView, draft]);

  const patch = useCallback(
    (p: Partial<Settings>) => {
      setDraft((d) => (d ? { ...d, ...p } : d));
      pending.current = { ...pending.current, ...p };
      setStatus('saving');
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(async () => {
        const toSave = pending.current;
        pending.current = {};
        try {
          await saveSettings(toSave);
          setStatus('saved');
          setTimeout(() => setStatus((s) => (s === 'saved' ? 'idle' : s)), 1500);
        } catch (e) {
          setStatus('error');
          toast.error(t('settings.toast.save_failed'), errorMessage(e));
        }
      }, 500);
    },
    [saveSettings, toast, t],
  );

  return { draft, patch, status, settingsView };
}

/** Highlights the section nearest the top of the scroll area (no-op where IntersectionObserver is missing, e.g. jsdom). */
function useScrollSpy(ids: string[], setActive: (id: string) => void) {
  useEffect(() => {
    if (typeof IntersectionObserver === 'undefined') return;
    const visible = new Map<string, number>();
    const obs = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          const id = e.target.id.replace(/^sec-/, '');
          if (e.isIntersecting) visible.set(id, e.boundingClientRect.top);
          else visible.delete(id);
        }
        const top = [...visible.entries()].sort((a, b) => a[1] - b[1])[0];
        if (top) setActive(top[0]);
      },
      { rootMargin: '-10% 0px -60% 0px', threshold: 0 },
    );
    for (const id of ids) {
      const el = document.getElementById(`sec-${id}`);
      if (el) obs.observe(el);
    }
    return () => obs.disconnect();
  }, [ids, setActive]);
}

export function SettingsPage() {
  const t = useT();
  const { draft, patch, status, settingsView } = useSettingsDraft();
  const [active, setActive] = useState('geral');
  const sections = useMemo(() => SECTIONS.filter((s) => s.id !== 'permissoes' || settingsView?.platform === 'macos'), [settingsView?.platform]);
  const ids = useMemo(() => sections.map((s) => s.id), [sections]);
  useScrollSpy(ids, setActive);

  if (!draft || !settingsView) return null;

  const go = (id: string) => {
    setActive(id);
    document.getElementById(`sec-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };

  return (
    <div data-testid="page-settings">
      <PageHeader
        title={t('settings.title')}
        subtitle={t('settings.subtitle')}
        actions={
          <span className="flex h-9 items-center gap-1.5 text-xs text-ink-3" aria-live="polite">
            {status === 'saving' && (
              <>
                <Loader2 className="size-3.5 animate-spin" strokeWidth={1.75} aria-hidden /> {t('settings.saving')}
              </>
            )}
            {status === 'saved' && (
              <>
                <Check className="size-3.5 text-signal" strokeWidth={2} aria-hidden /> {t('common.saved')}
              </>
            )}
            {status === 'error' && (
              <>
                <AlertTriangle className="size-3.5 text-rose" strokeWidth={1.75} aria-hidden /> {t('settings.not_saved')}
              </>
            )}
          </span>
        }
      />
      <div className="grid grid-cols-12 gap-6">
        <nav className="col-span-12 min-[1100px]:sticky min-[1100px]:top-0 min-[1100px]:col-span-3 min-[1100px]:self-start" aria-label={t('settings.sections_nav')}>
          <ul className="flex flex-row flex-wrap gap-0.5 min-[1100px]:flex-col">
            {sections.map(({ id, key, Icon }) => {
              const on = active === id;
              return (
                <li key={id} className="relative">
                  {on && <span className="absolute top-2 bottom-2 -left-3 hidden w-[3px] rounded-r-full bg-volt min-[1100px]:block" aria-hidden />}
                  <button
                    type="button"
                    onClick={() => go(id)}
                    aria-current={on ? 'true' : undefined}
                    className={clsx(
                      'flex h-10 w-full items-center gap-2.5 rounded-control px-3 text-left text-sm font-medium transition-colors duration-150',
                      on ? 'bg-volt-soft text-ink' : 'text-ink-2 hover:bg-panel-2 hover:text-ink',
                    )}
                  >
                    <Icon className={clsx('size-[18px] shrink-0', on ? 'text-volt' : 'text-ink-3')} strokeWidth={1.75} aria-hidden />
                    {t(`settings.section.${key}`)}
                  </button>
                </li>
              );
            })}
          </ul>
        </nav>
        <div className="col-span-12 flex flex-col gap-5 min-[1100px]:col-span-9">
          <GeneralSection />
          <AiSection draft={draft} patch={patch} view={settingsView} />
          <TrackingSection draft={draft} patch={patch} />
          <PrivacySection draft={draft} patch={patch} view={settingsView} />
          <ReportsSection draft={draft} patch={patch} />
          <UbiSection draft={draft} patch={patch} />
          {settingsView.platform === 'macos' && <PermissionsSection view={settingsView} />}
          <AboutSection view={settingsView} />
        </div>
      </div>
    </div>
  );
}

type SectionProps = { draft: Settings; patch: (p: Partial<Settings>) => void };

function Section({ id, title, description, action, children }: { id: string; title: string; description?: string; action?: React.ReactNode; children: React.ReactNode }) {
  return (
    <Card id={`sec-${id}`} className="scroll-mt-4" padded={false}>
      <div className="flex items-start justify-between gap-4 border-b border-line px-5 py-4">
        <div className="min-w-0">
          <h2 className="display text-[17px] leading-6">{title}</h2>
          {description && <p className="mt-1 text-xs leading-5 text-ink-2">{description}</p>}
        </div>
        {action && <div className="shrink-0">{action}</div>}
      </div>
      <div className="flex flex-col gap-5 p-5">{children}</div>
    </Card>
  );
}

/** A hairline between two groups of settings inside a section. */
function Divider() {
  return <hr className="border-line" />;
}

/** Renders the `<b>…</b>` spans of a message as <strong>, so a sentence with emphasis stays one translatable string. */
function Emphasis({ text }: { text: string }) {
  return (
    <>
      {text.split(/<b>(.*?)<\/b>/).map((part, i) =>
        i % 2 === 1 ? (
          <strong key={i} className="text-ink">
            {part}
          </strong>
        ) : (
          <Fragment key={i}>{part}</Fragment>
        ),
      )}
    </>
  );
}

function NumberField({ label, hint, value, onChange, min, max, step, suffix }: { label: string; hint?: string; value: number; onChange: (v: number) => void; min?: number; max?: number; step?: number; suffix?: string }) {
  return (
    <Field label={label} hint={hint}>
      {(id) => (
        <div className="flex items-center gap-2">
          <Input id={id} type="number" value={value} min={min} max={max} step={step} onChange={(e) => onChange(Number(e.target.value))} className="num w-28" />
          {suffix && <span className="text-xs whitespace-nowrap text-ink-3">{suffix}</span>}
        </div>
      )}
    </Field>
  );
}

/** Language of the whole UI: switches immediately and is persisted as Settings.language. */
function GeneralSection() {
  const t = useT();
  const locale = useLocale();
  const setLanguage = useAppStore((s) => s.setLanguage);
  const toast = useToast();
  const change = (l: Locale) => {
    if (l === locale) return;
    setLanguage(l).catch((e: unknown) => toast.error(t('settings.toast.language_failed'), errorMessage(e)));
  };
  return (
    <Section id="geral" title={t('settings.section.general')} description={t('settings.general.description')}>
      <Field label={t('common.language')} hint={t('settings.language_hint')} inline>
        {() => <LanguageSelect value={locale} onChange={change} />}
      </Field>
    </Section>
  );
}

export function ApiKeyForm({ view, provider, onSaved, compact }: { view: SettingsView; provider: AiProvider; onSaved?: () => void; compact?: boolean }) {
  const t = useT();
  const [key, setKey] = useState('');
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<{ valid: boolean; message: string } | null>(null);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const toast = useToast();
  const info = providerInfo(view, provider);
  const status = keyStatus(view, provider);
  const label = info?.label ?? provider;

  // a different provider = a different key: forget the draft and the last result
  useEffect(() => {
    setKey('');
    setResult(null);
  }, [provider]);

  const submit = async (value: string | null) => {
    setBusy(true);
    setResult(null);
    try {
      const r = await ipc.setApiKey(provider, value);
      setResult(r);
      if (r.valid) {
        setKey('');
        await loadSettings();
        onSaved?.();
        toast.success(value ? t('settings.toast.key_saved', { provider: label }) : t('settings.toast.key_removed', { provider: label }));
      }
    } catch (e) {
      setResult({ valid: false, message: errorMessage(e) });
    } finally {
      setBusy(false);
    }
  };

  const openConsole = () => {
    if (info) ipc.openExternal(info.console_url).catch((e: unknown) => toast.error(t('settings.toast.open_failed'), errorMessage(e)));
  };

  return (
    <div className="flex flex-col gap-3" data-testid={`api-key-form-${provider}`}>
      {status.configured && (
        <div className="flex flex-wrap items-center gap-2 rounded-control border border-signal/25 bg-signal/8 px-3 py-2 text-sm">
          <KeyRound className="size-4 text-signal" strokeWidth={1.75} aria-hidden />
          <span>{t('settings.key.configured')}</span>
          <span className="num font-mono text-ink-2">{status.hint ?? '…'}</span>
          <Button size="sm" variant="ghost" className="ml-auto" onClick={() => void submit(null)} loading={busy}>
            {t('common.remove')}
          </Button>
        </div>
      )}
      <Field
        label={status.configured ? t('settings.key.replace_label', { provider: label }) : t('settings.key.label', { provider: label })}
        hint={
          <span className="inline-flex flex-wrap items-center gap-x-1">
            {!compact && <span>{t('settings.key.keychain_hint')}</span>}
            {info && (
              <button type="button" onClick={openConsole} className="inline-flex items-center gap-1 text-volt underline underline-offset-2 transition-colors duration-120 hover:brightness-110">
                {t('settings.key.create')} <ExternalLink className="size-3" strokeWidth={1.75} aria-hidden />
              </button>
            )}
          </span>
        }
      >
        {(id) => (
          <div className="flex gap-2">
            <Input id={id} type="password" autoComplete="off" value={key} onChange={(e) => setKey(e.target.value)} placeholder={`${info?.key_prefix ?? ''}…`} className="font-mono" />
            <Button variant="primary" className="shrink-0" onClick={() => void submit(key.trim())} disabled={!key.trim()} loading={busy}>
              {t('settings.key.validate_save')}
            </Button>
          </div>
        )}
      </Field>
      {result && (
        <p className={clsx('flex items-start gap-1.5 text-xs leading-5', result.valid ? 'text-signal' : 'text-rose')} role="status">
          {result.valid ? <Check className="mt-0.5 size-3.5 shrink-0" strokeWidth={2} aria-hidden /> : <AlertTriangle className="mt-0.5 size-3.5 shrink-0" strokeWidth={1.75} aria-hidden />}
          {result.message}
        </p>
      )}
    </div>
  );
}

/** Segmented control over the providers the backend knows; shows a key icon on the ones with a stored key. */
export function ProviderPicker({ view, value, onChange, size = 'md' }: { view: SettingsView; value: AiProvider; onChange: (p: AiProvider) => void; size?: 'md' | 'lg' }) {
  const t = useT();
  return (
    <div role="radiogroup" aria-label={t('settings.provider.label')} className="inline-flex flex-wrap items-center gap-0.5 self-start rounded-control border border-line bg-panel-2 p-1">
      {view.providers.map((p) => {
        const configured = keyStatus(view, p.id).configured;
        const active = value === p.id;
        return (
          <button
            key={p.id}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(p.id)}
            className={clsx(
              'inline-flex items-center gap-1.5 rounded-[7px] px-3 font-medium transition-colors duration-150',
              size === 'lg' ? 'h-9 text-sm' : 'h-8 text-sm',
              active ? 'bg-panel text-ink shadow-[inset_0_0_0_1px_var(--line-2)]' : 'text-ink-2 hover:text-ink',
            )}
          >
            {p.label}
            {configured && <KeyRound className="size-3.5 text-signal" strokeWidth={1.75} aria-label={t('settings.provider.key_configured')} />}
          </button>
        );
      })}
    </div>
  );
}

/**
 * Settings patch for switching to `provider`: keeps custom model ids of the same vendor,
 * otherwise adopts the provider's defaults (same rule as `Settings::reconcile_models`).
 */
export function providerSwitchPatch(view: SettingsView, draft: Settings, provider: AiProvider): Partial<Settings> {
  const next = providerInfo(view, provider);
  const current = providerInfo(view, draft.ai_provider);
  if (!next) return { ai_provider: provider };
  const customised = current ? !sameModels(draft.models, current.default_models) : true;
  return { ai_provider: provider, models: customised ? reconcileModels(draft.models, next) : { ...next.default_models } };
}

/** The three model fields; label and hint come from t('settings.models.<slot>.label|hint'). */
const MODEL_SLOTS: (keyof AiModels)[] = ['classify', 'vision', 'report'];

/** The three model fields as free text with a datalist of the account's models. */
function ModelFields({ draft, patch, view }: SectionProps & { view: SettingsView }) {
  const t = useT();
  const provider = draft.ai_provider;
  const info = providerInfo(view, provider);
  const hasKey = keyStatus(view, provider).configured;
  const listId = useId();
  const [models, setModels] = useState<{ provider: AiProvider; ids: string[] } | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const listModels = async () => {
    setLoading(true);
    setError(null);
    try {
      const ids = await ipc.listModels(provider);
      setModels({ provider, ids });
    } catch (e) {
      setModels(null);
      setError(errorMessage(e));
    } finally {
      setLoading(false);
    }
  };

  const options = models?.provider === provider ? models.ids : [];
  const isDefault = info ? sameModels(draft.models, info.default_models) : true;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <div className="min-w-0">
          <span className="text-[13px] font-medium">{t('settings.models.title', { provider: info?.label ?? provider })}</span>
          <p className="text-xs text-ink-3">{t('settings.models.hint')}</p>
        </div>
        <div className="ml-auto flex items-center gap-1.5">
          <Button size="sm" icon={<RefreshCw className="size-3.5" strokeWidth={1.75} />} onClick={() => void listModels()} loading={loading} disabled={!hasKey} title={hasKey ? undefined : t('settings.models.list_disabled')}>
            {t('settings.models.list')}
          </Button>
          <Button size="sm" variant="ghost" icon={<ListRestart className="size-3.5" strokeWidth={1.75} />} disabled={!info || isDefault} onClick={() => info && patch({ models: { ...info.default_models } })}>
            {t('settings.models.defaults')}
          </Button>
        </div>
      </div>
      {error && (
        <p className="flex items-center gap-1.5 text-xs text-rose" role="status">
          <AlertTriangle className="size-3.5" strokeWidth={1.75} aria-hidden /> {t('settings.models.list_failed', { error })}
        </p>
      )}
      {models?.provider === provider && !error && (
        <p className="num text-xs text-ink-3" role="status">
          {t('settings.models.available', { count: models.ids.length })}
        </p>
      )}
      <datalist id={listId}>
        {options.map((m) => (
          <option key={m} value={m} />
        ))}
      </datalist>
      <div className="grid grid-cols-1 gap-4 min-[900px]:grid-cols-3">
        {MODEL_SLOTS.map((slot) => (
          <Field key={slot} label={t(`settings.models.${slot}.label`)} hint={t(`settings.models.${slot}.hint`)}>
            {(id) => <Input id={id} list={listId} value={draft.models[slot]} spellCheck={false} autoComplete="off" placeholder={info?.default_models[slot]} onChange={(e) => patch({ models: { ...draft.models, [slot]: e.target.value } })} className="font-mono text-xs" />}
          </Field>
        ))}
      </div>
    </div>
  );
}

function AiSection({ draft, patch, view }: SectionProps & { view: SettingsView }) {
  const t = useT();
  const health = view.ai_health;
  const pill =
    health.state === 'ok'
      ? { color: 'var(--signal)', text: t('settings.ai.status.ok') }
      : health.state === 'not_configured'
        ? { color: 'var(--ink-4)', text: t('settings.ai.status.not_configured') }
        : health.state === 'paused'
          ? { color: 'var(--amber)', text: t('settings.ai.status.paused', { reason: health.reason }) }
          : { color: 'var(--rose)', text: t('settings.ai.status.degraded', { reason: health.reason, time: fmtTime(health.until) }) };
  const pitch = providerPitch(draft.ai_provider, t);

  return (
    <Section id="ia" title={t('settings.section.ai')} description={t('settings.ai.description')} action={<StatusPill color={pill.color}>{pill.text}</StatusPill>}>
      <Field label={t('settings.provider.label')} hint={t('settings.provider.hint', { provider: pitch.short, cost: pitch.cost })}>
        {() => <ProviderPicker view={view} value={draft.ai_provider} onChange={(p) => patch(providerSwitchPatch(view, draft, p))} />}
      </Field>
      <ApiKeyForm view={view} provider={draft.ai_provider} />
      <Divider />
      <ModelFields draft={draft} patch={patch} view={view} />
      <Divider />
      <div className="grid grid-cols-1 gap-4 min-[720px]:grid-cols-2">
        <NumberField label={t('settings.budget.label')} hint={t('settings.budget.hint')} value={draft.ai_monthly_budget_usd} min={0} step={0.5} onChange={(v) => patch({ ai_monthly_budget_usd: v })} suffix={t('settings.budget.suffix')} />
        <NumberField label={t('settings.vision_per_hour.label')} hint={t('settings.vision_per_hour.hint')} value={draft.max_vision_per_hour} min={0} max={60} onChange={(v) => patch({ max_vision_per_hour: v })} suffix={t('settings.vision_per_hour.suffix')} />
      </div>
      <Field label={t('settings.local_only.label')} hint={t('settings.local_only.hint')} inline>
        {(id) => <Toggle id={id} checked={draft.local_only} onChange={(v) => patch({ local_only: v })} />}
      </Field>
    </Section>
  );
}

function TrackingSection({ draft, patch }: SectionProps) {
  const t = useT();
  const setTrackerState = useAppStore((s) => s.setTrackerState);
  const toast = useToast();
  const toggleTracking = async (v: boolean) => {
    patch({ tracking_enabled: v });
    try {
      await ipc.setTracking(v);
      setTrackerState(v ? 'running' : 'paused');
    } catch (e) {
      toast.error(t('settings.toast.tracking_failed'), errorMessage(e));
    }
  };
  return (
    <Section id="rastreamento" title={t('settings.section.tracking')} description={t('settings.tracking.description')}>
      <Field label={t('settings.tracking.enabled.label')} hint={t('settings.tracking.enabled.hint')} inline>
        {(id) => <Toggle id={id} checked={draft.tracking_enabled} onChange={(v) => void toggleTracking(v)} />}
      </Field>
      <div className="grid grid-cols-1 gap-4 min-[900px]:grid-cols-3">
        <NumberField label={t('settings.tracking.sample_interval')} value={draft.sample_interval_secs} min={1} max={60} onChange={(v) => patch({ sample_interval_secs: v })} suffix="s" />
        <NumberField label={t('settings.tracking.idle_threshold.label')} hint={t('settings.tracking.idle_threshold.hint')} value={draft.idle_threshold_secs} min={30} step={30} onChange={(v) => patch({ idle_threshold_secs: v })} suffix="s" />
        <NumberField label={t('settings.tracking.min_block.label')} hint={t('settings.tracking.min_block.hint')} value={draft.min_block_secs} min={5} onChange={(v) => patch({ min_block_secs: v })} suffix="s" />
      </div>
      <Field label={t('settings.tracking.launch.label')} hint={t('settings.tracking.launch.hint')} inline>
        {(id) => <Toggle id={id} checked={draft.launch_at_login} onChange={(v) => patch({ launch_at_login: v })} />}
      </Field>
    </Section>
  );
}

/** Durations offered for private mode; labels come from t('settings.private.<value>'). */
const PRIVATE_OPTIONS: Exclude<PrivateModeDuration, 'off'>[] = ['minutes30', 'hour1', 'until_tomorrow', 'indefinite'];

const VISION_MODES = ['never', 'only_apps', 'all_except_blocked'] as const;

function PrivacySection({ draft, patch, view }: SectionProps & { view: SettingsView }) {
  const t = useT();
  const [aiSentOpen, setAiSentOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const setTrackerState = useAppStore((s) => s.setTrackerState);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const toast = useToast();
  const mode = draft.vision_policy.mode;
  const onlyApps = draft.vision_policy.mode === 'only_apps' ? draft.vision_policy.apps : [];
  const isPrivate = view.tracker_state === 'private' || draft.private_mode;

  const setPolicy = (p: VisionPolicy) => patch({ vision_policy: p });

  const setPrivate = async (d: PrivateModeDuration) => {
    setBusy(true);
    try {
      await ipc.setPrivateMode(d);
      setTrackerState(d === 'off' ? (draft.tracking_enabled ? 'running' : 'paused') : 'private');
      await loadSettings();
      if (d === 'off') toast.success(t('settings.toast.private_off'), t('settings.toast.private_off_body'));
      else toast.success(t('settings.toast.private_on'), t('settings.toast.private_on_body'));
    } catch (e) {
      toast.error(t('settings.toast.private_failed'), errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const privateHint = isPrivate ? (draft.private_until ? t('settings.private.on_until', { time: fmtTime(draft.private_until) }) : t('settings.private.on_indefinite')) : t('settings.private.off_hint');

  return (
    <Section id="privacidade" title={t('settings.section.privacy')} description={t('settings.privacy.description')}>
      <div className="panel-raised p-4 text-sm leading-6 text-ink-2">
        <p className="mb-1 flex items-center gap-2 font-medium text-ink">
          <Shield className="size-4 text-volt" strokeWidth={1.75} aria-hidden /> {t('settings.privacy.leaves.title')}
        </p>
        <p>
          <Emphasis text={t('settings.privacy.leaves.body')} />
        </p>
        <button type="button" onClick={() => setAiSentOpen(true)} className="mt-2 inline-flex items-center gap-1 text-volt underline underline-offset-2 transition-colors duration-120 hover:brightness-110">
          {t('settings.privacy.see_sent')} <ExternalLink className="size-3" strokeWidth={1.75} aria-hidden />
        </button>
      </div>

      <div className="flex flex-wrap items-center gap-3 rounded-control border border-line px-4 py-3">
        <span className={clsx('flex size-8 shrink-0 items-center justify-center rounded-[8px]', isPrivate ? 'bg-violet/15 text-violet' : 'bg-panel-2 text-ink-3')} aria-hidden>
          <EyeOff className="size-4" strokeWidth={1.75} />
        </span>
        <div className="min-w-0 flex-1">
          <p className="text-sm font-medium">{t('settings.private.title')}</p>
          <p className="text-xs text-ink-3">{privateHint}</p>
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          {isPrivate ? (
            <Button size="sm" onClick={() => void setPrivate('off')} loading={busy}>
              {t('settings.private.turn_off')}
            </Button>
          ) : (
            PRIVATE_OPTIONS.map((value) => (
              <Button key={value} size="sm" onClick={() => void setPrivate(value)} disabled={busy}>
                {t(`settings.private.${value}`)}
              </Button>
            ))
          )}
        </div>
      </div>

      <Divider />

      <div className="flex flex-col gap-2">
        <span className="text-[13px] font-medium">{t('settings.vision.title')}</span>
        <div role="radiogroup" aria-label={t('settings.vision.policy_label')} className="flex flex-col gap-2">
          {VISION_MODES.map((value) => (
            <RadioRow key={value} name="vision_policy" checked={mode === value} title={t(`settings.vision.${value}.label`)} text={t(`settings.vision.${value}.hint`)} onChange={() => setPolicy(value === 'only_apps' ? { mode: 'only_apps', apps: onlyApps } : { mode: value })} />
          ))}
        </div>
        {mode === 'only_apps' && (
          <Field label={t('settings.vision.apps_label')}>{(id) => <TagInput id={id} value={onlyApps} onChange={(apps) => setPolicy({ mode: 'only_apps', apps })} placeholder={t('settings.vision.apps_placeholder')} />}</Field>
        )}
      </div>

      <div className="grid grid-cols-1 gap-4 min-[720px]:grid-cols-2">
        <Field label={t('settings.denied.label')} hint={t('settings.denied.hint')}>
          {(id) => <TagInput id={id} value={draft.vision_denied_apps} onChange={(v) => patch({ vision_denied_apps: v })} placeholder={t('settings.denied.placeholder')} />}
        </Field>
        <Field label={t('settings.blocked_apps.label')} hint={t('settings.blocked_apps.hint')}>
          {(id) => <TagInput id={id} value={draft.blocked_apps} onChange={(v) => patch({ blocked_apps: v })} placeholder={t('settings.blocked_apps.placeholder')} />}
        </Field>
        <Field label={t('settings.blocked_domains.label')} hint={t('settings.blocked_domains.hint')} className="min-[720px]:col-span-2">
          {(id) => <TagInput id={id} value={draft.blocked_domains} onChange={(v) => patch({ blocked_domains: v })} placeholder={t('settings.blocked_domains.placeholder')} />}
        </Field>
      </div>

      <Divider />

      <div className="grid grid-cols-1 gap-4 min-[900px]:grid-cols-3">
        <NumberField label={t('settings.screenshots.interval')} value={draft.screenshot_interval_secs} min={30} step={30} onChange={(v) => patch({ screenshot_interval_secs: v })} suffix="s" />
        <NumberField label={t('settings.screenshots.retention.label')} hint={t('settings.screenshots.retention.hint')} value={draft.screenshot_retention_hours} min={1} onChange={(v) => patch({ screenshot_retention_hours: v })} suffix="h" />
        <NumberField label={t('settings.screenshots.max_edge')} value={draft.screenshot_max_edge} min={256} step={64} onChange={(v) => patch({ screenshot_max_edge: v })} suffix="px" />
      </div>
      <Field label={t('settings.screenshots.keep.label')} hint={t('settings.screenshots.keep.hint')} inline>
        {(id) => <Toggle id={id} checked={draft.keep_screenshots_for_review} onChange={(v) => patch({ keep_screenshots_for_review: v })} />}
      </Field>

      <AiSentDialog open={aiSentOpen} onClose={() => setAiSentOpen(false)} />
    </Section>
  );
}

/** A selectable row with a native radio, styled: the chosen one gets a volt hairline and tint. */
function RadioRow({ name, checked, title, text, onChange }: { name: string; checked: boolean; title: string; text: string; onChange: () => void }) {
  return (
    <label className={clsx('flex cursor-pointer items-start gap-3 rounded-control border p-3 transition-colors duration-150', checked ? 'border-volt/60 bg-volt-soft' : 'border-line hover:bg-panel-2')}>
      <input type="radio" name={name} className="mt-1 accent-[var(--volt)]" checked={checked} onChange={onChange} />
      <span>
        <span className="block text-sm font-medium">{title}</span>
        <span className="block text-xs leading-5 text-ink-3">{text}</span>
      </span>
    </label>
  );
}

function AiSentDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const t = useT();
  const date = useAppStore((s) => s.date);
  const { data, loading } = useAsync(() => (open ? ipc.getAiSent(date) : Promise.resolve(null)), [open, date]);
  return (
    <Dialog open={open} onClose={onClose} title={t('settings.sent.title')} description={t('settings.sent.description', { date: fmtDateNumeric(date) })} width="lg">
      {loading && !data ? (
        <p className="text-sm text-ink-3">{t('settings.sent.loading')}</p>
      ) : !data?.length ? (
        <EmptyState title={t('settings.sent.empty')} />
      ) : (
        <ul className="flex flex-col gap-2">
          {data.map((b) => (
            <li key={b.id} className="rounded-control border border-line p-3">
              <div className="mb-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
                <span className="num font-medium">{fmtTime(b.started_at)}</span>
                <span className="font-medium">{b.app_name}</span>
                {b.domain && <span className="text-ink-2">{b.domain}</span>}
                <span className="ml-auto flex items-center gap-2 text-ink-3">
                  {b.screenshot_id && <Badge tone="violet">{t('settings.sent.with_image')}</Badge>}
                  {b.ai_sent_at && <span className="num">{fmtDateTime(b.ai_sent_at)}</span>}
                </span>
              </div>
              <pre className="scroll-thin overflow-x-auto rounded-[8px] bg-panel-2 p-2 font-mono text-[11px] leading-4 text-ink-2">{b.ai_payload ?? t('settings.sent.no_payload')}</pre>
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}

function ReportsSection({ draft, patch }: SectionProps) {
  const t = useT();
  return (
    <Section id="relatorios" title={t('settings.section.reports')} description={t('settings.reports.description')}>
      <Field label={t('settings.reports.default_time.label')} hint={t('settings.reports.default_time.hint')}>
        {(id) => <Input id={id} type="time" value={hhmmToInput(draft.report_default_time)} onChange={(e) => e.target.value && patch({ report_default_time: inputToHhmm(e.target.value) })} className="num w-36" />}
      </Field>
      <Field label={t('settings.reports.profile.label')} hint={t('settings.reports.profile.hint')}>
        {(id) => <Textarea id={id} value={draft.user_profile ?? ''} onChange={(e) => patch({ user_profile: e.target.value || null })} placeholder={t('settings.reports.profile.placeholder')} className="min-h-28" />}
      </Field>
    </Section>
  );
}

/** Nudge kinds the user can switch individually; copy from t('settings.ubi.kind.<kind>.label|hint'). */
const NUDGE_TOGGLES = ['unproductive', 'distracted', 'break_suggested', 'praise', 'idle'] as const;

/** Snooze presets: minutes → label key. */
const SNOOZE_PRESETS: { minutes: number; key: string }[] = [
  { minutes: 30, key: 'minutes30' },
  { minutes: 60, key: 'hour1' },
  { minutes: 4 * 60, key: 'hours4' },
];

function UbiSection({ draft, patch }: SectionProps) {
  const t = useT();
  const n = draft.nudges;
  const setN = (p: Partial<typeof n>) => patch({ nudges: { ...n, ...p } });
  const toast = useToast();
  const snooze = async (minutes: number) => {
    try {
      await ipc.snoozeNudges(minutes);
      setN({ snoozed_until: new Date(Date.now() + minutes * 60_000).toISOString() });
      const duration = minutes >= 60 ? t('settings.duration.hours', { count: Math.round(minutes / 60) }) : t('settings.duration.minutes', { count: minutes });
      toast.success(t('settings.toast.snoozed'), t('settings.toast.snoozed_body', { duration }));
    } catch (e) {
      toast.error(t('settings.toast.snooze_failed'), errorMessage(e));
    }
  };
  const snoozed = n.snoozed_until && new Date(n.snoozed_until).getTime() > Date.now();

  return (
    <Section
      id="ubi"
      title={t('settings.section.ubi')}
      description={t('settings.ubi.description')}
      action={snoozed && n.snoozed_until ? <Badge tone="amber">{t('settings.ubi.snoozed_until', { time: fmtTime(n.snoozed_until) })}</Badge> : undefined}
    >
      <Field label={t('settings.ubi.nudges.label')} hint={t('settings.ubi.nudges.hint')} inline>
        {(id) => <Toggle id={id} checked={n.enabled} onChange={(v) => setN({ enabled: v })} />}
      </Field>
      <div className={clsx('grid grid-cols-1 gap-x-8 gap-y-3 rounded-control border border-line p-4 min-[720px]:grid-cols-2', !n.enabled && 'opacity-60')}>
        {NUDGE_TOGGLES.map((key) => (
          <Field key={key} label={t(`settings.ubi.kind.${key}.label`)} hint={t(`settings.ubi.kind.${key}.hint`)} inline>
            {(id) => <Toggle id={id} size="sm" checked={n[key]} disabled={!n.enabled} onChange={(v) => setN({ [key]: v })} />}
          </Field>
        ))}
      </div>
      <div className="grid grid-cols-1 gap-4 min-[720px]:grid-cols-2">
        <NumberField label={t('settings.ubi.max_per_day')} value={n.max_per_day} min={0} max={50} onChange={(v) => setN({ max_per_day: v })} suffix={t('settings.ubi.max_per_day_suffix')} />
        <NumberField label={t('settings.ubi.cooldown')} value={n.cooldown_mins} min={1} onChange={(v) => setN({ cooldown_mins: v })} suffix="min" />
      </div>
      <Divider />
      <div className="grid grid-cols-1 items-end gap-4 min-[720px]:grid-cols-3">
        <Field label={t('settings.ubi.quiet.label')} hint={t('settings.ubi.quiet.hint')} inline>
          {(id) => <Toggle id={id} checked={draft.quiet_hours.enabled} onChange={(v) => patch({ quiet_hours: { ...draft.quiet_hours, enabled: v } })} />}
        </Field>
        <Field label={t('settings.ubi.quiet.from')}>
          {(id) => <Input id={id} type="time" className="num" value={hhmmToInput(draft.quiet_hours.start)} disabled={!draft.quiet_hours.enabled} onChange={(e) => e.target.value && patch({ quiet_hours: { ...draft.quiet_hours, start: inputToHhmm(e.target.value) } })} />}
        </Field>
        <Field label={t('settings.ubi.quiet.to')}>
          {(id) => <Input id={id} type="time" className="num" value={hhmmToInput(draft.quiet_hours.end)} disabled={!draft.quiet_hours.enabled} onChange={(e) => e.target.value && patch({ quiet_hours: { ...draft.quiet_hours, end: inputToHhmm(e.target.value) } })} />}
        </Field>
      </div>
      <Field label={t('settings.ubi.silent_apps.label')} hint={t('settings.ubi.silent_apps.hint')}>
        {(id) => <TagInput id={id} value={n.silent_apps} onChange={(v) => setN({ silent_apps: v })} placeholder={t('settings.ubi.silent_apps.placeholder')} />}
      </Field>
      <div className="flex flex-wrap items-center gap-2 border-t border-line pt-4">
        <span className="mr-1 text-sm text-ink-2">{t('settings.ubi.snooze_now')}</span>
        {SNOOZE_PRESETS.map((p) => (
          <Button key={p.minutes} size="sm" onClick={() => void snooze(p.minutes)}>
            {t(`settings.ubi.snooze.${p.key}`)}
          </Button>
        ))}
      </div>
    </Section>
  );
}

const PERM_COLOR: Record<PermissionState, string> = {
  granted: 'var(--signal)',
  denied: 'var(--rose)',
  unknown: 'var(--ink-4)',
  not_applicable: 'var(--ink-4)',
};

const PERMISSION_KINDS: PermissionKind[] = ['screen_recording', 'automation', 'accessibility'];

export function PermissionRows({ view, onChanged }: { view: SettingsView; onChanged?: () => void }) {
  const t = useT();
  const [busy, setBusy] = useState<PermissionKind | null>(null);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const toast = useToast();
  const request = async (kind: PermissionKind) => {
    setBusy(kind);
    try {
      await ipc.requestPermission(kind);
      await loadSettings();
      onChanged?.();
    } catch (e) {
      toast.error(t('settings.toast.permission_failed'), errorMessage(e));
    } finally {
      setBusy(null);
    }
  };
  return (
    <ul className="divide-y divide-line rounded-card border border-line">
      {PERMISSION_KINDS.map((kind) => {
        const state = view.permissions[kind];
        return (
          <li key={kind} className="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
            <div className="min-w-0 flex-1 basis-60">
              <p className="text-sm font-medium">{t(`settings.permissions.${kind}.label`)}</p>
              <p className="text-xs leading-5 text-ink-3">{t(`settings.permissions.${kind}.hint`)}</p>
            </div>
            <StatusPill color={PERM_COLOR[state]}>{t(`settings.permissions.state.${state}`)}</StatusPill>
            {state !== 'granted' && state !== 'not_applicable' && (
              <Button size="sm" variant="primary" onClick={() => void request(kind)} loading={busy === kind}>
                {t('settings.permissions.request')}
              </Button>
            )}
          </li>
        );
      })}
    </ul>
  );
}

function PermissionsSection({ view }: { view: SettingsView }) {
  const t = useT();
  const date = useAppStore((s) => s.date);
  const dash = useAppStore((s) => s.dashboards[date]);
  const titlesEmpty = useMemo(() => !!dash && dash.timeline.length > 0 && dash.timeline.filter((b) => b.app_id !== 'idle' && b.app_id !== 'private' && b.app_id !== 'privado').every((b) => !b.title), [dash]);
  const needsRestart = view.permissions.screen_recording === 'granted' && titlesEmpty;
  return (
    <Section id="permissoes" title={t('settings.section.permissions')} description={t('settings.permissions.description')}>
      <PermissionRows view={view} />
      {needsRestart && (
        <div className="flex flex-wrap items-center gap-3 rounded-control border border-amber/40 bg-amber/10 p-3 text-sm">
          <Camera className="size-4 shrink-0 text-amber" strokeWidth={1.75} aria-hidden />
          <span className="flex-1 basis-60 leading-5">{t('settings.permissions.restart_hint')}</span>
          <Button size="sm" variant="primary" onClick={() => void ipc.restartApp()}>
            {t('settings.permissions.restart')}
          </Button>
        </div>
      )}
    </Section>
  );
}

function AboutSection({ view }: { view: SettingsView }) {
  const t = useT();
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [busy, setBusy] = useState<'export' | 'delete' | null>(null);
  const bumpData = useAppStore((s) => s.bumpData);
  const toast = useToast();

  const exportData = async () => {
    setBusy('export');
    try {
      const path = await ipc.exportData();
      toast.success(t('settings.toast.exported'), path);
    } catch (e) {
      toast.error(t('settings.toast.export_failed'), errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const deleteAll = async () => {
    setBusy('delete');
    try {
      await ipc.deleteAllData();
      setConfirmDelete(false);
      bumpData();
      toast.success(t('settings.toast.deleted'));
    } catch (e) {
      toast.error(t('settings.toast.delete_failed'), errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  const engine = t(`settings.about.engine_state.${view.tracker_state}`);
  const ai = t(`settings.about.ai_state.${view.ai_health.state}`);

  return (
    <Section id="sobre" title={t('settings.section.about')} description={t('settings.about.description')}>
      <dl className="grid grid-cols-[140px_1fr] gap-x-4 gap-y-2.5 text-sm">
        <dt className="text-ink-3">{t('settings.about.version')}</dt>
        <dd className="num font-medium">ubiqX {view.version}</dd>
        <dt className="text-ink-3">{t('settings.about.data_dir')}</dt>
        <dd className="truncate font-mono text-xs leading-5" title={view.data_dir}>
          {view.data_dir}
        </dd>
        <dt className="text-ink-3">{t('settings.about.platform')}</dt>
        <dd>{view.platform === 'macos' ? 'macOS' : t('settings.about.platform_dev')}</dd>
        <dt className="text-ink-3">{t('settings.about.engine')}</dt>
        <dd>{engine}</dd>
        <dt className="text-ink-3">{t('settings.about.ai')}</dt>
        <dd>{ai}</dd>
      </dl>
      <div className="flex flex-wrap items-center gap-2 border-t border-line pt-4">
        <Button icon={<Download className="size-4" strokeWidth={1.75} />} onClick={() => void exportData()} loading={busy === 'export'}>
          {t('settings.about.export')}
        </Button>
        <Button
          icon={<ExternalLink className="size-4" strokeWidth={1.75} />}
          onClick={() => {
            ipc.openExternal(REPO_URL).catch((e: unknown) => toast.error(t('settings.toast.open_failed'), errorMessage(e)));
          }}
        >
          {t('settings.about.repo')}
        </Button>
        <Button variant="danger" className="ml-auto" icon={<Trash2 className="size-4" strokeWidth={1.75} />} onClick={() => setConfirmDelete(true)}>
          {t('settings.about.delete_all')}
        </Button>
      </div>

      <Dialog
        open={confirmDelete}
        onClose={() => setConfirmDelete(false)}
        title={t('settings.about.delete_title')}
        description={t('settings.about.delete_description')}
        width="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
              {t('common.cancel')}
            </Button>
            <Button variant="danger" onClick={() => void deleteAll()} loading={busy === 'delete'}>
              {t('settings.about.delete_confirm')}
            </Button>
          </>
        }
      >
        <p className="text-sm leading-6 text-ink-2">{t('settings.about.delete_warning')}</p>
      </Dialog>
    </Section>
  );
}
