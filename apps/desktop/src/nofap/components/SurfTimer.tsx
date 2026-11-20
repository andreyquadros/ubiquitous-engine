import { useEffect, useRef, useState } from 'react';
import clsx from 'clsx';
import { Button } from '../../components/ui/Button';
import { Progress } from '../../components/ui/misc';
import { useT } from '../../i18n';
import type { UserEvent } from '../types';

/** The five timed steps of the urge surf, 30 seconds each, then the rating and the closing line (design 5.5). */
export const SURF_STEPS = ['surf.start', 'surf.body', 'surf.breath', 'surf.wave', 'surf.thought'] as const;
export const SURF_STEP_SECS = 30;

interface Props {
  /** Counts `surf_started` when the exercise opens and `surf_done` when it is finished. Nothing else leaves it. */
  record: (event: UserEvent) => void;
  onDone: () => void;
}

/**
 * Three minutes of watching an urge instead of obeying it. The 0 to 10 answer is shown back on the same screen
 * and goes nowhere: it is not stored and not sent.
 */
export function SurfTimer({ record, onDone }: Props) {
  const t = useT();
  const [clock, setClock] = useState({ step: 0, left: SURF_STEP_SECS });
  const [rating, setRating] = useState<number | null>(null);
  const recordRef = useRef(record);
  recordRef.current = record;
  const started = useRef(false);
  const { step, left } = clock;
  const timed = step < SURF_STEPS.length;
  const rated = step === SURF_STEPS.length;
  const next = () => setClock((c) => ({ step: c.step + 1, left: SURF_STEP_SECS }));

  useEffect(() => {
    if (started.current) return;
    started.current = true;
    recordRef.current({ event: 'surf_started' });
  }, []);

  useEffect(() => {
    if (!timed) return;
    const timer = setInterval(() => setClock((c) => (c.step >= SURF_STEPS.length ? c : c.left > 1 ? { ...c, left: c.left - 1 } : { step: c.step + 1, left: SURF_STEP_SECS })), 1000);
    return () => clearInterval(timer);
  }, [timed]);

  const finish = () => {
    recordRef.current({ event: 'surf_done' });
    onDone();
  };

  return (
    <div className="flex flex-col gap-4" data-testid="surf-timer">
      {timed && (
        <>
          <p className="text-xs text-ink-3">{t('nofap.surf.step', { step: step + 1, total: SURF_STEPS.length })}</p>
          <p role="status" className="min-h-[72px] text-[15px] leading-6 text-ink" data-testid="surf-message">
            {t(`nofap.${SURF_STEPS[step]}`)}
          </p>
          <Progress value={SURF_STEP_SECS - left} max={SURF_STEP_SECS} />
          <div className="flex justify-end">
            <Button variant="ghost" size="sm" onClick={next}>
              {t('nofap.surf.next')}
            </Button>
          </div>
        </>
      )}
      {rated && (
        <>
          <p role="status" className="text-[15px] leading-6 text-ink" data-testid="surf-message">
            {t('nofap.surf.rate')}
          </p>
          <div role="radiogroup" aria-label={t('nofap.surf.rate_label')} className="flex flex-wrap gap-1.5">
            {Array.from({ length: 11 }, (_, n) => (
              <button
                key={n}
                type="button"
                role="radio"
                aria-checked={rating === n}
                onClick={() => setRating(n)}
                className={clsx(
                  'num size-8 rounded-[8px] border text-xs font-medium transition-colors duration-150',
                  rating === n ? 'border-volt bg-volt-soft text-ink' : 'border-line-2 bg-panel text-ink-2 hover:bg-panel-2 hover:text-ink',
                )}
              >
                {n}
              </button>
            ))}
          </div>
          {rating !== null && <p className="text-xs text-ink-3">{t('nofap.surf.rate_shown', { n: rating })}</p>}
          <div className="flex justify-end">
            <Button variant="primary" size="sm" onClick={next}>
              {t('nofap.btn.next')}
            </Button>
          </div>
        </>
      )}
      {!timed && !rated && (
        <>
          <p role="status" className="text-[15px] leading-6 text-ink" data-testid="surf-message">
            {t('nofap.surf.end')}
          </p>
          <div className="flex justify-end">
            <Button variant="primary" size="sm" onClick={finish}>
              {t('nofap.btn.done')}
            </Button>
          </div>
        </>
      )}
    </div>
  );
}
