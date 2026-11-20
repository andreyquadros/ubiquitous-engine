import clsx from 'clsx';
import { useState } from 'react';
import { Button } from '../../components/ui/Button';
import { useT } from '../../i18n';
import type { Trigger } from '../types';

export const TRIGGERS: Trigger[] = ['tired', 'bored', 'stressed', 'lonely', 'hungry', 'angry', 'other'];

/** Chips for "what was going on": optional, any number, counted and never stored as an event. */
export function TriggerChips({ options = TRIGGERS, value, onChange, label }: { options?: Trigger[]; value: Trigger[]; onChange: (v: Trigger[]) => void; label: string }) {
  const t = useT();
  const toggle = (trigger: Trigger) => onChange(value.includes(trigger) ? value.filter((x) => x !== trigger) : [...value, trigger]);
  return (
    <div role="group" aria-label={label} className="flex flex-wrap gap-1.5">
      {options.map((trigger) => (
        <button
          key={trigger}
          type="button"
          aria-pressed={value.includes(trigger)}
          onClick={() => toggle(trigger)}
          className={clsx(
            'h-8 rounded-pill border px-3 text-xs font-medium transition-colors duration-150',
            value.includes(trigger) ? 'border-volt bg-volt-soft text-ink' : 'border-line-2 bg-panel text-ink-2 hover:bg-panel-2 hover:text-ink',
          )}
        >
          {t(`nofap.trigger.${trigger}`)}
        </button>
      ))}
    </div>
  );
}

interface Props {
  /** Counts the lapse with whatever triggers were picked (none is fine). */
  onRecord: (triggers: Trigger[]) => void;
  /** "Ajustar um plano": the caller takes the user to the plans. */
  onAdjustPlan: () => void;
  onClose: () => void;
}

/**
 * "Aconteceu um deslize". Only the user starts it. Four short screens: it happened, what was going on (optional),
 * a plan for next time (optional), the next small step.
 */
export function LapseFlow({ onRecord, onAdjustPlan, onClose }: Props) {
  const t = useT();
  const [step, setStep] = useState(0);
  const [triggers, setTriggers] = useState<Trigger[]>([]);

  if (step === 0) {
    return (
      <div className="flex flex-col gap-3" data-testid="lapse-flow">
        <p className="text-[15px] leading-6 text-ink">{t('nofap.lapse.first')}</p>
        <p className="text-[13px] leading-5 text-ink-2">{t('nofap.lapse.kind')}</p>
        <div className="flex justify-end">
          <Button variant="primary" size="sm" onClick={() => setStep(1)}>
            {t('nofap.btn.next')}
          </Button>
        </div>
      </div>
    );
  }
  if (step === 1) {
    return (
      <div className="flex flex-col gap-3" data-testid="lapse-flow">
        <p className="text-[15px] leading-6 text-ink">{t('nofap.lapse.learn')}</p>
        <TriggerChips value={triggers} onChange={setTriggers} label={t('nofap.lapse.triggers')} />
        <div className="flex justify-end">
          <Button
            variant="primary"
            size="sm"
            onClick={() => {
              onRecord(triggers);
              setStep(2);
            }}
          >
            {t('nofap.btn.next')}
          </Button>
        </div>
      </div>
    );
  }
  if (step === 2) {
    return (
      <div className="flex flex-col gap-3" data-testid="lapse-flow">
        <p className="text-[15px] leading-6 text-ink">{t('nofap.lapse.plan')}</p>
        <div className="flex justify-end gap-2">
          <Button variant="ghost" size="sm" onClick={() => setStep(3)}>
            {t('nofap.btn.skip')}
          </Button>
          <Button size="sm" onClick={onAdjustPlan}>
            {t('nofap.lapse.plan_yes')}
          </Button>
        </div>
      </div>
    );
  }
  return (
    <div className="flex flex-col gap-3" data-testid="lapse-flow">
      <p className="text-[15px] leading-6 text-ink">{t('nofap.lapse.resume')}</p>
      <div className="flex justify-end">
        <Button variant="primary" size="sm" onClick={onClose}>
          {t('nofap.btn.close')}
        </Button>
      </div>
    </div>
  );
}
