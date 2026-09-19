import clsx from 'clsx';
import { AnimatePresence, motion, useIsPresent, useReducedMotion } from 'framer-motion';
import { Check, ChevronDown, Wand2 } from 'lucide-react';
import { Fragment, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { useLicenseGate } from '../components/layout/LicenseBanner';
import { Ubi } from '../components/ubi/Ubi';
import { AppAvatar, SourceBadge, SuggestionChips } from '../components/ui/BlockBits';
import { BlockDetails } from '../components/ui/BlockDetails';
import { Button } from '../components/ui/Button';
import { Card, CardHeader } from '../components/ui/Card';
import { DayNav } from '../components/ui/DayNav';
import { Kbd, Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { useLocale, useT } from '../i18n';
import { assignableCategories, categoryById, categoryLabel, iconFor, isUncategorized, UNCATEGORIZED_COLOR } from '../lib/categories';
import { fmtDateLong, fmtDuration, fmtMinutes, fmtPercent, fmtTime } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { ActivityBlock, BlockGroup, Category, Id, RuleSuggestion } from '../lib/types';
import { useAsync } from '../lib/useAsync';

type Translate = ReturnType<typeof useT>;

const isTypingTarget = (t: EventTarget | null): boolean => {
  const el = t as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable;
};

/** A group the user has not decided yet. Everything else was reviewed and collects at the bottom. */
const isPending = (g: BlockGroup, decided: Set<string>): boolean => g.source !== 'user' && !decided.has(g.key);

/** Queue order: what the classifier could not settle first (uncategorized, needs review), then by rising confidence. */
const queueRank = (g: BlockGroup): number => (isUncategorized(g.category_id) || g.needs_review ? 0 : 1);
const byQueueOrder = (a: BlockGroup, b: BlockGroup): number => queueRank(a) - queueRank(b) || a.min_confidence - b.min_confidence || b.total_secs - a.total_secs;

/** Page-local: a 3 px confidence line. Volt when the classifier is sure enough, ember when it needs a human. */
function ConfidenceLine({ value, className }: { value: number; className?: string }) {
  const t = useT();
  const pct = Math.max(0, Math.min(1, value));
  const low = pct < 0.6;
  return (
    <div className={clsx('flex items-center gap-2', className)} title={t('review.confidence', { value: fmtPercent(pct) })}>
      <div className="h-[3px] w-14 overflow-hidden rounded-pill bg-panel-3">
        <div className="h-full rounded-pill transition-[width] duration-300 ease-out" style={{ width: `${pct * 100}%`, background: low ? 'var(--ember)' : 'var(--volt)' }} />
      </div>
      <span className="num w-8 text-right text-[11px] text-ink-3">{fmtPercent(pct)}</span>
    </div>
  );
}

/**
 * Page-local category chip that never grows past its column: the label truncates with an ellipsis and the full
 * name lives in the tooltip. Same palette as ui/CategoryChip.
 */
function GroupCategoryChip({ categories, categoryId }: { categories: Category[]; categoryId: Id | null }) {
  useLocale();
  const cat = categoryById(categories, categoryId);
  const color = cat?.color ?? UNCATEGORIZED_COLOR;
  const Icon = iconFor(cat?.icon ?? 'circle-dashed');
  const label = categoryLabel(cat);
  return (
    <span
      className="inline-flex h-6 max-w-full min-w-0 items-center gap-1.5 rounded-pill border px-2 text-[11px] font-medium"
      style={{ color: `color-mix(in oklab, ${color} 82%, var(--ink))`, borderColor: `color-mix(in oklab, ${color} 35%, transparent)`, background: `color-mix(in oklab, ${color} 12%, transparent)` }}
      title={label}
      data-testid="group-category"
    >
      <Icon className="size-3 shrink-0" strokeWidth={1.75} style={{ color }} aria-hidden />
      <span className="min-w-0 truncate">{label}</span>
    </span>
  );
}

/** Page-local numbered list of assignable categories (1 to 9); long names truncate and keep the full name as a tooltip. */
function AssignPicker({ categories, value, onPick, disabled, className }: { categories: Category[]; value: Id | null; onPick: (id: Id) => void; disabled?: boolean; className?: string }) {
  const t = useT();
  const list = assignableCategories(categories);
  return (
    <ul className={clsx('flex flex-col', className)} role="listbox" aria-label={t('ui.pick_category')}>
      {list.map((c, i) => {
        const Icon = iconFor(c.icon);
        const active = c.id === value;
        const label = categoryLabel(c);
        return (
          <li key={c.id}>
            <button
              type="button"
              role="option"
              aria-selected={active}
              disabled={disabled}
              onClick={() => onPick(c.id)}
              title={label}
              className={clsx(
                'flex w-full min-w-0 items-center gap-2.5 rounded-[8px] px-2 py-1.5 text-left text-sm transition-colors duration-120 hover:bg-panel-2 disabled:opacity-50',
                active && 'bg-volt-soft',
              )}
            >
              <span className="flex size-6 shrink-0 items-center justify-center rounded-md" style={{ background: `color-mix(in oklab, ${c.color} 16%, transparent)`, color: c.color }}>
                <Icon className="size-3.5" strokeWidth={1.75} />
              </span>
              <span className="min-w-0 flex-1 truncate">{label}</span>
              {i < 9 && <kbd>{i + 1}</kbd>}
              {active && <Check className="size-3.5 shrink-0 text-volt" strokeWidth={2} />}
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/** One sentence from the numbers: how much the queue asks of the user today. */
function subtitle(t: Translate, date: string, pending: BlockGroup[] | null, uncategorizedSecs: number): string {
  const day = fmtDateLong(date);
  if (!pending) return day;
  if (!pending.length) return t('review.subtitle.empty', { day });
  const first = t('review.subtitle.groups', { count: pending.length });
  const second = uncategorizedSecs > 0 ? t('review.subtitle.uncategorized', { duration: fmtMinutes(uncategorizedSecs) }) : t('review.subtitle.nothing_uncategorized');
  return t('review.subtitle.sentence', { day, first, second });
}

/** Renders a translated sentence whose '{k<digit>}' placeholders stand for keyboard keys (e.g. '{k1}' → <Kbd>1</Kbd>). */
function withKbd(text: string): ReactNode {
  return text.split(/(\{k\d\})/).map((part, i) => {
    const key = /^\{k(\d)\}$/.exec(part);
    return key ? <Kbd key={i}>{key[1]}</Kbd> : <Fragment key={i}>{part}</Fragment>;
  });
}

interface RowProps {
  group: BlockGroup;
  index: number;
  active: boolean;
  busy: boolean;
  open: boolean;
  categories: Category[];
  blocks: ActivityBlock[];
  blocksLoading: boolean;
  onSelect: () => void;
  onToggle: () => void;
  children?: ReactNode;
}

/**
 * A queue row. Fixed-width columns (duration, category, chevron) never overlap: the text column and the chip
 * both have min-w-0 and truncate. Inside an AnimatePresence, a leaving row hides from the tests and from
 * assistive tech while it animates out.
 */
function GroupRow({ group: g, index, active, busy, open, categories, blocks, blocksLoading, onSelect, onToggle, children }: RowProps) {
  const t = useT();
  const isPresent = useIsPresent();
  return (
    <div
      role="listitem"
      data-index={isPresent ? index : undefined}
      data-testid={isPresent ? 'review-row' : 'review-row-leaving'}
      data-key={g.key}
      aria-current={active ? 'true' : undefined}
      aria-hidden={isPresent ? undefined : true}
      onClick={onSelect}
      className={clsx('relative cursor-default border-b border-line transition-colors duration-150 last:border-b-0', active ? 'bg-volt-soft' : 'hover:bg-panel-2/60', busy && 'opacity-60')}
    >
      {active && <span className="absolute top-2 bottom-2 left-0 w-[3px] rounded-r-full bg-volt shadow-[0_0_12px_2px_rgb(77_141_255/.55)]" aria-hidden />}
      <div className="flex items-center gap-3 px-4 py-2.5">
        <AppAvatar name={g.app_name} />
        <div className="min-w-0 flex-1">
          <p className="flex min-w-0 items-baseline gap-x-2 text-sm leading-5 font-medium text-ink">
            <span className="max-w-[24ch] shrink-0 truncate">{g.app_name}</span>
            {g.domain && <span className="min-w-0 truncate text-xs font-normal text-ink-3">{g.domain}</span>}
          </p>
          <p className="truncate text-xs leading-4 text-ink-3">{g.description ?? g.title}</p>
        </div>
        <span className="num w-16 shrink-0 text-right">
          <span className="block text-xs leading-4 font-medium text-ink-2">{fmtDuration(g.total_secs, { compact: true })}</span>
          <span className="block text-[11px] leading-4 text-ink-3" title={t('review.blocks_in_group')}>
            {t('common.blocks', { count: g.block_ids.length })}
          </span>
        </span>
        <span className="flex w-40 min-w-0 shrink-0 items-center justify-end gap-2">
          {g.needs_review && <span className="size-1.5 shrink-0 rounded-full bg-ember shadow-[0_0_8px_rgb(255_122_31/.6)]" role="img" aria-label={t('review.needs_review')} title={t('review.needs_review')} />}
          <GroupCategoryChip categories={categories} categoryId={g.category_id} />
        </span>
        <ConfidenceLine value={g.min_confidence} className="hidden shrink-0 min-[1280px]:flex" />
        <SourceBadge source={g.source} />
        <button
          type="button"
          aria-expanded={open}
          aria-label={open ? t('review.details.close') : t('review.details.open')}
          title={open ? t('review.details.close') : t('review.details.open')}
          onClick={(e) => {
            e.stopPropagation();
            onSelect();
            onToggle();
          }}
          className="flex size-6 shrink-0 items-center justify-center rounded-md text-ink-4 transition-colors duration-150 hover:bg-panel-3 hover:text-ink-2 focus-visible:outline-2 focus-visible:outline-volt"
        >
          <ChevronDown className={clsx('size-4 transition-transform duration-180', open && 'rotate-180')} strokeWidth={1.75} aria-hidden />
        </button>
      </div>
      {(active || open) && (
        <div className="flex flex-col gap-2 px-4 pb-3 pl-[60px]">
          <p className="flex flex-wrap gap-x-4 gap-y-1 text-xs leading-4 text-ink-3">
            <span>
              {t('review.started_at')} <span className="num text-ink-2">{fmtTime(g.first_started_at)}</span>
            </span>
            <span>
              {t('review.min_confidence')} <span className="num text-ink-2">{fmtPercent(g.min_confidence)}</span>
            </span>
            {g.title && g.description && <span className="truncate">{g.title}</span>}
          </p>
          {active && <p className="text-xs text-ink-3">{withKbd(t('review.hint', { count: g.block_ids.length }))}</p>}
          {children}
          {open && <BlockDetails blocks={blocks} loading={blocksLoading} className="mt-1" />}
        </div>
      )}
    </div>
  );
}

export function Review() {
  const t = useT();
  const date = useAppStore((s) => s.date);
  const setDate = useAppStore((s) => s.setDate);
  const categories = useAppStore((s) => s.categories);
  const loadCategories = useAppStore((s) => s.loadCategories);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const bumpData = useAppStore((s) => s.bumpData);
  const toast = useToast();
  const reduce = useReducedMotion();

  const { data: groups, loading, setData, reload } = useAsync(() => ipc.getReviewGroups(date), [date, dataVersion]);
  const { data: timeline, loading: timelineLoading } = useAsync(() => ipc.getTimeline(date), [date, dataVersion]);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [openKeys, setOpenKeys] = useState<Set<string>>(new Set());
  const [decided, setDecided] = useState<Set<string>>(new Set());
  const [reviewedOpen, setReviewedOpen] = useState(false);
  const [suggestions, setSuggestions] = useState<Record<string, RuleSuggestion[]>>({});
  const [lastDecided, setLastDecided] = useState<BlockGroup | null>(null);
  const [accepted, setAccepted] = useState<Set<string>>(new Set());
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [classifying, setClassifying] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!categories.length) void loadCategories();
  }, [categories.length, loadCategories]);

  // a new day starts a fresh session: nothing decided, nothing open, nothing selected
  useEffect(() => {
    setDecided(new Set());
    setOpenKeys(new Set());
    setSelectedKey(null);
    setLastDecided(null);
    setReviewedOpen(false);
  }, [date]);

  const assignable = useMemo(() => assignableCategories(categories), [categories]);
  const blocksById = useMemo(() => new Map((timeline ?? []).map((b) => [b.id, b])), [timeline]);
  const blocksOf = useCallback((g: BlockGroup): ActivityBlock[] => g.block_ids.map((id) => blocksById.get(id)).filter((b): b is ActivityBlock => !!b), [blocksById]);

  const pending = useMemo(() => (groups ? groups.filter((g) => isPending(g, decided)).sort(byQueueOrder) : null), [groups, decided]);
  const reviewed = useMemo(() => (groups ?? []).filter((g) => !isPending(g, decided)).sort((a, b) => a.first_started_at.localeCompare(b.first_started_at)), [groups, decided]);
  /** Rows the arrow keys walk through: the queue, then the reviewed section when it is open. */
  const visible = useMemo(() => [...(pending ?? []), ...(reviewedOpen ? reviewed : [])], [pending, reviewed, reviewedOpen]);

  const uncategorizedSecs = useMemo(() => (pending ?? []).filter((g) => isUncategorized(g.category_id)).reduce((s, g) => s + g.total_secs, 0), [pending]);

  // the selection is derived: an unknown (or missing) key falls back to the first row, so there is always one when the list has rows
  const foundIndex = visible.findIndex((g) => g.key === selectedKey);
  const selectedIndex = foundIndex >= 0 ? foundIndex : visible.length ? 0 : -1;
  const current = selectedIndex >= 0 ? (visible[selectedIndex] ?? null) : null;

  const toggleOpen = useCallback((key: string) => {
    setOpenKeys((s) => {
      const n = new Set(s);
      if (n.has(key)) n.delete(key);
      else n.add(key);
      return n;
    });
  }, []);

  const assign = useCallback(
    async (group: BlockGroup, categoryId: Id) => {
      if (busyKey) return;
      setBusyKey(group.key);
      try {
        const outcome = await ipc.reclassifyGroup(date, group.key, categoryId);
        const updated: BlockGroup = { ...group, category_id: categoryId, min_confidence: 1, needs_review: false, source: 'user' };
        setData((groups ?? []).map((g) => (g.key === group.key ? updated : g)));
        setSuggestions((s) => ({ ...s, [group.key]: outcome.suggestions }));
        setLastDecided(updated);
        // the decided group leaves the queue; the selection moves on to the next pending group
        const wasPending = (pending ?? []).some((g) => g.key === group.key);
        if (wasPending) {
          const rest = (pending ?? []).filter((g) => g.key !== group.key);
          const at = (pending ?? []).findIndex((g) => g.key === group.key);
          const next = rest[Math.min(at, rest.length - 1)] ?? null;
          setDecided((d) => new Set(d).add(group.key));
          setSelectedKey(next?.key ?? null);
        }
        toast.success(
          t('review.toast.classified', { count: outcome.block_ids.length }),
          outcome.backfilled ? t('review.toast.backfilled', { count: outcome.backfilled }) : undefined,
        );
        if (outcome.disabled_rules.length) {
          toast.info(t('review.toast.rule_disabled_title'), t('review.toast.rule_disabled_body', { pattern: outcome.disabled_rules[0]?.pattern ?? '' }));
        }
        bumpData();
      } catch (e) {
        toast.error(t('review.toast.assign_failed'), e instanceof Error ? e.message : String(e));
      } finally {
        setBusyKey(null);
      }
    },
    [busyKey, date, groups, pending, setData, toast, bumpData, t],
  );

  // keyboard: ↑/↓ (or j/k) move, Enter opens/closes the details, 1–9 assign the nth category.
  // The handler is kept in a ref updated on every render, so the single window listener always sees the
  // latest rows/selection (re-subscribing in an effect lagged behind the DOM right after the data loaded).
  const onKeyRef = useRef<(e: KeyboardEvent) => void>(() => undefined);
  onKeyRef.current = (e: KeyboardEvent) => {
    if (!visible.length || isTypingTarget(e.target) || e.metaKey || e.ctrlKey || e.altKey) return;
    if (document.querySelector('[role="dialog"]')) return;
    const idx = selectedIndex;
    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault();
      setSelectedKey(visible[Math.min(visible.length - 1, idx + 1)]?.key ?? null);
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      setSelectedKey(visible[Math.max(0, idx - 1)]?.key ?? null);
    } else if (e.key === 'Enter') {
      const group = visible[idx];
      if (group) {
        e.preventDefault();
        toggleOpen(group.key);
      }
    } else if (/^[1-9]$/.test(e.key)) {
      const cat = assignable[Number(e.key) - 1];
      const group = visible[idx];
      if (cat && group) {
        e.preventDefault();
        void assign(group, cat.id);
      }
    }
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => onKeyRef.current(e);
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  useEffect(() => {
    if (selectedIndex < 0) return;
    const el = listRef.current?.querySelector<HTMLElement>(`[data-index="${selectedIndex}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  // Hard enforcement only: without a license the dialog explains the plans instead of running the AI.
  const licenseGate = useLicenseGate();

  const classifyNow = async () => {
    setClassifying(true);
    try {
      const r = await ipc.classifyNow();
      const body = t('review.toast.classify_done_body', { local: r.local, remote: r.remote, vision: r.vision, needs_review: r.needs_review });
      toast.success(t('review.toast.classify_done_title'), r.skipped_remote ? `${body} ${t('review.toast.classify_skipped_remote')}` : body);
      await reload();
      bumpData();
    } catch (e) {
      toast.error(t('review.toast.classify_failed'), e instanceof Error ? e.message : String(e));
    } finally {
      setClassifying(false);
    }
  };

  const acceptSuggestion = async (s: RuleSuggestion) => {
    try {
      await ipc.acceptRuleSuggestion(s);
      setAccepted((a) => new Set(a).add(`${s.matcher}:${s.pattern}`));
      toast.success(t('review.toast.rule_created'));
    } catch (e) {
      toast.error(t('review.toast.rule_failed'), e instanceof Error ? e.message : String(e));
    }
  };

  const renderRow = (g: BlockGroup, index: number) => (
    <GroupRow
      group={g}
      index={index}
      active={g.key === current?.key}
      busy={busyKey === g.key}
      open={openKeys.has(g.key)}
      categories={categories}
      blocks={blocksOf(g)}
      blocksLoading={timelineLoading && !timeline}
      onSelect={() => setSelectedKey(g.key)}
      onToggle={() => toggleOpen(g.key)}
    >
      {suggestions[g.key] && <SuggestionChips suggestions={suggestions[g.key] ?? []} categories={categories} accepted={accepted} onAccept={(s) => void acceptSuggestion(s)} />}
    </GroupRow>
  );

  const lastSuggestions = lastDecided ? (suggestions[lastDecided.key] ?? []) : [];
  const showLastSuggestions = !!lastDecided && lastSuggestions.length > 0 && lastDecided.key !== current?.key;

  return (
    <div data-testid="page-review">
      <PageHeader
        title={t('review.title')}
        subtitle={subtitle(t, date, pending, uncategorizedSecs)}
        actions={
          <>
            <DayNav date={date} onChange={setDate} />
            <Button variant="primary" icon={<Wand2 className="size-[18px]" strokeWidth={1.75} aria-hidden />} loading={classifying} onClick={licenseGate.guard(() => void classifyNow())}>
              {t('review.classify_now')}
            </Button>
          </>
        }
      />

      {loading && !groups ? (
        <div className="grid grid-cols-12 gap-5">
          <div className="col-span-12 flex flex-col gap-2 min-[1100px]:col-span-8">
            {[...Array(6)].map((_, i) => (
              <Skeleton key={i} className="h-14" />
            ))}
          </div>
          <Skeleton className="col-span-12 h-72 min-[1100px]:col-span-4" />
        </div>
      ) : !groups?.length ? (
        <Card className="flex flex-col items-center gap-4 py-12 text-center">
          <Ubi mood="excited" size={132} speaking={t('review.empty.speech')} />
          <div>
            <p className="display text-lg text-ink">{t('review.empty.title')}</p>
            <p className="mx-auto mt-1 max-w-sm text-sm leading-5 text-ink-2">{t('review.empty.body')}</p>
          </div>
          <Link to="/timeline" className="inline-flex h-9 items-center rounded-control border border-line-2 bg-panel px-3.5 text-sm font-medium text-ink transition-colors duration-150 hover:bg-panel-2">
            {t('review.empty.cta')}
          </Link>
        </Card>
      ) : (
        <div className="grid grid-cols-12 items-start gap-5">
          <Card padded={false} className="col-span-12 overflow-hidden min-[1100px]:col-span-8">
            <div ref={listRef}>
              {pending?.length ? (
                <div role="list" aria-label={t('review.list_label')}>
                  <AnimatePresence initial={false}>
                    {pending.map((g, i) => (
                      <motion.div
                        key={g.key}
                        layout={reduce ? false : 'position'}
                        exit={{ opacity: 0, scaleY: 0.96 }}
                        style={{ transformOrigin: 'top' }}
                        transition={{ duration: reduce ? 0 : 0.18, ease: [0.16, 1, 0.3, 1] }}
                        className="overflow-hidden"
                      >
                        {renderRow(g, i)}
                      </motion.div>
                    ))}
                  </AnimatePresence>
                </div>
              ) : (
                <div className="flex flex-col items-center gap-4 px-5 py-12 text-center" data-testid="review-done">
                  <Ubi mood="excited" size={132} speaking={t('review.done.speech')} />
                  <div>
                    <p className="display text-lg text-ink">{t('review.done.title')}</p>
                    <p className="mx-auto mt-1 max-w-sm text-sm leading-5 text-ink-2">{t('review.done.body', { count: reviewed.length })}</p>
                  </div>
                  <Link to="/timeline" className="inline-flex h-9 items-center rounded-control border border-line-2 bg-panel px-3.5 text-sm font-medium text-ink transition-colors duration-150 hover:bg-panel-2">
                    {t('review.empty.cta')}
                  </Link>
                </div>
              )}

              <section className="border-t border-line" aria-label={t('review.reviewed.title', { count: reviewed.length })}>
                <button
                  type="button"
                  aria-expanded={reviewedOpen}
                  aria-label={reviewedOpen ? t('review.reviewed.hide') : t('review.reviewed.show')}
                  onClick={() => setReviewedOpen((o) => !o)}
                  className="flex w-full items-center justify-between gap-3 px-4 py-2.5 text-left transition-colors duration-150 hover:bg-panel-2/60"
                >
                  <span className="min-w-0">
                    <span className="block text-sm leading-5 font-medium text-ink-2">{t('review.reviewed.title', { count: reviewed.length })}</span>
                    {reviewedOpen && <span className="block text-xs leading-4 text-ink-3">{reviewed.length ? t('review.reviewed.hint') : t('review.reviewed.empty')}</span>}
                  </span>
                  <ChevronDown className={clsx('size-4 shrink-0 text-ink-4 transition-transform duration-180', reviewedOpen && 'rotate-180')} strokeWidth={1.75} aria-hidden />
                </button>
                {reviewedOpen && reviewed.length > 0 && (
                  <div role="list" aria-label={t('review.reviewed.title', { count: reviewed.length })} data-testid="reviewed-list" className="border-t border-line">
                    {reviewed.map((g, i) => (
                      <Fragment key={g.key}>{renderRow(g, (pending?.length ?? 0) + i)}</Fragment>
                    ))}
                  </div>
                )}
              </section>
            </div>
          </Card>

          <Card className="col-span-12 min-[1100px]:sticky min-[1100px]:top-2 min-[1100px]:col-span-4" aria-label={t('review.assign_label')}>
            <CardHeader
              title={t('review.assign_title')}
              subtitle={
                current
                  ? t(current.domain ? 'review.selected_with_domain' : 'review.selected', {
                      app: current.app_name,
                      domain: current.domain ?? '',
                      duration: fmtDuration(current.total_secs, { compact: true }),
                    })
                  : t('review.select_a_group')
              }
            />
            <AssignPicker categories={categories} value={current?.category_id ?? null} disabled={!current || !!busyKey} onPick={(id) => current && void assign(current, id)} className="-mx-1.5" />
            {showLastSuggestions && lastDecided && (
              <div className="mt-4 flex flex-col gap-1.5 border-t border-line pt-3" data-testid="last-suggestions">
                <p className="truncate text-[11px] leading-4 text-ink-3" title={lastDecided.domain ?? lastDecided.app_name}>
                  {t('review.suggestions.after')}
                </p>
                <SuggestionChips suggestions={lastSuggestions} categories={categories} accepted={accepted} onAccept={(s) => void acceptSuggestion(s)} />
              </div>
            )}
            <div className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-1.5 border-t border-line pt-3 text-xs text-ink-3">
              <span className="flex items-center gap-1.5">
                <Kbd>↑</Kbd>
                <Kbd>↓</Kbd> {t('review.keys.move')}
              </span>
              <span className="flex items-center gap-1.5">
                <Kbd>Enter</Kbd> {t('review.keys.details')}
              </span>
              <span className="flex items-center gap-1.5">
                <Kbd>1</Kbd>–<Kbd>9</Kbd> {t('review.keys.assign')}
              </span>
            </div>
          </Card>
        </div>
      )}
      {licenseGate.dialog}
    </div>
  );
}
