import { motion, useReducedMotion } from 'framer-motion';
import { useEffect, useState } from 'react';
import { categoryById, UNCATEGORIZED_COLOR } from '../../lib/categories';
import { dayFraction, fmtDuration, fmtTime, isToday } from '../../lib/format';
import type { ActivityBlock, Category, IsoDate } from '../../lib/types';
import { useT } from '../../i18n';

interface Props {
  blocks: ActivityBlock[];
  categories: Category[];
  onSelect?: (block: ActivityBlock) => void;
  height?: number;
  /** The day shown; when it is today a volt now-marker is drawn at the current time. */
  date?: IsoDate;
  /** Segments rise in with a 35 ms stagger on mount (the Hoje load moment). */
  animate?: boolean;
}

const TICKS = [0, 3, 6, 9, 12, 15, 18, 21, 24];
const EASE = [0.16, 1, 0.3, 1] as const;

/** 24-hour track: one coloured segment per block, soft glow on hover, now-marker, glass tooltip. */
export function DayBar({ blocks, categories, onSelect, height = 34, date, animate = false }: Props) {
  const [hover, setHover] = useState<{ block: ActivityBlock; x: number } | null>(null);
  const reduce = useReducedMotion();
  const t = useT();
  const live = animate && !reduce;
  const today = !!date && isToday(date);
  const [now, setNow] = useState(() => dayFraction(new Date().toISOString()));

  useEffect(() => {
    if (!today) return;
    const t = setInterval(() => setNow(dayFraction(new Date().toISOString())), 60_000);
    return () => clearInterval(t);
  }, [today]);

  return (
    <div className="relative">
      <div className="relative w-full overflow-hidden rounded-control border border-line bg-panel-2" style={{ height }} role="list" aria-label={t('charts.day_track')}>
        {/* hour grid */}
        {TICKS.slice(1, -1).map((h) => (
          <span key={h} className="absolute top-0 bottom-0 w-px bg-line" style={{ left: `${(h / 24) * 100}%` }} aria-hidden />
        ))}
        {blocks.map((b, i) => {
          const start = dayFraction(b.started_at);
          const endRaw = dayFraction(b.ended_at);
          const end = endRaw < start ? 1 : endRaw;
          const cat = categoryById(categories, b.category_id);
          const color = cat?.color ?? UNCATEGORIZED_COLOR;
          const w = Math.max(0.0015, end - start);
          const active = hover?.block.id === b.id;
          return (
            <motion.button
              type="button"
              role="listitem"
              key={b.id}
              aria-label={`${fmtTime(b.started_at)}–${fmtTime(b.ended_at)} ${b.app_name} ${b.title}`}
              className="absolute top-[5px] bottom-[5px] rounded-[3px] focus-visible:z-10"
              style={{
                left: `${start * 100}%`,
                width: `calc(${w * 100}% - 1px)`,
                background: color,
                opacity: b.category_id ? 1 : 0.5,
                boxShadow: active ? `0 0 10px 1px color-mix(in oklab, ${color} 70%, transparent)` : 'none',
                zIndex: active ? 5 : undefined,
                transformOrigin: 'bottom',
              }}
              initial={live ? { scaleY: 0 } : false}
              animate={{ scaleY: 1 }}
              transition={live ? { duration: 0.5, ease: EASE, delay: 0.25 + Math.min(i, 40) * 0.035 } : { duration: 0 }}
              onMouseEnter={(e) => setHover({ block: b, x: e.clientX })}
              onMouseMove={(e) => setHover({ block: b, x: e.clientX })}
              onMouseLeave={() => setHover(null)}
              onClick={() => onSelect?.(b)}
            />
          );
        })}
        {today && (
          <span className="pointer-events-none absolute top-0 bottom-0 z-[6] w-px bg-volt shadow-[0_0_8px_1px_rgb(77_141_255/.7)]" style={{ left: `${now * 100}%` }} aria-hidden>
            <span className="absolute -top-px left-1/2 size-1.5 -translate-x-1/2 rounded-full bg-volt" />
          </span>
        )}
      </div>
      <div className="num mt-1.5 flex justify-between text-[10px] text-ink-3" aria-hidden>
        {TICKS.map((h) => (
          <span key={h}>{t('charts.hour_tick', { hour: String(h).padStart(2, '0') })}</span>
        ))}
      </div>
      {hover && (
        <div
          className="glass pointer-events-none absolute z-20 -translate-x-1/2 px-3 py-2 text-xs"
          style={{ left: `clamp(90px, ${hover.x - (document.body.getBoundingClientRect().left || 0)}px, calc(100% - 90px))`, top: height + 18 }}
        >
          <div className="flex items-center gap-1.5 font-medium">
            <span className="size-2 rounded-full" style={{ background: categoryById(categories, hover.block.category_id)?.color ?? UNCATEGORIZED_COLOR }} />
            {hover.block.app_name}
          </div>
          {hover.block.title && <p className="mt-0.5 max-w-64 truncate text-ink-2">{hover.block.title}</p>}
          <p className="num mt-0.5 text-ink-3">
            {fmtTime(hover.block.started_at)}–{fmtTime(hover.block.ended_at)}
            <span className="ml-2">{fmtDuration(Math.max(0, (new Date(hover.block.ended_at).getTime() - new Date(hover.block.started_at).getTime()) / 1000))}</span>
          </p>
        </div>
      )}
    </div>
  );
}
