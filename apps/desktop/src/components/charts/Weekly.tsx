import { Bar, BarChart, CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { categoryById, UNCATEGORIZED_COLOR } from '../../lib/categories';
import { fmtDateShort, fmtDuration, fmtWeekday } from '../../lib/format';
import type { Category, DashboardData, IsoDate } from '../../lib/types';

export interface WeekDay {
  date: IsoDate;
  data: DashboardData | null;
}

type Row = Record<string, number | string> & { date: IsoDate; label: string };

/** Stacked hours per category per day. */
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
        <BarChart data={rows} margin={{ top: 4, right: 4, bottom: 0, left: 4 }} barCategoryGap={14}>
          <CartesianGrid vertical={false} />
          <XAxis dataKey="label" axisLine={false} tickLine={false} tickMargin={8} />
          <YAxis axisLine={false} tickLine={false} unit="h" width={40} tickMargin={6} />
          <Tooltip
            cursor={{ fill: 'color-mix(in oklab, var(--ink) 5%, transparent)', radius: 6 }}
            content={({ active, payload, label }) => {
              if (!active || !payload?.length) return null;
              const row = payload[0]?.payload as Row;
              return (
                <div className="glass min-w-44 px-3 py-2 text-xs">
                  <p className="mb-1 font-medium">
                    {String(label)} <span className="ml-1 text-ink-3">{fmtDateShort(row.date)}</span>
                  </p>
                  {[...payload].reverse().map((p) => (
                    <p key={String(p.dataKey)} className="flex items-center gap-1.5">
                      <span className="size-2 rounded-full" style={{ background: keys.get(String(p.dataKey))?.color }} />
                      <span className="flex-1 text-ink-2">{keys.get(String(p.dataKey))?.name}</span>
                      <span className="num">{fmtDuration(Number(p.value) * 3600, { compact: true })}</span>
                    </p>
                  ))}
                </div>
              );
            }}
          />
          {series.map((id, i) => (
            <Bar key={id} dataKey={id} stackId="a" fill={keys.get(id)?.color} stroke="var(--panel)" strokeWidth={1} isAnimationActive={false} radius={i === series.length - 1 ? [6, 6, 0, 0] : 0} />
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

/** Focus score across the days, volt line with panel-filled dots. */
export function FocusTrend({ days, height = 180 }: { days: WeekDay[]; height?: number }) {
  const rows = days.map((d) => ({ date: d.date, label: fmtWeekday(d.date), score: d.data && d.data.stats.total_secs > 0 ? d.data.stats.focus_score : null }));
  return (
    <ResponsiveContainer width="100%" height={height}>
      <LineChart data={rows} margin={{ top: 8, right: 12, bottom: 0, left: 0 }}>
        <CartesianGrid vertical={false} />
        <XAxis dataKey="label" axisLine={false} tickLine={false} tickMargin={10} padding={{ left: 8, right: 8 }} />
        <YAxis domain={[0, 100]} ticks={[25, 50, 75, 100]} axisLine={false} tickLine={false} width={34} tickMargin={6} />
        <Tooltip
          cursor={{ stroke: 'var(--line-2)' }}
          content={({ active, payload }) => {
            const p = payload?.[0]?.payload as { date: IsoDate; score: number | null } | undefined;
            if (!active || !p) return null;
            return (
              <div className="glass px-3 py-2 text-xs">
                <span className="font-medium">{fmtDateShort(p.date)}</span>
                <span className="num ml-2 text-ink-2">score {p.score ?? '—'}</span>
              </div>
            );
          }}
        />
        <Line type="monotone" dataKey="score" stroke="var(--volt)" strokeWidth={2} dot={{ r: 4, strokeWidth: 2, fill: 'var(--panel)' }} activeDot={{ r: 6, fill: 'var(--volt)', stroke: 'var(--panel)' }} connectNulls isAnimationActive={false} />
      </LineChart>
    </ResponsiveContainer>
  );
}
