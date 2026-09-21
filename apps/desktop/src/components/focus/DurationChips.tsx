import clsx from 'clsx';
import { useT } from '../../i18n';

export const DURATION_PRESETS = [25, 45, 90] as const;

interface Props {
  value: number;
  onChange: (minutes: number) => void;
  /** Extra values (the user's default length) shown alongside the presets. */
  extra?: number[];
  size?: 'sm' | 'md';
  disabled?: boolean;
  label: string;
}

/** 25 / 45 / 90 min chips (plus the configured default when it differs); a radio group. */
export function DurationChips({ value, onChange, extra = [], size = 'md', disabled, label }: Props) {
  const t = useT();
  const options = [...new Set<number>([...DURATION_PRESETS, ...extra.filter((m) => m >= 5 && m <= 240)])].sort((a, b) => a - b);
  return (
    <div role="radiogroup" aria-label={label} className="flex flex-wrap gap-1.5">
      {options.map((m) => {
        const active = m === value;
        return (
          <button
            key={m}
            type="button"
            role="radio"
            aria-checked={active}
            disabled={disabled}
            onClick={() => onChange(m)}
            className={clsx(
              'num inline-flex items-center rounded-pill border font-medium transition-[background-color,border-color,color] duration-150 disabled:cursor-not-allowed disabled:opacity-50',
              size === 'sm' ? 'h-7 px-2.5 text-[11px]' : 'h-8 px-3 text-xs',
              active ? 'border-volt/40 bg-volt-soft text-volt' : 'border-line-2 bg-panel text-ink-2 hover:bg-panel-2 hover:text-ink',
            )}
          >
            {t('focus.session.minutes', { count: m })}
          </button>
        );
      })}
    </div>
  );
}
