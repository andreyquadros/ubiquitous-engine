import { Phone } from 'lucide-react';
import { useLocale, useT } from '../../i18n';
import { resourceNote } from '../labels';
import type { HelpResource } from '../types';

/**
 * The help resources the engine sends. Numbers and addresses are plain text on purpose: the page opens nothing
 * by itself, and the address can be read out or typed anywhere.
 */
export function ResourceList({ resources }: { resources: HelpResource[] }) {
  const t = useT();
  const locale = useLocale();
  if (!resources.length) return <p className="text-xs text-ink-3">{t('nofap.crisis.unavailable')}</p>;
  return (
    <ul className="flex flex-col divide-y divide-line" aria-label={t('nofap.resources.title')} data-testid="resource-list">
      {resources.map((resource) => (
        <li key={resource.id} className="flex flex-col gap-0.5 py-2.5 first:pt-0 last:pb-0">
          <p className="flex flex-wrap items-center gap-x-2 text-sm font-medium text-ink">
            {resource.name}
            {resource.phone && (
              <span className="num inline-flex items-center gap-1 text-volt" aria-label={t('nofap.resources.phone', { phone: resource.phone })}>
                <Phone className="size-3.5" strokeWidth={1.75} aria-hidden />
                {resource.phone}
              </span>
            )}
          </p>
          <p className="text-xs leading-5 text-ink-3">{resourceNote(resource, locale)}</p>
          {resource.url && <p className="text-xs break-all text-ink-2 select-text">{resource.url.replace(/^https?:\/\//, '')}</p>}
        </li>
      ))}
    </ul>
  );
}
