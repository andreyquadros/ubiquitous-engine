import { useEffect, useRef, useState } from 'react';
import { Button } from '../../components/ui/Button';
import { useLocale, useT } from '../../i18n';
import { crisisText, fmtClock } from '../labels';
import type { HelpResource, IfThenPlan, UserEvent } from '../types';
import { ResourceList } from './ResourceList';
import { SurfTimer } from './SurfTimer';

export const STEP_AWAY_SECS = 600;

export type PanicView = 'options' | 'surf' | 'away' | 'talk';

interface Props {
  values: string;
  plans: IfThenPlan[];
  resources: HelpResource[];
  record: (event: UserEvent) => void;
  onClose: () => void;
  /** Opens directly on one of the three options (the card window's buttons). */
  start?: PanicView;
  /** `false` when the caller already counted this opening (a card's own button). */
  countPanic?: boolean;
}

/** The crisis sentence with the numbers of the resources payload, and the list itself. One step from every screen. */
export function CrisisCard({ resources }: { resources: HelpResource[] }) {
  const t = useT();
  const locale = useLocale();
  const text = crisisText(locale, resources);
  return (
    <div className="flex flex-col gap-3" data-testid="crisis-card">
      <h3 className="text-sm font-semibold text-ink">{t('nofap.crisis.title')}</h3>
      {text && <p className="text-[13px] leading-5 text-ink">{text}</p>}
      <ResourceList resources={resources} />
    </div>
  );
}

/** Ten minutes away from the screen. A countdown and nothing else: nothing is locked. */
function StepAway({ onBack }: { onBack: () => void }) {
  const t = useT();
  const [left, setLeft] = useState(STEP_AWAY_SECS);
  useEffect(() => {
    const timer = setInterval(() => setLeft((v) => Math.max(0, v - 1)), 1000);
    return () => clearInterval(timer);
  }, []);
  return (
    <div className="flex flex-col gap-3" data-testid="step-away">
      <h3 className="text-sm font-semibold text-ink">{t('nofap.away.title')}</h3>
      <p className="text-[15px] leading-6 text-ink">{t('nofap.panic.away')}</p>
      <p role="timer" className="num display text-[22px] leading-7 text-ink">
        {left > 0 ? t('nofap.away.remaining', { time: fmtClock(left) }) : t('nofap.away.done')}
      </p>
      <div className="flex justify-end">
        <Button size="sm" onClick={onBack}>
          {t('nofap.away.back')}
        </Button>
      </div>
    </div>
  );
}

/**
 * "Preciso de um respiro": the user's own sentence and plan, then three ways through the moment: the 3-minute
 * surf, ten minutes away from the screen, or a person to talk to.
 */
export function PanicSheet({ values, plans, resources, record, onClose, start = 'options', countPanic = true }: Props) {
  const t = useT();
  const [view, setView] = useState<PanicView>(start);
  const recordRef = useRef(record);
  recordRef.current = record;

  const counted = useRef(false);

  useEffect(() => {
    if (!countPanic || counted.current) return;
    counted.current = true;
    recordRef.current({ event: 'panic' });
  }, [countPanic]);

  if (view === 'surf') return <SurfTimer record={record} onDone={onClose} />;
  if (view === 'away') return <StepAway onBack={onClose} />;
  if (view === 'talk') {
    return (
      <div className="flex flex-col gap-3">
        <CrisisCard resources={resources} />
        <div className="flex justify-end">
          <Button size="sm" onClick={onClose}>
            {t('nofap.btn.close')}
          </Button>
        </div>
      </div>
    );
  }

  const plan = plans[0];
  return (
    <div className="flex flex-col gap-3" data-testid="panic-sheet">
      <p className="text-[15px] leading-6 text-ink">{t('nofap.panic.open')}</p>
      {values && <p className="panel-raised p-3 text-[13px] leading-5 text-ink-2">{t('nofap.panic.reason', { reason: values })}</p>}
      {plan && <p className="text-[13px] leading-5 text-ink-2">{t('nofap.plans.item', { when: plan.when, then: plan.then })}</p>}
      <p className="text-[13px] leading-5 text-ink-2">{t('nofap.panic.options')}</p>
      <div className="flex flex-col gap-2">
        <Button variant="primary" onClick={() => setView('surf')}>
          {t('nofap.btn.surf')}
        </Button>
        <Button
          onClick={() => {
            record({ event: 'step_away' });
            setView('away');
          }}
        >
          {t('nofap.btn.step_away')}
        </Button>
        <Button onClick={() => setView('talk')}>{t('nofap.btn.talk')}</Button>
      </div>
    </div>
  );
}
