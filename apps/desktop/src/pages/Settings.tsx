import clsx from 'clsx';
import { AlertTriangle, Bell, BrainCircuit, Camera, Check, Download, ExternalLink, Info, KeyRound, ListRestart, Loader2, RefreshCw, Shield, ShieldCheck, Trash2, type LucideIcon } from 'lucide-react';
import { useCallback, useEffect, useId, useMemo, useRef, useState } from 'react';
import { Badge, StatusPill } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { Card, CardHeader } from '../components/ui/Card';
import { Dialog } from '../components/ui/Dialog';
import { Field, Input, Textarea } from '../components/ui/Field';
import { EmptyState } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { TagInput } from '../components/ui/TagInput';
import { Toggle } from '../components/ui/Toggle';
import { fmtDateTime, fmtTime, hhmmToInput, inputToHhmm } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import { PROVIDER_PITCH, keyStatus, providerInfo, reconcileModels, sameModels } from '../lib/providers';
import type { AiModels, AiProvider, PermissionKind, PermissionState, Settings, SettingsView, VisionPolicy } from '../lib/types';
import { useAsync } from '../lib/useAsync';

export const REPO_URL = 'https://github.com/andreyquadros/ubiquitous-engine';

const SECTIONS: { id: string; label: string; Icon: LucideIcon }[] = [
  { id: 'ia', label: 'IA', Icon: BrainCircuit },
  { id: 'rastreamento', label: 'Rastreamento', Icon: RefreshCw },
  { id: 'privacidade', label: 'Privacidade', Icon: Shield },
  { id: 'relatorios', label: 'Relatórios', Icon: Info },
  { id: 'ubi', label: 'UBI & notificações', Icon: Bell },
  { id: 'permissoes', label: 'Permissões (macOS)', Icon: ShieldCheck },
  { id: 'sobre', label: 'Sobre', Icon: Info },
];

/** Local draft of the settings with debounced persistence. */
function useSettingsDraft() {
  const settingsView = useAppStore((s) => s.settingsView);
  const saveSettings = useAppStore((s) => s.saveSettings);
  const [draft, setDraft] = useState<Settings | null>(settingsView?.settings ?? null);
  const [status, setStatus] = useState<'idle' | 'saving' | 'saved' | 'error'>('idle');
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pending = useRef<Partial<Settings>>({});
  const toast = useToast();

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
          toast.error('Não foi possível salvar', e instanceof Error ? e.message : String(e));
        }
      }, 500);
    },
    [saveSettings, toast],
  );

  return { draft, patch, status, settingsView };
}

export function SettingsPage() {
  const { draft, patch, status, settingsView } = useSettingsDraft();
  const [active, setActive] = useState('ia');

  if (!draft || !settingsView) return null;

  const go = (id: string) => {
    setActive(id);
    document.getElementById(`sec-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  };

  return (
    <div data-testid="page-settings">
      <PageHeader
        title="Configurações"
        subtitle="Tudo é salvo automaticamente."
        actions={
          <span className="flex items-center gap-1.5 text-xs text-ink-3" aria-live="polite">
            {status === 'saving' && (
              <>
                <Loader2 className="size-3.5 animate-spin" /> Salvando…
              </>
            )}
            {status === 'saved' && (
              <>
                <Check className="size-3.5 text-emerald-500" /> Salvo
              </>
            )}
            {status === 'error' && (
              <>
                <AlertTriangle className="size-3.5 text-red-500" /> Erro ao salvar
              </>
            )}
          </span>
        }
      />
      <div className="grid grid-cols-12 gap-6">
        <nav className="col-span-12 min-[1100px]:sticky min-[1100px]:top-0 min-[1100px]:col-span-3 min-[1100px]:self-start" aria-label="Seções">
          <ul className="flex flex-row flex-wrap gap-1 min-[1100px]:flex-col">
            {SECTIONS.filter((s) => s.id !== 'permissoes' || settingsView.platform === 'macos').map(({ id, label, Icon }) => (
              <li key={id}>
                <button
                  type="button"
                  onClick={() => go(id)}
                  className={clsx('flex w-full items-center gap-2.5 rounded-xl px-3 py-2 text-left text-sm transition-colors', active === id ? 'bg-surface text-ink shadow-soft' : 'text-ink-2 hover:bg-surface-2 hover:text-ink')}
                >
                  <Icon className="size-4 shrink-0" />
                  {label}
                </button>
              </li>
            ))}
          </ul>
        </nav>
        <div className="col-span-12 flex flex-col gap-6 min-[1100px]:col-span-9">
          <AiSection draft={draft} patch={patch} view={settingsView} />
          <TrackingSection draft={draft} patch={patch} />
          <PrivacySection draft={draft} patch={patch} />
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

function Section({ id, title, description, children }: { id: string; title: string; description?: string; children: React.ReactNode }) {
  return (
    <Card id={`sec-${id}`} className="scroll-mt-4">
      <CardHeader title={<span className="text-base">{title}</span>} subtitle={description} />
      <div className="flex flex-col gap-5">{children}</div>
    </Card>
  );
}

function NumberField({ label, hint, value, onChange, min, max, step, suffix }: { label: string; hint?: string; value: number; onChange: (v: number) => void; min?: number; max?: number; step?: number; suffix?: string }) {
  return (
    <Field label={label} hint={hint}>
      {(id) => (
        <div className="flex items-center gap-2">
          <Input id={id} type="number" value={value} min={min} max={max} step={step} onChange={(e) => onChange(Number(e.target.value))} className="w-28 tabular-nums" />
          {suffix && <span className="text-xs text-ink-3">{suffix}</span>}
        </div>
      )}
    </Field>
  );
}

export function ApiKeyForm({ view, provider, onSaved, compact }: { view: SettingsView; provider: AiProvider; onSaved?: () => void; compact?: boolean }) {
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
        toast.success(value ? `Chave da ${label} salva no Keychain` : `Chave da ${label} removida`);
      }
    } catch (e) {
      setResult({ valid: false, message: e instanceof Error ? e.message : String(e) });
    } finally {
      setBusy(false);
    }
  };

  const openConsole = () => {
    if (info) ipc.openExternal(info.console_url).catch((e: unknown) => toast.error('Não foi possível abrir', e instanceof Error ? e.message : String(e)));
  };

  return (
    <div className="flex flex-col gap-3" data-testid={`api-key-form-${provider}`}>
      {status.configured && (
        <div className="flex items-center gap-2 text-sm">
          <KeyRound className="size-4 text-emerald-500" />
          Chave configurada <span className="font-mono text-ink-2">{status.hint ?? '…'}</span>
          <Button size="sm" variant="ghost" onClick={() => void submit(null)} loading={busy}>
            Remover
          </Button>
        </div>
      )}
      <Field
        label={status.configured ? `Trocar chave de API da ${label}` : `Chave de API da ${label}`}
        hint={
          <span className="inline-flex flex-wrap items-center gap-x-1">
            {!compact && <span>Armazenada apenas no Keychain do macOS. Nunca é gravada em arquivo.</span>}
            {info && (
              <button type="button" onClick={openConsole} className="inline-flex items-center gap-1 text-brand-600 underline underline-offset-2 hover:text-brand-700 dark:text-brand-400">
                Criar chave <ExternalLink className="size-3" />
              </button>
            )}
          </span>
        }
      >
        {(id) => (
          <div className="flex gap-2">
            <Input id={id} type="password" autoComplete="off" value={key} onChange={(e) => setKey(e.target.value)} placeholder={`${info?.key_prefix ?? ''}…`} className="font-mono" />
            <Button variant="primary" className="shrink-0 whitespace-nowrap" onClick={() => void submit(key.trim())} disabled={!key.trim()} loading={busy}>
              Validar e salvar
            </Button>
          </div>
        )}
      </Field>
      {result && (
        <p className={clsx('flex items-center gap-1.5 text-xs', result.valid ? 'text-emerald-600 dark:text-emerald-400' : 'text-red-600 dark:text-red-400')} role="status">
          {result.valid ? <Check className="size-3.5" /> : <AlertTriangle className="size-3.5" />}
          {result.message}
        </p>
      )}
    </div>
  );
}

/** Segmented control over the providers the backend knows; shows a key icon on the ones with a stored key. */
export function ProviderPicker({ view, value, onChange, size = 'md' }: { view: SettingsView; value: AiProvider; onChange: (p: AiProvider) => void; size?: 'md' | 'lg' }) {
  return (
    <div role="radiogroup" aria-label="Provedor de IA" className="inline-flex flex-wrap items-center gap-0.5 rounded-xl bg-surface-2 p-1">
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
            className={clsx('inline-flex items-center gap-1.5 rounded-lg px-3 font-medium transition-colors', size === 'lg' ? 'h-9 text-sm' : 'h-8 text-sm', active ? 'bg-surface text-ink shadow-sm' : 'text-ink-2 hover:text-ink')}
          >
            {p.label}
            {configured && <KeyRound className="size-3.5 text-emerald-500" aria-label="chave configurada" />}
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

const MODEL_SLOTS: { key: keyof AiModels; label: string; hint: string }[] = [
  { key: 'classify', label: 'Modelo de classificação', hint: 'Chamado a cada lote de blocos; escolha o mais barato.' },
  { key: 'vision', label: 'Modelo de visão', hint: 'Precisa aceitar imagens.' },
  { key: 'report', label: 'Modelo de relatórios', hint: 'Escreve os relatórios e as recomendações.' },
];

/** The three model fields as free text with a datalist of the account's models. */
function ModelFields({ draft, patch, view }: SectionProps & { view: SettingsView }) {
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
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const options = models?.provider === provider ? models.ids : [];
  const isDefault = info ? sameModels(draft.models, info.default_models) : true;

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-[13px] font-medium">Modelos</span>
        <span className="text-xs text-ink-3">{info?.label ?? provider}</span>
        <div className="ml-auto flex items-center gap-2">
          <Button size="sm" icon={<RefreshCw className="size-3.5" />} onClick={() => void listModels()} loading={loading} disabled={!hasKey} title={hasKey ? undefined : 'Salve a chave deste provedor para listar os modelos'}>
            Listar modelos da conta
          </Button>
          <Button size="sm" variant="ghost" icon={<ListRestart className="size-3.5" />} disabled={!info || isDefault} onClick={() => info && patch({ models: { ...info.default_models } })}>
            Padrões do provedor
          </Button>
        </div>
      </div>
      {error && (
        <p className="flex items-center gap-1.5 text-xs text-red-600 dark:text-red-400" role="status">
          <AlertTriangle className="size-3.5" /> Não foi possível listar os modelos: {error}
        </p>
      )}
      {models?.provider === provider && !error && (
        <p className="text-xs text-ink-3" role="status">
          {models.ids.length} modelos disponíveis na conta — digite ou escolha nos campos abaixo.
        </p>
      )}
      <datalist id={listId}>
        {options.map((m) => (
          <option key={m} value={m} />
        ))}
      </datalist>
      <div className="grid grid-cols-3 gap-4">
        {MODEL_SLOTS.map((slot) => (
          <Field key={slot.key} label={slot.label} hint={slot.hint}>
            {(id) => <Input id={id} list={listId} value={draft.models[slot.key]} spellCheck={false} autoComplete="off" placeholder={info?.default_models[slot.key]} onChange={(e) => patch({ models: { ...draft.models, [slot.key]: e.target.value } })} className="font-mono text-xs" />}
          </Field>
        ))}
      </div>
    </div>
  );
}

function AiSection({ draft, patch, view }: SectionProps & { view: SettingsView }) {
  const health = view.ai_health;
  const pill =
    health.state === 'ok'
      ? { color: '#10b981', text: 'IA ativa' }
      : health.state === 'not_configured'
        ? { color: '#94a3b8', text: 'Não configurada' }
        : health.state === 'paused'
          ? { color: '#f59e0b', text: `Pausada — ${health.reason}` }
          : { color: '#ef4444', text: `Instável — ${health.reason} (até ${fmtTime(health.until)})` };

  return (
    <Section id="ia" title="IA" description="Classificação, análise visual e relatórios usam a API do provedor escolhido com a sua chave. Você pode trocar de provedor a qualquer momento; cada um guarda a própria chave.">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <StatusPill color={pill.color}>{pill.text}</StatusPill>
        <span className="text-xs text-ink-3">{PROVIDER_PITCH[draft.ai_provider].cost} estimados com 8 h/dia</span>
      </div>
      <Field label="Provedor de IA" hint="O ícone de chave indica os provedores que já têm chave salva.">
        {() => <ProviderPicker view={view} value={draft.ai_provider} onChange={(p) => patch(providerSwitchPatch(view, draft, p))} />}
      </Field>
      <ApiKeyForm view={view} provider={draft.ai_provider} />
      <ModelFields draft={draft} patch={patch} view={view} />
      <div className="grid grid-cols-2 gap-4">
        <NumberField label="Orçamento mensal" hint="Ao atingir, a IA pausa até o próximo mês (a classificação local continua)." value={draft.ai_monthly_budget_usd} min={0} step={0.5} onChange={(v) => patch({ ai_monthly_budget_usd: v })} suffix="US$/mês" />
        <NumberField label="Máximo de imagens por hora" hint="Limita quantos screenshots vão para o modelo de visão." value={draft.max_vision_per_hour} min={0} max={60} onChange={(v) => patch({ max_vision_per_hour: v })} suffix="imagens/h" />
      </div>
      <Field label="Somente local" hint="Nada é enviado à API: só regras e memória classificam. Relatórios e recomendações ficam indisponíveis." inline>
        {(id) => <Toggle id={id} checked={draft.local_only} onChange={(v) => patch({ local_only: v })} />}
      </Field>
    </Section>
  );
}

function TrackingSection({ draft, patch }: SectionProps) {
  const setTrackerState = useAppStore((s) => s.setTrackerState);
  const toast = useToast();
  const toggleTracking = async (v: boolean) => {
    patch({ tracking_enabled: v });
    try {
      await ipc.setTracking(v);
      setTrackerState(v ? 'running' : 'paused');
    } catch (e) {
      toast.error('Não foi possível alterar o rastreamento', e instanceof Error ? e.message : String(e));
    }
  };
  return (
    <Section id="rastreamento" title="Rastreamento" description="Como o ubiqX observa o app ativo.">
      <Field label="Rastreamento ativo" hint="Desligue para pausar completamente (o ícone na barra de menus mostra o estado)." inline>
        {(id) => <Toggle id={id} checked={draft.tracking_enabled} onChange={(v) => void toggleTracking(v)} />}
      </Field>
      <div className="grid grid-cols-3 gap-4">
        <NumberField label="Intervalo de amostragem" value={draft.sample_interval_secs} min={1} max={60} onChange={(v) => patch({ sample_interval_secs: v })} suffix="s" />
        <NumberField label="Limiar de ociosidade" hint="Sem teclado/mouse por esse tempo = ocioso." value={draft.idle_threshold_secs} min={30} step={30} onChange={(v) => patch({ idle_threshold_secs: v })} suffix="s" />
        <NumberField label="Bloco mínimo" hint="Blocos menores são mesclados ao vizinho." value={draft.min_block_secs} min={5} onChange={(v) => patch({ min_block_secs: v })} suffix="s" />
      </div>
      <Field label="Iniciar com o sistema" hint="Abre o ubiqX na barra de menus ao fazer login." inline>
        {(id) => <Toggle id={id} checked={draft.launch_at_login} onChange={(v) => patch({ launch_at_login: v })} />}
      </Field>
    </Section>
  );
}

function PrivacySection({ draft, patch }: SectionProps) {
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [aiSentOpen, setAiSentOpen] = useState(false);
  const [busy, setBusy] = useState<'export' | 'delete' | null>(null);
  const bumpData = useAppStore((s) => s.bumpData);
  const toast = useToast();
  const mode = draft.vision_policy.mode;
  const onlyApps = draft.vision_policy.mode === 'only_apps' ? draft.vision_policy.apps : [];

  const setPolicy = (p: VisionPolicy) => patch({ vision_policy: p });

  const exportData = async () => {
    setBusy('export');
    try {
      const path = await ipc.exportData();
      toast.success('Dados exportados', path);
    } catch (e) {
      toast.error('Falha ao exportar', e instanceof Error ? e.message : String(e));
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
      toast.success('Todos os dados foram apagados');
    } catch (e) {
      toast.error('Falha ao apagar', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Section id="privacidade" title="Privacidade" description="Tudo fica no seu Mac. Só sai o mínimo necessário para a IA classificar e escrever relatórios.">
      <div className="rounded-xl border border-brand-100 bg-brand-50/60 p-4 text-sm leading-6 text-ink-2 dark:border-brand-900/50 dark:bg-brand-900/15">
        <p className="mb-1 flex items-center gap-2 font-medium text-ink">
          <Shield className="size-4 text-brand-600" /> O que sai da sua máquina
        </p>
        <p>
          Para a IA escolhida (Anthropic, OpenAI ou xAI) vão apenas: <strong>nome do app, título da janela, domínio</strong> (nunca a URL completa nem o conteúdo da página) e, quando a
          política visual permite, um <strong>screenshot reduzido da janela ativa</strong> em blocos ambíguos. Apps e domínios bloqueados nunca são registrados.
          Nada mais — sem telemetria, sem sincronização.
        </p>
        <button type="button" onClick={() => setAiSentOpen(true)} className="mt-2 inline-flex items-center gap-1 text-brand-600 underline underline-offset-2 hover:text-brand-700 dark:text-brand-400">
          Ver dados enviados à IA hoje <ExternalLink className="size-3" />
        </button>
      </div>

      <div className="flex flex-col gap-2">
        <span className="text-[13px] font-medium">Análise visual (screenshots para a IA)</span>
        <div role="radiogroup" aria-label="Política de análise visual" className="flex flex-col gap-2">
          {(
            [
              { value: 'never', label: 'Nunca', hint: 'Só texto vai para a IA. Blocos ambíguos ficam para a sua revisão.' },
              { value: 'only_apps', label: 'Apenas nestes apps', hint: 'Screenshots só quando o app ativo estiver na lista.' },
              { value: 'all_except_blocked', label: 'Todos, exceto bloqueados', hint: 'Padrão. Screenshots reduzidos de qualquer app que não esteja bloqueado ou negado.' },
            ] as const
          ).map((opt) => (
            <label key={opt.value} className={clsx('flex cursor-pointer items-start gap-3 rounded-xl border p-3 transition-colors', mode === opt.value ? 'border-brand-500 bg-brand-50/60 dark:bg-brand-900/20' : 'border-line hover:bg-surface-2')}>
              <input
                type="radio"
                name="vision_policy"
                className="mt-1 accent-brand-600"
                checked={mode === opt.value}
                onChange={() => setPolicy(opt.value === 'only_apps' ? { mode: 'only_apps', apps: onlyApps } : { mode: opt.value })}
              />
              <span>
                <span className="block text-sm font-medium">{opt.label}</span>
                <span className="block text-xs text-ink-3">{opt.hint}</span>
              </span>
            </label>
          ))}
        </div>
        {mode === 'only_apps' && (
          <Field label="Apps com análise visual">{(id) => <TagInput id={id} value={onlyApps} onChange={(apps) => setPolicy({ mode: 'only_apps', apps })} placeholder="Google Chrome, Preview…" />}</Field>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <Field label="Apps sem análise visual" hint="Nunca enviam screenshot, mesmo com a política acima.">
          {(id) => <TagInput id={id} value={draft.vision_denied_apps} onChange={(v) => patch({ vision_denied_apps: v })} placeholder="1Password…" />}
        </Field>
        <Field label="Apps bloqueados" hint="Não são registrados de forma alguma (nem título, nem tempo).">
          {(id) => <TagInput id={id} value={draft.blocked_apps} onChange={(v) => patch({ blocked_apps: v })} placeholder="1Password, Banco…" />}
        </Field>
        <Field label="Domínios bloqueados" hint="Sites que nunca entram no registro." className="col-span-2">
          {(id) => <TagInput id={id} value={draft.blocked_domains} onChange={(v) => patch({ blocked_domains: v })} placeholder="bb.com.br, nubank.com.br…" />}
        </Field>
      </div>

      <div className="grid grid-cols-3 gap-4">
        <NumberField label="Intervalo de screenshots" value={draft.screenshot_interval_secs} min={30} step={30} onChange={(v) => patch({ screenshot_interval_secs: v })} suffix="s" />
        <NumberField label="Retenção" hint="Imagens são apagadas depois disso." value={draft.screenshot_retention_hours} min={1} onChange={(v) => patch({ screenshot_retention_hours: v })} suffix="h" />
        <NumberField label="Tamanho máximo" value={draft.screenshot_max_edge} min={256} step={64} onChange={(v) => patch({ screenshot_max_edge: v })} suffix="px" />
      </div>
      <Field label="Manter screenshots para revisão" hint="Permite ver a imagem ao revisar um bloco. Desligado, a imagem é apagada logo após o uso." inline>
        {(id) => <Toggle id={id} checked={draft.keep_screenshots_for_review} onChange={(v) => patch({ keep_screenshots_for_review: v })} />}
      </Field>

      <div className="flex flex-wrap items-center gap-2 border-t border-line pt-4">
        <Button icon={<Download className="size-4" />} onClick={() => void exportData()} loading={busy === 'export'}>
          Exportar meus dados
        </Button>
        <Button variant="danger" icon={<Trash2 className="size-4" />} onClick={() => setConfirmDelete(true)}>
          Apagar todos os dados
        </Button>
      </div>

      <Dialog
        open={confirmDelete}
        onClose={() => setConfirmDelete(false)}
        title="Apagar todos os dados?"
        description="Blocos, screenshots, relatórios, correções e regras aprendidas serão removidos permanentemente. As configurações e a chave de API são mantidas."
        width="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>
              Cancelar
            </Button>
            <Button variant="danger" onClick={() => void deleteAll()} loading={busy === 'delete'}>
              Apagar tudo
            </Button>
          </>
        }
      >
        <p className="text-sm text-ink-2">Esta ação não pode ser desfeita. Exporte seus dados antes, se quiser guardar um histórico.</p>
      </Dialog>

      <AiSentDialog open={aiSentOpen} onClose={() => setAiSentOpen(false)} />
    </Section>
  );
}

function AiSentDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const date = useAppStore((s) => s.date);
  const { data, loading } = useAsync(() => (open ? ipc.getAiSent(date) : Promise.resolve(null)), [open, date]);
  return (
    <Dialog open={open} onClose={onClose} title="Dados enviados à IA" description={`Exatamente o que saiu da sua máquina em ${date}.`} width="lg">
      {loading && !data ? (
        <p className="text-sm text-ink-3">Carregando…</p>
      ) : !data?.length ? (
        <EmptyState title="Nada foi enviado neste dia" />
      ) : (
        <ul className="flex flex-col gap-2">
          {data.map((b) => (
            <li key={b.id} className="rounded-xl border border-line p-3">
              <div className="mb-1 flex items-center justify-between gap-2 text-xs">
                <span className="font-medium">
                  {fmtTime(b.started_at)} · {b.app_name}
                  {b.domain ? ` · ${b.domain}` : ''}
                </span>
                <span className="text-ink-3">{b.ai_sent_at ? fmtDateTime(b.ai_sent_at) : ''}{b.screenshot_id ? ' · com imagem' : ''}</span>
              </div>
              <pre className="scroll-thin overflow-x-auto rounded-lg bg-surface-2 p-2 font-mono text-[11px] leading-4 text-ink-2">{b.ai_payload ?? '(payload não retido)'}</pre>
            </li>
          ))}
        </ul>
      )}
    </Dialog>
  );
}

function ReportsSection({ draft, patch }: SectionProps) {
  return (
    <Section id="relatorios" title="Relatórios" description="Contexto que a IA usa para escrever relatórios na sua voz.">
      <Field label="Horário padrão" hint="Usado pelas categorias sem horário próprio.">
        {(id) => <Input id={id} type="time" value={hhmmToInput(draft.report_default_time)} onChange={(e) => e.target.value && patch({ report_default_time: inputToHhmm(e.target.value) })} className="w-36" />}
      </Field>
      <Field label="Quem é você" hint="Cargo, instituições, projetos, como gosta que os relatórios sejam escritos.">
        {(id) => (
          <Textarea
            id={id}
            value={draft.user_profile ?? ''}
            onChange={(e) => patch({ user_profile: e.target.value || null })}
            placeholder="Ex.: Professor de informática no IFRO, coordenador da incubadora do campus e pesquisador em cidades inteligentes. Relatórios em tom objetivo, na primeira pessoa."
            className="min-h-28"
          />
        )}
      </Field>
    </Section>
  );
}

function UbiSection({ draft, patch }: SectionProps) {
  const n = draft.nudges;
  const setN = (p: Partial<typeof n>) => patch({ nudges: { ...n, ...p } });
  const toast = useToast();
  const snooze = async (minutes: number) => {
    try {
      await ipc.snoozeNudges(minutes);
      setN({ snoozed_until: new Date(Date.now() + minutes * 60_000).toISOString() });
      toast.success('UBI em silêncio', `Sem avisos por ${minutes >= 60 ? `${Math.round(minutes / 60)} h` : `${minutes} min`}.`);
    } catch (e) {
      toast.error('Não foi possível silenciar', e instanceof Error ? e.message : String(e));
    }
  };
  const snoozed = n.snoozed_until && new Date(n.snoozed_until).getTime() > Date.now();

  return (
    <Section id="ubi" title="UBI & notificações" description="Quando e como o mascote pode te interromper.">
      <Field label="Avisos do UBI" hint="Desligue para o UBI só observar." inline>
        {(id) => <Toggle id={id} checked={n.enabled} onChange={(v) => setN({ enabled: v })} />}
      </Field>
      <div className="grid grid-cols-2 gap-x-6 gap-y-3">
        {(
          [
            ['unproductive', 'Improdutividade', 'Muito tempo em categorias não produtivas.'],
            ['distracted', 'Distração', 'Sites e apps de distração durante o expediente.'],
            ['break_suggested', 'Sugestão de pausa', 'Mais de 90 min sem pausa.'],
            ['praise', 'Elogios', 'Blocos longos de foco merecem um parabéns.'],
            ['idle', 'Ociosidade', 'Ocioso por muito tempo em horário de trabalho.'],
          ] as const
        ).map(([key, label, hint]) => (
          <Field key={key} label={label} hint={hint} inline>
            {(id) => <Toggle id={id} size="sm" checked={n[key]} disabled={!n.enabled} onChange={(v) => setN({ [key]: v })} />}
          </Field>
        ))}
      </div>
      <div className="grid grid-cols-2 gap-4">
        <NumberField label="Máximo por dia" value={n.max_per_day} min={0} max={50} onChange={(v) => setN({ max_per_day: v })} suffix="avisos" />
        <NumberField label="Intervalo mínimo" value={n.cooldown_mins} min={1} onChange={(v) => setN({ cooldown_mins: v })} suffix="min" />
      </div>
      <div className="grid grid-cols-3 items-end gap-4">
        <Field label="Horário silencioso" inline>
          {(id) => <Toggle id={id} checked={draft.quiet_hours.enabled} onChange={(v) => patch({ quiet_hours: { ...draft.quiet_hours, enabled: v } })} />}
        </Field>
        <Field label="Das">
          {(id) => <Input id={id} type="time" value={hhmmToInput(draft.quiet_hours.start)} disabled={!draft.quiet_hours.enabled} onChange={(e) => e.target.value && patch({ quiet_hours: { ...draft.quiet_hours, start: inputToHhmm(e.target.value) } })} />}
        </Field>
        <Field label="Até">
          {(id) => <Input id={id} type="time" value={hhmmToInput(draft.quiet_hours.end)} disabled={!draft.quiet_hours.enabled} onChange={(e) => e.target.value && patch({ quiet_hours: { ...draft.quiet_hours, end: inputToHhmm(e.target.value) } })} />}
        </Field>
      </div>
      <Field label="Apps silenciosos" hint="Nenhum aviso enquanto estes apps estiverem em primeiro plano (reuniões, apresentações).">
        {(id) => <TagInput id={id} value={n.silent_apps} onChange={(v) => setN({ silent_apps: v })} placeholder="Microsoft Teams, Keynote…" />}
      </Field>
      <div className="flex flex-wrap items-center gap-2 border-t border-line pt-4">
        <span className="text-sm text-ink-2">Silenciar agora:</span>
        <Button size="sm" onClick={() => void snooze(30)}>
          30 min
        </Button>
        <Button size="sm" onClick={() => void snooze(60)}>
          1 hora
        </Button>
        <Button size="sm" onClick={() => void snooze(4 * 60)}>
          4 horas
        </Button>
        {snoozed && n.snoozed_until && <Badge tone="warning">silenciado até {fmtTime(n.snoozed_until)}</Badge>}
      </div>
    </Section>
  );
}

const PERM_LABEL: Record<PermissionState, { text: string; color: string }> = {
  granted: { text: 'Concedida', color: '#10b981' },
  denied: { text: 'Negada', color: '#ef4444' },
  unknown: { text: 'Não solicitada', color: '#94a3b8' },
  not_applicable: { text: 'Não se aplica', color: '#94a3b8' },
};

export function PermissionRows({ view, onChanged }: { view: SettingsView; onChanged?: () => void }) {
  const [busy, setBusy] = useState<PermissionKind | null>(null);
  const loadSettings = useAppStore((s) => s.loadSettings);
  const toast = useToast();
  const rows: { kind: PermissionKind; label: string; hint: string }[] = [
    { kind: 'screen_recording', label: 'Gravação de Tela', hint: 'Necessária para ler títulos de janela e tirar screenshots. Sem ela, os blocos ficam sem título.' },
    { kind: 'automation', label: 'Automação (navegadores)', hint: 'Lê a URL da aba ativa do Chrome/Safari/Arc via AppleScript. O macOS pergunta uma vez por navegador.' },
    { kind: 'accessibility', label: 'Acessibilidade', hint: 'Opcional. Melhora a detecção de ociosidade e do app ativo em alguns apps.' },
  ];
  const request = async (kind: PermissionKind) => {
    setBusy(kind);
    try {
      await ipc.requestPermission(kind);
      await loadSettings();
      onChanged?.();
    } catch (e) {
      toast.error('Não foi possível solicitar', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(null);
    }
  };
  return (
    <ul className="divide-y divide-line rounded-xl border border-line">
      {rows.map((r) => {
        const state = view.permissions[r.kind];
        const p = PERM_LABEL[state];
        return (
          <li key={r.kind} className="flex items-center gap-4 p-3">
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium">{r.label}</p>
              <p className="text-xs text-ink-3">{r.hint}</p>
            </div>
            <StatusPill color={p.color}>{p.text}</StatusPill>
            {state !== 'granted' && state !== 'not_applicable' && (
              <Button size="sm" variant="primary" onClick={() => void request(r.kind)} loading={busy === r.kind}>
                Solicitar
              </Button>
            )}
          </li>
        );
      })}
    </ul>
  );
}

function PermissionsSection({ view }: { view: SettingsView }) {
  const date = useAppStore((s) => s.date);
  const dash = useAppStore((s) => s.dashboards[date]);
  const titlesEmpty = useMemo(() => !!dash && dash.timeline.length > 0 && dash.timeline.filter((b) => b.app_id !== 'idle' && b.app_id !== 'private').every((b) => !b.title), [dash]);
  const needsRestart = view.permissions.screen_recording === 'granted' && titlesEmpty;
  return (
    <Section id="permissoes" title="Permissões (macOS)" description="O macOS exige permissões explícitas para ler janelas e capturar a tela.">
      <PermissionRows view={view} />
      {needsRestart && (
        <div className="flex items-center gap-3 rounded-xl border border-amber-200 bg-amber-50 p-3 text-sm dark:border-amber-900/50 dark:bg-amber-900/20">
          <Camera className="size-4 text-amber-600" />
          <span className="flex-1">A permissão foi concedida, mas os títulos ainda chegam vazios. O macOS só aplica a Gravação de Tela após reiniciar o app.</span>
          <Button size="sm" variant="primary" onClick={() => void ipc.restartApp()}>
            Reiniciar o ubiqX
          </Button>
        </div>
      )}
    </Section>
  );
}

function AboutSection({ view }: { view: SettingsView }) {
  const toast = useToast();
  return (
    <Section id="sobre" title="Sobre">
      <dl className="grid grid-cols-[140px_1fr] gap-x-4 gap-y-2 text-sm">
        <dt className="text-ink-3">Versão</dt>
        <dd className="font-medium">ubiqX {view.version}</dd>
        <dt className="text-ink-3">Pasta de dados</dt>
        <dd className="truncate font-mono text-xs" title={view.data_dir}>
          {view.data_dir}
        </dd>
        <dt className="text-ink-3">Plataforma</dt>
        <dd>{view.platform === 'macos' ? 'macOS' : 'Outra (modo de desenvolvimento)'}</dd>
        <dt className="text-ink-3">Motor</dt>
        <dd>
          {view.tracker_state === 'running' ? 'rastreando' : view.tracker_state} · IA {view.ai_health.state === 'ok' ? 'ok' : view.ai_health.state}
        </dd>
      </dl>
      <div className="flex gap-2">
        <Button
          icon={<ExternalLink className="size-4" />}
          onClick={() => {
            ipc.openExternal(REPO_URL).catch((e: unknown) => toast.error('Não foi possível abrir', e instanceof Error ? e.message : String(e)));
          }}
        >
          Repositório no GitHub
        </Button>
      </div>
      <p className="text-xs text-ink-3">
        Feito com Rust, Tauri 2 e React. O UBI agradece as correções: cada uma o deixa mais esperto.
      </p>
    </Section>
  );
}

