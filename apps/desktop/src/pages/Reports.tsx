import clsx from 'clsx';
import { Copy, Download, FileText, RefreshCw } from 'lucide-react';
import { Fragment, useEffect, useMemo, useState, type ReactNode } from 'react';
import ReactMarkdown from 'react-markdown';
import { Badge } from '../components/ui/Badge';
import { useLicenseGate } from '../components/layout/LicenseBanner';
import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { DayNav } from '../components/ui/DayNav';
import { Field, Input, Select } from '../components/ui/Field';
import { EmptyState, Skeleton, Tabs } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { useT, type Vars } from '../i18n';
import { iconFor, reportCategories } from '../lib/categories';
import { fmtDateLong, fmtDateTime, fmtDuration, fmtNumber, hhmmToInput, intlLocale, KINDS } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { ActivityKind, Category, DailyReport, ReportItem } from '../lib/types';
import { useAsync } from '../lib/useAsync';

type T = (key: string, vars?: Vars) => string;

async function copyText(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) await navigator.clipboard.writeText(text);
  else {
    const ta = document.createElement('textarea');
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    ta.remove();
  }
}

function reportToMarkdown(r: DailyReport, t: T, cat?: Category): string {
  const items = r.items.map((i) => `- **${i.activity}** — ${i.minutes} min (${t(`common.kind.${i.kind}`)}, ${i.time_range})`).join('\n');
  return `# ${cat?.name ?? r.category_id} — ${r.date}\n\n${r.summary_md.trim()}\n\n## ${t('reports.md_activities')}\n\n${items}\n`;
}

/** pt "setembro de 2026" / en "September 2026" from "2026-09". */
function fmtMonth(ym: string): string {
  const [y, m] = ym.split('-').map(Number);
  if (!y || !m) return ym;
  return new Date(y, m - 1, 1).toLocaleDateString(intlLocale(), { month: 'long', year: 'numeric' });
}

/**
 * Renders a translated sentence whose '{name}' slots are React nodes, so numbers inside it can keep the `num`
 * styling without splitting the sentence into concatenated fragments. Pass the raw template (t() without vars).
 */
function Msg({ text, slots }: { text: string; slots: Record<string, ReactNode> }) {
  return (
    <>
      {text.split(/(\{\w+\})/g).map((part, i) => {
        const m = /^\{(\w+)\}$/.exec(part);
        return m && m[1]! in slots ? <Fragment key={i}>{slots[m[1]!]}</Fragment> : part;
      })}
    </>
  );
}

/** Page-local: category icon in its own colour, used as the report card's mark. */
function CategoryMark({ category, size = 'md' }: { category: Category; size?: 'sm' | 'md' }) {
  const Icon = iconFor(category.icon);
  return (
    <span
      className={clsx('flex shrink-0 items-center justify-center rounded-control border', size === 'sm' ? 'size-7' : 'size-9')}
      style={{ color: category.color, borderColor: `color-mix(in oklab, ${category.color} 35%, transparent)`, background: `color-mix(in oklab, ${category.color} 14%, transparent)` }}
      aria-hidden
    >
      <Icon className={size === 'sm' ? 'size-3.5' : 'size-[18px]'} strokeWidth={1.75} />
    </span>
  );
}

export function Reports() {
  const t = useT();
  const [tab, setTab] = useState<'daily' | 'monthly'>('daily');
  return (
    <div data-testid="page-reports">
      <PageHeader
        title={t('reports.title')}
        subtitle={t('reports.subtitle')}
        actions={
          <Tabs
            value={tab}
            onChange={setTab}
            items={[
              { value: 'daily', label: t('reports.tab_daily') },
              { value: 'monthly', label: t('reports.tab_monthly') },
            ]}
          />
        }
      />
      {tab === 'daily' ? <Daily /> : <Monthly />}
    </div>
  );
}

function Daily() {
  const t = useT();
  const date = useAppStore((s) => s.date);
  const setDate = useAppStore((s) => s.setDate);
  const categories = useAppStore((s) => s.categories);
  const settingsView = useAppStore((s) => s.settingsView);
  const reportsVersion = useAppStore((s) => s.reportsVersion);
  const { data: reports, loading, setData } = useAsync(() => ipc.getReports(date), [date, reportsVersion]);
  const cats = useMemo(() => reportCategories(categories), [categories]);
  const defaultTime = hhmmToInput(settingsView?.settings.report_default_time) || '18:00';

  const upsert = (r: DailyReport) => setData([...(reports ?? []).filter((x) => x.id !== r.id && !(x.category_id === r.category_id && x.date === r.date)), r]);

  const ready = reports?.length ?? 0;
  const status = ready === 0 ? t('reports.status_none') : ready === cats.length ? t('reports.status_all') : t('reports.status_some', { ready, total: cats.length });

  return (
    <>
      <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm text-ink-2 first-letter:uppercase">
          {fmtDateLong(date)}
          {reports && cats.length > 0 && (
            <span className="text-ink-3">
              {': '}
              {status}.
            </span>
          )}
        </p>
        <DayNav date={date} onChange={setDate} />
      </div>
      {loading && !reports ? (
        <div className="grid gap-5">
          <Skeleton className="h-64" />
          <Skeleton className="h-64" />
        </div>
      ) : (
        <div className="grid gap-5">
          {cats.map((cat) => {
            const report = reports?.find((r) => r.category_id === cat.id) ?? null;
            return <ReportCard key={cat.id} category={cat} report={report} date={date} onChange={upsert} defaultTime={hhmmToInput(cat.report_time) || defaultTime} />;
          })}
          {!cats.length && (
            <Card>
              <EmptyState icon={<FileText className="size-5" strokeWidth={1.75} />} title={t('reports.empty_categories_title')} description={t('reports.empty_categories_description')} />
            </Card>
          )}
        </div>
      )}
    </>
  );
}

function ReportCard({ category, report, date, onChange, defaultTime }: { category: Category; report: DailyReport | null; date: string; onChange: (r: DailyReport) => void; defaultTime: string }) {
  const t = useT();
  const [busy, setBusy] = useState(false);
  const [draft, setDraft] = useState<DailyReport | null>(report);
  const [dirty, setDirty] = useState(false);
  const toast = useToast();

  useEffect(() => {
    setDraft(report);
    setDirty(false);
  }, [report]);

  // Hard enforcement only: without a license the dialog explains the plans instead of calling the AI.
  const licenseGate = useLicenseGate();

  const generate = async () => {
    setBusy(true);
    try {
      const r = await ipc.generateReport(date, category.id);
      onChange(r);
      toast.success(t('reports.toast_generated'), t('reports.toast_generated_body', { category: category.name, model: r.model }));
    } catch (e) {
      toast.error(t('reports.toast_generate_failed'), e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const save = async () => {
    if (!draft) return;
    setBusy(true);
    try {
      const r = await ipc.updateReport(draft);
      onChange(r);
      setDirty(false);
      toast.success(t('reports.toast_saved'));
    } catch (e) {
      toast.error(t('reports.toast_save_failed'), e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const editItem = (idx: number, patch: Partial<ReportItem>) => {
    if (!draft) return;
    setDraft({ ...draft, items: draft.items.map((it, i) => (i === idx ? { ...it, ...patch } : it)) });
    setDirty(true);
  };

  const copy = async () => {
    if (!draft) return;
    await copyText(reportToMarkdown(draft, t, category));
    toast.success(t('reports.toast_copied'));
  };

  return (
    <Card padded={false} className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-4">
        <CategoryMark category={category} />
        <div className="min-w-0 flex-1">
          <h3 className="display flex flex-wrap items-center gap-2 text-[17px] leading-6 text-ink">
            {category.name}
            {draft?.stale && <Badge tone="amber">{t('reports.badge_stale')}</Badge>}
            {draft?.edited && <Badge tone="neutral">{t('reports.badge_edited')}</Badge>}
          </h3>
          {draft ? (
            <p className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-ink-3">
              <span>
                <Msg text={t('reports.generated_meta')} slots={{ time: <span className="num">{fmtDateTime(draft.generated_at)}</span>, model: draft.model }} />
              </span>
              <span>
                <Msg text={t('reports.tracked_meta')} slots={{ duration: <span className="num">{fmtDuration(draft.total_secs, { compact: true })}</span> }} />
              </span>
              <span>
                <Msg
                  text={t('reports.tokens_meta')}
                  slots={{ input: <span className="num">{fmtNumber(draft.input_tokens)}</span>, output: <span className="num">{fmtNumber(draft.output_tokens)}</span> }}
                />
              </span>
            </p>
          ) : (
            <p className="mt-0.5 text-xs text-ink-3">
              <Msg text={t('reports.auto_at')} slots={{ time: <span className="num">{defaultTime}</span> }} />
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {dirty && (
            <Button size="sm" variant="primary" onClick={() => void save()} loading={busy}>
              {t('common.save_changes')}
            </Button>
          )}
          {draft && (
            <Button size="sm" icon={<Copy className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={() => void copy()}>
              {t('reports.copy_markdown')}
            </Button>
          )}
          <Button size="sm" variant={draft || dirty ? 'secondary' : 'primary'} icon={<RefreshCw className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={licenseGate.guard(() => void generate())} loading={busy}>
            {draft ? t('reports.regenerate') : t('reports.generate_now')}
          </Button>
        </div>
      </div>
      {!draft ? (
        <EmptyState className="py-8" title={t('reports.empty_report_title', { category: category.name })} description={t('reports.empty_report_description', { time: defaultTime })} />
      ) : (
        <div className="grid gap-6 p-5 min-[1280px]:grid-cols-5">
          <div className="min-[1280px]:col-span-2">
            <div className="md">
              <ReactMarkdown>{draft.summary_md}</ReactMarkdown>
            </div>
            {draft.highlights.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-1.5" aria-label={t('reports.highlights')}>
                {draft.highlights.map((h) => (
                  <Badge key={h} tone="volt">
                    {h}
                  </Badge>
                ))}
              </div>
            )}
          </div>
          <div className="min-[1280px]:col-span-3">
            <div className="grid grid-cols-[1fr_80px_150px_104px] gap-x-2 border-b border-line pb-2 text-xs font-medium text-ink-3" aria-hidden>
              <span>{t('reports.col_activity')}</span>
              <span>{t('reports.col_minutes')}</span>
              <span>{t('reports.col_kind')}</span>
              <span className="text-right">{t('reports.col_time')}</span>
            </div>
            <ul className="divide-y divide-line" aria-label={t('reports.items_label')}>
              {draft.items.map((it, i) => (
                <li key={i} className="grid grid-cols-[1fr_80px_150px_104px] items-start gap-x-2 py-2">
                  <div className="min-w-0">
                    <Input aria-label={t('reports.col_activity')} value={it.activity} onChange={(e) => editItem(i, { activity: e.target.value })} className="h-8 text-[13px]" />
                    {it.evidence.length > 0 && (
                      <p className="mt-1 flex flex-wrap gap-x-2 text-[11px] leading-4 text-ink-3">
                        {it.evidence.map((ev, k) => {
                          // "app · título" evidence from the model is shown as source + detail, no middle dot
                          const [source, ...rest] = ev.split(' · ');
                          return (
                            <span key={k} className="max-w-full truncate">
                              <span className="text-ink-2">{source}</span>
                              {rest.length > 0 && <span className="ml-1.5">{rest.join(', ')}</span>}
                            </span>
                          );
                        })}
                      </p>
                    )}
                    {it.continuation_of && <p className="mt-0.5 text-[11px] text-ink-3">{t('reports.continues_from', { activity: it.continuation_of })}</p>}
                  </div>
                  <Input
                    aria-label={t('reports.col_minutes')}
                    type="number"
                    min={0}
                    value={it.minutes}
                    onChange={(e) => editItem(i, { minutes: Number(e.target.value) })}
                    className="num h-8 px-2 text-right text-[13px] [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
                  />
                  <Select aria-label={t('reports.col_kind')} value={it.kind} onChange={(e) => editItem(i, { kind: e.target.value as ActivityKind })} className="h-8 text-[13px]">
                    {KINDS.map((k) => (
                      <option key={k} value={k}>
                        {t(`common.kind.${k}`)}
                      </option>
                    ))}
                  </Select>
                  <span className="num pt-2 text-right text-xs text-ink-2">{it.time_range}</span>
                </li>
              ))}
              {!draft.items.length && <li className="py-4 text-center text-xs text-ink-3">{t('reports.no_items')}</li>}
            </ul>
          </div>
        </div>
      )}
      {licenseGate.dialog}
    </Card>
  );
}

function Monthly() {
  const t = useT();
  const categories = useAppStore((s) => s.categories);
  const cats = useMemo(() => reportCategories(categories), [categories]);
  const now = new Date();
  const [categoryId, setCategoryId] = useState('');
  const [month, setMonth] = useState(`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`);
  const [md, setMd] = useState<string | null>(null);
  const [generated, setGenerated] = useState<{ categoryId: string; month: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  useEffect(() => {
    if (!categoryId && cats[0]) setCategoryId(cats[0].id);
  }, [cats, categoryId]);

  const load = async () => {
    if (!categoryId || !month) return;
    const [y, m] = month.split('-').map(Number);
    setBusy(true);
    try {
      setMd(await ipc.getMonthlyReport(categoryId, y ?? now.getFullYear(), m ?? now.getMonth() + 1));
      setGenerated({ categoryId, month });
    } catch (e) {
      toast.error(t('reports.toast_monthly_failed'), e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const generatedCat = generated ? cats.find((c) => c.id === generated.categoryId) : undefined;

  const download = () => {
    if (!md) return;
    const cat = generatedCat ?? cats.find((c) => c.id === categoryId);
    const ym = generated?.month ?? month;
    const blob = new Blob([md], { type: 'text/markdown;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = t('reports.file_name', { category: (cat?.name ?? t('reports.file_category_fallback')).toLowerCase().replace(/\s+/g, '-'), month: ym });
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="grid gap-5">
      <Card className="flex flex-wrap items-end gap-3">
        <Field label={t('common.category')} className="w-56">
          {(id) => (
            <Select id={id} value={categoryId} onChange={(e) => setCategoryId(e.target.value)}>
              {cats.map((c) => (
                <option key={c.id} value={c.id}>
                  {c.name}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <Field label={t('reports.month')} className="w-44">
          {(id) => <Input id={id} type="month" className="num" value={month} max={`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`} onChange={(e) => setMonth(e.target.value)} />}
        </Field>
        <Button variant="primary" onClick={() => void load()} loading={busy} icon={<FileText className="size-[18px]" strokeWidth={1.75} aria-hidden />}>
          {t('reports.generate_monthly')}
        </Button>
      </Card>

      {md ? (
        <Card padded={false} className="overflow-hidden">
          <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-4">
            {generatedCat && <CategoryMark category={generatedCat} />}
            <div className="min-w-0 flex-1">
              <h3 className="display text-[17px] leading-6 text-ink">{generatedCat?.name ?? t('reports.monthly_report')}</h3>
              <p className="mt-0.5 text-xs text-ink-3 first-letter:uppercase">{t('reports.monthly_meta', { month: fmtMonth(generated?.month ?? month) })}</p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                icon={<Copy className="size-3.5" strokeWidth={1.75} aria-hidden />}
                onClick={() => {
                  void copyText(md).then(() => toast.success(t('reports.toast_copied')));
                }}
              >
                {t('reports.copy_markdown')}
              </Button>
              <Button size="sm" icon={<Download className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={download}>
                {t('reports.download_md')}
              </Button>
            </div>
          </div>
          <div className="md max-w-[76ch] p-5">
            <ReactMarkdown>{md}</ReactMarkdown>
          </div>
        </Card>
      ) : (
        <Card>
          <EmptyState icon={<FileText className="size-5" strokeWidth={1.75} />} title={t('reports.empty_monthly_title')} description={t('reports.empty_monthly_description')} />
        </Card>
      )}
    </div>
  );
}
