import clsx from 'clsx';
import { Archive, Lock, Plus, Trash2 } from 'lucide-react';
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

  return (
    <div data-testid="page-categories">
      <PageHeader
        title="Categorias"
        subtitle="Cada categoria tem sua descrição, palavras-chave e horário de relatório. A IA usa tudo isso para classificar."
        actions={
          <Button variant="primary" icon={<Plus className="size-4" />} onClick={startNew}>
            Nova categoria
          </Button>
        }
      />

      <div className="grid grid-cols-12 gap-5">
        <Card padded={false} className="col-span-12 self-start min-[1100px]:col-span-4">
          <CategoryList title="Suas categorias" items={sorted.user} selectedId={selectedId} onSelect={(id) => { setIsNew(false); setSelectedId(id); }} />
          {sorted.archived.length > 0 && <CategoryList title="Arquivadas" items={sorted.archived} selectedId={selectedId} onSelect={(id) => { setIsNew(false); setSelectedId(id); }} muted />}
          <CategoryList title="Sistema" items={sorted.system} selectedId={selectedId} onSelect={(id) => { setIsNew(false); setSelectedId(id); }} locked />
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
            <EmptyState title="Selecione uma categoria" description="Ou crie uma nova para começar." />
          )}
        </Card>
      </div>

      <RulesSection categories={cats ?? []} />
    </div>
  );
}

function CategoryList({ title, items, selectedId, onSelect, locked, muted }: { title: string; items: Category[]; selectedId: string | null; onSelect: (id: string) => void; locked?: boolean; muted?: boolean }) {
  return (
    <div className="border-b border-line last:border-b-0">
      <p className="px-4 pt-3 pb-1.5 text-[11px] font-medium tracking-wide text-ink-3 uppercase">{title}</p>
      <ul className="px-2 pb-2">
        {items.map((c) => {
          const Icon = iconFor(c.icon);
          const active = c.id === selectedId;
          return (
            <li key={c.id}>
              <button
                type="button"
                onClick={() => onSelect(c.id)}
                className={clsx('flex w-full items-center gap-3 rounded-xl px-2.5 py-2 text-left text-sm transition-colors', active ? 'bg-brand-50 dark:bg-brand-900/30' : 'hover:bg-surface-2', muted && 'opacity-60')}
              >
                <span className="flex size-8 items-center justify-center rounded-lg" style={{ background: `${c.color}22`, color: c.color }}>
                  <Icon className="size-4" />
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block truncate font-medium">{c.name}</span>
                  <span className="block truncate text-[11px] text-ink-3">{c.is_productive ? 'produtiva' : 'não produtiva'}{c.report_time ? ` · relatório ${hhmmToInput(c.report_time)}` : ''}</span>
                </span>
                {locked && <Lock className="size-3.5 text-ink-3" />}
              </button>
            </li>
          );
        })}
        {!items.length && <li className="px-2.5 py-2 text-xs text-ink-3">Nenhuma.</li>}
      </ul>
    </div>
  );
}

function CategoryForm({ draft, onChange, onSave, onArchive, onDelete, isNew }: { draft: Category; onChange: (c: Category) => void; onSave: () => void; onArchive: () => void; onDelete: () => void; isNew: boolean }) {
  const ro = draft.is_system;
  const set = (patch: Partial<Category>) => onChange({ ...draft, ...patch });
  const Icon = iconFor(draft.icon);

  return (
    <form
      className="flex flex-col gap-5"
      onSubmit={(e) => {
        e.preventDefault();
        onSave();
      }}
    >
      <div className="flex items-center gap-3">
        <span className="flex size-12 items-center justify-center rounded-2xl" style={{ background: `${draft.color}22`, color: draft.color }}>
          <Icon className="size-6" />
        </span>
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-lg font-semibold">{draft.name || 'Nova categoria'}</h2>
          <p className="text-xs text-ink-3">{ro ? 'Categoria do sistema — somente cor e ícone podem ser alterados.' : isNew ? 'Ainda não salva' : `Criada em ${fmtDateTime(draft.created_at)}`}</p>
        </div>
        {ro && <Badge tone="neutral"><Lock className="size-3" /> sistema</Badge>}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <Field label="Nome">{(id) => <Input id={id} value={draft.name} disabled={ro} onChange={(e) => set({ name: e.target.value })} placeholder="Ex.: IFRO" required />}</Field>
        <Field label="Horário do relatório" hint="Vazio usa o horário padrão das configurações.">
          {(id) => <Input id={id} type="time" disabled={ro} value={hhmmToInput(draft.report_time)} onChange={(e) => set({ report_time: e.target.value ? inputToHhmm(e.target.value) : null })} />}
        </Field>
      </div>

      <Field label="O que conta como trabalho desta categoria" hint="Seja específico: sistemas, projetos, pessoas, tipos de tarefa. A IA lê isto antes de classificar cada bloco.">
        {(id) => <Textarea id={id} value={draft.description} disabled={ro} onChange={(e) => set({ description: e.target.value })} placeholder="Ex.: Aulas de Programação Web, orientação de TCC, SEI/SUAP, reuniões de colegiado…" className="min-h-24" />}
      </Field>

      <Field label="Palavras-chave" hint="Termos que, se aparecerem no título ou URL, indicam fortemente esta categoria. Enter para adicionar.">
        {(id) => <TagInput id={id} value={draft.keywords} disabled={ro} onChange={(keywords) => set({ keywords })} placeholder="sei, suap, tcc…" />}
      </Field>

      <div className="grid grid-cols-2 gap-4">
        <div className="flex flex-col gap-1.5">
          <span className="text-[13px] font-medium">Cor</span>
          <div className="flex flex-wrap gap-2" role="radiogroup" aria-label="Cor">
            {COLOR_CHOICES.map((c) => (
              <button
                key={c}
                type="button"
                role="radio"
                aria-checked={draft.color.toUpperCase() === c}
                aria-label={c}
                onClick={() => set({ color: c })}
                className={clsx('size-7 rounded-full border-2 transition-transform hover:scale-110', draft.color.toUpperCase() === c ? 'border-ink' : 'border-transparent')}
                style={{ background: c }}
              />
            ))}
            <label className="relative size-7 cursor-pointer overflow-hidden rounded-full border border-line-strong" title="Cor personalizada">
              <input type="color" value={draft.color} onChange={(e) => set({ color: e.target.value.toUpperCase() })} className="absolute inset-0 size-full cursor-pointer opacity-0" aria-label="Cor personalizada" />
              <span className="absolute inset-0 rounded-full" style={{ background: `conic-gradient(from 0deg, #f97316, #eab308, #10b981, #2563eb, #a855f7, #f97316)` }} />
            </label>
          </div>
        </div>
        <div className="flex flex-col gap-1.5">
          <span className="text-[13px] font-medium">Ícone</span>
          <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label="Ícone">
            {ICON_CHOICES.map(({ name, Icon: I }) => (
              <button
                key={name}
                type="button"
                role="radio"
                aria-checked={draft.icon === name}
                aria-label={name}
                onClick={() => set({ icon: name })}
                className={clsx('flex size-8 items-center justify-center rounded-lg border transition-colors', draft.icon === name ? 'border-brand-500 bg-brand-50 text-brand-700 dark:bg-brand-900/30 dark:text-brand-300' : 'border-line text-ink-2 hover:bg-surface-2')}
              >
                <I className="size-4" />
              </button>
            ))}
          </div>
        </div>
      </div>

      <Field label="Template do relatório" hint="Opcional. Use {resumo}, {itens} e {proximos} como marcadores.">
        {(id) => <Textarea id={id} value={draft.report_template ?? ''} disabled={ro} onChange={(e) => set({ report_template: e.target.value || null })} placeholder={'## Resumo\n\n{resumo}\n\n## Atividades\n\n{itens}'} className="font-mono text-xs" />}
      </Field>

      <Field label="Produtiva" hint="Conta para o tempo produtivo e o score de foco." inline>
        {(id) => <Toggle id={id} checked={draft.is_productive} disabled={ro} onChange={(v) => set({ is_productive: v })} />}
      </Field>

      <div className="flex items-center justify-between gap-2 border-t border-line pt-4">
        <div className="flex gap-2">
          {!ro && !isNew && (
            <>
              <Button variant="ghost" icon={<Archive className="size-4" />} onClick={onArchive}>
                {draft.archived ? 'Restaurar' : 'Arquivar'}
              </Button>
              <Button variant="danger" icon={<Trash2 className="size-4" />} onClick={onDelete}>
                Remover
              </Button>
            </>
          )}
        </div>
        <Button type="submit" variant="primary" disabled={ro && !isNew ? false : undefined}>
          {isNew ? 'Criar categoria' : 'Salvar'}
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

  return (
    <Card className="mt-5" padded={false}>
      <div className="px-5 pt-5">
        <CardHeader
          title="Regras"
          subtitle="Regras são determinísticas e gratuitas: rodam antes da memória e da IA. As aprendidas vêm das suas correções."
          action={
            <Button size="sm" variant="primary" icon={<Plus className="size-3.5" />} onClick={() => setOpen(true)}>
              Nova regra
            </Button>
          }
        />
      </div>
      <table className="w-full text-sm">
        <thead>
          <tr className="border-y border-line bg-surface-2/60 text-left text-[11px] tracking-wide text-ink-3 uppercase">
            <th className="px-5 py-2 font-medium">Regra</th>
            <th className="px-3 py-2 font-medium">Categoria</th>
            <th className="px-3 py-2 font-medium">Origem</th>
            <th className="px-3 py-2 text-right font-medium">Acertos / erros</th>
            <th className="px-3 py-2 font-medium">Ativa</th>
            <th className="px-3 py-2" />
          </tr>
        </thead>
        <tbody className="divide-y divide-line">
          {(rules ?? []).map((r) => (
            <tr key={r.id} className={clsx(!r.enabled && 'opacity-60')}>
              <td className="px-5 py-2.5">
                <span className="text-xs text-ink-3">{MATCHERS.find((m) => m.value === r.matcher)?.label}</span>
                <span className="ml-2 font-mono text-[13px]">{r.pattern}</span>
              </td>
              <td className="px-3 py-2.5">
                <CategoryChip categories={categories} categoryId={r.category_id} />
              </td>
              <td className="px-3 py-2.5">
                <Badge tone={r.origin === 'learned' ? 'violet' : 'brand'}>{r.origin === 'learned' ? 'Aprendida' : 'Sua'}</Badge>
              </td>
              <td className="px-3 py-2.5 text-right text-xs tabular-nums text-ink-2">
                <span className="text-emerald-600 dark:text-emerald-400">{r.hit_count}</span> / <span className="text-red-500">{r.miss_count}</span>
                {r.last_contradicted_at && <span className="ml-1 text-ink-3" title={`Última contradição: ${fmtDateTime(r.last_contradicted_at)}`}>⚠</span>}
              </td>
              <td className="px-3 py-2.5">
                <Toggle size="sm" checked={r.enabled} onChange={(v) => void toggle(r, v)} label={`Ativar regra ${r.pattern}`} />
              </td>
              <td className="px-3 py-2.5 text-right">
                <IconButton label="Remover regra" size="sm" onClick={() => void remove(r)}>
                  <Trash2 className="size-3.5" />
                </IconButton>
              </td>
            </tr>
          ))}
          {!rules?.length && (
            <tr>
              <td colSpan={6} className="px-5 py-6 text-center text-xs text-ink-3">
                Nenhuma regra ainda. Corrija blocos na Revisão e aceite as sugestões, ou crie uma manualmente.
              </td>
            </tr>
          )}
        </tbody>
      </table>

      <Dialog
        open={open}
        onClose={() => setOpen(false)}
        title="Nova regra"
        width="sm"
        footer={
          <>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancelar
            </Button>
            <Button variant="primary" onClick={() => void create()} loading={busy} disabled={!pattern.trim()}>
              Criar
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
