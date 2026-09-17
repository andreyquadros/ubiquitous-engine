import { useState } from 'react';
import { categoryById, UNCATEGORIZED_COLOR } from '../../lib/categories';
import { dayFraction, fmtDuration, fmtTime } from '../../lib/format';
import type { ActivityBlock, Category } from '../../lib/types';

interface Props {
  blocks: ActivityBlock[];
  categories: Category[];
  onSelect?: (block: ActivityBlock) => void;
  height?: number;
}

const TICKS = [0, 3, 6, 9, 12, 15, 18, 21, 24];

/** Horizontal 24-hour bar with one segment per block, coloured by category. */
export function DayBar({ blocks, categories, onSelect, height = 34 }: Props) {
  const [hover, setHover] = useState<{ block: ActivityBlock; x: number } | null>(null);

  return (
    <div className="relative">
      <div className="relative w-full overflow-hidden rounded-lg bg-surface-2" style={{ height }} role="list" aria-label="Linha do tempo do dia">
        {blocks.map((b) => {
          const start = dayFraction(b.started_at);
          const endRaw = dayFraction(b.ended_at);
          const end = endRaw < start ? 1 : endRaw;
          const cat = categoryById(categories, b.category_id);
          const color = cat?.color ?? UNCATEGORIZED_COLOR;
          const w = Math.max(0.0015, end - start);
          return (
            <button
              type="button"
              role="listitem"
              key={b.id}
              aria-label={`${fmtTime(b.started_at)}–${fmtTime(b.ended_at)} ${b.app_name} ${b.title}`}
              className="absolute top-1 bottom-1 rounded-[3px] transition-[filter] hover:brightness-110 focus-visible:z-10"
              style={{ left: `${start * 100}%`, width: `calc(${w * 100}% - 1px)`, background: color, opacity: b.category_id ? 1 : 0.55 }}
              onMouseEnter={(e) => setHover({ block: b, x: e.clientX })}
              onMouseMove={(e) => setHover({ block: b, x: e.clientX })}
              onMouseLeave={() => setHover(null)}
              onClick={() => onSelect?.(b)}
            />
          );
        })}
      </div>
      <div className="mt-1 flex justify-between text-[10px] tabular-nums text-ink-3" aria-hidden>
        {TICKS.map((h) => (
          <span key={h}>{String(h).padStart(2, '0')}h</span>
        ))}
      </div>
      {hover && (
        <div className="card pointer-events-none absolute z-20 -translate-x-1/2 px-3 py-2 text-xs shadow-pop" style={{ left: `clamp(90px, ${hover.x - (document.body.getBoundingClientRect().left || 0)}px, calc(100% - 90px))`, top: height + 18 }}>
          <div className="flex items-center gap-1.5 font-medium">
            <span className="size-2 rounded-full" style={{ background: categoryById(categories, hover.block.category_id)?.color ?? UNCATEGORIZED_COLOR }} />
            {hover.block.app_name}
          </div>
          {hover.block.title && <p className="mt-0.5 max-w-64 truncate text-ink-2">{hover.block.title}</p>}
          <p className="mt-0.5 tabular-nums text-ink-3">
            {fmtTime(hover.block.started_at)}–{fmtTime(hover.block.ended_at)} · {fmtDuration(Math.max(0, (new Date(hover.block.ended_at).getTime() - new Date(hover.block.started_at).getTime()) / 1000))}
          </p>
        </div>
      )}
    </div>
  );
}
