import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Field } from '../../components/ui/Field';
import { Skeleton } from '../../components/ui/misc';
import { Toggle } from '../../components/ui/Toggle';
import { useT } from '../../i18n';
import { CheckinCard } from '../components/CheckinCard';
import { HelpCard } from '../components/HelpCard';
import { ProgressCard } from '../components/ProgressCard';
import { ValuesAndPlans } from '../components/ValuesAndPlans';
import type { NofapDialog } from '../NofapPage';
import type { NofapChange, NofapView, SupportView, UserEvent } from '../types';

interface Props {
  view: NofapView;
  support: SupportView | null;
  record: (event: UserEvent) => void;
  onRequest: (change: NofapChange) => Promise<void>;
  onOpen: (dialog: NofapDialog) => void;
}

/** "Apoio": the exercises, the days, the check-in, the user's own sentence and plans. */
export function SupportTab({ view, support, record, onRequest, onOpen }: Props) {
  const t = useT();
  const ui = view.config.ui;
  if (!support) return <Skeleton className="h-64" />;
  return (
    <div className="flex flex-col gap-5" data-testid="tab-support">
      <Card>
        <CardHeader title={t('nofap.support.breather_title')} subtitle={t('nofap.surf.start')} />
        <div className="flex flex-wrap gap-2">
          <Button variant="primary" size="sm" onClick={() => onOpen('surf')}>
            {t('nofap.btn.surf')}
          </Button>
          <Button size="sm" onClick={() => onOpen('panic')}>
            {t('nofap.btn.panic')}
          </Button>
          <Button variant="ghost" size="sm" onClick={() => onOpen('lapse')} data-testid="nofap-lapse">
            {t('nofap.lapse.button')}
          </Button>
        </div>
      </Card>

      {support.help_due && <HelpCard resources={support.resources} />}
      <ProgressCard progress={support.progress} />
      {ui.daily_checkin && <CheckinCard today={support.checkin_today} onCheckin={(mood, triggers) => record({ event: 'checkin', mood, triggers })} />}
      <ValuesAndPlans values={support.values} plans={support.plans} onSaveValues={(text) => onRequest({ kind: 'set_values', text })} onSavePlans={(plans) => onRequest({ kind: 'set_plans', plans })} />

      <Card>
        <div className="flex flex-col gap-4">
          <Field label={t('nofap.prefs.checkin')} hint={t('nofap.prefs.checkin_hint')} inline>
            {(id) => <Toggle id={id} size="sm" checked={ui.daily_checkin} onChange={(daily_checkin) => void onRequest({ kind: 'set_prefs', prefs: { ...ui, daily_checkin } })} />}
          </Field>
          <Field label={t('nofap.prefs.streak')} hint={t('nofap.prefs.streak_hint')} inline>
            {(id) => <Toggle id={id} size="sm" checked={ui.show_streak} onChange={(show_streak) => void onRequest({ kind: 'set_prefs', prefs: { ...ui, show_streak } })} />}
          </Field>
        </div>
      </Card>
    </div>
  );
}
