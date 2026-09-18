import clsx from 'clsx';
import { Camera, Plus, Scissors } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Badge } from '../components/ui/Badge';
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
import { assignableCategories, categoryColor, isUncategorized } from '../lib/categories';
import { fmtDateLong, fmtDuration, fmtTime, localTimeToIso } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { ActivityBlock, Id, RuleSuggestion } from '../lib/types';
import { useAsync } from '../lib/useAsync';

const secs = (b: ActivityBlock) => Math.max(0, (new Date(b.ended_at).getTime() - new Date(b.started_at).getTime()) / 1000);
const hourOf = (iso: string) => new Date(iso).getHours();

/** One sentence from the numbers: how much of the day is on the track and how much still waits. */
function summary(blocks: ActivityBlock[]): string {
  if (!blocks.length) return 'Nenhum bloco registrado neste dia.';
  const total = blocks.reduce((s, b) => s + secs(b), 0);
  const open = blocks.filter((b) => isUncategorized(b.category_id) || b.needs_review).length;
  const first = `${fmtDuration(total, { compact: true })} em ${blocks.length} ${blocks.length === 1 ? 'bloco' : 'blocos'}`;
  if (open === 0) return `${first}; tudo classificado.`;
  return `${first}; ${open} ${open === 1 ? 'ainda espera' : 'ainda esperam'} uma categoria.`;
}

export function Timeline() {
  const date = useAppStore((s) => s.date);
  const setDate = useAppStore((s) => s.setDate);
  const categories = useAppStore((s) => s.categories);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const bumpData = useAppStore((s) => s.bumpData);
  const settingsView = useAppStore((s) => s.settingsView);
  const toast = useToast();
  const [params, setParams] = useSearchParams();
  const highlight = params.get('block');

  const { data: blocks, loading, reload, setData } = useAsync(() => ipc.getTimeline(date), [date, dataVersion]);

  const [filterCat, setFilterCat] = useState<string>('all');
  const [onlyUnclassified, setOnlyUnclassified] = useState(false);
  const [selected, setSelected] = useState<Id | null>(null);
  const [openPicker, setOpenPicker] = useState<Id | null>(null);
  const [suggestions, setSuggestions] = useState<Record<Id, RuleSuggestion[]>>({});
  const [accepted, setAccepted] = useState<Set<string>>(new Set());
  const [splitting, setSplitting] = useState<ActivityBlock | null>(null);
  const [manualOpen, setManualOpen] = useState(false);

  // ?block=id (from Hoje): select the block, bring it into view, then drop the param.
  useEffect(() => {
    if (!highlight || !blocks) return;
    setSelected(highlight);
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

  const filtered = filterCat !== 'all' || onlyUnclassified;

  // macOS applies Screen Recording only after a relaunch: granted permission + empty titles = restart.
  const needsRestart = useMemo(() => {
    if (!blocks?.length || settingsView?.permissions.screen_recording !== 'granted') return false;
    const real = blocks.filter((b) => b.app_id !== 'idle' && b.app_id !== 'private' && !b.is_manual);
    return real.length > 0 && real.every((b) => !b.title);
  }, [blocks, settingsView]);

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
        subtitle={blocks ? `${fmtDateLong(date)}: ${summary(blocks)}` : fmtDateLong(date)}
        actions={
          <>
            <DayNav date={date} onChange={setDate} />
            <Button variant="primary" icon={<Plus className="size-[18px]" strokeWidth={1.75} aria-hidden />} onClick={() => setManualOpen(true)}>
              Adicionar atividade manual
            </Button>
          </>
        }
      />

      {needsRestart && (
        <div role="alert" className="mb-5 flex flex-wrap items-center gap-3 rounded-card border border-ember/30 bg-ember-soft px-4 py-3 text-sm">
          <Camera className="size-[18px] shrink-0 text-ember" strokeWidth={1.75} aria-hidden />
          <p className="min-w-0 flex-1 text-ink">
            Os títulos das janelas estão chegando vazios. O macOS só aplica a Gravação de Tela depois que o app reinicia.
          </p>
          <Button variant="accent" size="sm" onClick={() => void ipc.restartApp()}>
            Reiniciar o ubiqX
          </Button>
        </div>
      )}

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <label className="sr-only" htmlFor="filter-cat">
          Filtrar por categoria
        </label>
        <div className="w-56">
          <Select id="filter-cat" value={filterCat} onChange={(e) => setFilterCat(e.target.value)}>
            <option value="all">Todas as categorias</option>
            <option value="none">Sem categoria</option>
            {assignableCategories(categories).map((c) => (
              <option key={c.id} value={c.id}>
                {c.name}
              </option>
            ))}
          </Select>
        </div>
        <button
          type="button"
          aria-pressed={onlyUnclassified}
          onClick={() => setOnlyUnclassified((v) => !v)}
          className={clsx(
            'inline-flex h-9 items-center gap-2 rounded-control border px-3 text-sm font-medium transition-colors duration-150',
            onlyUnclassified ? 'border-volt/40 bg-volt-soft text-volt' : 'border-line-2 bg-panel text-ink-2 hover:bg-panel-2 hover:text-ink',
          )}
        >
          <span className={clsx('size-1.5 rounded-full', onlyUnclassified ? 'bg-volt' : 'bg-ink-4')} aria-hidden />
          Somente não classificados
        </button>
        <span className="num ml-auto text-xs text-ink-3">
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
          <EmptyState
            title={filtered ? 'Nenhum bloco combina com os filtros' : 'Nenhum bloco neste dia'}
            description={filtered ? 'Limpe os filtros para ver o dia inteiro.' : 'Quando o rastreador registrar algo, os blocos aparecem aqui. Você também pode adicionar uma atividade manual.'}
            action={
              filtered ? (
                <Button
                  onClick={() => {
                    setFilterCat('all');
                    setOnlyUnclassified(false);
                  }}
                >
                  Limpar filtros
                </Button>
              ) : (
                <Button variant="primary" icon={<Plus className="size-[18px]" strokeWidth={1.75} aria-hidden />} onClick={() => setManualOpen(true)}>
                  Adicionar atividade manual
                </Button>
              )
            }
          />
        </Card>
      ) : (
        <Card padded={false} className="overflow-hidden">
          <ol className="py-2" aria-label="Blocos do dia">
            {visible.map((b, i) => {
              const isPrivate = b.app_id === 'private';
              const isIdle = b.app_id === 'idle';
              const muted = isPrivate || isIdle;
              const prev = visible[i - 1];
              const newHour = !prev || hourOf(prev.started_at) !== hourOf(b.started_at);
              const color = muted ? 'var(--ink-4)' : categoryColor(categories, b.category_id);
              const isSelected = selected === b.id;
              const dur = secs(b);
              return (
                <li key={b.id}>
                  {newHour && (
                    <div className="flex items-center gap-3 px-5 pt-3 pb-1" aria-hidden>
                      <span className="num w-[76px] shrink-0 text-right text-[11px] font-medium text-ink-3">{String(hourOf(b.started_at)).padStart(2, '0')}:00</span>
                      <span className="h-px flex-1 bg-line" />
                    </div>
                  )}
                  <article
                    id={`block-${b.id}`}
                    data-selected={isSelected || undefined}
                    onClick={() => setSelected(b.id)}
                    className={clsx('group grid grid-cols-[76px_3px_1fr] gap-x-3 px-5 py-2 transition-colors duration-150', isSelected ? 'bg-volt-soft' : 'hover:bg-panel-2/60')}
                    aria-label={`${fmtTime(b.started_at)} a ${b.is_open ? 'agora' : fmtTime(b.ended_at)}, ${b.app_name}`}
                  >
                    {/* time gutter */}
                    <div className="num pt-0.5 text-right" title={`${fmtTime(b.started_at)} até ${b.is_open ? 'agora' : fmtTime(b.ended_at)}`}>
                      <p className={clsx('text-[13px] leading-5 font-medium', muted ? 'text-ink-3' : 'text-ink')}>{fmtTime(b.started_at)}</p>
                      <p className="flex items-center justify-end gap-1.5 text-[11px] leading-4 text-ink-3">
                        {b.is_open && <span className="size-1.5 animate-pulse rounded-full bg-signal motion-reduce:animate-none" aria-hidden />}
                        {b.is_open ? 'em andamento' : fmtDuration(dur)}
                      </p>
                    </div>

                    {/* category rail */}
                    <span
                      className="my-0.5 w-[3px] rounded-pill transition-[box-shadow] duration-150"
                      style={{ background: color, opacity: muted ? 0.5 : 1, boxShadow: isSelected ? `0 0 12px 1px ${color}` : undefined }}
                      aria-hidden
                    />

                    {/* content */}
                    <div className={clsx('flex min-w-0 gap-3', muted && 'opacity-70')}>
                      <AppAvatar name={b.app_name} className="mt-0.5" />
                      <div className="min-w-0 flex-1">
                        <div className="flex items-baseline gap-x-2 gap-y-0.5">
                          <p className="min-w-0 truncate text-sm leading-5 font-medium text-ink">{isPrivate ? 'Modo privado' : isIdle ? 'Ocioso' : b.title || b.app_name}</p>
                          {b.is_manual && <Badge tone="neutral">manual</Badge>}
                        </div>
                        {!muted && (b.title || b.domain) && (
                          <p className="flex flex-wrap gap-x-3 text-xs leading-4 text-ink-3">
                            {b.title && <span className="truncate">{b.app_name}</span>}
                            {b.domain && <span className="truncate">{b.domain}</span>}
                          </p>
                        )}
                        {b.description && <p className="mt-1 text-xs leading-4 text-ink-2">{b.description}</p>}
                        <div className="mt-2 flex flex-wrap items-center gap-2">
                          <Popover
                            open={openPicker === b.id}
                            onClose={() => setOpenPicker(null)}
                            anchor={
                              <CategoryChip
                                categories={categories}
                                categoryId={b.category_id}
                                interactive={!muted}
                                aria-haspopup="listbox"
                                aria-expanded={openPicker === b.id}
                                onClick={() => setOpenPicker(openPicker === b.id ? null : b.id)}
                              />
                            }
                          >
                            <p className="px-2 pt-1 pb-1.5 text-[11px] font-medium text-ink-3">Reclassificar este bloco</p>
                            <CategoryPicker categories={categories} value={b.category_id} onPick={(id) => void reclassify(b, id)} />
                          </Popover>
                          {!muted && <ConfidenceBar value={b.confidence} />}
                          <SourceBadge source={b.source} />
                          <AiSentBadge at={b.ai_sent_at} />
                          {b.needs_review && <Badge tone="ember">precisa de revisão</Badge>}
                        </div>
                        {suggestions[b.id] && (
                          <div className="mt-2">
                            <SuggestionChips suggestions={suggestions[b.id] ?? []} categories={categories} accepted={accepted} onAccept={(s) => void acceptSuggestion(s)} />
                          </div>
                        )}
                      </div>
                      {!muted && !b.is_open && (
                        <IconButton
                          label="Dividir bloco"
                          size="sm"
                          onClick={(e) => {
                            e.stopPropagation();
                            setSplitting(b);
                          }}
                          className={clsx('self-start transition-opacity duration-150', !isSelected && 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100')}
                        >
                          <Scissors className="size-3.5" strokeWidth={1.75} />
                        </IconButton>
                      )}
                    </div>
                  </article>
                </li>
              );
            })}
          </ol>
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
      description={block ? `${block.app_name}, das ${fmtTime(block.started_at)} às ${fmtTime(block.ended_at)}` : undefined}
      width="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancelar
          </Button>
          <Button variant="primary" icon={<Scissors className="size-4" strokeWidth={1.75} aria-hidden />} onClick={() => void submit()} loading={busy}>
            Dividir
          </Button>
        </>
      }
    >
      <Field label="Dividir em" hint="O bloco será separado neste horário; as duas partes mantêm a categoria.">
        {(id) => <Input id={id} type="time" className="num" value={time} onChange={(e) => setTime(e.target.value)} />}
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
        <Field label="Início">{(id) => <Input id={id} type="time" className="num" value={start} onChange={(e) => setStart(e.target.value)} />}</Field>
        <Field label="Fim">{(id) => <Input id={id} type="time" className="num" value={end} onChange={(e) => setEnd(e.target.value)} />}</Field>
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
