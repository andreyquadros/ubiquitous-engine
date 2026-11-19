import clsx from 'clsx';
import { useT } from '../../i18n';
import type { ResolverId } from '../types';

const OPTIONS: (ResolverId | null)[] = ['cloudflare_family', null];

/**
 * Who answers this computer's name lookups: the family resolver, with its operator named, or nobody new
 * ("só a lista neste computador", the choice the DNS disclosure points to).
 */
export function ResolverChoice({ value, onChange, disabled }: { value: ResolverId | null; onChange: (resolver: ResolverId | null) => void; disabled?: boolean }) {
  const t = useT();
  return (
    <div role="radiogroup" aria-label={t('nofap.system.dns_choice')} className="flex flex-wrap gap-1.5">
      {OPTIONS.map((option) => (
        <button
          key={option ?? 'none'}
          type="button"
          role="radio"
          aria-checked={value === option}
          disabled={disabled}
          onClick={() => value !== option && onChange(option)}
          className={clsx(
            'h-8 rounded-[8px] border px-3 text-xs font-medium transition-colors duration-150 disabled:cursor-not-allowed disabled:opacity-50',
            value === option ? 'border-volt bg-volt-soft text-ink' : 'border-line-2 bg-panel text-ink-2 hover:bg-panel-2 hover:text-ink',
          )}
        >
          {t(option ? `nofap.resolver.${option}` : 'nofap.resolver.none')}
        </button>
      ))}
    </div>
  );
}
