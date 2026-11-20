import clsx from 'clsx';
import { useT } from '../../i18n';
import { DELAY_CHOICES, fmtDelay, fmtDelayShort } from '../labels';

/** The waiting time as a row of chips (0, 5, 10, 30, 60 minutes, 4 h, 24 h). A stored value off the list gets its own chip. */
export function DelayPicker({ value, onChange, disabled }: { value: number; onChange: (secs: number) => void; disabled?: boolean }) {
  const t = useT();
  const choices: number[] = (DELAY_CHOICES as readonly number[]).includes(value) ? [...DELAY_CHOICES] : [...DELAY_CHOICES, value].sort((a, b) => a - b);
  return (
    <div role="radiogroup" aria-label={t('nofap.delay.label')} className="flex flex-wrap gap-1.5">
      {choices.map((secs) => (
        <button
          key={secs}
          type="button"
          role="radio"
          aria-checked={value === secs}
          aria-label={secs === 0 ? t('nofap.delay.none') : fmtDelay(secs, t)}
          disabled={disabled}
          onClick={() => onChange(secs)}
          className={clsx(
            'num h-8 rounded-[8px] border px-3 text-xs font-medium transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-50',
            value === secs ? 'border-volt bg-volt-soft text-ink' : 'border-line-2 bg-panel text-ink-2 hover:bg-panel-2 hover:text-ink',
          )}
        >
          {fmtDelayShort(secs, t)}
        </button>
      ))}
    </div>
  );
}
