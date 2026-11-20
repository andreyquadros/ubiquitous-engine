import { AppWindow, Globe, ShieldCheck } from 'lucide-react';
import { useT } from '../../i18n';
import { fmtTime } from '../../lib/format';
import type { Intervention } from '../../lib/types';
import { Badge } from '../ui/Badge';
import { EmptyState } from '../ui/misc';

/** Time, the target as a chip, what UBI said and what it did. */
export function InterventionsList({ interventions, limit = 12 }: { interventions: Intervention[]; limit?: number }) {
  const t = useT();
  if (!interventions.length) {
    return <EmptyState icon={<ShieldCheck className="size-5" strokeWidth={1.75} aria-hidden />} title={t('focus.interventions.empty_title')} description={t('focus.interventions.empty_description')} className="py-8" />;
  }
  return (
    <ul className="divide-y divide-line" data-testid="focus-interventions">
      {interventions.slice(0, limit).map((i) => {
        const Icon = i.kind === 'app' ? AppWindow : Globe;
        return (
          <li key={i.id} className="flex items-start gap-3 py-2.5">
            <span className="num w-11 shrink-0 pt-0.5 text-xs text-ink-3">{fmtTime(i.at)}</span>
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <Badge tone="neutral">
                  <Icon className="size-3" strokeWidth={1.75} aria-hidden />
                  {i.name}
                </Badge>
                <span className="text-[11px] text-ink-3">{t(`focus.action.${i.action}`)}</span>
              </div>
              <p className="mt-1 text-sm leading-5 text-ink-2">{i.message}</p>
            </div>
          </li>
        );
      })}
    </ul>
  );
}
