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
import { useT } from '../i18n';
import { assignableCategories, categoryColor, categoryLabel, isUncategorized } from '../lib/categories';
import { fmtDateLong, fmtDuration, fmtTime, localTimeToIso } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { ActivityBlock, Id, RuleSuggestion } from '../lib/types';
import { useAsync } from '../lib/useAsync';

type Translate = ReturnType<typeof useT>;

const secs = (b: ActivityBlock) => Math.max(0, (new Date(b.ended_at).getTime() - new Date(b.started_at).getTime()) / 1000);
const hourOf = (iso: string) => new Date(iso).getHours();
/** Private-mode blocks: the engine stores app_id 'privado' (the mock uses 'private'). */
const isPrivateBlock = (b: ActivityBlock) => b.app_id === 'private' || b.app_id === 'privado';
/** App name for labels: the localized 'Modo privado' / 'Ocioso' instead of the stored placeholder names. */
const appLabel = (b: ActivityBlock, t: Translate): string => (isPrivateBlock(b) ? t('timeline.private_mode') : b.app_id === 'idle' ? t('timeline.idle') : b.app_name);

/** One sentence from the numbers: how much of the day is on the track and how much still waits. */
function summary(t: Translate, blocks: ActivityBlock[]): string {
  if (!blocks.length) return t('timeline.summary_empty');
  const total = blocks.reduce((s, b) => s + secs(b), 0);
  const open = blocks.filter((b) => isUncategorized(b.category_id) || b.needs_review).length;
  const first = t('timeline.summary_total', { duration: fmtDuration(total, { compact: true }), count: blocks.length });
  if (open === 0) return t('timeline.summary_all_classified', { total: first });
  return t('timeline.summary_pending', { total: first, count: open });
}

export function Timeline() {
  const t = useT();
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

  // ?block=id (from the Today page): select the block, bring it into view, then drop the param.
  useEffect(() => {
    if (!highlight || !blocks) return;
    setSelected(highlight);
    const el = document.getElementById(`block-${highlight}`);
    el?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    const timer = setTimeout(() => setParams({}, { replace: true }), 2500);
    return () => clearTimeout(timer);
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
    const real = blocks.filter((b) => b.app_id !== 'idle' && !isPrivateBlock(b) && !b.is_manual);
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
        toast.success(t('timeline.toast_reclassified'), outcome.backfilled ? t('timeline.toast_backfilled', { count: outcome.backfilled }) : undefined);
        if (outcome.disabled_rules.length) {
          toast.info(t('timeline.toast_rule_disabled'), t('timeline.toast_rule_disabled_body', { pattern: outcome.disabled_rules[0]?.pattern ?? '' }));
        }
        bumpData();
      } catch (e) {
        toast.error(t('timeline.toast_reclassify_failed'), e instanceof Error ? e.message : String(e));
      }
    },
    [blocks, setData, toast, bumpData, t],
  );

  const acceptSuggestion = async (s: RuleSuggestion) => {
    try {
      await ipc.acceptRuleSuggestion(s);
      setAccepted((a) => new Set(a).add(`${s.matcher}:${s.pattern}`));
      toast.success(t('timeline.toast_rule_created'), t('timeline.toast_rule_created_body', { pattern: s.pattern }));
    } catch (e) {
      toast.error(t('timeline.toast_rule_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  const addManualButton = (
    <Button variant="primary" icon={<Plus className="size-[18px]" strokeWidth={1.75} aria-hidden />} onClick={() => setManualOpen(true)}>
      {t('timeline.add_manual')}
    </Button>
  );

  return (
    <div data-testid="page-timeline">
      <PageHeader
        title={t('timeline.title')}
        subtitle={blocks ? t('timeline.subtitle', { date: fmtDateLong(date), summary: summary(t, blocks) }) : fmtDateLong(date)}
        actions={
          <>
            <DayNav date={date} onChange={setDate} />
            {addManualButton}
          </>
        }
      />

      {needsRestart && (
        <div role="alert" className="mb-5 flex flex-wrap items-center gap-3 rounded-card border border-ember/30 bg-ember-soft px-4 py-3 text-sm">
          <Camera className="size-[18px] shrink-0 text-ember" strokeWidth={1.75} aria-hidden />
          <p className="min-w-0 flex-1 text-ink">{t('timeline.restart_notice')}</p>
          <Button variant="accent" size="sm" onClick={() => void ipc.restartApp()}>
            {t('timeline.restart_button')}
          </Button>
        </div>
      )}

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <label className="sr-only" htmlFor="filter-cat">
          {t('timeline.filter_label')}
        </label>
        <div className="w-56">
          <Select id="filter-cat" value={filterCat} onChange={(e) => setFilterCat(e.target.value)}>
            <option value="all">{t('timeline.filter_all')}</option>
            <option value="none">{t('common.uncategorized')}</option>
            {assignableCategories(categories).map((c) => (
              <option key={c.id} value={c.id}>
                {categoryLabel(c)}
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
          {t('timeline.only_unclassified')}
        </button>
        <span className="num ml-auto text-xs text-ink-3">{t('timeline.shown_count', { shown: visible.length, count: blocks?.length ?? 0 })}</span>
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
            title={filtered ? t('timeline.empty_filtered_title') : t('timeline.empty_title')}
            description={filtered ? t('timeline.empty_filtered_desc') : t('timeline.empty_desc')}
            action={
              filtered ? (
                <Button
                  onClick={() => {
                    setFilterCat('all');
                    setOnlyUnclassified(false);
                  }}
                >
                  {t('timeline.clear_filters')}
                </Button>
              ) : (
                addManualButton
              )
            }
          />
        </Card>
      ) : (
        <Card padded={false} className="overflow-hidden">
          <ol className="py-2" aria-label={t('timeline.list_label')}>
            {visible.map((b, i) => {
              const isPrivate = isPrivateBlock(b);
              const isIdle = b.app_id === 'idle';
              const muted = isPrivate || isIdle;
              const prev = visible[i - 1];
              const newHour = !prev || hourOf(prev.started_at) !== hourOf(b.started_at);
              const color = muted ? 'var(--ink-4)' : categoryColor(categories, b.category_id);
              const isSelected = selected === b.id;
              const dur = secs(b);
              const start = fmtTime(b.started_at);
              const end = b.is_open ? t('common.now') : fmtTime(b.ended_at);
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
                    aria-label={t('timeline.block_label', { start, end, app: appLabel(b, t) })}
                  >
                    {/* time gutter */}
                    <div className="num pt-0.5 text-right" title={t('timeline.block_range', { start, end })}>
                      <p className={clsx('text-[13px] leading-5 font-medium', muted ? 'text-ink-3' : 'text-ink')}>{start}</p>
                      <p className="flex items-center justify-end gap-1.5 text-[11px] leading-4 text-ink-3">
                        {b.is_open && <span className="size-1.5 animate-pulse rounded-full bg-signal motion-reduce:animate-none" aria-hidden />}
                        {b.is_open ? t('timeline.in_progress') : fmtDuration(dur)}
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
                          <p className="min-w-0 truncate text-sm leading-5 font-medium text-ink">
                            {isPrivate ? t('timeline.private_mode') : isIdle ? t('timeline.idle') : b.title || b.app_name}
                          </p>
                          {b.is_manual && <Badge tone="neutral">{t('timeline.manual_badge')}</Badge>}
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
                            <p className="px-2 pt-1 pb-1.5 text-[11px] font-medium text-ink-3">{t('timeline.reclassify_heading')}</p>
                            <CategoryPicker categories={categories} value={b.category_id} onPick={(id) => void reclassify(b, id)} />
                          </Popover>
                          {!muted && <ConfidenceBar value={b.confidence} />}
                          <SourceBadge source={b.source} />
                          <AiSentBadge at={b.ai_sent_at} />
                          {b.needs_review && <Badge tone="ember">{t('timeline.needs_review')}</Badge>}
                        </div>
                        {suggestions[b.id] && (
                          <div className="mt-2">
                            <SuggestionChips suggestions={suggestions[b.id] ?? []} categories={categories} accepted={accepted} onAccept={(s) => void acceptSuggestion(s)} />
                          </div>
                        )}
                      </div>
                      {!muted && !b.is_open && (
                        <IconButton
                          label={t('timeline.split_action')}
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
  const t = useT();
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
      toast.success(t('timeline.toast_split'));
      onDone();
      onClose();
    } catch (e) {
      toast.error(t('timeline.toast_split_failed'), e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={!!block}
      onClose={onClose}
      title={t('timeline.split_action')}
      description={block ? t('timeline.split_description', { app: appLabel(block, t), start: fmtTime(block.started_at), end: fmtTime(block.ended_at) }) : undefined}
      width="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button variant="primary" icon={<Scissors className="size-4" strokeWidth={1.75} aria-hidden />} onClick={() => void submit()} loading={busy}>
            {t('timeline.split_confirm')}
          </Button>
        </>
      }
    >
      <Field label={t('timeline.split_at')} hint={t('timeline.split_hint')}>
        {(id) => <Input id={id} type="time" className="num" value={time} onChange={(e) => setTime(e.target.value)} />}
      </Field>
    </Dialog>
  );
}

function ManualDialog({ open, onClose, date, onDone }: { open: boolean; onClose: () => void; date: string; onDone: () => void }) {
  const t = useT();
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
      toast.error(t('timeline.toast_invalid_time'), t('timeline.toast_invalid_time_body'));
      return;
    }
    setBusy(true);
    try {
      await ipc.addManualEntry(localTimeToIso(date, start), localTimeToIso(date, end), categoryId, note || undefined);
      toast.success(t('timeline.toast_manual_added'));
      setNote('');
      onDone();
      onClose();
    } catch (e) {
      toast.error(t('timeline.toast_manual_failed'), e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={t('timeline.add_manual')}
      description={t('timeline.manual_description')}
      width="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {t('common.cancel')}
          </Button>
          <Button variant="primary" onClick={() => void submit()} loading={busy}>
            {t('common.add')}
          </Button>
        </>
      }
    >
      <div className="grid grid-cols-2 gap-3">
        <Field label={t('timeline.manual_start')}>{(id) => <Input id={id} type="time" className="num" value={start} onChange={(e) => setStart(e.target.value)} />}</Field>
        <Field label={t('timeline.manual_end')}>{(id) => <Input id={id} type="time" className="num" value={end} onChange={(e) => setEnd(e.target.value)} />}</Field>
        <Field label={t('common.category')} className="col-span-2">
          {(id) => (
            <Select id={id} value={categoryId} onChange={(e) => setCategoryId(e.target.value)}>
              {list.map((c) => (
                <option key={c.id} value={c.id}>
                  {categoryLabel(c)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t('timeline.manual_note')} className="col-span-2" hint={t('timeline.manual_note_hint')}>
          {(id) => <Input id={id} value={note} onChange={(e) => setNote(e.target.value)} placeholder={t('timeline.manual_note_placeholder')} />}
        </Field>
      </div>
    </Dialog>
  );
}
