import clsx from 'clsx';
import { ArrowLeft, ArrowRight, BrainCircuit, Check, Eye, Info, KeyRound, Lock, Rocket, Shield, ShieldCheck, Tags, Timer } from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';
import { Ubi } from '../components/ubi/Ubi';
import { Button } from '../components/ui/Button';
import { Field, Input, Textarea } from '../components/ui/Field';
import { Toggle } from '../components/ui/Toggle';
import { COLOR_CHOICES, ICON_CHOICES, iconFor, newCategoryId } from '../lib/categories';
import { hhmmToInput, inputToHhmm } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { Toaster, useToast } from '../lib/toast';
import { PROVIDER_PITCH, keyStatus, providerInfo } from '../lib/providers';
import type { AiProvider, Category, Mood, Settings, SettingsView, VisionPolicy } from '../lib/types';
import { ApiKeyForm, PermissionRows, providerSwitchPatch } from './Settings';

const STEPS = [
  { id: 'intro', title: 'Como funciona', Icon: Shield },
  { id: 'ai', title: 'Escolha sua IA', Icon: BrainCircuit },
  { id: 'perms', title: 'Permissões', Icon: ShieldCheck },
  { id: 'cats', title: 'Categorias', Icon: Tags },
  { id: 'times', title: 'Horários', Icon: Timer },
  { id: 'vision', title: 'Análise visual', Icon: Eye },
  { id: 'finish', title: 'Concluir', Icon: Rocket },
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

const SUGGESTED: Omit<CatDraft, 'id' | 'existing'>[] = [
  { name: 'IFRO', color: '#2563EB', icon: 'graduation-cap', description: 'Docência no IFRO: aulas, orientação de TCC, reuniões de colegiado, SEI/SUAP, editais e e-mails institucionais.' },
  { name: 'Incubadora', color: '#F97316', icon: 'rocket', description: 'Incubadora de startups: mentorias, desenvolvimento da API de inscrições, Demo Day e roadmap.' },
  { name: 'Cidades Inteligentes', color: '#10B981', icon: 'building-2', description: 'Projeto de pesquisa com a prefeitura: sensores IoT, ingestão de dados, dashboards e dados abertos.' },
];

export function Onboarding() {
  const settingsView = useAppStore((s) => s.settingsView);
  const saveSettings = useAppStore((s) => s.saveSettings);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const loadCategories = useAppStore((s) => s.loadCategories);
  const navigate = useNavigate();
  const toast = useToast();
  const [step, setStep] = useState(stepFromQuery);
  const [draft, setDraft] = useState<Settings | null>(settingsView?.settings ?? null);
  const [cats, setCats] = useState<CatDraft[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (settingsView && !draft) setDraft(settingsView.settings);
  }, [settingsView, draft]);

  useEffect(() => {
    void ipc.listCategories().then((list) => {
      const user = list.filter((c) => !c.is_system && !c.archived);
      if (user.length) setCats(user.map((c) => ({ id: c.id, existing: c, name: c.name, color: c.color, icon: c.icon, description: c.description })));
      else setCats(SUGGESTED.map((s) => ({ ...s, id: newCategoryId(), existing: null })));
    });
  }, []);

  const patch = (p: Partial<Settings>) => setDraft((d) => (d ? { ...d, ...p } : d));

  /** Picking a provider card persists it right away so the key form and the backend health follow. */
  const selectProvider = async (provider: AiProvider) => {
    if (!draft || !settingsView || provider === draft.ai_provider) return;
    const p = providerSwitchPatch(settingsView, draft, provider);
    patch(p);
    try {
      const v = await saveSettings(p);
      if (v) patch({ ai_provider: v.settings.ai_provider, models: v.settings.models });
    } catch (e) {
      toast.error('Não foi possível trocar o provedor', e instanceof Error ? e.message : String(e));
    }
  };

  const mood: Mood = useMemo(() => (step === 0 ? 'calm' : step === STEPS.length - 1 ? 'excited' : 'focused'), [step]);

  if (!draft || !settingsView) return null;

  const current = STEPS[step] ?? STEPS[0];

  const saveCats = async () => {
    const valid = cats.filter((c) => c.name.trim());
    if (valid.some((c) => !c.description.trim())) {
      toast.error('Descreva cada categoria', 'A descrição é o que a IA usa para decidir onde cada atividade entra.');
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
      toast.error('Não foi possível salvar as categorias', e instanceof Error ? e.message : String(e));
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
      toast.success('Tudo pronto!', 'O UBI começou a observar. Volte em algumas horas para ver o dia tomando forma.');
      navigate('/', { replace: true });
    } catch (e) {
      toast.error('Não foi possível concluir', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-canvas" data-testid="page-onboarding">
      {/* progress rail */}
      <aside className="flex w-[280px] shrink-0 flex-col border-r border-line bg-surface/70">
        <div data-tauri-drag-region className="h-[38px]" />
        <div className="flex flex-col items-center px-6 pt-2 pb-6">
          <Ubi mood={mood} size={120} variant="svg" />
          <p className="mt-2 text-lg font-semibold tracking-tight">
            ubiq<span className="text-brand-600">X</span>
          </p>
          <p className="text-xs text-ink-3">Configuração inicial</p>
        </div>
        <ol className="flex flex-col gap-1 px-4" aria-label="Etapas">
          {STEPS.map((s, i) => {
            const done = i < step;
            const active = i === step;
            return (
              <li key={s.id}>
                <button
                  type="button"
                  onClick={() => i < step && setStep(i)}
                  disabled={i > step}
                  aria-current={active ? 'step' : undefined}
                  className={clsx('flex w-full items-center gap-3 rounded-xl px-3 py-2 text-left text-sm transition-colors', active ? 'bg-brand-50 text-brand-700 dark:bg-brand-900/30 dark:text-brand-300' : done ? 'text-ink hover:bg-surface-2' : 'text-ink-3')}
                >
                  <span className={clsx('flex size-6 shrink-0 items-center justify-center rounded-full text-[11px] font-semibold', active ? 'bg-brand-600 text-white' : done ? 'bg-emerald-500 text-white' : 'bg-surface-3 text-ink-3')}>
                    {done ? <Check className="size-3.5" /> : i + 1}
                  </span>
                  {s.title}
                </button>
              </li>
            );
          })}
        </ol>
      </aside>

      {/* content */}
      <main className="flex min-w-0 flex-1 flex-col">
        <div data-tauri-drag-region className="h-[38px] shrink-0" />
        <div className="scroll-thin min-h-0 flex-1 overflow-y-auto px-10 pb-8">
          <div className="mx-auto w-full max-w-2xl">
            <p className="text-xs font-medium tracking-wide text-ink-3 uppercase">
              Passo {step + 1} de {STEPS.length}
            </p>
            {current.id === 'intro' && <IntroStep />}
            {current.id === 'ai' && <AiStep view={settingsView} draft={draft} onSelect={(p) => void selectProvider(p)} />}
            {current.id === 'perms' && <PermsStep view={settingsView} />}
            {current.id === 'cats' && <CatsStep cats={cats} setCats={setCats} />}
            {current.id === 'times' && <TimesStep draft={draft} patch={patch} />}
            {current.id === 'vision' && <VisionStep draft={draft} patch={patch} />}
            {current.id === 'finish' && <FinishStep draft={draft} patch={patch} view={settingsView} />}
          </div>
        </div>
        <div className="flex items-center justify-between border-t border-line bg-surface/70 px-10 py-4">
          <Button variant="ghost" icon={<ArrowLeft className="size-4" />} disabled={step === 0} onClick={() => setStep((s) => Math.max(0, s - 1))}>
            Voltar
          </Button>
          <div className="flex items-center gap-2">
            {current.id === 'ai' && (
              <Button variant="ghost" onClick={() => setStep((s) => s + 1)}>
                Pular por enquanto
              </Button>
            )}
            {current.id === 'finish' ? (
              <Button variant="primary" size="lg" icon={<Rocket className="size-4" />} loading={busy} onClick={() => void finish()}>
                Concluir
              </Button>
            ) : (
              <Button variant="primary" size="lg" loading={busy} onClick={() => void next()}>
                Continuar <ArrowRight className="size-4" />
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
      <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
      <p className="mt-1.5 text-sm leading-6 text-ink-2">{children}</p>
    </div>
  );
}

function IntroStep() {
  const items: { Icon: typeof Shield; title: string; text: string }[] = [
    { Icon: Timer, title: 'O que é registrado', text: 'A cada poucos segundos: nome do app, título da janela e, em navegadores, o domínio da aba ativa. Isso vira blocos de atividade (ex.: “45 min no SEI”).' },
    { Icon: Eye, title: 'Screenshots', text: 'Esparsos, só da janela ativa, reduzidos a 1024 px e apagados em 48 h. Servem para a IA entender blocos ambíguos (WhatsApp, Finder…). Você controla a política no passo 6.' },
    { Icon: Shield, title: 'O que vai para a IA escolhida (Anthropic, OpenAI ou xAI)', text: 'Apenas app + título + domínio dos blocos a classificar e, quando permitido, um screenshot reduzido. Nunca a URL completa, o conteúdo da página ou o que você digita.' },
    { Icon: Lock, title: 'Nada mais sai do seu Mac', text: 'Banco SQLite local, chave no Keychain, sem contas, sem telemetria. Apps bloqueados (bancos, 1Password) nunca são registrados e o Modo privado pausa tudo com um clique.' },
  ];
  return (
    <>
      <StepTitle title="Oi, eu sou o UBI.">Vou registrar o que você faz no computador, organizar por categoria e escrever seus relatórios. Antes, o combinado sobre privacidade:</StepTitle>
      <ul className="flex flex-col gap-3">
        {items.map(({ Icon, title, text }) => (
          <li key={title} className="card flex gap-4 p-4">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-xl bg-brand-50 text-brand-600 dark:bg-brand-900/30 dark:text-brand-300">
              <Icon className="size-4" />
            </span>
            <div>
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
  const selected = draft.ai_provider;
  const info = providerInfo(view, selected);
  return (
    <>
      <StepTitle title="Escolha sua IA">
        Quem vai ler os blocos ambíguos e escrever os seus relatórios. Cada provedor usa a própria chave de API, guardada no Keychain do macOS — dá para trocar depois em Configurações → IA. Sem chave, o ubiqX funciona só com regras e memória.
      </StepTitle>
      <div role="radiogroup" aria-label="Provedor de IA" className="grid grid-cols-3 gap-3" data-testid="provider-cards">
        {view.providers.map((p) => {
          const pitch = PROVIDER_PITCH[p.id];
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
              className={clsx('card flex h-full flex-col items-start gap-2 p-4 text-left transition-colors', active ? 'border-brand-500 ring-2 ring-brand-500/20' : 'hover:bg-surface-2')}
            >
              <span className="flex w-full items-center justify-between gap-2">
                <span className="text-base font-semibold tracking-tight">{pitch.short}</span>
                <span className={clsx('flex size-5 shrink-0 items-center justify-center rounded-full', active ? 'bg-brand-600 text-white' : 'bg-surface-3 text-ink-3')}>{active ? <Check className="size-3" /> : status.configured ? <KeyRound className="size-3" /> : null}</span>
              </span>
              <span className="-mt-1 text-xs text-ink-3">{p.label}</span>
              <span className="text-sm leading-5 text-ink-2">{pitch.pitch}</span>
              <span className="text-[11px] leading-4 text-ink-3">
                Recomendados: <span className="font-mono">{p.default_models.classify}</span> para classificar e <span className="font-mono">{p.default_models.report}</span> para relatórios.
              </span>
              <span className="mt-auto pt-1 text-xs">
                <strong className="text-ink">{pitch.cost}</strong> <span className="text-ink-3">· estimativa com 8 h/dia</span>
              </span>
              {status.configured && <span className="text-[11px] text-emerald-600 dark:text-emerald-400">Chave configurada {status.hint}</span>}
            </button>
          );
        })}
      </div>
      {view.providers
        .filter((p) => PROVIDER_PITCH[p.id].note)
        .map((p) => (
          <p key={p.id} className="mt-3 flex items-start gap-2 text-xs leading-5 text-ink-3">
            <Info className="mt-0.5 size-3.5 shrink-0" />
            <span>
              <strong className="font-medium text-ink-2">{PROVIDER_PITCH[p.id].short}:</strong> {PROVIDER_PITCH[p.id].note}
            </span>
          </p>
        ))}
      <div className="card mt-4 p-5">
        <ApiKeyForm view={view} provider={selected} compact />
        <p className="mt-3 text-xs text-ink-3">
          {info ? `Crie a chave em ${info.console_url.replace(/^https?:\/\//, '')} (link “Criar chave” acima). ` : ''}Ela fica no Keychain do macOS e pode ser trocada em Configurações → IA.
        </p>
      </div>
    </>
  );
}

function PermsStep({ view }: { view: SettingsView }) {
  return (
    <>
      <StepTitle title="Permissões do macOS">O sistema vai abrir Ajustes → Privacidade e Segurança. Marque o ubiqX na lista e volte aqui.</StepTitle>
      {view.platform === 'macos' ? (
        <>
          <PermissionRows view={view} />
          <div className="mt-4 rounded-xl border border-amber-200 bg-amber-50 p-3 text-xs leading-5 text-amber-900 dark:border-amber-900/50 dark:bg-amber-900/20 dark:text-amber-200">
            <strong>Importante:</strong> a Gravação de Tela só passa a valer depois de <strong>reiniciar o ubiqX</strong>. Se os títulos das janelas aparecerem vazios na Timeline, use “Reiniciar o ubiqX” em Configurações → Permissões.
            A Automação é pedida pelo macOS separadamente para cada navegador (Chrome, Safari, Arc…) na primeira vez que a URL for lida.
          </div>
        </>
      ) : (
        <div className="card p-5 text-sm text-ink-2">Fora do macOS não há permissões a conceder — este é o modo de desenvolvimento com dados simulados.</div>
      )}
    </>
  );
}

function CatsStep({ cats, setCats }: { cats: CatDraft[]; setCats: (c: CatDraft[]) => void }) {
  const update = (id: string, p: Partial<CatDraft>) => setCats(cats.map((c) => (c.id === id ? { ...c, ...p } : c)));
  const remove = (id: string) => setCats(cats.filter((c) => c.id !== id));
  const add = () => setCats([...cats, { id: newCategoryId(), existing: null, name: '', color: COLOR_CHOICES[cats.length % COLOR_CHOICES.length] ?? '#2563EB', icon: 'briefcase', description: '' }]);
  return (
    <>
      <StepTitle title="Suas categorias de trabalho">
        Sugeri três a partir do seu perfil — edite à vontade. A <strong>descrição é obrigatória</strong>: é ela que a IA lê para decidir onde cada atividade entra. Quanto mais específica (sistemas, projetos, pessoas), melhor.
      </StepTitle>
      <div className="flex flex-col gap-3">
        {cats.map((c) => {
          const Icon = iconFor(c.icon);
          return (
            <div key={c.id} className="card flex flex-col gap-3 p-4">
              <div className="flex items-center gap-3">
                <span className="flex size-10 shrink-0 items-center justify-center rounded-xl" style={{ background: `${c.color}22`, color: c.color }}>
                  <Icon className="size-5" />
                </span>
                <Input value={c.name} onChange={(e) => update(c.id, { name: e.target.value })} placeholder="Nome da categoria" aria-label="Nome da categoria" className="max-w-xs font-medium" />
                <div className="ml-auto flex items-center gap-1" role="radiogroup" aria-label="Cor">
                  {COLOR_CHOICES.slice(0, 8).map((col) => (
                    <button key={col} type="button" role="radio" aria-checked={c.color === col} aria-label={col} onClick={() => update(c.id, { color: col })} className={clsx('size-5 rounded-full border-2', c.color === col ? 'border-ink' : 'border-transparent')} style={{ background: col }} />
                  ))}
                </div>
                <select aria-label="Ícone" value={c.icon} onChange={(e) => update(c.id, { icon: e.target.value })} className="h-8 rounded-lg border border-line-strong bg-surface px-2 text-xs">
                  {ICON_CHOICES.map((i) => (
                    <option key={i.name} value={i.name}>
                      {i.name}
                    </option>
                  ))}
                </select>
                <Button size="sm" variant="ghost" onClick={() => remove(c.id)} aria-label={`Remover ${c.name || 'categoria'}`}>
                  Remover
                </Button>
              </div>
              <Textarea value={c.description} onChange={(e) => update(c.id, { description: e.target.value })} placeholder="O que conta como trabalho desta categoria? (obrigatório)" aria-label="Descrição" className="min-h-16" required />
            </div>
          );
        })}
        <Button onClick={add} className="self-start">
          + Adicionar categoria
        </Button>
      </div>
    </>
  );
}

function TimesStep({ draft, patch }: { draft: Settings; patch: (p: Partial<Settings>) => void }) {
  return (
    <>
      <StepTitle title="Horários">Quando gerar os relatórios diários e quando o UBI deve ficar em silêncio.</StepTitle>
      <div className="card flex flex-col gap-5 p-5">
        <Field label="Horário dos relatórios" hint="Cada categoria pode ter o seu; este é o padrão. O relatório usa tudo que foi registrado até esse horário e pode ser regenerado depois.">
          {(id) => <Input id={id} type="time" value={hhmmToInput(draft.report_default_time)} onChange={(e) => e.target.value && patch({ report_default_time: inputToHhmm(e.target.value) })} className="w-36" />}
        </Field>
        <Field label="Horário silencioso" hint="Sem avisos do UBI e sem notificações neste período." inline>
          {(id) => <Toggle id={id} checked={draft.quiet_hours.enabled} onChange={(v) => patch({ quiet_hours: { ...draft.quiet_hours, enabled: v } })} />}
        </Field>
        <div className="grid grid-cols-2 gap-4">
          <Field label="Das">{(id) => <Input id={id} type="time" disabled={!draft.quiet_hours.enabled} value={hhmmToInput(draft.quiet_hours.start)} onChange={(e) => e.target.value && patch({ quiet_hours: { ...draft.quiet_hours, start: inputToHhmm(e.target.value) } })} />}</Field>
          <Field label="Até">{(id) => <Input id={id} type="time" disabled={!draft.quiet_hours.enabled} value={hhmmToInput(draft.quiet_hours.end)} onChange={(e) => e.target.value && patch({ quiet_hours: { ...draft.quiet_hours, end: inputToHhmm(e.target.value) } })} />}</Field>
        </div>
      </div>
    </>
  );
}

function VisionStep({ draft, patch }: { draft: Settings; patch: (p: Partial<Settings>) => void }) {
  const mode = draft.vision_policy.mode;
  const set = (p: VisionPolicy) => patch({ vision_policy: p });
  const options: { value: VisionPolicy['mode']; title: string; text: string }[] = [
    { value: 'all_except_blocked', title: 'Todos os apps, exceto bloqueados (recomendado)', text: 'Quando um bloco é ambíguo, um screenshot reduzido da janela ativa vai para a IA — exceto em apps bloqueados ou negados (bancos, senhas). Máximo de 6 imagens por hora.' },
    { value: 'only_apps', title: 'Apenas em apps que eu escolher', text: 'Você lista os apps (ex.: Google Chrome, Preview). Nos demais, só texto.' },
    { value: 'never', title: 'Nunca enviar imagens', text: 'Só app, título e domínio vão para a IA. Blocos ambíguos ficam para a sua revisão manual.' },
  ];
  return (
    <>
      <StepTitle title="Análise visual">Screenshots ajudam a IA a descrever o que você fez em apps genéricos (“respondeu mensagens sobre o edital X”). Escolha até onde ela pode ir.</StepTitle>
      <div className="flex flex-col gap-2" role="radiogroup" aria-label="Política de análise visual">
        {options.map((o) => (
          <label key={o.value} className={clsx('card flex cursor-pointer items-start gap-3 p-4 transition-colors', mode === o.value ? 'border-brand-500 ring-2 ring-brand-500/20' : 'hover:bg-surface-2')}>
            <input type="radio" name="vision" className="mt-1 accent-brand-600" checked={mode === o.value} onChange={() => set(o.value === 'only_apps' ? { mode: 'only_apps', apps: draft.vision_policy.mode === 'only_apps' ? draft.vision_policy.apps : ['Google Chrome'] } : { mode: o.value })} />
            <span>
              <span className="block text-sm font-medium">{o.title}</span>
              <span className="mt-0.5 block text-sm leading-6 text-ink-2">{o.text}</span>
            </span>
          </label>
        ))}
      </div>
      {mode === 'only_apps' && draft.vision_policy.mode === 'only_apps' && (
        <div className="mt-3">
          <Field label="Apps com análise visual (separe por vírgula)">
            {(id) => <Input id={id} value={draft.vision_policy.mode === 'only_apps' ? draft.vision_policy.apps.join(', ') : ''} onChange={(e) => set({ mode: 'only_apps', apps: e.target.value.split(',').map((s) => s.trim()).filter(Boolean) })} />}
          </Field>
        </div>
      )}
    </>
  );
}

function FinishStep({ draft, patch, view }: { draft: Settings; patch: (p: Partial<Settings>) => void; view: SettingsView }) {
  const info = providerInfo(view, draft.ai_provider);
  const key = keyStatus(view, draft.ai_provider);
  return (
    <>
      <StepTitle title="Quase lá">O UBI vai morar na barra de menus. Deixe-o iniciar com o sistema para não perder nenhum dia.</StepTitle>
      <div className="mb-4 flex items-center gap-3 rounded-xl border border-line bg-surface-2/60 px-4 py-3 text-sm" data-testid="finish-ai-summary">
        <BrainCircuit className="size-4 shrink-0 text-brand-600" />
        <span className="min-w-0">
          <span className="font-medium">IA: {info?.label ?? draft.ai_provider}</span>
          <span className="text-ink-3">
            {' '}
            · <span className="font-mono">{draft.models.classify}</span> para classificar, <span className="font-mono">{draft.models.report}</span> para relatórios ·{' '}
          </span>
          {key.configured ? <span className="text-emerald-600 dark:text-emerald-400">chave configurada {key.hint}</span> : <span className="text-amber-600 dark:text-amber-400">sem chave — só regras e memória até configurar em Configurações → IA</span>}
        </span>
      </div>
      <div className="card flex flex-col gap-5 p-5">
        <Field label="Iniciar com o sistema" hint="Abre o ubiqX na barra de menus ao fazer login. Pode mudar em Configurações → Rastreamento." inline>
          {(id) => <Toggle id={id} checked={draft.launch_at_login} onChange={(v) => patch({ launch_at_login: v })} />}
        </Field>
        <Field label="Rastreamento ativo" hint="Começar a registrar assim que concluir." inline>
          {(id) => <Toggle id={id} checked={draft.tracking_enabled} onChange={(v) => patch({ tracking_enabled: v })} />}
        </Field>
      </div>
      <ul className="mt-6 grid grid-cols-3 gap-3 text-xs text-ink-2">
        <li className="card p-3">
          <p className="font-medium text-ink">Hoje</p>
          Score de foco, tempo por categoria e o UBI com dicas.
        </li>
        <li className="card p-3">
          <p className="font-medium text-ink">Revisão</p>
          Corrija classificações em um clique — o UBI aprende com cada correção.
        </li>
        <li className="card p-3">
          <p className="font-medium text-ink">Relatórios</p>
          Diários por categoria, mensais em Markdown para o SEI ou o edital.
        </li>
      </ul>
    </>
  );
}
