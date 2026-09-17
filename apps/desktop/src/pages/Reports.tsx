import { Copy, Download, FileText, RefreshCw } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { Badge } from '../components/ui/Badge';
import { Button } from '../components/ui/Button';
import { Card } from '../components/ui/Card';
import { CategoryChip } from '../components/ui/CategoryChip';
import { DayNav } from '../components/ui/DayNav';
import { Field, Input, Select } from '../components/ui/Field';
import { EmptyState, Skeleton, Tabs } from '../components/ui/misc';
import { PageHeader } from '../components/ui/PageHeader';
import { reportCategories } from '../lib/categories';
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

export function Reports() {
  const [tab, setTab] = useState<'daily' | 'monthly'>('daily');
  return (
    <div data-testid="page-reports">
      <PageHeader
        title="Relatórios"
        subtitle="Gerados automaticamente no horário de cada categoria, editáveis a qualquer momento."
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

  return (
    <>
      <div className="mb-5 flex items-center justify-between gap-3">
        <p className="text-sm text-ink-2 first-letter:uppercase">{fmtDateLong(date)}</p>
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
              <EmptyState icon={<FileText className="size-5" />} title="Nenhuma categoria com relatório" description="Crie categorias produtivas em Categorias para receber relatórios diários." />
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
  const categories = useAppStore((s) => s.categories);
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
      toast.success('Relatório gerado', `${category.name} · ${r.model}`);
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
    <Card padded={false}>
      <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-3.5">
        <CategoryChip categories={categories} categoryId={category.id} size="md" />
        {draft ? (
          <span className="text-xs text-ink-3">
            gerado {fmtDateTime(draft.generated_at)} · {draft.model} · {fmtDuration(draft.total_secs)} · {draft.input_tokens.toLocaleString('pt-BR')}/{draft.output_tokens.toLocaleString('pt-BR')} tokens
          </span>
        ) : (
          <span className="text-xs text-ink-3">gera automaticamente às {defaultTime}</span>
        )}
        {draft?.stale && <Badge tone="warning">Desatualizado</Badge>}
        {draft?.edited && <Badge tone="neutral">Editado</Badge>}
        <div className="ml-auto flex items-center gap-2">
          {dirty && (
            <Button size="sm" variant="primary" onClick={() => void save()} loading={busy}>
              Salvar alterações
            </Button>
          )}
          {draft && (
            <Button size="sm" icon={<Copy className="size-3.5" />} onClick={() => void copy()}>
              Copiar Markdown
            </Button>
          )}
          <Button size="sm" variant={draft ? 'secondary' : 'primary'} icon={<RefreshCw className="size-3.5" />} onClick={() => void generate()} loading={busy}>
            {draft ? 'Regenerar' : 'Gerar agora'}
          </Button>
        </div>
      </div>
      {!draft ? (
        <EmptyState
          className="py-8"
          title={`Nenhum relatório de ${category.name} para este dia`}
          description={`Os relatórios são gerados automaticamente todo dia às ${defaultTime} a partir dos blocos classificados nesta categoria. Você também pode gerar agora.`}
        />
      ) : (
        <div className="grid gap-6 p-5 min-[1280px]:grid-cols-5">
          <div className="md min-[1280px]:col-span-2">
            <ReactMarkdown>{draft.summary_md}</ReactMarkdown>
            {draft.highlights.length > 0 && (
              <div className="mt-3 flex flex-wrap gap-1.5">
                {draft.highlights.map((h) => (
                  <Badge key={h} tone="brand">
                    {h}
                  </Badge>
                ))}
              </div>
            )}
          </div>
          <div className="min-[1280px]:col-span-3">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-[11px] tracking-wide text-ink-3 uppercase">
                  <th className="pb-2 font-medium">Atividade</th>
                  <th className="w-24 pb-2 font-medium">Min</th>
                  <th className="w-40 pb-2 font-medium">Tipo</th>
                  <th className="w-28 pb-2 font-medium">Horário</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-line">
                {draft.items.map((it, i) => (
                  <tr key={i} className="align-top">
                    <td className="py-1.5 pr-2">
                      <Input aria-label="Atividade" value={it.activity} onChange={(e) => editItem(i, { activity: e.target.value })} className="h-8 text-[13px]" />
                      {it.evidence.length > 0 && <p className="mt-1 truncate text-[11px] text-ink-3">{it.evidence.join(' · ')}</p>}
                    </td>
                    <td className="py-1.5 pr-2">
                      <Input aria-label="Minutos" type="number" min={0} value={it.minutes} onChange={(e) => editItem(i, { minutes: Number(e.target.value) })} className="h-8 px-2 text-[13px] tabular-nums [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none" />
                    </td>
                    <td className="py-1.5 pr-2">
                      <Select aria-label="Tipo" value={it.kind} onChange={(e) => editItem(i, { kind: e.target.value as ActivityKind })} className="h-8 text-[13px]">
                        {KINDS.map((k) => (
                          <option key={k} value={k}>
                            {KIND_LABEL[k]}
                          </option>
                        ))}
                      </Select>
                    </td>
                    <td className="py-2.5 text-xs tabular-nums text-ink-2">{it.time_range}</td>
                  </tr>
                ))}
                {!draft.items.length && (
                  <tr>
                    <td colSpan={4} className="py-4 text-center text-xs text-ink-3">
                      Sem atividades neste relatório.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
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
    } catch (e) {
      toast.error('Não foi possível gerar o relatório mensal', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const download = () => {
    if (!md) return;
    const cat = cats.find((c) => c.id === categoryId);
    const blob = new Blob([md], { type: 'text/markdown;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `relatorio-${(cat?.name ?? 'categoria').toLowerCase().replace(/\s+/g, '-')}-${month}.md`;
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
          {(id) => <Input id={id} type="month" value={month} max={`${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`} onChange={(e) => setMonth(e.target.value)} />}
        </Field>
        <Button variant="primary" onClick={() => void load()} loading={busy} icon={<FileText className="size-4" />}>
          Gerar relatório mensal
        </Button>
        {md && (
          <div className="ml-auto flex gap-2">
            <Button
              icon={<Copy className="size-4" />}
              onClick={() => {
                void copyText(md).then(() => toast.success('Markdown copiado'));
              }}
            >
              Copiar
            </Button>
            <Button icon={<Download className="size-4" />} onClick={download}>
              Baixar .md
            </Button>
          </div>
        )}
      </Card>
      <Card>
        {md ? (
          <div className="md">
            <ReactMarkdown>{md}</ReactMarkdown>
          </div>
        ) : (
          <EmptyState
            icon={<FileText className="size-5" />}
            title="Relatório mensal"
            description="Consolida os relatórios diários de uma categoria em um único Markdown — ideal para o relatório de atividades docentes ou da incubadora."
          />
        )}
      </Card>
    </div>
  );
}
