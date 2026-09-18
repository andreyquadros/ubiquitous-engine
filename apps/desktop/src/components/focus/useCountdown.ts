import { useEffect, useRef, useState } from 'react';
import type { FocusSession } from '../../lib/types';

const remainingOf = (session: FocusSession): number => Math.max(0, Math.round((new Date(session.ends_at).getTime() - Date.now()) / 1000));

/**
 * Seconds left in `session`, ticking once a second locally (seeded from the status' `remaining_secs` when given,
 * then derived from `ends_at`). Calls `onDone` once when it reaches zero. `null` without an active session.
 */
export function useCountdown(session: FocusSession | null, remainingSecs: number | null = null, onDone?: () => void): number | null {
  const [secs, setSecs] = useState<number | null>(session ? (remainingSecs ?? remainingOf(session)) : null);
  const doneRef = useRef(onDone);
  doneRef.current = onDone;
  const id = session?.id ?? null;
  const endsAt = session?.ends_at ?? null;

  useEffect(() => {
    if (!session) {
      setSecs(null);
      return;
    }
    setSecs(remainingSecs ?? remainingOf(session));
    const timer = setInterval(() => {
      const v = remainingOf(session);
      setSecs(v);
      if (v === 0) {
        clearInterval(timer);
        doneRef.current?.();
      }
    }, 1000);
    return () => clearInterval(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id, endsAt]);

  return secs;
}
