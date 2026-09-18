import clsx from 'clsx';
import { AlertTriangle, Archive, ArchiveRestore, ChevronDown, ChevronUp, Lock, Plus, Tags, Trash2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { Badge } from '../components/ui/Badge';
import { Button, IconButton } from '../components/ui/Button';
import { Card, CardHeader } from '../components/ui/Card';
import { CategoryChip } from '../components/ui/CategoryChip';
import { Dialog } from '../components/ui/Dialog';
import { Field, Input, Select, Textarea } from '../components/ui/Field';
import { EmptyState } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { TagInput } from '../components/ui/TagInput';
import { Toggle } from '../components/ui/Toggle';
import { blankCategory, COLOR_CHOICES, ICON_CHOICES, iconFor } from '../lib/categories';
import { fmtDateTime, hhmmToInput, inputToHhmm } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { Category, Rule, RuleMatcher } from '../lib/types';
import { useAsync } from '../lib/useAsync';

const MATCHERS: { value: RuleMatcher; label: string; hint: string }[] = [
  { value: 'domain', label: 'Domínio', hint: 'ex.: sei.ifro.edu.br' },
  { value: 'app', label: 'App', hint: 'ex.: Microsoft Teams' },
  { value: 'title_contains', label: 'Título contém', hint: 'ex.: incubadora' },
  { value: 'regex', label: 'Regex', hint: 'ex.: cidades|sensor' },
];

/** Tinted square with the category icon: the category's own colour, never a token. */
function IconTile({ color, icon, size = 'md' }: { color: string; icon: string; size?: 'sm' | 'md' | 'lg' }) {
  const Icon = iconFor(icon);
  const box = size === 'lg' ? 'size-12 rounded-card' : size === 'sm' ? 'size-7 rounded-[8px]' : 'size-8 rounded-[8px]';
  const glyph = size === 'lg' ? 'size-[22px]' : size === 'sm' ? 'size-3.5' : 'size-4';
  return (
    <span
      className={clsx('flex shrink-0 items-center justify-center', box)}
      style={{ background: `color-mix(in oklab, ${color} 16%, transparent)`, color, boxShadow: `inset 0 0 0 1px color-mix(in oklab, ${color} 28%, transparent)` }}
      aria-hidden
    >
      <Icon className={glyph} strokeWidth={1.75} />
    </span>
  );
}

export function Categories() {
  const { data: cats, reload: reloadCats, setData: setCats } = useAsync(() => ipc.listCategories(true), []);
  const loadCategories = useAppStore((s) => s.loadCategories);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState<Category | null>(null);
  const [isNew, setIsNew] = useState(false);
  const toast = useToast();

  const sorted = useMemo(() => {
    const list = cats ?? [];
    return {
      user: list.filter((c) => !c.is_system && !c.archived).sort((a, b) => a.sort_order - b.sort_order),
      archived: list.filter((c) => !c.is_system && c.archived),
      system: list.filter((c) => c.is_system).sort((a, b) => a.sort_order - b.sort_order),
    };
  }, [cats]);

  useEffect(() => {
    if (!cats?.length) return;
    if (!selectedId) {
      const first = sorted.user[0] ?? sorted.system[0];
      if (first) setSelectedId(first.id);
    }
  }, [cats, selectedId, sorted]);

  useEffect(() => {
    if (isNew) return;
    const c = cats?.find((x) => x.id === selectedId);
    setDraft(c ? structuredClone(c) : null);
  }, [selectedId, cats, isNew]);

  const select = (id: string) => {
    setIsNew(false);
    setSelectedId(id);
  };

  const startNew = () => {
    setIsNew(true);
    setSelectedId(null);
    setDraft(blankCategory(sorted.user.length));
  };

  const save = async () => {
    if (!draft) return;
    if (!draft.name.trim()) {
      toast.error('Dê um nome à categoria');
      return;
    }
    if (!draft.is_system && !draft.description.trim()) {
      toast.error('Descreva o que conta como trabalho desta categoria', 'A IA usa a descrição para classificar com precisão.');
      return;
    }
    try {
      const saved = await ipc.saveCategory(draft);
      await reloadCats();
      await loadCategories();
      setIsNew(false);
      setSelectedId(saved.id);
      toast.success('Categoria salva');
    } catch (e) {
      toast.error('Não foi possível salvar', e instanceof Error ? e.message : String(e));
    }
  };

  const archive = async (c: Category, archived: boolean) => {
    try {
      await ipc.saveCategory({ ...c, archived });
      await reloadCats();
      await loadCategories();
      toast.success(archived ? 'Categoria arquivada' : 'Categoria restaurada');
    } catch (e) {
      toast.error('Não foi possível arquivar', e instanceof Error ? e.message : String(e));
    }
  };

  const remove = async (c: Category) => {
    if (!window.confirm(`Remover “${c.name}”? Os blocos ficam sem categoria.`)) return;
    try {
      await ipc.deleteCategory(c.id);
      setCats((cats ?? []).filter((x) => x.id !== c.id));
      await loadCategories();
      setSelectedId(null);
      toast.success('Categoria removida');
    } catch (e) {
      toast.error('Não foi possível remover', e instanceof Error ? e.message : String(e));
    }
  };

  /** Drag-free ordering: swap the sort order with the neighbour and persist both. */
  const move = async (c: Category, dir: -1 | 1) => {
    const list = sorted.user;
    const i = list.findIndex((x) => x.id === c.id);
    const other = list[i + dir];
    if (i < 0 || !other) return;
    const a = { ...c, sort_order: i + dir };
    const b = { ...other, sort_order: i };
    try {
      await ipc.saveCategory(a);
      await ipc.saveCategory(b);
      await reloadCats();
      await loadCategories();
    } catch (e) {
      toast.error('Não foi possível reordenar', e instanceof Error ? e.message : String(e));
    }
  };

  const hasUser = sorted.user.length > 0 || sorted.archived.length > 0;

  return (
    <div data-testid="page-categories">
      <PageHeader
        title="Categorias"
        subtitle="Descrição, palavras-chave e horário de relatório de cada área do seu trabalho. A IA lê tudo isso antes de classificar."
        actions={
          <Button variant="primary" icon={<Plus className="size-4" strokeWidth={1.75} />} onClick={startNew}>
            Nova categoria
          </Button>
        }
      />

      <div className="grid grid-cols-12 gap-5">
        <Card padded={false} className="col-span-12 self-start min-[1100px]:col-span-4">
          {cats && !hasUser && !isNew ? (
            <EmptyState
              icon={<Tags className="size-5" strokeWidth={1.75} />}
              title="Nenhuma categoria sua ainda"
              description="Crie uma para cada instituição ou projeto. Sem categorias, tudo fica em “Sem categoria”."
              action={
                <Button variant="primary" size="sm" icon={<Plus className="size-3.5" strokeWidth={1.75} />} onClick={startNew}>
                  Criar a primeira
                </Button>
              }
            />
          ) : (
            <CategoryList title="Suas categorias" items={sorted.user} selectedId={selectedId} onSelect={select} onMove={(c, d) => void move(c, d)} />
          )}
          {sorted.archived.length > 0 && <CategoryList title="Arquivadas" items={sorted.archived} selectedId={selectedId} onSelect={select} muted />}
          <CategoryList title="Do sistema" items={sorted.system} selectedId={selectedId} onSelect={select} locked />
        </Card>

        <Card className="col-span-12 min-[1100px]:col-span-8">
          {draft ? (
            <CategoryForm
              key={draft.id}
              draft={draft}
              onChange={setDraft}
              onSave={() => void save()}
              onArchive={() => void archive(draft, !draft.archived)}
              onDelete={() => void remove(draft)}
              isNew={isNew}
            />
          ) : (
            <EmptyState
              icon={<Tags className="size-5" strokeWidth={1.75} />}
              title="Escolha uma categoria ao lado"
              description="Ou crie uma nova para começar."
              action={
                <Button size="sm" icon={<Plus className="size-3.5" strokeWidth={1.75} />} onClick={startNew}>
                  Nova categoria
                </Button>
              }
            />
          )}
        </Card>
      </div>

      <RulesSection categories={cats ?? []} />
    </div>
  );
}

function CategoryList({
  title,
  items,
  selectedId,
  onSelect,
  onMove,
  locked,
  muted,
}: {
  title: string;
  items: Category[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onMove?: (c: Category, dir: -1 | 1) => void;
  locked?: boolean;
  muted?: boolean;
}) {
  return (
    <section className="border-b border-line last:border-b-0" aria-label={title}>
      <div className="flex items-baseline justify-between px-4 pt-3.5 pb-1">
        <p className="eyebrow">{title}</p>
        <span className="num text-[11px] text-ink-4">{items.length}</span>
      </div>
      <ul className="px-2 pb-2">
        {items.map((c, i) => {
          const active = c.id === selectedId;
          const meta = c.is_productive ? (c.report_time ? `Produtiva, relatório às ${hhmmToInput(c.report_time)}` : 'Produtiva') : 'Não conta como trabalho';
          return (
            <li key={c.id} className="group relative">
              {active && <span className="absolute top-2.5 bottom-2.5 -left-2 w-[3px] rounded-r-full bg-volt" aria-hidden />}
              <button
                type="button"
                onClick={() => onSelect(c.id)}
                aria-current={active ? 'true' : undefined}
                className={clsx(
                  'flex min-h-11 w-full items-center gap-3 rounded-control py-1.5 pr-2 pl-2.5 text-left text-sm transition-colors duration-120',
                  active ? 'bg-volt-soft text-ink' : 'text-ink hover:bg-panel-2',
                  muted && 'opacity-60',
                )}
              >
                <IconTile color={c.color} icon={c.icon} />
                <span className="min-w-0 flex-1">
                  <span className="block truncate font-medium">{c.name}</span>
                  <span className="block truncate text-[11px] text-ink-3">{c.is_system ? 'Categoria do sistema' : meta}</span>
                </span>
                {locked && <Lock className="size-3.5 shrink-0 text-ink-4" strokeWidth={1.75} aria-hidden />}
              </button>
              {onMove && items.length > 1 && (
                <span className={clsx('absolute top-1/2 right-1.5 flex -translate-y-1/2 flex-col opacity-0 transition-opacity duration-120 group-hover:opacity-100 focus-within:opacity-100', active && 'opacity-100')}>
                  <IconButton label={`Mover ${c.name} para cima`} size="sm" className="size-5 rounded-[6px]" disabled={i === 0} onClick={() => onMove(c, -1)}>
                    <ChevronUp className="size-3.5" strokeWidth={1.75} />
                  </IconButton>
                  <IconButton label={`Mover ${c.name} para baixo`} size="sm" className="size-5 rounded-[6px]" disabled={i === items.length - 1} onClick={() => onMove(c, 1)}>
                    <ChevronDown className="size-3.5" strokeWidth={1.75} />
                  </IconButton>
                </span>
              )}
            </li>
          );
        })}
        {!items.length && <li className="px-2.5 py-2 text-xs text-ink-3">Nenhuma por enquanto.</li>}
      </ul>
    </section>
  );
}

function CategoryForm({ draft, onChange, onSave, onArchive, onDelete, isNew }: { draft: Category; onChange: (c: Category) => void; onSave: () => void; onArchive: () => void; onDelete: () => void; isNew: boolean }) {
  const ro = draft.is_system;
  const set = (patch: Partial<Category>) => onChange({ ...draft, ...patch });
  const selectedColor = draft.color.toUpperCase();

  return (
    <form
      className="flex flex-col gap-5"
      onSubmit={(e) => {
        e.preventDefault();
        onSave();
      }}
    >
      <div className="flex items-center gap-4">
        <IconTile color={draft.color} icon={draft.icon} size="lg" />
        <div className="min-w-0 flex-1">
          <h2 className="display truncate text-[20px] leading-7">{draft.name || 'Nova categoria'}</h2>
          <p className="mt-0.5 text-xs text-ink-3">{ro ? 'Categoria do sistema: só a cor e o ícone podem mudar.' : isNew ? 'Ainda não salva' : `Criada em ${fmtDateTime(draft.created_at)}`}</p>
        </div>
        {ro && (
          <Badge tone="neutral">
            <Lock className="size-3" strokeWidth={1.75} aria-hidden /> sistema
          </Badge>
        )}
        {draft.archived && <Badge tone="amber">arquivada</Badge>}
      </div>

      <div className="grid grid-cols-1 gap-4 min-[720px]:grid-cols-2">
        <Field label="Nome">{(id) => <Input id={id} value={draft.name} disabled={ro} onChange={(e) => set({ name: e.target.value })} placeholder="Ex.: IFRO" required />}</Field>
        <Field label="Horário do relatório" hint="Vazio usa o horário padrão das configurações.">
          {(id) => <Input id={id} type="time" disabled={ro} value={hhmmToInput(draft.report_time)} onChange={(e) => set({ report_time: e.target.value ? inputToHhmm(e.target.value) : null })} className="num" />}
        </Field>
      </div>

      <Field label="O que conta como trabalho desta categoria" hint="Seja específico: sistemas, projetos, pessoas, tipos de tarefa. A IA lê isto antes de classificar cada bloco.">
        {(id) => <Textarea id={id} value={draft.description} disabled={ro} onChange={(e) => set({ description: e.target.value })} placeholder="Ex.: Aulas de Programação Web, orientação de TCC, SEI/SUAP, reuniões de colegiado…" className="min-h-24" />}
      </Field>

      <Field label="Palavras-chave" hint="Termos que, no título ou na URL, indicam fortemente esta categoria. Enter para adicionar.">
        {(id) => <TagInput id={id} value={draft.keywords} disabled={ro} onChange={(keywords) => set({ keywords })} placeholder="sei, suap, tcc…" />}
      </Field>

      <div className="grid grid-cols-1 gap-5 min-[720px]:grid-cols-[auto_1fr] min-[720px]:gap-8">
        <div className="flex flex-col gap-2">
          <span className="text-[13px] font-medium">Cor</span>
          <div className="grid grid-cols-7 gap-2" role="radiogroup" aria-label="Cor">
            {COLOR_CHOICES.map((c) => {
              const on = selectedColor === c;
              return (
                <button
                  key={c}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  aria-label={c}
                  onClick={() => set({ color: c })}
                  className="size-7 rounded-full transition-[box-shadow,transform] duration-120 hover:scale-105"
                  style={{ background: c, boxShadow: on ? `0 0 0 2px var(--panel), 0 0 0 4px ${c}` : undefined }}
                />
              );
            })}
            <label
              className="relative size-7 cursor-pointer overflow-hidden rounded-full transition-[box-shadow] duration-120"
              title="Cor personalizada"
              style={{
                background: COLOR_CHOICES.includes(selectedColor) ? 'conic-gradient(from 0deg, #ff7a1f, #ffc24d, #2ee6a6, #4d8dff, #9b8cff, #ff5c7a, #ff7a1f)' : draft.color,
                boxShadow: COLOR_CHOICES.includes(selectedColor) ? undefined : `0 0 0 2px var(--panel), 0 0 0 4px ${draft.color}`,
              }}
            >
              <input type="color" value={draft.color} onChange={(e) => set({ color: e.target.value.toUpperCase() })} className="absolute inset-0 size-full cursor-pointer opacity-0" aria-label="Cor personalizada" />
              <Plus className="absolute top-1/2 left-1/2 size-3.5 -translate-x-1/2 -translate-y-1/2 text-white drop-shadow-[0_1px_1px_rgb(0_0_0/.5)]" strokeWidth={2.5} aria-hidden />
            </label>
          </div>
        </div>
        <div className="flex flex-col gap-2">
          <span className="text-[13px] font-medium">Ícone</span>
          <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label="Ícone">
            {ICON_CHOICES.map(({ name, Icon: I }) => {
              const on = draft.icon === name;
              return (
                <button
                  key={name}
                  type="button"
                  role="radio"
                  aria-checked={on}
                  aria-label={name}
                  onClick={() => set({ icon: name })}
                  className={clsx('flex size-8 items-center justify-center rounded-[8px] border transition-colors duration-120', on ? 'border-volt/60 bg-volt-soft text-volt' : 'border-line text-ink-3 hover:bg-panel-2 hover:text-ink')}
                >
                  <I className="size-4" strokeWidth={1.75} />
                </button>
              );
            })}
          </div>
        </div>
      </div>

      <Field label="Template do relatório" hint="Opcional. Use {resumo}, {itens} e {proximos} como marcadores.">
        {(id) => <Textarea id={id} value={draft.report_template ?? ''} disabled={ro} onChange={(e) => set({ report_template: e.target.value || null })} placeholder={'## Resumo\n\n{resumo}\n\n## Atividades\n\n{itens}'} className="font-mono text-xs" />}
      </Field>

      <Field label="Conta como trabalho" hint="Entra no tempo produtivo e no score de foco." inline>
        {(id) => <Toggle id={id} checked={draft.is_productive} disabled={ro} onChange={(v) => set({ is_productive: v })} />}
      </Field>

      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line pt-4">
        <div className="flex gap-1">
          {!ro && !isNew && (
            <>
              <Button variant="ghost" icon={draft.archived ? <ArchiveRestore className="size-4" strokeWidth={1.75} /> : <Archive className="size-4" strokeWidth={1.75} />} onClick={onArchive}>
                {draft.archived ? 'Restaurar' : 'Arquivar'}
              </Button>
              <Button variant="ghost" className="text-rose hover:bg-rose/10 hover:text-rose" icon={<Trash2 className="size-4" strokeWidth={1.75} />} onClick={onDelete}>
                Remover
              </Button>
            </>
          )}
        </div>
        <Button type="submit" variant="primary" size="lg">
          {isNew ? 'Criar categoria' : 'Salvar alterações'}
        </Button>
      </div>
    </form>
  );
}

function RulesSection({ categories }: { categories: Category[] }) {
  const { data: rules, reload, setData } = useAsync(() => ipc.listRules(), []);
  const [open, setOpen] = useState(false);
  const [matcher, setMatcher] = useState<RuleMatcher>('domain');
  const [pattern, setPattern] = useState('');
  const [categoryId, setCategoryId] = useState('');
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const options = categories.filter((c) => !c.archived && c.id !== 'sys-private' && c.id !== 'sys-uncategorized');

  useEffect(() => {
    if (!categoryId && options[0]) setCategoryId(options[0].id);
  }, [options, categoryId]);

  const toggle = async (r: Rule, enabled: boolean) => {
    try {
      const saved = await ipc.saveRule({ ...r, enabled });
      setData((rules ?? []).map((x) => (x.id === saved.id ? saved : x)));
    } catch (e) {
      toast.error('Não foi possível atualizar a regra', e instanceof Error ? e.message : String(e));
    }
  };

  const remove = async (r: Rule) => {
    try {
      await ipc.deleteRule(r.id);
      setData((rules ?? []).filter((x) => x.id !== r.id));
      toast.success('Regra removida');
    } catch (e) {
      toast.error('Não foi possível remover', e instanceof Error ? e.message : String(e));
    }
  };

  const create = async () => {
    if (!pattern.trim() || !categoryId) return;
    setBusy(true);
    try {
      await ipc.saveRule({
        id: crypto.randomUUID(),
        category_id: categoryId,
        matcher,
        pattern: pattern.trim(),
        priority: 10,
        origin: 'user',
        enabled: true,
        created_at: new Date().toISOString(),
        hit_count: 0,
        miss_count: 0,
        last_contradicted_at: null,
      });
      await reload();
      setPattern('');
      setOpen(false);
      toast.success('Regra criada');
    } catch (e) {
      toast.error('Não foi possível criar', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const learned = (rules ?? []).filter((r) => r.origin === 'learned').length;

  return (
    <Card className="mt-5" padded={false}>
      <div className="px-5 pt-5">
        <CardHeader
          title="Regras"
          subtitle={
            rules?.length
              ? `${rules.length} ${rules.length === 1 ? 'regra' : 'regras'}, ${learned} ${learned === 1 ? 'aprendida' : 'aprendidas'} das suas correções. Rodam antes da memória e da IA, sem custo.`
              : 'Regras são determinísticas e gratuitas: rodam antes da memória e da IA. As aprendidas vêm das suas correções.'
          }
          action={
            <Button size="sm" icon={<Plus className="size-3.5" strokeWidth={1.75} />} onClick={() => setOpen(true)}>
              Nova regra
            </Button>
          }
        />
      </div>
      {rules?.length ? (
        <table className="w-full text-sm">
          <thead>
            <tr className="border-y border-line text-left text-xs text-ink-3">
              <th className="px-5 py-2 font-medium">Regra</th>
              <th className="px-3 py-2 font-medium">Categoria</th>
              <th className="px-3 py-2 font-medium">Origem</th>
              <th className="px-3 py-2 text-right font-medium">Acertos e erros</th>
              <th className="px-3 py-2 font-medium">Ativa</th>
              <th className="px-3 py-2" />
            </tr>
          </thead>
          <tbody className="divide-y divide-line">
            {rules.map((r) => (
              <tr key={r.id} className={clsx('transition-opacity duration-150', !r.enabled && 'opacity-50')}>
                <td className="px-5 py-2.5">
                  <span className="mr-2 inline-flex h-5 items-center rounded-md border border-line bg-panel-2 px-1.5 text-[11px] text-ink-3">{MATCHERS.find((m) => m.value === r.matcher)?.label}</span>
                  <span className="font-mono text-[13px]">{r.pattern}</span>
                </td>
                <td className="px-3 py-2.5">
                  <CategoryChip categories={categories} categoryId={r.category_id} />
                </td>
                <td className="px-3 py-2.5">
                  <Badge tone={r.origin === 'learned' ? 'violet' : 'volt'}>{r.origin === 'learned' ? 'Aprendida' : 'Sua'}</Badge>
                </td>
                <td className="num px-3 py-2.5 text-right text-xs text-ink-2">
                  <span className="text-signal">{r.hit_count}</span>
                  <span className="mx-1 text-ink-4">/</span>
                  <span className={r.miss_count > 0 ? 'text-rose' : 'text-ink-3'}>{r.miss_count}</span>
                  {r.last_contradicted_at && (
                    <AlertTriangle className="ml-1.5 inline size-3.5 align-[-2px] text-amber" strokeWidth={1.75} aria-label={`Última contradição: ${fmtDateTime(r.last_contradicted_at)}`} />
                  )}
                </td>
                <td className="px-3 py-2.5">
                  <Toggle size="sm" checked={r.enabled} onChange={(v) => void toggle(r, v)} label={`Ativar regra ${r.pattern}`} />
                </td>
                <td className="px-3 py-2.5 text-right">
                  <IconButton label="Remover regra" size="sm" onClick={() => void remove(r)}>
                    <Trash2 className="size-3.5" strokeWidth={1.75} />
                  </IconButton>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <div className="border-t border-line">
          <EmptyState
            className="py-8"
            title="Nenhuma regra ainda"
            description="Corrija blocos na Revisão e aceite as sugestões, ou crie uma regra à mão."
            action={
              <Button size="sm" icon={<Plus className="size-3.5" strokeWidth={1.75} />} onClick={() => setOpen(true)}>
                Criar regra
              </Button>
            }
          />
        </div>
      )}

      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        title="Nova regra"
        description="Quando o padrão bater, o bloco recebe a categoria sem passar pela IA."
        width="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancelar
            </Button>
            <Button variant="primary" onClick={() => void create()} loading={busy} disabled={!pattern.trim()}>
              Criar regra
            </Button>
          </>
        }
      >
        <div className="flex flex-col gap-3">
          <Field label="Tipo">
            {(id) => (
              <Select id={id} value={matcher} onChange={(e) => setMatcher(e.target.value as RuleMatcher)}>
                {MATCHERS.map((m) => (
                  <option key={m.value} value={m.value}>
                    {m.label}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label="Padrão" hint={MATCHERS.find((m) => m.value === matcher)?.hint}>
            {(id) => <Input id={id} value={pattern} onChange={(e) => setPattern(e.target.value)} className="font-mono" />}
          </Field>
          <Field label="Categoria">
            {(id) => (
              <Select id={id} value={categoryId} onChange={(e) => setCategoryId(e.target.value)}>
                {options.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                  </option>
                ))}
              </Select>
            )}
          </Field>
        </div>
      </Dialog>
    </Card>
  );
}
