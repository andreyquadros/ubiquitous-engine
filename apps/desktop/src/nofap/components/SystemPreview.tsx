import { Skeleton } from '../../components/ui/misc';
import { useT } from '../../i18n';
import { PREVIEW_PLACEHOLDER } from '../labels';
import type { PlannedChange } from '../types';

/** The exact list of changes, shown before the operating system asks for the administrator password. */
export function SystemPreview({ lines }: { lines: PlannedChange[] | null }) {
  const t = useT();
  if (!lines) return <Skeleton className="h-20" />;
  if (!lines.length) return <p className="text-sm text-ink-2">{t('nofap.system.preview_empty')}</p>;
  return (
    <div className="flex flex-col gap-3" data-testid="system-preview">
      <ul className="flex flex-col gap-2">
        {lines.map((line, i) => {
          const placeholder = PREVIEW_PLACEHOLDER[line.text_key];
          return (
            <li key={`${line.text_key}-${i}`} className="flex gap-2.5 text-[13px] leading-5 text-ink-2">
              <span className="mt-2 size-1.5 shrink-0 rounded-full bg-volt" aria-hidden />
              <span>{t(`nofap.${line.text_key}`, placeholder && line.detail !== null ? { [placeholder]: line.detail } : undefined)}</span>
            </li>
          );
        })}
      </ul>
      <p className="text-xs text-ink-3">{t('nofap.system.prompt_next')}</p>
    </div>
  );
}
