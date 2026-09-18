import { motion, useReducedMotion } from 'framer-motion';
import { Bar, BarChart, CartesianGrid, Cell, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { focusColor } from '../../lib/format';

interface Datum {
  hour: number;
  label: string;
  score: number | null;
}

interface ShapeProps {
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  fill?: string;
  index?: number;
}

const EASE = [0.16, 1, 0.3, 1] as const;

/** Rounded bar that rises from the baseline with a per-bar delay. */
function RisingBar({ x = 0, y = 0, width = 0, height = 0, fill, index = 0, live }: ShapeProps & { live: boolean }) {
  if (height <= 0 || width <= 0) return null;
  const r = Math.min(6, width / 2);
  return (
    <motion.rect
      x={x}
      y={y}
      width={width}
      height={height}
      rx={r}
      ry={r}
      fill={fill}
      style={{ transformOrigin: `${x + width / 2}px ${y + height}px` }}
      initial={live ? { scaleY: 0, opacity: 0.4 } : false}
      animate={{ scaleY: 1, opacity: 1 }}
      transition={live ? { duration: 0.55, ease: EASE, delay: 0.35 + index * 0.035 } : { duration: 0 }}
    />
  );
}

export function HourlyFocus({ hourly, height = 160, animate = false }: { hourly: (number | null)[]; height?: number; animate?: boolean }) {
  const reduce = useReducedMotion();
  const live = animate && !reduce;
  const data: Datum[] = hourly.map((score, hour) => ({ hour, label: `${String(hour).padStart(2, '0')}h`, score }));
  const first = data.findIndex((d) => d.score !== null);
  const last = data.length - 1 - [...data].reverse().findIndex((d) => d.score !== null);
  const visible = first === -1 ? data.slice(6, 22) : data.slice(Math.max(0, first - 1), Math.min(24, last + 2));

  return (
    <ResponsiveContainer width="100%" height={height}>
      <BarChart data={visible} margin={{ top: 4, right: 4, bottom: 0, left: 0 }} barCategoryGap={4}>
        <CartesianGrid vertical={false} />
        <XAxis dataKey="label" axisLine={false} tickLine={false} tickMargin={8} interval={visible.length > 12 ? 1 : 0} />
        <YAxis domain={[0, 100]} ticks={[0, 50, 100]} axisLine={false} tickLine={false} width={34} tickMargin={6} />
        <Tooltip
          cursor={{ fill: 'color-mix(in oklab, var(--ink) 5%, transparent)', radius: 6 }}
          content={({ active, payload }) => {
            const p = payload?.[0]?.payload as Datum | undefined;
            if (!active || !p) return null;
            return (
              <div className="glass px-3 py-2 text-xs">
                <span className="font-medium">{p.label}</span>
                <span className="num ml-2 text-ink-2">{p.score === null ? 'sem dados' : `foco ${p.score}`}</span>
              </div>
            );
          }}
        />
        <Bar dataKey="score" isAnimationActive={false} minPointSize={2} shape={(props: ShapeProps) => <RisingBar {...props} live={live} />}>
          {visible.map((d) => (
            <Cell key={d.hour} fill={d.score === null ? 'var(--panel-3)' : focusColor(d.score)} />
          ))}
        </Bar>
      </BarChart>
    </ResponsiveContainer>
  );
}
