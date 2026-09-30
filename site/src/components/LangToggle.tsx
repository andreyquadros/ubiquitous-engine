import { LOCALES, fill, useCopy, type Locale } from '../i18n';
import { cx } from './cx';

/** pt-BR / EN segmented control. */
export function LangToggle({ className }: { className?: string }) {
  const { locale, copy, setLocale } = useCopy();
  const names: Record<Locale, string> = { 'pt-BR': 'Português', en: 'English' };
  return (
    <div className={cx('inline-flex h-9 items-center rounded-control border border-line-2 bg-panel-2 p-0.5', className)} role="group" aria-label={copy.nav.language}>
      {LOCALES.map((l) => (
        <button
          key={l}
          type="button"
          lang={l}
          aria-pressed={locale === l}
          aria-label={fill(copy.lang.switchTo, { lang: names[l] })}
          onClick={() => setLocale(l)}
          className={cx(
            'h-full rounded-[8px] px-2.5 text-xs font-semibold transition-colors',
            locale === l ? 'bg-panel-3 text-ink' : 'text-ink-3 hover:text-ink-2',
          )}
        >
          {copy.lang[l]}
        </button>
      ))}
    </div>
  );
}
