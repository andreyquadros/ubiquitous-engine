import { Cell, Pie, PieChart, ResponsiveContainer, Tooltip } from 'recharts';
import { categoryById, UNCATEGORIZED_COLOR } from '../../lib/categories';
import { fmtDuration, fmtHours } from '../../lib/format';
import type { Category, CategoryTotal } from '../../lib/types';

interface Datum {
  id: string;
  name: string;
  secs: number;
  color: string;
}

export function CategoryDonut({ totals, categories, height = 200 }: { totals: CategoryTotal[]; categories: Category[]; height?: number }) {
  const data: Datum[] = totals
    .filter((t) => t.secs > 0)
    .map((t) => {
      const cat = categoryById(categories, t.category_id);
      return { id: t.category_id ?? 'none', name: cat?.name ?? 'Sem categoria', secs: t.secs, color: cat?.color ?? UNCATEGORIZED_COLOR };
    });
  const total = data.reduce((s, d) => s + d.secs, 0);

  if (!data.length) return <p className="py-10 text-center text-sm text-ink-3">Sem atividade neste dia.</p>;

  return (
    <div className="flex items-center gap-4">
      <div className="relative shrink-0" style={{ width: height, height }}>
        <ResponsiveContainer width="100%" height="100%">
          <PieChart>
            <Pie data={data} dataKey="secs" nameKey="name" innerRadius="68%" outerRadius="100%" paddingAngle={2} stroke="var(--surface)" strokeWidth={2} isAnimationActive={false}>
              {data.map((d) => (
                <Cell key={d.id} fill={d.color} />
              ))}
            </Pie>
            <Tooltip
              content={({ active, payload }) => {
                const p = payload?.[0]?.payload as Datum | undefined;
                if (!active || !p) return null;
                return (
                  <div className="card px-2.5 py-1.5 text-xs shadow-pop">
                    <span className="mr-1.5 inline-block size-2 rounded-full" style={{ background: p.color }} />
                    <span className="font-medium">{p.name}</span> · {fmtDuration(p.secs)} ({Math.round((p.secs / total) * 100)}%)
                  </div>
                );
              }}
            />
          </PieChart>
        </ResponsiveContainer>
        <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center">
          <span className="text-lg font-semibold tabular-nums">{fmtHours(total)}</span>
          <span className="text-[10px] text-ink-3">registradas</span>
        </div>
      </div>
      <ul className="flex min-w-0 flex-1 flex-col gap-1.5" aria-label="Legenda">
        {data.map((d) => (
          <li key={d.id} className="flex items-center gap-2 text-[11px]">
            <span className="size-2.5 shrink-0 rounded-full" style={{ background: d.color }} />
            <span className="min-w-0 flex-1 truncate text-ink-2">{d.name}</span>
            <span className="font-medium tabular-nums">{fmtDuration(d.secs, { compact: true })}</span>
            <span className="w-7 text-right tabular-nums text-ink-3">{Math.round((d.secs / total) * 100)}%</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
