import clsx from 'clsx';
import { AlertTriangle, ChevronDown, Sparkles, Wand2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { AppAvatar, ConfidenceBar, SourceBadge, SuggestionChips } from '../components/ui/BlockBits';
import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { CategoryChip } from '../components/ui/CategoryChip';
import { DayNav } from '../components/ui/DayNav';
import { EmptyState, Kbd, Skeleton } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { assignableCategories, iconFor, isUncategorized } from '../lib/categories';
import { fmtDateLong, fmtDuration, fmtMinutes } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { BlockGroup, Id, RuleSuggestion } from '../lib/types';
import { useAsync } from '../lib/useAsync';

const isTypingTarget = (t: EventTarget | null): boolean => {
  const el = t as HTMLElement | null;
  if (!el) return false;
  const tag = el.tagName;
  return tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable;
};

export function Review() {
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
          `${outcome.block_ids.length} bloco(s) classificado(s)`,
          outcome.backfilled ? `${outcome.backfilled} bloco(s) anteriores foram preenchidos retroativamente.` : undefined,
        );
        if (outcome.disabled_rules.length) toast.info('Regra desativada', `A regra “${outcome.disabled_rules[0]?.pattern}” foi contradita e desativada.`);
        bumpData();
      } catch (e) {
        toast.error('Não foi possível classificar', e instanceof Error ? e.message : String(e));
      } finally {
        setBusyKey(null);
      }
    },
    [busyKey, date, groups, setData, toast, bumpData],
  );

  // keyboard: ↑/↓ move, 1–9 assign nth category
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
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
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [groups, selected, assignable, assign]);

  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(`[data-index="${selected}"]`);
    el?.scrollIntoView({ block: 'nearest' });
  }, [selected]);

  const classifyNow = async () => {
    setClassifying(true);
    try {
      const r = await ipc.classifyNow();
      toast.success(
        'Classificação concluída',
        `${r.local} local · ${r.remote} pela IA · ${r.vision} por visão · ${r.needs_review} para revisar${r.skipped_remote ? ' · IA ignorada (somente local)' : ''}`,
      );
      await reload();
      bumpData();
    } catch (e) {
      toast.error('Falha ao classificar', e instanceof Error ? e.message : String(e));
    } finally {
      setClassifying(false);
    }
  };

  const acceptSuggestion = async (s: RuleSuggestion) => {
    try {
      await ipc.acceptRuleSuggestion(s);
      setAccepted((a) => new Set(a).add(`${s.matcher}:${s.pattern}`));
      toast.success('Regra criada');
    } catch (e) {
      toast.error('Não foi possível criar a regra', e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div data-testid="page-review">
      <PageHeader
        title="Revisão"
        subtitle={groups ? `${groups.length} grupos · ${fmtMinutes(uncategorizedSecs)} sem categoria · ${fmtDateLong(date)}` : fmtDateLong(date)}
        actions={
          <>
            <DayNav date={date} onChange={setDate} />
            <Button variant="primary" icon={<Wand2 className="size-4" />} loading={classifying} onClick={() => void classifyNow()}>
              Classificar agora
            </Button>
          </>
        }
      />

      <div className="mb-4 flex flex-wrap items-center gap-x-4 gap-y-2 text-xs text-ink-3">
        <span className="flex items-center gap-1.5">
          <Kbd>↑</Kbd>
          <Kbd>↓</Kbd> mover
        </span>
        <span className="flex items-center gap-1.5">
          <Kbd>1</Kbd>–<Kbd>9</Kbd> atribuir categoria
        </span>
        <ul className="flex flex-wrap items-center gap-2" aria-label="Atalhos de categoria">
          {assignable.slice(0, 9).map((c, i) => {
            const Icon = iconFor(c.icon);
            return (
              <li key={c.id} className="flex items-center gap-1 rounded-md border border-line bg-surface px-1.5 py-0.5">
                <Kbd>{i + 1}</Kbd>
                <Icon className="size-3" style={{ color: c.color }} />
                <span className="text-ink-2">{c.name}</span>
              </li>
            );
          })}
        </ul>
      </div>

      {loading && !groups ? (
        <div className="flex flex-col gap-2">
          {[...Array(6)].map((_, i) => (
            <Skeleton key={i} className="h-14" />
          ))}
        </div>
      ) : !groups?.length ? (
        <Card>
          <EmptyState icon={<Sparkles className="size-5" />} title="Nada para revisar" description="Todos os blocos do dia estão classificados com boa confiança." />
        </Card>
      ) : (
        <Card padded={false} className="divide-y divide-line">
          <div ref={listRef} role="list" aria-label="Grupos para revisão">
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
                  className={clsx('cursor-default border-b border-line transition-colors last:border-b-0', active ? 'bg-brand-50/60 dark:bg-brand-900/15' : 'hover:bg-surface-2/60')}
                >
                  <div className="flex items-center gap-3 px-4 py-2.5">
                    <span className={clsx('h-8 w-0.5 shrink-0 rounded-full', active ? 'bg-brand-600' : 'bg-transparent')} aria-hidden />
                    <AppAvatar name={g.app_name} />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-sm font-medium">
                        {g.app_name}
                        {g.domain && <span className="ml-1.5 font-normal text-ink-3">· {g.domain}</span>}
                      </p>
                      <p className="truncate text-xs text-ink-3">{g.description ?? g.title}</p>
                    </div>
                    <span className="w-16 shrink-0 text-right text-xs font-medium tabular-nums text-ink-2">{fmtDuration(g.total_secs, { compact: true })}</span>
                    <span className="w-9 shrink-0 text-center text-[11px] tabular-nums text-ink-3" title="Blocos no grupo">
                      ×{g.block_ids.length}
                    </span>
                    <CategoryChip categories={categories} categoryId={g.category_id} className="w-40 justify-center" />
                    <ConfidenceBar value={g.min_confidence} className="w-24" />
                    <span className="w-5 shrink-0 text-amber-500" title={g.needs_review ? 'Precisa de revisão' : ''}>
                      {g.needs_review && <AlertTriangle className="size-4" />}
                    </span>
                    <SourceBadge source={g.source} />
                    <ChevronDown className={clsx('size-4 shrink-0 text-ink-3 transition-transform', active && 'rotate-180')} />
                  </div>
                  {active && (
                    <div className="flex flex-col gap-2 px-4 pb-3 pl-[76px]">
                      <div className="flex flex-wrap gap-1.5" role="group" aria-label="Atribuir categoria">
                        {assignable.map((c, n) => {
                          const Icon = iconFor(c.icon);
                          const current = c.id === g.category_id;
                          return (
                            <button
                              key={c.id}
                              type="button"
                              disabled={busy}
                              onClick={(e) => {
                                e.stopPropagation();
                                void assign(g, c.id);
                              }}
                              className={clsx(
                                'inline-flex h-7 items-center gap-1.5 rounded-lg border px-2 text-xs font-medium transition-colors disabled:opacity-60',
                                current ? 'border-transparent text-white' : 'border-line bg-surface text-ink-2 hover:bg-surface-2 hover:text-ink',
                              )}
                              style={current ? { background: c.color } : undefined}
                            >
                              {n < 9 && <kbd className={clsx(current && 'border-white/40 bg-white/20 text-white')}>{n + 1}</kbd>}
                              <Icon className="size-3.5" style={current ? undefined : { color: c.color }} />
                              {c.name}
                            </button>
                          );
                        })}
                      </div>
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
      )}
    </div>
  );
}
