import { useT } from '../../i18n';

/**
 * The line under a switch whose change is waiting out the delay. The switch itself is disabled: asking again
 * would be the same request, and the way to change one's mind is the banner ("Manter proteção").
 */
export function WaitingNote({ text = 'nofap.pending.switch_waiting', testId }: { text?: 'nofap.pending.switch_waiting' | 'nofap.pending.scope_waiting' | 'nofap.overall.pending_hint'; testId: string }) {
  const t = useT();
  return (
    <p className="text-xs leading-5 text-ink-3" data-testid={testId}>
      {t(text)}
    </p>
  );
}
