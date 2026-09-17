import { Bar, BarChart, Cell, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { focusColor } from '../../lib/format';

interface Datum {
  hour: number;
  label: string;
  score: number | null;
}

export function HourlyFocus({ hourly, height = 160 }: { hourly: (number | null)[]; height?: number }) {
  const data: Datum[] = hourly.map((score, hour) => ({ hour, label: `${String(hour).padStart(2, '0')}h`, score }));
  const first = data.findIndex((d) => d.score !== null);
  const last = data.length - 1 - [...data].reverse().findIndex((d) => d.score !== null);
  const visible = first === -1 ? data.slice(6, 22) : data.slice(Math.max(0, first - 1), Math.min(24, last + 2));

  return (
    <ResponsiveContainer width="100%" height={height}>
      <BarChart data={visible} margin={{ top: 4, right: 4, bottom: 0, left: 0 }} barCategoryGap={3}>
        <XAxis dataKey="label" axisLine={false} tickLine={false} interval={visible.length > 12 ? 1 : 0} />
        <YAxis domain={[0, 100]} ticks={[0, 50, 100]} axisLine={false} tickLine={false} width={40} tickMargin={6} />
        <Tooltip
          cursor={{ fill: 'color-mix(in oklab, var(--ink) 6%, transparent)' }}
          content={({ active, payload }) => {
            const p = payload?.[0]?.payload as Datum | undefined;
            if (!active || !p) return null;
            return (
              <div className="card px-2.5 py-1.5 text-xs shadow-pop">
                <span className="font-medium">{p.label}</span> · {p.score === null ? 'sem dados' : `foco ${p.score}`}
              </div>
            );
          }}
        />
        <Bar dataKey="score" radius={[4, 4, 0, 0]} isAnimationActive={false} minPointSize={2}>
          {visible.map((d) => (
            <Cell key={d.hour} fill={d.score === null ? 'var(--surface-3)' : focusColor(d.score)} />
          ))}
        </Bar>
      </BarChart>
    </ResponsiveContainer>
  );
}
