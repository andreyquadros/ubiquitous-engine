import clsx from 'clsx';
import { Copy, Download, FileText, RefreshCw } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { DayNav } from '../components/ui/DayNav';
import { Field, Input, Select } from '../components/ui/Field';
import { EmptyState, Skeleton, Tabs } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { iconFor, reportCategories } from '../lib/categories';
import { fmtDateLong, fmtDateTime, fmtDuration, hhmmToInput, KIND_LABEL } from '../lib/format';
import { ipc } from '../lib/ipc';
import { useAppStore } from '../lib/store';
import { useToast } from '../lib/toast';
import type { ActivityKind, Category, DailyReport, ReportItem } from '../lib/types';
import { useAsync } from '../lib/useAsync';

const KINDS = Object.keys(KIND_LABEL) as ActivityKind[];

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

function reportToMarkdown(r: DailyReport, cat?: Category): string {
  const items = r.items.map((i) => `- **${i.activity}** — ${i.minutes} min (${KIND_LABEL[i.kind]}, ${i.time_range})`).join('\n');
  return `# ${cat?.name ?? r.category_id} — ${r.date}\n\n${r.summary_md.trim()}\n\n## Atividades\n\n${items}\n`;
}

/** "setembro de 2026" from "2026-09". */
function fmtMonth(ym: string): string {
  const [y, m] = ym.split('-').map(Number);
  if (!y || !m) return ym;
  return new Date(y, m - 1, 1).toLocaleDateString('pt-BR', { month: 'long', year: 'numeric' });
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
  const [tab, setTab] = useState<'daily' | 'monthly'>('daily');
  return (
    <div data-testid="page-reports">
      <PageHeader
        title="Relatórios"
        subtitle="Gerados no horário de cada categoria e editáveis a qualquer momento."
        actions={
          <Tabs
            value={tab}
            onChange={setTab}
            items={[
              { value: 'daily', label: 'Diário' },
              { value: 'monthly', label: 'Mensal' },
            ]}
          />
        }
      />
      {tab === 'daily' ? <Daily /> : <Monthly />}
    </div>
  );
}

function Daily() {
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

  return (
    <>
      <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
        <p className="text-sm text-ink-2 first-letter:uppercase">
          {fmtDateLong(date)}
          {reports && cats.length > 0 && (
            <span className="text-ink-3">
              {': '}
              {ready === 0 ? 'nenhum relatório gerado ainda' : ready === cats.length ? 'todos os relatórios prontos' : `${ready} de ${cats.length} relatórios prontos`}.
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
              <EmptyState icon={<FileText className="size-5" strokeWidth={1.75} />} title="Nenhuma categoria com relatório" description="Crie categorias produtivas em Categorias para receber relatórios diários." />
            </Card>
          )}
        </div>
      )}
    </>
  );
}

function ReportCard({ category, report, date, onChange, defaultTime }: { category: Category; report: DailyReport | null; date: string; onChange: (r: DailyReport) => void; defaultTime: string }) {
  const [busy, setBusy] = useState(false);
  const [draft, setDraft] = useState<DailyReport | null>(report);
  const [dirty, setDirty] = useState(false);
  const toast = useToast();

  useEffect(() => {
    setDraft(report);
    setDirty(false);
  }, [report]);

  const generate = async () => {
    setBusy(true);
    try {
      const r = await ipc.generateReport(date, category.id);
      onChange(r);
      toast.success('Relatório gerado', `${category.name}, com ${r.model}.`);
    } catch (e) {
      toast.error('Não foi possível gerar', e instanceof Error ? e.message : String(e));
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
      toast.success('Relatório salvo');
    } catch (e) {
      toast.error('Não foi possível salvar', e instanceof Error ? e.message : String(e));
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
    await copyText(reportToMarkdown(draft, category));
    toast.success('Markdown copiado');
  };

  return (
    <Card padded={false} className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-4">
        <CategoryMark category={category} />
        <div className="min-w-0 flex-1">
          <h3 className="display flex flex-wrap items-center gap-2 text-[17px] leading-6 text-ink">
            {category.name}
            {draft?.stale && <Badge tone="amber">Desatualizado</Badge>}
            {draft?.edited && <Badge tone="neutral">Editado</Badge>}
          </h3>
          {draft ? (
            <p className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-ink-3">
              <span>
                Gerado <span className="num">{fmtDateTime(draft.generated_at)}</span> com {draft.model}
              </span>
              <span>
                <span className="num">{fmtDuration(draft.total_secs, { compact: true })}</span> registradas
              </span>
              <span>
                <span className="num">{draft.input_tokens.toLocaleString('pt-BR')}</span> tokens de entrada, <span className="num">{draft.output_tokens.toLocaleString('pt-BR')}</span> de saída
              </span>
            </p>
          ) : (
            <p className="mt-0.5 text-xs text-ink-3">
              Gera automaticamente às <span className="num">{defaultTime}</span>
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          {dirty && (
            <Button size="sm" variant="primary" onClick={() => void save()} loading={busy}>
              Salvar alterações
            </Button>
          )}
          {draft && (
            <Button size="sm" icon={<Copy className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={() => void copy()}>
              Copiar Markdown
            </Button>
          )}
          <Button size="sm" variant={draft || dirty ? 'secondary' : 'primary'} icon={<RefreshCw className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={() => void generate()} loading={busy}>
            {draft ? 'Regenerar' : 'Gerar agora'}
          </Button>
        </div>
      </div>
      {!draft ? (
        <EmptyState
          className="py-8"
          title={`Nenhum relatório de ${category.name} para este dia`}
          description={`Ele será escrito às ${defaultTime} a partir dos blocos desta categoria. Se preferir, gere agora.`}
        />
      ) : (
        <div className="grid gap-6 p-5 min-[1280px]:grid-cols-5">
          <div className="min-[1280px]:col-span-2">
            <div className="md">
              <ReactMarkdown>{draft.summary_md}</ReactMarkdown>
            </div>
            {draft.highlights.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-1.5" aria-label="Destaques">
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
              <span>Atividade</span>
              <span>Minutos</span>
              <span>Tipo</span>
              <span className="text-right">Horário</span>
            </div>
            <ul className="divide-y divide-line" aria-label="Atividades do relatório">
              {draft.items.map((it, i) => (
                <li key={i} className="grid grid-cols-[1fr_80px_150px_104px] items-start gap-x-2 py-2">
                  <div className="min-w-0">
                    <Input aria-label="Atividade" value={it.activity} onChange={(e) => editItem(i, { activity: e.target.value })} className="h-8 text-[13px]" />
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
                    {it.continuation_of && <p className="mt-0.5 text-[11px] text-ink-3">Continua de {it.continuation_of}</p>}
                  </div>
                  <Input
                    aria-label="Minutos"
                    type="number"
                    min={0}
                    value={it.minutes}
                    onChange={(e) => editItem(i, { minutes: Number(e.target.value) })}
                    className="num h-8 px-2 text-right text-[13px] [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none"
                  />
                  <Select aria-label="Tipo" value={it.kind} onChange={(e) => editItem(i, { kind: e.target.value as ActivityKind })} className="h-8 text-[13px]">
                    {KINDS.map((k) => (
                      <option key={k} value={k}>
                        {KIND_LABEL[k]}
                      </option>
                    ))}
                  </Select>
                  <span className="num pt-2 text-right text-xs text-ink-2">{it.time_range}</span>
                </li>
              ))}
              {!draft.items.length && <li className="py-4 text-center text-xs text-ink-3">Sem atividades neste relatório.</li>}
            </ul>
          </div>
        </div>
      )}
    </Card>
  );
}

function Monthly() {
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
      toast.error('Não foi possível gerar o relatório mensal', e instanceof Error ? e.message : String(e));
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
    a.download = `relatorio-${(cat?.name ?? 'categoria').toLowerCase().replace(/\s+/g, '-')}-${ym}.md`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="grid gap-5">
      <Card className="flex flex-wrap items-end gap-3">
        <Field label="Categoria" className="w-56">
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
        <Field label="Mês" className="w-44">
          {(id) => <Input id={id} type="month" className="num" value={month} max={`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`} onChange={(e) => setMonth(e.target.value)} />}
        </Field>
        <Button variant="primary" onClick={() => void load()} loading={busy} icon={<FileText className="size-[18px]" strokeWidth={1.75} aria-hidden />}>
          Gerar relatório mensal
        </Button>
      </Card>

      {md ? (
        <Card padded={false} className="overflow-hidden">
          <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-4">
            {generatedCat && <CategoryMark category={generatedCat} />}
            <div className="min-w-0 flex-1">
              <h3 className="display text-[17px] leading-6 text-ink">{generatedCat?.name ?? 'Relatório mensal'}</h3>
              <p className="mt-0.5 text-xs text-ink-3 first-letter:uppercase">{fmtMonth(generated?.month ?? month)}, consolidado a partir dos relatórios diários</p>
            </div>
            <div className="flex items-center gap-2">
              <Button
                size="sm"
                icon={<Copy className="size-3.5" strokeWidth={1.75} aria-hidden />}
                onClick={() => {
                  void copyText(md).then(() => toast.success('Markdown copiado'));
                }}
              >
                Copiar Markdown
              </Button>
              <Button size="sm" icon={<Download className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={download}>
                Baixar .md
              </Button>
            </div>
          </div>
          <div className="md max-w-[76ch] p-5">
            <ReactMarkdown>{md}</ReactMarkdown>
          </div>
        </Card>
      ) : (
        <Card>
          <EmptyState
            icon={<FileText className="size-5" strokeWidth={1.75} />}
            title="Nenhum mês gerado ainda"
            description="Escolha a categoria e o mês acima e gere o relatório: um único Markdown por categoria, pronto para o relatório de atividades docentes ou da incubadora."
          />
        </Card>
      )}
    </div>
  );
}
