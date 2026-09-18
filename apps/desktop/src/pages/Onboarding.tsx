import clsx from 'clsx';
import { motion, useReducedMotion } from 'framer-motion';
import { ArrowLeft, BrainCircuit, Check, Eye, Info, KeyRound, Lock, Plus, Rocket, Shield, ShieldCheck, Tags, Timer, Trash2 } from 'lucide-react';
import { Fragment, useEffect, useMemo, useState, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { BrandMark } from '../components/layout/Sidebar';
import { Ubi } from '../components/ubi/Ubi';
import { Button, IconButton } from '../components/ui/Button';
import { Field, Input, Textarea } from '../components/ui/Field';
import { IconPicker } from '../components/ui/IconPicker';
import { LanguageSelect } from '../components/ui/LanguageSelect';
import { Toggle } from '../components/ui/Toggle';
import { useLocale, useT, type Locale } from '../i18n';
import { COLOR_CHOICES, iconFor, newCategoryId } from '../lib/categories';
import { hhmmToInput, inputToHhmm } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { Toaster, useToast } from '../lib/toast';
import { keyStatus, providerInfo, providerPitch } from '../lib/providers';
import type { AiProvider, Category, Mood, Settings, SettingsView, VisionPolicy } from '../lib/types';
import { ApiKeyForm, PermissionRows, providerSwitchPatch } from './Settings';

type Translate = ReturnType<typeof useT>;

/** Step ids and icons; titles and UBI's rail speech come from t('onboarding.step.<id>') / t('onboarding.rail.<id>'). */
const STEPS = [
  { id: 'intro', Icon: Shield },
  { id: 'ai', Icon: BrainCircuit },
  { id: 'perms', Icon: ShieldCheck },
  { id: 'cats', Icon: Tags },
  { id: 'times', Icon: Timer },
  { id: 'vision', Icon: Eye },
  { id: 'finish', Icon: Rocket },
] as const;

/** `?step=N` (1-based) opens the wizard on that step — used by the screenshot script; defaults to step 1. */
function stepFromQuery(): number {
  try {
    const n = Number(new URLSearchParams(window.location.search).get('step'));
    return Number.isInteger(n) && n >= 1 && n <= STEPS.length ? n - 1 : 0;
  } catch {
    return 0;
  }
}

interface CatDraft {
  id: string;
  existing: Category | null;
  name: string;
  color: string;
  icon: string;
  description: string;
}

/** Starter categories in the current language. "IFRO" is an institution acronym and reads the same in both languages. */
const suggestedCategories = (t: Translate): Omit<CatDraft, 'id' | 'existing'>[] => [
  { name: 'IFRO', color: '#2563EB', icon: 'graduation-cap', description: t('onboarding.suggested.ifro.description') },
  { name: t('onboarding.suggested.incubator.name'), color: '#F97316', icon: 'rocket', description: t('onboarding.suggested.incubator.description') },
  { name: t('onboarding.suggested.smart_cities.name'), color: '#10B981', icon: 'building-2', description: t('onboarding.suggested.smart_cities.description') },
];

/** Splits a message on its `{name}` placeholders and swaps each for a React node (inline code, mono spans). */
function richText(template: string, nodes: Record<string, ReactNode>): ReactNode {
  return template.split(/(\{\w+\})/g).map((part, i) => {
    const m = /^\{(\w+)\}$/.exec(part);
    const name = m?.[1];
    return name !== undefined && name in nodes ? <Fragment key={i}>{nodes[name]}</Fragment> : part;
  });
}

export function Onboarding() {
  const t = useT();
  const locale = useLocale();
  const settingsView = useAppStore((s) => s.settingsView);
  const saveSettings = useAppStore((s) => s.saveSettings);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const loadCategories = useAppStore((s) => s.loadCategories);
  const setLanguage = useAppStore((s) => s.setLanguage);
  const navigate = useNavigate();
  const toast = useToast();
  const reduce = useReducedMotion();
  const [step, setStep] = useState(stepFromQuery);
  const [draft, setDraft] = useState<Settings | null>(settingsView?.settings ?? null);
  const [cats, setCats] = useState<CatDraft[]>([]);
  /** True while the list is still the untouched starters: they follow the language until the user edits them. */
  const [starters, setStarters] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (settingsView && !draft) setDraft(settingsView.settings);
  }, [settingsView, draft]);

  useEffect(() => {
    void ipc.listCategories().then((list) => {
      const user = list.filter((c) => !c.is_system && !c.archived);
      if (user.length) setCats(user.map((c) => ({ id: c.id, existing: c, name: c.name, color: c.color, icon: c.icon, description: c.description })));
      else setStarters(true);
    });
  }, []);

  useEffect(() => {
    if (starters) setCats(suggestedCategories(t).map((s) => ({ ...s, id: newCategoryId(), existing: null })));
  }, [starters, t]);

  const editCats = (next: CatDraft[]) => {
    setStarters(false);
    setCats(next);
  };

  const patch = (p: Partial<Settings>) => setDraft((d) => (d ? { ...d, ...p } : d));

  /** Switching the language persists it right away and keeps the local draft in sync so later saves do not revert it. */
  const changeLanguage = (l: Locale) => {
    patch({ language: l });
    void setLanguage(l);
  };

  /** Picking a provider card persists it right away so the key form and the backend health follow. */
  const selectProvider = async (provider: AiProvider) => {
    if (!draft || !settingsView || provider === draft.ai_provider) return;
    const p = providerSwitchPatch(settingsView, draft, provider);
    patch(p);
    try {
      const v = await saveSettings(p);
      if (v) patch({ ai_provider: v.settings.ai_provider, models: v.settings.models });
    } catch (e) {
      toast.error(t('onboarding.provider_switch_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  const mood: Mood = useMemo(() => (step === 0 ? 'calm' : step === STEPS.length - 1 ? 'excited' : 'focused'), [step]);

  if (!draft || !settingsView) return null;

  const current = STEPS[step] ?? STEPS[0];

  const saveCats = async () => {
    const valid = cats.filter((c) => c.name.trim());
    if (valid.some((c) => !c.description.trim())) {
      toast.error(t('onboarding.cats_missing_description_title'), t('onboarding.cats_missing_description_body'));
      return false;
    }
    setBusy(true);
    try {
      let order = 0;
      for (const c of valid) {
        const base: Category = c.existing ?? {
          id: c.id,
          name: '',
          color: c.color,
          icon: c.icon,
          description: '',
          keywords: [],
          report_time: null,
          report_template: null,
          is_productive: true,
          is_system: false,
          archived: false,
          sort_order: order,
          created_at: new Date().toISOString(),
        };
        await ipc.saveCategory({ ...base, name: c.name.trim(), color: c.color, icon: c.icon, description: c.description.trim(), sort_order: order++ });
      }
      await loadCategories();
      return true;
    } catch (e) {
      toast.error(t('onboarding.cats_save_failed'), e instanceof Error ? e.message : String(e));
      return false;
    } finally {
      setBusy(false);
    }
  };

  const next = async () => {
    if (current.id === 'cats' && !(await saveCats())) return;
    if (current.id === 'times' || current.id === 'vision') {
      await saveSettings(draft).catch(() => undefined);
    }
    setStep((s) => Math.min(STEPS.length - 1, s + 1));
  };

  const finish = async () => {
    setBusy(true);
    try {
      await saveSettings({ ...draft, onboarding_done: true });
      await loadSettings();
      toast.success(t('onboarding.finish_success_title'), t('onboarding.finish_success_body'));
      navigate('/', { replace: true });
    } catch (e) {
      toast.error(t('onboarding.finish_failed'), e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-canvas" data-testid="page-onboarding">
      {/* Left: UBI, the brand and the step list. The only glow in the flow sits behind the mascot. */}
      <aside className="relative flex w-[300px] shrink-0 flex-col overflow-hidden border-r border-line bg-panel/70 backdrop-blur-xl min-[1180px]:w-[360px]" aria-label={t('onboarding.progress')}>
        <div className="pointer-events-none absolute inset-x-0 top-0 h-[420px]" style={{ background: 'var(--hero-glow)' }} aria-hidden />
        <div data-tauri-drag-region className="relative h-[38px] shrink-0" />
        <div className="relative flex items-center gap-2.5 px-6 pt-1">
          <BrandMark />
          <span className="display text-[17px]">
            ubiq<span className="text-volt">X</span>
          </span>
          <span className="ml-auto text-xs text-ink-3">{t('onboarding.initial_setup')}</span>
        </div>

        <div className="relative flex flex-col items-center px-6 pt-9 pb-4">
          <Ubi mood={mood} size={176} speaking={t(`onboarding.rail.${current.id}`)} />
        </div>

        <ol className="relative mt-2 flex flex-col px-6" aria-label={t('onboarding.steps')}>
          {STEPS.map((s, i) => {
            const done = i < step;
            const active = i === step;
            const last = i === STEPS.length - 1;
            return (
              <li key={s.id} className="relative">
                {!last && <span className={clsx('absolute top-8 bottom-0 left-[15px] w-px', done ? 'bg-signal/50' : 'bg-line-2')} aria-hidden />}
                <button
                  type="button"
                  onClick={() => i < step && setStep(i)}
                  disabled={i > step}
                  aria-current={active ? 'step' : undefined}
                  className={clsx(
                    'relative flex h-11 w-full items-center gap-3 rounded-control pr-2 text-left text-sm transition-colors duration-150',
                    active ? 'font-medium text-ink' : done ? 'text-ink-2 hover:text-ink' : 'text-ink-3',
                  )}
                >
                  <span
                    className={clsx(
                      'num flex size-[30px] shrink-0 items-center justify-center rounded-full border text-[12px] font-semibold transition-[background-color,border-color,box-shadow] duration-150',
                      active ? 'glow-volt border-volt bg-volt text-on-volt' : done ? 'border-signal/40 bg-signal/15 text-signal' : 'border-line-2 bg-panel text-ink-4',
                    )}
                  >
                    {done ? <Check className="size-3.5" strokeWidth={2.25} aria-label={t('onboarding.step_done')} /> : i + 1}
                  </span>
                  {t(`onboarding.step.${s.id}`)}
                </button>
              </li>
            );
          })}
        </ol>

        <p className="relative mt-auto px-6 pb-5 text-xs leading-5 text-ink-3">{t('onboarding.rail_footer')}</p>
      </aside>

      {/* Right: the step panel */}
      <main className="flex min-w-0 flex-1 flex-col">
        <div data-tauri-drag-region className="h-[38px] shrink-0" />
        <div className="scroll-thin min-h-0 flex-1 overflow-y-auto px-10 pb-8">
          <motion.div
            key={current.id}
            className="mx-auto w-full max-w-[680px]"
            initial={reduce ? { opacity: 0 } : { opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: reduce ? 0 : 0.18, ease: [0.16, 1, 0.3, 1] }}
          >
            <div className="flex min-h-7 items-center justify-between gap-4">
              <p className="eyebrow num">{t('onboarding.step_of', { step: step + 1, total: STEPS.length })}</p>
              {/* The language switch sits at the top of the first step so the rest of the flow can be read in the chosen language. */}
              {current.id === 'intro' && <LanguageSelect compact value={locale} onChange={changeLanguage} className="-my-1" />}
            </div>
            {current.id === 'intro' && <IntroStep />}
            {current.id === 'ai' && <AiStep view={settingsView} draft={draft} onSelect={(p) => void selectProvider(p)} />}
            {current.id === 'perms' && <PermsStep view={settingsView} />}
            {current.id === 'cats' && <CatsStep cats={cats} setCats={editCats} />}
            {current.id === 'times' && <TimesStep draft={draft} patch={patch} />}
            {current.id === 'vision' && <VisionStep draft={draft} patch={patch} />}
            {current.id === 'finish' && <FinishStep draft={draft} patch={patch} view={settingsView} />}
          </motion.div>
        </div>
        <div className="flex items-center justify-between border-t border-line bg-panel/70 px-10 py-4 backdrop-blur-xl">
          <Button variant="ghost" icon={<ArrowLeft className="size-4" strokeWidth={1.75} />} disabled={step === 0} onClick={() => setStep((s) => Math.max(0, s - 1))}>
            {t('common.back')}
          </Button>
          <div className="flex items-center gap-2">
            {current.id === 'ai' && (
              <Button variant="ghost" onClick={() => setStep((s) => s + 1)}>
                {t('onboarding.skip_for_now')}
              </Button>
            )}
            {current.id === 'finish' ? (
              <Button variant="primary" size="lg" icon={<Rocket className="size-4" strokeWidth={1.75} />} loading={busy} onClick={() => void finish()}>
                {t('common.finish')}
              </Button>
            ) : (
              <Button variant="primary" size="lg" loading={busy} onClick={() => void next()}>
                {t('common.continue')}
              </Button>
            )}
          </div>
        </div>
      </main>
      <Toaster />
    </div>
  );
}

function StepTitle({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mt-2 mb-6">
      <h1 className="display text-[26px] leading-8">{title}</h1>
      <p className="mt-2 max-w-[62ch] text-sm leading-6 text-ink-2">{children}</p>
    </div>
  );
}

function IntroStep() {
  const t = useT();
  const items: { Icon: typeof Shield; title: string; text: string }[] = [
    { Icon: Timer, title: t('onboarding.intro.logged_title'), text: t('onboarding.intro.logged_text') },
    { Icon: Eye, title: t('onboarding.intro.screenshots_title'), text: t('onboarding.intro.screenshots_text') },
    { Icon: Shield, title: t('onboarding.intro.ai_title'), text: t('onboarding.intro.ai_text') },
    { Icon: Lock, title: t('onboarding.intro.local_title'), text: t('onboarding.intro.local_text') },
  ];
  return (
    <>
      <StepTitle title={t('onboarding.intro.title')}>{t('onboarding.intro.lead')}</StepTitle>
      <ul className="panel divide-y divide-line">
        {items.map(({ Icon, title, text }) => (
          <li key={title} className="flex gap-4 p-4">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-control bg-volt-soft text-volt" aria-hidden>
              <Icon className="size-[18px]" strokeWidth={1.75} />
            </span>
            <div className="min-w-0">
              <p className="text-sm font-medium">{title}</p>
              <p className="mt-0.5 text-sm leading-6 text-ink-2">{text}</p>
            </div>
          </li>
        ))}
      </ul>
    </>
  );
}

function AiStep({ view, draft, onSelect }: { view: SettingsView; draft: Settings; onSelect: (p: AiProvider) => void }) {
  const t = useT();
  const selected = draft.ai_provider;
  const info = providerInfo(view, selected);
  return (
    <>
      <StepTitle title={t('onboarding.ai.title')}>{t('onboarding.ai.lead')}</StepTitle>
      <div role="radiogroup" aria-label={t('onboarding.ai.provider_group')} className="grid grid-cols-1 gap-3 min-[900px]:grid-cols-3" data-testid="provider-cards">
        {view.providers.map((p) => {
          const pitch = providerPitch(p.id, t);
          const status = keyStatus(view, p.id);
          const active = selected === p.id;
          return (
            <button
              key={p.id}
              type="button"
              role="radio"
              aria-checked={active}
              aria-label={p.label}
              onClick={() => onSelect(p.id)}
              className={clsx(
                'flex h-full flex-col items-start gap-2 rounded-card border p-4 text-left transition-[border-color,background-color,box-shadow] duration-150',
                active ? 'glow-volt border-volt bg-volt-soft' : 'border-line bg-panel hover:bg-panel-2',
              )}
            >
              <span className="flex w-full items-start justify-between gap-2">
                <span className="min-w-0">
                  <span className="display block text-[20px] leading-7">{pitch.short}</span>
                  <span className="block text-xs text-ink-3">{p.label}</span>
                </span>
                <span className={clsx('flex size-6 shrink-0 items-center justify-center rounded-full', active ? 'bg-volt text-on-volt' : status.configured ? 'bg-signal/15 text-signal' : 'border border-line-2 text-ink-4')} aria-hidden>
                  {active ? <Check className="size-3.5" strokeWidth={2.25} /> : status.configured ? <KeyRound className="size-3" strokeWidth={1.75} /> : null}
                </span>
              </span>
              <span className="text-sm leading-5 text-ink-2">{pitch.pitch}</span>
              <span className="text-[11px] leading-4 text-ink-3">
                {richText(t('onboarding.ai.recommended'), {
                  classify: <span className="font-mono text-ink-2">{p.default_models.classify}</span>,
                  report: <span className="font-mono text-ink-2">{p.default_models.report}</span>,
                })}
              </span>
              <span className="mt-auto flex w-full flex-col border-t border-line pt-2.5">
                <span className="display num text-[18px] leading-6 whitespace-nowrap text-ink">{pitch.cost}</span>
                <span className="text-[11px] text-ink-3">{t('onboarding.ai.cost_basis')}</span>
              </span>
              {status.configured && (
                <span className="flex items-center gap-1 text-[11px] text-signal">
                  <KeyRound className="size-3" strokeWidth={1.75} aria-hidden /> {t('onboarding.ai.key_configured', { hint: status.hint ?? '' }).trim()}
                </span>
              )}
            </button>
          );
        })}
      </div>
      {view.providers
        .map((p) => ({ id: p.id, pitch: providerPitch(p.id, t) }))
        .filter(({ pitch }) => pitch.note)
        .map(({ id, pitch }) => (
          <p key={id} className="mt-3 flex items-start gap-2 text-xs leading-5 text-ink-3">
            <Info className="mt-0.5 size-3.5 shrink-0" strokeWidth={1.75} aria-hidden />
            <span>
              <strong className="font-medium text-ink-2">{pitch.short}:</strong> {pitch.note}
            </span>
          </p>
        ))}
      <div className="panel mt-5 p-5">
        <ApiKeyForm view={view} provider={selected} compact />
        <p className="mt-3 text-xs leading-5 text-ink-3">
          {info ? `${t('onboarding.ai.create_key_at', { host: info.console_url.replace(/^https?:\/\//, '') })} ` : ''}
          {t('onboarding.ai.key_storage')}
        </p>
      </div>
    </>
  );
}

function PermsStep({ view }: { view: SettingsView }) {
  const t = useT();
  return (
    <>
      <StepTitle title={t('onboarding.perms.title')}>{t('onboarding.perms.lead')}</StepTitle>
      {view.platform === 'macos' ? (
        <>
          <PermissionRows view={view} />
          <div className="mt-4 flex gap-3 rounded-control border border-amber/40 bg-amber/10 p-3 text-xs leading-5 text-ink-2">
            <Info className="mt-0.5 size-3.5 shrink-0 text-amber" strokeWidth={1.75} aria-hidden />
            <span>
              <strong className="text-ink">{t('onboarding.perms.restart_strong')}</strong> {t('onboarding.perms.restart_text')}
            </span>
          </div>
        </>
      ) : (
        <div className="panel p-5 text-sm leading-6 text-ink-2">{t('onboarding.perms.not_macos')}</div>
      )}
    </>
  );
}

function CatsStep({ cats, setCats }: { cats: CatDraft[]; setCats: (c: CatDraft[]) => void }) {
  const t = useT();
  const update = (id: string, p: Partial<CatDraft>) => setCats(cats.map((c) => (c.id === id ? { ...c, ...p } : c)));
  const remove = (id: string) => setCats(cats.filter((c) => c.id !== id));
  const add = () => setCats([...cats, { id: newCategoryId(), existing: null, name: '', color: COLOR_CHOICES[cats.length % COLOR_CHOICES.length] ?? '#2563EB', icon: 'briefcase', description: '' }]);
  return (
    <>
      <StepTitle title={t('onboarding.cats.title')}>{t('onboarding.cats.lead')}</StepTitle>
      <div className="flex flex-col gap-3">
        {cats.map((c) => {
          const Icon = iconFor(c.icon);
          return (
            <div key={c.id} className="panel flex flex-col gap-3 p-4">
              <div className="flex items-center gap-3">
                <span className="flex size-10 shrink-0 items-center justify-center rounded-control" style={{ background: `color-mix(in oklab, ${c.color} 16%, transparent)`, color: c.color, boxShadow: `inset 0 0 0 1px color-mix(in oklab, ${c.color} 28%, transparent)` }} aria-hidden>
                  <Icon className="size-5" strokeWidth={1.75} />
                </span>
                <Input value={c.name} onChange={(e) => update(c.id, { name: e.target.value })} placeholder={t('onboarding.cats.name_placeholder')} aria-label={t('onboarding.cats.name_placeholder')} className="min-w-0 flex-1 font-medium" />
                <IconButton label={t('onboarding.cats.remove', { name: c.name || t('onboarding.cats.unnamed') })} onClick={() => remove(c.id)}>
                  <Trash2 className="size-4" strokeWidth={1.75} />
                </IconButton>
              </div>
              <div className="flex flex-wrap items-start gap-x-8 gap-y-3">
                <div className="flex flex-col gap-1.5">
                  <span className="text-xs text-ink-3">{t('onboarding.cats.color')}</span>
                  <div className="flex h-8 items-center gap-1.5" role="radiogroup" aria-label={t('onboarding.cats.color')}>
                    {COLOR_CHOICES.slice(0, 8).map((col) => (
                      <button
                        key={col}
                        type="button"
                        role="radio"
                        aria-checked={c.color === col}
                        aria-label={col}
                        onClick={() => update(c.id, { color: col })}
                        className="size-5 rounded-full transition-[box-shadow] duration-120"
                        style={{ background: col, boxShadow: c.color === col ? `0 0 0 2px var(--panel), 0 0 0 3.5px ${col}` : undefined }}
                      />
                    ))}
                  </div>
                </div>
                <div className="flex min-w-0 flex-col gap-1.5">
                  <span className="text-xs text-ink-3">{t('onboarding.cats.icon')}</span>
                  <IconPicker value={c.icon} onChange={(name) => update(c.id, { icon: name })} color={c.color} className="w-[296px] max-w-full" />
                </div>
              </div>
              <Textarea value={c.description} onChange={(e) => update(c.id, { description: e.target.value })} placeholder={t('onboarding.cats.description_placeholder')} aria-label={t('onboarding.cats.description')} className="min-h-16" required />
            </div>
          );
        })}
        <Button onClick={add} className="self-start" icon={<Plus className="size-4" strokeWidth={1.75} />}>
          {t('onboarding.cats.add')}
        </Button>
      </div>
    </>
  );
}

function TimesStep({ draft, patch }: { draft: Settings; patch: (p: Partial<Settings>) => void }) {
  const t = useT();
  return (
    <>
      <StepTitle title={t('onboarding.times.title')}>{t('onboarding.times.lead')}</StepTitle>
      <div className="panel flex flex-col gap-5 p-5">
        <Field label={t('onboarding.times.report_time')} hint={t('onboarding.times.report_time_hint')}>
          {(id) => <Input id={id} type="time" value={hhmmToInput(draft.report_default_time)} onChange={(e) => e.target.value && patch({ report_default_time: inputToHhmm(e.target.value) })} className="num w-36" />}
        </Field>
        <hr className="border-line" />
        <Field label={t('onboarding.times.quiet_hours')} hint={t('onboarding.times.quiet_hours_hint')} inline>
          {(id) => <Toggle id={id} checked={draft.quiet_hours.enabled} onChange={(v) => patch({ quiet_hours: { ...draft.quiet_hours, enabled: v } })} />}
        </Field>
        <div className="grid grid-cols-2 gap-4">
          <Field label={t('onboarding.times.from')}>{(id) => <Input id={id} type="time" className="num" disabled={!draft.quiet_hours.enabled} value={hhmmToInput(draft.quiet_hours.start)} onChange={(e) => e.target.value && patch({ quiet_hours: { ...draft.quiet_hours, start: inputToHhmm(e.target.value) } })} />}</Field>
          <Field label={t('onboarding.times.to')}>{(id) => <Input id={id} type="time" className="num" disabled={!draft.quiet_hours.enabled} value={hhmmToInput(draft.quiet_hours.end)} onChange={(e) => e.target.value && patch({ quiet_hours: { ...draft.quiet_hours, end: inputToHhmm(e.target.value) } })} />}</Field>
        </div>
      </div>
    </>
  );
}

function VisionStep({ draft, patch }: { draft: Settings; patch: (p: Partial<Settings>) => void }) {
  const t = useT();
  const mode = draft.vision_policy.mode;
  const set = (p: VisionPolicy) => patch({ vision_policy: p });
  const options: { value: VisionPolicy['mode']; title: string; text: string; recommended?: boolean }[] = [
    { value: 'all_except_blocked', title: t('onboarding.vision.all_title'), text: t('onboarding.vision.all_text'), recommended: true },
    { value: 'only_apps', title: t('onboarding.vision.only_apps_title'), text: t('onboarding.vision.only_apps_text') },
    { value: 'never', title: t('onboarding.vision.never_title'), text: t('onboarding.vision.never_text') },
  ];
  return (
    <>
      <StepTitle title={t('onboarding.vision.title')}>{t('onboarding.vision.lead')}</StepTitle>
      <div className="flex flex-col gap-2" role="radiogroup" aria-label={t('onboarding.vision.policy_group')}>
        {options.map((o) => {
          const on = mode === o.value;
          return (
            <label key={o.value} className={clsx('flex cursor-pointer items-start gap-3 rounded-card border p-4 transition-colors duration-150', on ? 'border-volt/60 bg-volt-soft' : 'border-line bg-panel hover:bg-panel-2')}>
              <input type="radio" name="vision" className="mt-1 accent-[var(--volt)]" checked={on} onChange={() => set(o.value === 'only_apps' ? { mode: 'only_apps', apps: draft.vision_policy.mode === 'only_apps' ? draft.vision_policy.apps : ['Google Chrome'] } : { mode: o.value })} />
              <span className="min-w-0">
                <span className="flex flex-wrap items-center gap-2 text-sm font-medium">
                  {o.title}
                  {o.recommended && <span className="rounded-pill border border-volt/30 bg-volt-soft px-1.5 py-px text-[11px] font-medium text-volt">{t('onboarding.vision.recommended')}</span>}
                </span>
                <span className="mt-0.5 block text-sm leading-6 text-ink-2">{o.text}</span>
              </span>
            </label>
          );
        })}
      </div>
      {mode === 'only_apps' && draft.vision_policy.mode === 'only_apps' && (
        <div className="mt-3">
          <Field label={t('onboarding.vision.apps_label')} hint={t('onboarding.vision.apps_hint')}>
            {(id) => <Input id={id} value={draft.vision_policy.mode === 'only_apps' ? draft.vision_policy.apps.join(', ') : ''} onChange={(e) => set({ mode: 'only_apps', apps: e.target.value.split(',').map((s) => s.trim()).filter(Boolean) })} />}
          </Field>
        </div>
      )}
    </>
  );
}

function FinishStep({ draft, patch, view }: { draft: Settings; patch: (p: Partial<Settings>) => void; view: SettingsView }) {
  const t = useT();
  const info = providerInfo(view, draft.ai_provider);
  const key = keyStatus(view, draft.ai_provider);
  const places = [
    { title: t('onboarding.finish.today_title'), text: t('onboarding.finish.today_text') },
    { title: t('onboarding.finish.review_title'), text: t('onboarding.finish.review_text') },
    { title: t('onboarding.finish.reports_title'), text: t('onboarding.finish.reports_text') },
  ];
  return (
    <>
      <StepTitle title={t('onboarding.finish.title')}>{t('onboarding.finish.lead')}</StepTitle>
      <div className="panel-raised mb-4 flex items-start gap-3 px-4 py-3 text-sm" data-testid="finish-ai-summary">
        <BrainCircuit className="mt-0.5 size-4 shrink-0 text-volt" strokeWidth={1.75} aria-hidden />
        <div className="min-w-0 leading-6">
          <span className="font-medium">{t('onboarding.finish.ai_label', { provider: info?.label ?? draft.ai_provider })}</span>
          <span className="block text-xs leading-5 text-ink-3">
            {richText(t('onboarding.finish.models'), {
              classify: <span className="font-mono text-ink-2">{draft.models.classify}</span>,
              report: <span className="font-mono text-ink-2">{draft.models.report}</span>,
            })}
          </span>
          {key.configured ? (
            <span className="mt-1 flex items-center gap-1.5 text-xs text-signal">
              <KeyRound className="size-3" strokeWidth={1.75} aria-hidden />
              <span>{t('onboarding.finish.key_configured', { hint: key.hint ?? '' }).trim()}</span>
            </span>
          ) : (
            <span className="mt-1 block text-xs text-amber">{t('onboarding.finish.no_key')}</span>
          )}
        </div>
      </div>
      <div className="panel flex flex-col gap-5 p-5">
        <Field label={t('onboarding.finish.launch_at_login')} hint={t('onboarding.finish.launch_at_login_hint')} inline>
          {(id) => <Toggle id={id} checked={draft.launch_at_login} onChange={(v) => patch({ launch_at_login: v })} />}
        </Field>
        <Field label={t('onboarding.finish.tracking')} hint={t('onboarding.finish.tracking_hint')} inline>
          {(id) => <Toggle id={id} checked={draft.tracking_enabled} onChange={(v) => patch({ tracking_enabled: v })} />}
        </Field>
      </div>
      <p className="eyebrow mt-6 mb-2">{t('onboarding.finish.what_next')}</p>
      <ul className="divide-y divide-line border-y border-line text-sm">
        {places.map((p) => (
          <li key={p.title} className="grid grid-cols-[100px_1fr] gap-4 py-2.5">
            <span className="font-medium text-ink">{p.title}</span>
            <span className="text-ink-2">{p.text}</span>
          </li>
        ))}
      </ul>
    </>
  );
}
