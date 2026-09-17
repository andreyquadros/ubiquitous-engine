import { Bar, BarChart, CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { categoryById, UNCATEGORIZED_COLOR } from '../../lib/categories';
import { fmtDateShort, fmtDuration, fmtWeekday } from '../../lib/format';
import type { Category, DashboardData, IsoDate } from '../../lib/types';

export interface WeekDay {
  date: IsoDate;
  data: DashboardData | null;
}

type Row = Record<string, number | string> & { date: IsoDate; label: string };

export function WeeklyStacked({ days, categories, height = 220 }: { days: WeekDay[]; categories: Category[]; height?: number }) {
  const keys = new Map<string, { name: string; color: string }>();
  const rows: Row[] = days.map((d) => {
    const row: Row = { date: d.date, label: fmtWeekday(d.date) };
    for (const t of d.data?.totals ?? []) {
      if (t.category_id === 'sys-break' || t.category_id === 'sys-private') continue;
      const id = t.category_id ?? 'none';
      const cat = categoryById(categories, t.category_id);
      keys.set(id, { name: cat?.name ?? 'Sem categoria', color: cat?.color ?? UNCATEGORIZED_COLOR });
      row[id] = Math.round((t.secs / 3600) * 10) / 10;
    }
    return row;
  });
  // fixed order: user categories by sort order, then system
  const order = [...categories].sort((a, b) => Number(a.is_system) - Number(b.is_system) || a.sort_order - b.sort_order).map((c) => c.id).concat('none');
  const series = order.filter((id) => keys.has(id));

  return (
    <div>
      <ResponsiveContainer width="100%" height={height}>
        <BarChart data={rows} margin={{ top: 4, right: 4, bottom: 0, left: 4 }} barCategoryGap={12}>
          <CartesianGrid vertical={false} strokeDasharray="3 3" />
          <XAxis dataKey="label" axisLine={false} tickLine={false} />
          <YAxis axisLine={false} tickLine={false} unit="h" width={44} tickMargin={6} />
          <Tooltip
            cursor={{ fill: 'color-mix(in oklab, var(--ink) 6%, transparent)' }}
            content={({ active, payload, label }) => {
              if (!active || !payload?.length) return null;
              const row = payload[0]?.payload as Row;
              return (
                <div className="card min-w-40 px-3 py-2 text-xs shadow-pop">
                  <p className="mb-1 font-medium">
                    {String(label)} · {fmtDateShort(row.date)}
                  </p>
                  {[...payload].reverse().map((p) => (
                    <p key={String(p.dataKey)} className="flex items-center gap-1.5">
                      <span className="size-2 rounded-full" style={{ background: keys.get(String(p.dataKey))?.color }} />
                      <span className="flex-1 text-ink-2">{keys.get(String(p.dataKey))?.name}</span>
                      <span className="tabular-nums">{fmtDuration(Number(p.value) * 3600, { compact: true })}</span>
                    </p>
                  ))}
                </div>
              );
            }}
          />
          {series.map((id, i) => (
            <Bar key={id} dataKey={id} stackId="a" fill={keys.get(id)?.color} stroke="var(--surface)" strokeWidth={1} isAnimationActive={false} radius={i === series.length - 1 ? [4, 4, 0, 0] : 0} />
          ))}
        </BarChart>
      </ResponsiveContainer>
      <ul className="mt-2 flex flex-wrap gap-x-4 gap-y-1" aria-label="Legenda">
        {series.map((id) => (
          <li key={id} className="flex items-center gap-1.5 text-xs text-ink-2">
            <span className="size-2.5 rounded-full" style={{ background: keys.get(id)?.color }} />
            {keys.get(id)?.name}
          </li>
        ))}
      </ul>
    </div>
  );
}

export function FocusTrend({ days, height = 180 }: { days: WeekDay[]; height?: number }) {
  const rows = days.map((d) => ({ date: d.date, label: fmtWeekday(d.date), score: d.data && d.data.stats.total_secs > 0 ? d.data.stats.focus_score : null }));
  return (
    <ResponsiveContainer width="100%" height={height}>
      <LineChart data={rows} margin={{ top: 8, right: 8, bottom: 0, left: 0 }}>
        <CartesianGrid vertical={false} strokeDasharray="3 3" />
        <XAxis dataKey="label" axisLine={false} tickLine={false} />
        <YAxis domain={[0, 100]} ticks={[0, 25, 50, 75, 100]} axisLine={false} tickLine={false} width={40} tickMargin={6} />
        <Tooltip
          content={({ active, payload }) => {
            const p = payload?.[0]?.payload as { date: IsoDate; score: number | null } | undefined;
            if (!active || !p) return null;
            return (
              <div className="card px-2.5 py-1.5 text-xs shadow-pop">
                <span className="font-medium">{fmtDateShort(p.date)}</span> · score {p.score ?? '—'}
              </div>
            );
          }}
        />
        <Line type="monotone" dataKey="score" stroke="var(--color-brand-600)" strokeWidth={2} dot={{ r: 4, strokeWidth: 2, fill: 'var(--surface)' }} activeDot={{ r: 6 }} connectNulls isAnimationActive={false} />
      </LineChart>
    </ResponsiveContainer>
  );
}
