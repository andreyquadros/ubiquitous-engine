import clsx from 'clsx';
import { LOCALES, useT, type Locale } from '../../i18n';

interface Props {
  value: Locale;
  onChange: (l: Locale) => void;
  /** Smaller control for the rail or a dialog footer. */
  compact?: boolean;
  className?: string;
}

/** Segmented control listing the supported languages, each label written in its own language. */
export function LanguageSelect({ value, onChange, compact, className }: Props) {
  const t = useT();
  return (
    <div role="radiogroup" aria-label={t('ui.language')} className={clsx('inline-flex items-center gap-0.5 rounded-control border border-line bg-panel-2 p-1', className)}>
      {LOCALES.map((l) => {
        const active = l.id === value;
        return (
          <button
            key={l.id}
            type="button"
            role="radio"
            aria-checked={active}
            lang={l.id}
            onClick={() => onChange(l.id)}
            className={clsx(
              'rounded-[7px] font-medium transition-colors duration-150',
              compact ? 'h-6 px-2 text-xs' : 'h-7 px-3 text-sm',
              active ? 'bg-panel text-ink shadow-[inset_0_0_0_1px_var(--line-2)]' : 'text-ink-2 hover:text-ink',
            )}
          >
            {l.label}
          </button>
        );
      })}
    </div>
  );
}
