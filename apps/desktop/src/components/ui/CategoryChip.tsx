import clsx from 'clsx';
import { Check } from 'lucide-react';
import type { ButtonHTMLAttributes } from 'react';
import { assignableCategories, categoryById, iconFor, UNCATEGORIZED_COLOR } from '../../lib/categories';
import type { Category, Id } from '../../lib/types';

interface ChipProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  categories: Category[];
  categoryId: Id | null;
  size?: 'sm' | 'md';
  interactive?: boolean;
}

/** Coloured category pill. Renders as a button when `interactive`. */
export function CategoryChip({ categories, categoryId, size = 'sm', interactive, className, ...rest }: ChipProps) {
  const cat = categoryById(categories, categoryId);
  const color = cat?.color ?? UNCATEGORIZED_COLOR;
  const Icon = iconFor(cat?.icon ?? 'circle-dashed');
  const cls = clsx(
    'inline-flex items-center gap-1.5 rounded-pill border font-medium whitespace-nowrap transition-[filter,background-color] duration-150',
    size === 'sm' ? 'h-6 px-2 text-[11px]' : 'h-7 px-2.5 text-xs',
    interactive && 'cursor-pointer hover:brightness-110',
    className,
  );
  const style = {
    color: `color-mix(in oklab, ${color} 82%, var(--ink))`,
    borderColor: `color-mix(in oklab, ${color} 35%, transparent)`,
    background: `color-mix(in oklab, ${color} 12%, transparent)`,
  };
  const inner = (
    <>
      <Icon className={size === 'sm' ? 'size-3' : 'size-3.5'} strokeWidth={1.75} style={{ color }} aria-hidden />
      {cat?.name ?? 'Sem categoria'}
    </>
  );
  if (interactive) {
    return (
      <button type="button" className={cls} style={style} {...rest}>
        {inner}
      </button>
    );
  }
  return (
    <span className={cls} style={style}>
      {inner}
    </span>
  );
}

interface PickerProps {
  categories: Category[];
  value: Id | null;
  onPick: (id: Id) => void;
  numbered?: boolean;
  disabled?: boolean;
  className?: string;
}

/** Vertical list of assignable categories (used inside popovers and on the review page). */
export function CategoryPicker({ categories, value, onPick, numbered, disabled, className }: PickerProps) {
  const list = assignableCategories(categories);
  return (
    <ul className={clsx('flex flex-col', className)} role="listbox" aria-label="Escolher categoria">
      {list.map((c, i) => {
        const Icon = iconFor(c.icon);
        const active = c.id === value;
        return (
          <li key={c.id}>
            <button
              type="button"
              role="option"
              aria-selected={active}
              disabled={disabled}
              onClick={() => onPick(c.id)}
              className={clsx(
                'flex w-full items-center gap-2.5 rounded-[8px] px-2 py-1.5 text-left text-sm transition-colors duration-120 hover:bg-panel-2 disabled:opacity-50',
                active && 'bg-volt-soft',
              )}
            >
              <span className="flex size-6 items-center justify-center rounded-md" style={{ background: `color-mix(in oklab, ${c.color} 16%, transparent)`, color: c.color }}>
                <Icon className="size-3.5" strokeWidth={1.75} />
              </span>
              <span className="flex-1 truncate">{c.name}</span>
              {numbered && i < 9 && <kbd>{i + 1}</kbd>}
              {active && <Check className="size-3.5 text-volt" strokeWidth={2} />}
            </button>
          </li>
        );
      })}
    </ul>
  );
}
