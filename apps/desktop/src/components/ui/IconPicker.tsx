import clsx from 'clsx';
import { useRef, type KeyboardEvent } from 'react';
import { ICON_CHOICES } from '../../lib/categories';
import { useT } from '../../i18n';

interface Props {
  /** Selected icon name (kebab-case lucide name as stored on the category). */
  value: string;
  onChange: (name: string) => void;
  /** Category colour used to tint the selected tile; volt when absent. */
  color?: string;
  /** Accessible name of the group; defaults to "Ícone" / "Icon". */
  label?: string;
  className?: string;
}

const COLUMNS = 8;

/** Grid of the curated category icons. Radio semantics: arrow keys move, Enter/Space select, each tile named in the current language. */
export function IconPicker({ value, onChange, color, label, className }: Props) {
  const t = useT();
  const grid = useRef<HTMLDivElement>(null);
  const tint = color ?? 'var(--volt)';
  const selectedIndex = Math.max(
    0,
    ICON_CHOICES.findIndex((c) => c.name === value),
  );

  const focusAt = (i: number) => {
    const buttons = grid.current?.querySelectorAll<HTMLButtonElement>('button[role="radio"]');
    const n = ICON_CHOICES.length;
    const idx = ((i % n) + n) % n;
    buttons?.[idx]?.focus();
  };

  const onKey = (e: KeyboardEvent<HTMLButtonElement>, i: number) => {
    const moves: Record<string, number> = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: COLUMNS, ArrowUp: -COLUMNS };
    const delta = moves[e.key];
    if (delta !== undefined) {
      e.preventDefault();
      const next = ((i + delta) % ICON_CHOICES.length + ICON_CHOICES.length) % ICON_CHOICES.length;
      focusAt(next);
      onChange(ICON_CHOICES[next]!.name);
    } else if (e.key === 'Home') {
      e.preventDefault();
      focusAt(0);
      onChange(ICON_CHOICES[0]!.name);
    } else if (e.key === 'End') {
      e.preventDefault();
      focusAt(ICON_CHOICES.length - 1);
      onChange(ICON_CHOICES[ICON_CHOICES.length - 1]!.name);
    }
  };

  return (
    <div ref={grid} role="radiogroup" aria-label={label ?? t('ui.icon_picker')} className={clsx('grid gap-1.5', className)} style={{ gridTemplateColumns: `repeat(${COLUMNS}, minmax(0, 1fr))` }}>
      {ICON_CHOICES.map(({ name, Icon }, i) => {
        const active = name === value;
        const title = t(`icons.${name}`);
        return (
          <button
            key={name}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={title}
            title={title}
            tabIndex={i === selectedIndex ? 0 : -1}
            onClick={() => onChange(name)}
            onKeyDown={(e) => onKey(e, i)}
            className={clsx(
              'flex aspect-square items-center justify-center rounded-control border transition-colors duration-150 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-volt/40',
              active ? 'border-transparent' : 'border-line bg-panel-2 text-ink-2 hover:bg-panel-3 hover:text-ink',
            )}
            style={
              active
                ? { color: tint, background: `color-mix(in oklab, ${tint} 16%, transparent)`, boxShadow: `inset 0 0 0 1px color-mix(in oklab, ${tint} 45%, transparent)` }
                : undefined
            }
          >
            <Icon className="size-4" strokeWidth={1.75} aria-hidden />
          </button>
        );
      })}
    </div>
  );
}
