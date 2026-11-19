import { useState } from 'react';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { useT } from '../../i18n';
import type { HelpResource } from '../types';
import { ResourceList } from './ResourceList';

/**
 * Shown when the last week was hard several times (a product heuristic, and the text says it is not a
 * diagnosis): a person can help, here is how to find one, and what the mascot is and is not.
 */
export function HelpCard({ resources }: { resources: HelpResource[] }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  return (
    <Card data-testid="help-card">
      <CardHeader title={t('nofap.help.title')} />
      <p className="text-[15px] leading-6 text-ink">{t('nofap.help.pattern')}</p>
      <p className="mt-2 text-[13px] leading-5 text-ink-2">{t('nofap.help.how')}</p>
      {open ? (
        <div className="mt-3 flex flex-col gap-3">
          <ResourceList resources={resources} />
          <p className="text-xs leading-5 text-ink-3">{t('nofap.help.limits')}</p>
        </div>
      ) : (
        <div className="mt-3">
          <Button size="sm" onClick={() => setOpen(true)}>
            {t('nofap.btn.help')}
          </Button>
        </div>
      )}
    </Card>
  );
}
