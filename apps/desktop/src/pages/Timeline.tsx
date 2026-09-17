import clsx from 'clsx';
import { Filter, Plus, Scissors } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { AiSentBadge, AppAvatar, ConfidenceBar, SourceBadge, SuggestionChips } from '../components/ui/BlockBits';
import { Button, IconButton } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { CategoryChip, CategoryPicker } from '../components/ui/CategoryChip';
import { DayNav } from '../components/ui/DayNav';
import { Dialog } from '../components/ui/Dialog';
import { Field, Input, Select } from '../components/ui/Field';
import { EmptyState, Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { Popover } from '../components/ui/Popover';
import { assignableCategories, isUncategorized } from '../lib/categories';
import { fmtDateLong, fmtDuration, fmtTime, localTimeToIso } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { ActivityBlock, Id, RuleSuggestion } from '../lib/types';
import { useAsync } from '../lib/useAsync';

const secs = (b: ActivityBlock) => Math.max(0, (new Date(b.ended_at).getTime() - new Date(b.started_at).getTime()) / 1000);

export function Timeline() {
  const date = useAppStore((s) => s.date);
  const setDate = useAppStore((s) => s.setDate);
  const categories = useAppStore((s) => s.categories);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const bumpData = useAppStore((s) => s.bumpData);
  const toast = useToast();
  const [params, setParams] = useSearchParams();
  const highlight = params.get('block');

  const { data: blocks, loading, reload, setData } = useAsync(() => ipc.getTimeline(date), [date, dataVersion]);

  const [filterCat, setFilterCat] = useState<string>('all');
  const [onlyUnclassified, setOnlyUnclassified] = useState(false);
  const [openPicker, setOpenPicker] = useState<Id | null>(null);
  const [suggestions, setSuggestions] = useState<Record<Id, RuleSuggestion[]>>({});
  const [accepted, setAccepted] = useState<Set<string>>(new Set());
  const [splitting, setSplitting] = useState<ActivityBlock | null>(null);
  const [manualOpen, setManualOpen] = useState(false);

  useEffect(() => {
    if (!highlight || !blocks) return;
    const el = document.getElementById(`block-${highlight}`);
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    const t = setTimeout(() => setParams({}, { replace: true }), 2500);
    return () => clearTimeout(t);
  }, [highlight, blocks, setParams]);

  const visible = useMemo(() => {
    if (!blocks) return [];
    return blocks.filter((b) => {
      if (onlyUnclassified && !isUncategorized(b.category_id) && !b.needs_review) return false;
      if (filterCat !== 'all' && (b.category_id ?? 'none') !== filterCat) return false;
      return true;
    });
  }, [blocks, onlyUnclassified, filterCat]);

  const reclassify = useCallback(
    async (block: ActivityBlock, categoryId: Id) => {
      setOpenPicker(null);
      try {
        const outcome = await ipc.reclassify(block.id, categoryId, 'block');
        if (blocks) {
          setData(blocks.map((b) => (outcome.block_ids.includes(b.id) ? { ...b, category_id: categoryId, confidence: 1, source: 'user', needs_review: false } : b)));
        }
        setSuggestions((s) => ({ ...s, [block.id]: outcome.suggestions }));
        toast.success('Categoria atualizada', outcome.backfilled ? `${outcome.backfilled} bloco(s) semelhante(s) também foram ajustados.` : undefined);
        if (outcome.disabled_rules.length) toast.info('Regra desativada', `A regra “${outcome.disabled_rules[0]?.pattern}” foi contradita e desativada.`);
        bumpData();
      } catch (e) {
        toast.error('Não foi possível reclassificar', e instanceof Error ? e.message : String(e));
      }
    },
    [blocks, setData, toast, bumpData],
  );

  const acceptSuggestion = async (s: RuleSuggestion) => {
    try {
      await ipc.acceptRuleSuggestion(s);
      setAccepted((a) => new Set(a).add(`${s.matcher}:${s.pattern}`));
      toast.success('Regra criada', `${s.pattern} → sempre a mesma categoria.`);
    } catch (e) {
      toast.error('Não foi possível criar a regra', e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div data-testid="page-timeline">
      <PageHeader
        title="Timeline"
        subtitle={fmtDateLong(date)}
        actions={
          <>
            <DayNav date={date} onChange={setDate} />
            <Button variant="primary" icon={<Plus className="size-4" />} onClick={() => setManualOpen(true)}>
              Adicionar atividade manual
            </Button>
          </>
        }
      />

      <div className="mb-4 flex flex-wrap items-center gap-3">
        <div className="flex items-center gap-2">
          <Filter className="size-4 text-ink-3" />
          <label className="sr-only" htmlFor="filter-cat">
            Filtrar por categoria
          </label>
          <Select id="filter-cat" value={filterCat} onChange={(e) => setFilterCat(e.target.value)} className="w-52">
            <option value="all">Todas as categorias</option>
            <option value="none">Sem categoria</option>
            {assignableCategories(categories).map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </Select>
        </div>
        <label className="flex items-center gap-2 text-sm text-ink-2">
          <input type="checkbox" className="size-4 accent-brand-600" checked={onlyUnclassified} onChange={(e) => setOnlyUnclassified(e.target.checked)} />
          Somente não classificados
        </label>
        <span className="ml-auto text-xs text-ink-3">
          {visible.length} de {blocks?.length ?? 0} blocos
        </span>
      </div>

      {loading && !blocks ? (
        <div className="flex flex-col gap-2">
          {[...Array(6)].map((_, i) => (
            <Skeleton key={i} className="h-16" />
          ))}
        </div>
      ) : !visible.length ? (
        <Card>
          <EmptyState title="Nenhum bloco para mostrar" description="Ajuste os filtros ou escolha outro dia." />
        </Card>
      ) : (
        <Card padded={false} className="divide-y divide-line">
          {visible.map((b) => {
            const isPrivate = b.app_id === 'private';
            const isIdle = b.app_id === 'idle';
            return (
              <article
                key={b.id}
                id={`block-${b.id}`}
                className={clsx('flex gap-4 px-4 py-3 transition-colors', highlight === b.id && 'bg-brand-50 dark:bg-brand-900/20', (isPrivate || isIdle) && 'opacity-70')}
                aria-label={`${fmtTime(b.started_at)} a ${fmtTime(b.ended_at)}, ${b.app_name}`}
              >
                <div className="w-24 shrink-0 pt-0.5 text-xs tabular-nums text-ink-2">
                  <p className="font-medium text-ink">
                    {fmtTime(b.started_at)} – {b.is_open ? 'agora' : fmtTime(b.ended_at)}
                  </p>
                  <p className="text-ink-3">{fmtDuration(secs(b))}</p>
                </div>
                <AppAvatar name={b.app_name} />
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                    <p className="truncate text-sm font-medium">{isPrivate ? 'Modo privado' : isIdle ? 'Ocioso' : b.title || b.app_name}</p>
                    {b.domain && <span className="truncate text-xs text-ink-3">{b.domain}</span>}
                    {b.is_manual && <span className="text-[11px] text-ink-3">· manual</span>}
                    {b.is_open && <span className="size-1.5 animate-pulse rounded-full bg-emerald-500" title="Bloco em andamento" />}
                  </div>
                  <p className="mt-0.5 truncate text-xs text-ink-3">{b.title && !isPrivate ? b.app_name : ''}</p>
                  {b.description && <p className="mt-1 text-xs text-ink-2 italic">{b.description}</p>}
                  <div className="mt-2 flex flex-wrap items-center gap-2">
                    <Popover
                      open={openPicker === b.id}
                      onClose={() => setOpenPicker(null)}
                      anchor={
                        <CategoryChip
                          categories={categories}
                          categoryId={b.category_id}
                          interactive={!isPrivate && !isIdle}
                          aria-haspopup="listbox"
                          aria-expanded={openPicker === b.id}
                          onClick={() => setOpenPicker(openPicker === b.id ? null : b.id)}
                        />
                      }
                    >
                      <p className="px-2 pt-1 pb-1.5 text-[11px] font-medium text-ink-3">Reclassificar este bloco</p>
                      <CategoryPicker categories={categories} value={b.category_id} onPick={(id) => void reclassify(b, id)} />
                    </Popover>
                    {!isPrivate && !isIdle && <ConfidenceBar value={b.confidence} />}
                    <SourceBadge source={b.source} />
                    <AiSentBadge at={b.ai_sent_at} />
                    {b.needs_review && (
                      <span className="inline-flex h-5 items-center rounded-md bg-amber-100 px-1.5 text-[11px] font-medium text-amber-800 dark:bg-amber-900/40 dark:text-amber-200">precisa de revisão</span>
                    )}
                  </div>
                  {suggestions[b.id] && (
                    <div className="mt-2">
                      <SuggestionChips suggestions={suggestions[b.id] ?? []} categories={categories} accepted={accepted} onAccept={(s) => void acceptSuggestion(s)} />
                    </div>
                  )}
                </div>
                {!isPrivate && !isIdle && !b.is_open && (
                  <IconButton label="Dividir bloco" size="sm" onClick={() => setSplitting(b)} className="self-start">
                    <Scissors className="size-3.5" />
                  </IconButton>
                )}
              </article>
            );
          })}
        </Card>
      )}

      <SplitDialog block={splitting} onClose={() => setSplitting(null)} onDone={() => void reload()} />
      <ManualDialog open={manualOpen} onClose={() => setManualOpen(false)} date={date} onDone={() => void reload()} />
    </div>
  );
}

function SplitDialog({ block, onClose, onDone }: { block: ActivityBlock | null; onClose: () => void; onDone: () => void }) {
  const [time, setTime] = useState('');
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const date = useAppStore((s) => s.date);

  useEffect(() => {
    if (!block) return;
    const mid = new Date((new Date(block.started_at).getTime() + new Date(block.ended_at).getTime()) / 2);
    setTime(`${String(mid.getHours()).padStart(2, '0')}:${String(mid.getMinutes()).padStart(2, '0')}`);
  }, [block]);

  const submit = async () => {
    if (!block || !time) return;
    setBusy(true);
    try {
      await ipc.splitBlock(block.id, localTimeToIso(date, time));
      toast.success('Bloco dividido');
      onDone();
      onClose();
    } catch (e) {
      toast.error('Não foi possível dividir', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={!!block}
      onClose={onClose}
      title="Dividir bloco"
      description={block ? `${fmtTime(block.started_at)} – ${fmtTime(block.ended_at)} · ${block.app_name}` : undefined}
      width="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancelar
          </Button>
          <Button variant="primary" onClick={() => void submit()} loading={busy}>
            Dividir
          </Button>
        </>
      }
    >
      <Field label="Dividir em" hint="O bloco será separado neste horário; ambas as partes mantêm a categoria.">
        {(id) => <Input id={id} type="time" value={time} onChange={(e) => setTime(e.target.value)} />}
      </Field>
    </Dialog>
  );
}

function ManualDialog({ open, onClose, date, onDone }: { open: boolean; onClose: () => void; date: string; onDone: () => void }) {
  const categories = useAppStore((s) => s.categories);
  const [start, setStart] = useState('09:00');
  const [end, setEnd] = useState('10:00');
  const [categoryId, setCategoryId] = useState('');
  const [note, setNote] = useState('');
  const [busy, setBusy] = useState(false);
  const toast = useToast();
  const list = assignableCategories(categories);

  useEffect(() => {
    if (open && !categoryId && list[0]) setCategoryId(list[0].id);
  }, [open, categoryId, list]);

  const submit = async () => {
    if (!categoryId) return;
    if (end <= start) {
      toast.error('Horário inválido', 'O fim precisa ser depois do início.');
      return;
    }
    setBusy(true);
    try {
      await ipc.addManualEntry(localTimeToIso(date, start), localTimeToIso(date, end), categoryId, note || undefined);
      toast.success('Atividade adicionada');
      setNote('');
      onDone();
      onClose();
    } catch (e) {
      toast.error('Não foi possível adicionar', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Adicionar atividade manual"
      description="Para reuniões presenciais, leituras no papel ou qualquer coisa fora do computador."
      width="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancelar
          </Button>
          <Button variant="primary" onClick={() => void submit()} loading={busy}>
            Adicionar
          </Button>
        </>
      }
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label="Início">{(id) => <Input id={id} type="time" value={start} onChange={(e) => setStart(e.target.value)} />}</Field>
        <Field label="Fim">{(id) => <Input id={id} type="time" value={end} onChange={(e) => setEnd(e.target.value)} />}</Field>
        <Field label="Categoria" className="col-span-2">
          {(id) => (
            <Select id={id} value={categoryId} onChange={(e) => setCategoryId(e.target.value)}>
              {list.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label="Observação" className="col-span-2" hint="Vai para o relatório do dia.">
          {(id) => <Input id={id} value={note} onChange={(e) => setNote(e.target.value)} placeholder="Ex.: Banca de TCC presencial" />}
        </Field>
      </div>
    </Dialog>
  );
}
