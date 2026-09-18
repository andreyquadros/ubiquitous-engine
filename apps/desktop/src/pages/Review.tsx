import clsx from 'clsx';
import { ChevronDown, Wand2 } from 'lucide-react';
import { Fragment, useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { Ubi } from '../components/ubi/Ubi';
import { AppAvatar, SourceBadge, SuggestionChips } from '../components/ui/BlockBits';
import { Button } from '../components/ui/Button';
import { Card, CardHeader } from '../components/ui/Card';
import { CategoryChip, CategoryPicker } from '../components/ui/CategoryChip';
import { DayNav } from '../components/ui/DayNav';
import { Kbd, Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { useT } from '../i18n';
import { assignableCategories, isUncategorized } from '../lib/categories';
import { fmtDateLong, fmtDuration, fmtMinutes, fmtPercent, fmtTime } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { BlockGroup, Id, RuleSuggestion } from '../lib/types';
import { useAsync } from '../lib/useAsync';

type Translate = ReturnType<typeof useT>;

const isTypingTarget = (t: EventTarget | null): boolean => {
  const el = t as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable;
};

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

/** One sentence from the numbers: how much the queue asks of the user today. */
function subtitle(t: Translate, date: string, groups: BlockGroup[] | null, uncategorizedSecs: number): string {
  const day = fmtDateLong(date);
  if (!groups) return day;
  if (!groups.length) return t('review.subtitle.empty', { day });
  const first = t('review.subtitle.groups', { count: groups.length });
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

export function Review() {
  const t = useT();
  const date = useAppStore((s) => s.date);
  const setDate = useAppStore((s) => s.setDate);
  const categories = useAppStore((s) => s.categories);
  const loadCategories = useAppStore((s) => s.loadCategories);
  const dataVersion = useAppStore((s) => s.dataVersion);
  const bumpData = useAppStore((s) => s.bumpData);
  const toast = useToast();

  const { data: groups, loading, setData, reload } = useAsync(() => ipc.getReviewGroups(date), [date, dataVersion]);
  const [selected, setSelected] = useState(0);
  const [suggestions, setSuggestions] = useState<Record<string, RuleSuggestion[]>>({});
  const [accepted, setAccepted] = useState<Set<string>>(new Set());
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [classifying, setClassifying] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!categories.length) void loadCategories();
  }, [categories.length, loadCategories]);

  const assignable = useMemo(() => assignableCategories(categories), [categories]);

  const uncategorizedSecs = useMemo(() => (groups ?? []).filter((g) => isUncategorized(g.category_id)).reduce((s, g) => s + g.total_secs, 0), [groups]);

  const assign = useCallback(
    async (group: BlockGroup, categoryId: Id) => {
      if (busyKey) return;
      setBusyKey(group.key);
      try {
        const outcome = await ipc.reclassifyGroup(date, group.key, categoryId);
        setData((groups ?? []).map((g) => (g.key === group.key ? { ...g, category_id: categoryId, min_confidence: 1, needs_review: false, source: 'user' } : g)));
        setSuggestions((s) => ({ ...s, [group.key]: outcome.suggestions }));
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
    [busyKey, date, groups, setData, toast, bumpData, t],
  );

  // keyboard: ↑/↓ (or j/k) move, 1–9 assign the nth category.
  // The handler is kept in a ref updated on every render, so the single window listener always sees the
  // latest groups/selection (re-subscribing in an effect lagged behind the DOM right after the data loaded).
  const onKeyRef = useRef<(e: KeyboardEvent) => void>(() => undefined);
  onKeyRef.current = (e: KeyboardEvent) => {
    if (!groups?.length || isTypingTarget(e.target) || e.metaKey || e.ctrlKey || e.altKey) return;
    if (document.querySelector('[role="dialog"]')) return;
    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault();
      setSelected((i) => Math.min(groups.length - 1, i + 1));
    } else if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      setSelected((i) => Math.max(0, i - 1));
    } else if (/^[1-9]$/.test(e.key)) {
      const cat = assignable[Number(e.key) - 1];
      const group = groups[selected];
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
    const el = listRef.current?.querySelector<HTMLElement>(`[data-index="${selected}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  }, [selected]);

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

  const current = groups?.[selected] ?? null;

  return (
    <div data-testid="page-review">
      <PageHeader
        title={t('review.title')}
        subtitle={subtitle(t, date, groups, uncategorizedSecs)}
        actions={
          <>
            <DayNav date={date} onChange={setDate} />
            <Button variant="primary" icon={<Wand2 className="size-[18px]" strokeWidth={1.75} aria-hidden />} loading={classifying} onClick={() => void classifyNow()}>
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
          <Ubi mood="excited" size={132} variant="flat" speaking={t('review.empty.speech')} />
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
            <div ref={listRef} role="list" aria-label={t('review.list_label')}>
              {groups.map((g, i) => {
                const active = i === selected;
                const busy = busyKey === g.key;
                return (
                  <div
                    key={g.key}
                    role="listitem"
                    data-index={i}
                    data-testid="review-row"
                    aria-current={active ? 'true' : undefined}
                    onClick={() => setSelected(i)}
                    className={clsx(
                      'relative cursor-default border-b border-line transition-colors duration-150 last:border-b-0',
                      active ? 'bg-volt-soft' : 'hover:bg-panel-2/60',
                      busy && 'opacity-60',
                    )}
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
                      <span className="num w-14 shrink-0 text-right">
                        <span className="block text-xs leading-4 font-medium text-ink-2">{fmtDuration(g.total_secs, { compact: true })}</span>
                        <span className="block text-[11px] leading-4 text-ink-3" title={t('review.blocks_in_group')}>
                          {t('common.blocks', { count: g.block_ids.length })}
                        </span>
                      </span>
                      <span className="flex w-36 shrink-0 items-center justify-end gap-2">
                        {g.needs_review && <span className="size-1.5 shrink-0 rounded-full bg-ember shadow-[0_0_8px_rgb(255_122_31/.6)]" role="img" aria-label={t('review.needs_review')} title={t('review.needs_review')} />}
                        <CategoryChip categories={categories} categoryId={g.category_id} />
                      </span>
                      <ConfidenceLine value={g.min_confidence} className="hidden shrink-0 min-[1280px]:flex" />
                      <SourceBadge source={g.source} />
                      <ChevronDown className={clsx('size-4 shrink-0 text-ink-4 transition-transform duration-180', active && 'rotate-180')} strokeWidth={1.75} aria-hidden />
                    </div>
                    {active && (
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
                        <p className="text-xs text-ink-3">{withKbd(t('review.hint', { count: g.block_ids.length }))}</p>
                        {suggestions[g.key] && (
                          <SuggestionChips suggestions={suggestions[g.key] ?? []} categories={categories} accepted={accepted} onAccept={(s) => void acceptSuggestion(s)} />
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
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
            <CategoryPicker categories={categories} value={current?.category_id ?? null} numbered disabled={!current || !!busyKey} onPick={(id) => current && void assign(current, id)} className="-mx-1.5" />
            <div className="mt-4 flex flex-wrap items-center gap-x-4 gap-y-1.5 border-t border-line pt-3 text-xs text-ink-3">
              <span className="flex items-center gap-1.5">
                <Kbd>↑</Kbd>
                <Kbd>↓</Kbd> {t('review.keys.move')}
              </span>
              <span className="flex items-center gap-1.5">
                <Kbd>1</Kbd>–<Kbd>9</Kbd> {t('review.keys.assign')}
              </span>
            </div>
          </Card>
        </div>
      )}
    </div>
  );
}
