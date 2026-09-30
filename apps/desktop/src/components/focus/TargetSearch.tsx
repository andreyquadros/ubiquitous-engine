import clsx from 'clsx';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import { AppWindow, Globe, Search, ShieldPlus } from 'lucide-react';
import { useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from 'react';
import { useT } from '../../i18n';
import { fmtDuration, foldText, looksLikeDomain, normalizeDomain } from '../../lib/format';
import { useToast } from '../../lib/toast';
import type { FocusTarget, FocusTargetKind, InstalledApp, KnownDomain } from '../../lib/types';
import { Badge } from '../ui/Badge';

interface Result {
  id: string;
  kind: FocusTargetKind;
  name: string;
  key: string;
  group: 'apps' | 'domains' | 'custom';
  seconds?: number;
  /** Already on the list and enabled. */
  listed: boolean;
}

interface Props {
  installedApps: InstalledApp[];
  knownDomains: KnownDomain[];
  targets: FocusTarget[];
  onAdd: (kind: FocusTargetKind, name: string, key: string) => Promise<unknown>;
}

const MAX_PER_GROUP = 6;

/** Builds the result list for a query: installed apps (name contains, case/diacritic-insensitive), known domains, and a "block this site" entry when the text reads as a domain. */
export function searchResults(query: string, apps: InstalledApp[], domains: KnownDomain[], targets: FocusTarget[]): Result[] {
  const q = foldText(query);
  if (!q) return [];
  const listed = (kind: FocusTargetKind, key: string) => targets.some((x) => x.kind === kind && x.key === key && x.enabled);
  const out: Result[] = [];
  for (const a of apps) {
    if (!foldText(a.name).includes(q)) continue;
    out.push({ id: `app:${a.bundle_id}`, kind: 'app', name: a.name, key: a.bundle_id, group: 'apps', listed: listed('app', a.bundle_id) });
    if (out.length >= MAX_PER_GROUP) break;
  }
  let n = 0;
  const qDomain = normalizeDomain(query);
  for (const d of domains) {
    if (!d.domain.includes(q) && !(qDomain && d.domain.includes(qDomain))) continue;
    out.push({ id: `site:${d.domain}`, kind: 'site', name: d.domain, key: d.domain, group: 'domains', seconds: d.seconds, listed: listed('site', d.domain) });
    if (++n >= MAX_PER_GROUP) break;
  }
  if (looksLikeDomain(query) && !out.some((r) => r.kind === 'site' && r.key === qDomain)) {
    out.push({ id: `custom:${qDomain}`, kind: 'site', name: qDomain, key: qDomain, group: 'custom', listed: listed('site', qDomain) });
  }
  return out;
}

/** Search field with a results popover; Enter picks the highlighted result (the first by default) and adds it to the block list. A result already on the list only gets a notice. */
export function TargetSearch({ installedApps, knownDomains, targets, onAdd }: Props) {
  const t = useT();
  const toast = useToast();
  const reduce = useReducedMotion();
  const listId = useId();
  const root = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [busy, setBusy] = useState(false);
  const results = useMemo(() => searchResults(query, installedApps, knownDomains, targets), [query, installedApps, knownDomains, targets]);
  const showList = open && query.trim().length > 0;

  useEffect(() => setActive(0), [query]);

  useEffect(() => {
    if (!showList) return;
    const onDown = (e: MouseEvent) => {
      if (root.current && !root.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', onDown);
    return () => document.removeEventListener('mousedown', onDown);
  }, [showList]);

  const pick = async (r: Result) => {
    if (r.listed) {
      // already on the list and enabled: nothing to add, just say so
      toast.info(t('focus.targets.already_listed', { name: r.name }));
      setQuery('');
      setOpen(false);
      return;
    }
    setBusy(true);
    try {
      await onAdd(r.kind, r.name, r.key);
      setQuery('');
      setOpen(false);
    } finally {
      setBusy(false);
    }
  };

  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setOpen(true);
      setActive((i) => Math.min(results.length - 1, i + 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActive((i) => Math.max(0, i - 1));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const r = results[active] ?? results[0];
      if (r && !busy) void pick(r);
    } else if (e.key === 'Escape') {
      setOpen(false);
    }
  };

  const groups: { key: Result['group']; label: string }[] = [
    { key: 'apps', label: t('focus.targets.group_apps') },
    { key: 'domains', label: t('focus.targets.group_domains') },
    { key: 'custom', label: '' },
  ];

  return (
    <div ref={root} className="relative">
      <div className="relative">
        <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-ink-4" strokeWidth={1.75} aria-hidden />
        <input
          type="text"
          role="combobox"
          aria-label={t('focus.targets.search_label')}
          aria-expanded={showList}
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={showList && results[active] ? `${listId}-${active}` : undefined}
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          onKeyDown={onKey}
          placeholder={t('focus.targets.search_placeholder')}
          autoComplete="off"
          spellCheck={false}
          disabled={busy}
          className="h-10 w-full rounded-control border border-line-2 bg-panel-2 pr-3 pl-9 text-sm text-ink placeholder:text-ink-4 transition-[border-color,box-shadow] duration-150 focus:border-volt focus:outline-none focus:ring-2 focus:ring-volt/25 disabled:opacity-50"
        />
      </div>
      <AnimatePresence>
        {showList && (
          <motion.div
            initial={reduce ? { opacity: 0 } : { opacity: 0, y: -4, scale: 0.99 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={reduce ? { opacity: 0 } : { opacity: 0, y: -4, scale: 0.99 }}
            transition={{ duration: reduce ? 0 : 0.12 }}
            className="glass absolute top-full right-0 left-0 z-40 mt-1.5 max-h-80 overflow-y-auto p-1.5"
          >
            {results.length === 0 ? (
              <p className="px-2.5 py-2 text-xs leading-5 text-ink-3">{t('focus.targets.no_results')}</p>
            ) : (
              <ul id={listId} role="listbox" aria-label={t('focus.targets.results')} className="flex flex-col">
                {groups.map(({ key, label }) => {
                  const items = results.map((r, i) => [r, i] as const).filter(([r]) => r.group === key);
                  if (!items.length) return null;
                  return (
                    <li key={key} role="presentation" className="flex flex-col">
                      {label && <span className="px-2.5 pt-1.5 pb-1 text-[11px] font-medium text-ink-3">{label}</span>}
                      <ul role="group" aria-label={label || undefined} className="flex flex-col">
                        {items.map(([r, i]) => {
                          const Icon = r.group === 'custom' ? ShieldPlus : r.kind === 'app' ? AppWindow : Globe;
                          return (
                            <li key={r.id}>
                              <button
                                type="button"
                                role="option"
                                id={`${listId}-${i}`}
                                aria-selected={i === active}
                                onMouseEnter={() => setActive(i)}
                                onMouseDown={(e) => e.preventDefault()}
                                onClick={() => void pick(r)}
                                className={clsx('flex w-full items-center gap-2.5 rounded-[8px] px-2.5 py-2 text-left text-sm transition-colors duration-100', i === active ? 'bg-volt-soft text-ink' : 'text-ink-2 hover:bg-panel-2')}
                              >
                                <span className={clsx('flex size-7 shrink-0 items-center justify-center rounded-[8px] border border-line bg-panel', r.group === 'custom' ? 'text-volt' : 'text-ink-3')}>
                                  <Icon className="size-3.5" strokeWidth={1.75} aria-hidden />
                                </span>
                                <span className="min-w-0 flex-1">
                                  <span className="block truncate font-medium text-ink">{r.group === 'custom' ? t('focus.targets.block_site', { domain: r.key }) : r.name}</span>
                                  {r.group === 'apps' && <span className="block truncate font-mono text-[11px] text-ink-3">{r.key}</span>}
                                  {r.group === 'domains' && r.seconds !== undefined && <span className="num block text-[11px] text-ink-3">{fmtDuration(r.seconds, { compact: true })}</span>}
                                </span>
                                {r.listed && <Badge tone="volt">{t('focus.targets.in_list')}</Badge>}
                              </button>
                            </li>
                          );
                        })}
                      </ul>
                    </li>
                  );
                })}
              </ul>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
