import { Hourglass } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { Button } from '../../components/ui/Button';
import { useT } from '../../i18n';
import { fmtClock, fmtDelay, pendingLabel } from '../labels';
import type { Id, NofapView, PendingView, SitesView } from '../types';

interface Props {
  view: NofapView;
  sites: SitesView | null;
  /** "Manter proteção": cancels the request. One tap, always available. */
  onCancel: (id: Id) => void;
  /** Only offered once the request is ready. */
  onConfirm: (id: Id) => void;
  onSurf: () => void;
  /** Called when a countdown reaches zero, so the page reads the fresh state. */
  onElapsed: () => void;
}

/** Seconds left of `pending`, counted down locally from the value the engine sent. */
function useRemaining(pending: PendingView, onElapsed: () => void): number {
  const [left, setLeft] = useState(pending.remaining_secs);
  const elapsed = useRef(onElapsed);
  elapsed.current = onElapsed;
  useEffect(() => {
    setLeft(pending.remaining_secs);
    if (pending.ready || pending.remaining_secs <= 0) return;
    const startedAt = Date.now();
    const timer = setInterval(() => {
      const v = Math.max(0, pending.remaining_secs - Math.round((Date.now() - startedAt) / 1000));
      setLeft(v);
      if (v === 0) {
        clearInterval(timer);
        elapsed.current();
      }
    }, 1000);
    return () => clearInterval(timer);
  }, [pending.id, pending.remaining_secs, pending.ready]);
  return left;
}

function PendingRow({ pending, view, sites, onCancel, onConfirm, onElapsed }: Omit<Props, 'onSurf'> & { pending: PendingView }) {
  const t = useT();
  const left = useRemaining(pending, onElapsed);
  const name = view.display_name;
  const isDisable = pending.kind === 'disable';
  return (
    <li className="flex flex-col gap-2.5 py-3 first:pt-0 last:pb-0" data-testid={`pending-${pending.id}`}>
      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <p className="text-sm font-medium text-ink">{pendingLabel(pending, view.config, sites, name, t)}</p>
        {!pending.ready && (
          <p role="timer" className="num display text-[22px] leading-7 text-ink" data-testid="pending-countdown">
            {fmtClock(left)}
          </p>
        )}
      </div>
      <p className="text-[13px] leading-5 text-ink-2">
        {pending.ready ? t(isDisable ? 'nofap.pending.ready' : 'nofap.pending.ready_generic') : t('nofap.pending.wait', { delay: fmtDelay(left, t) })}
        {pending.ready && pending.expires_at ? ` ${t('nofap.pending.expires', { when: new Date(pending.expires_at).toLocaleString([], { day: '2-digit', month: '2-digit', hour: '2-digit', minute: '2-digit' }) })}` : ''}
      </p>
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="primary" size="sm" onClick={() => onCancel(pending.id)}>
          {t('nofap.btn.keep_protection')}
        </Button>
        {pending.ready && (
          <Button variant="ghost" size="sm" onClick={() => onConfirm(pending.id)}>
            {t(isDisable ? 'nofap.btn.turn_off' : 'nofap.btn.confirm')}
          </Button>
        )}
      </div>
    </li>
  );
}

/**
 * Requests that loosen protection, waiting out the delay: a quiet countdown, the user's own sentence, the way
 * back ("Manter proteção") and the surf button. No text argues.
 */
export function PendingBanner({ view, sites, onCancel, onConfirm, onSurf, onElapsed }: Props) {
  const t = useT();
  const pending = view.status.pending;
  if (!pending.length) return null;
  return (
    <section className="panel flex flex-col gap-3 p-5" aria-label={t('nofap.pending.title')} data-testid="pending-banner">
      <div className="flex items-center justify-between gap-3">
        <h2 className="flex items-center gap-2 text-sm font-semibold text-ink">
          <Hourglass className="size-4 text-ink-3" strokeWidth={1.75} aria-hidden />
          {t('nofap.pending.title')}
        </h2>
        <Button variant="ghost" size="sm" onClick={onSurf}>
          {t('nofap.btn.surf')}
        </Button>
      </div>
      {view.config.values && <p className="panel-raised p-3 text-[13px] leading-5 text-ink-2">{t('nofap.panic.reason', { reason: view.config.values })}</p>}
      <ul className="flex flex-col divide-y divide-line">
        {pending.map((p) => (
          <PendingRow key={p.id} pending={p} view={view} sites={sites} onCancel={onCancel} onConfirm={onConfirm} onElapsed={onElapsed} />
        ))}
      </ul>
    </section>
  );
}
