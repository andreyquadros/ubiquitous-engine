import { Card, CardHeader } from '../../components/ui/Card';
import { useT } from '../../i18n';
import type { ProgressView } from '../types';

/** The only milestones there are; they show on the page and nowhere else. */
const MILESTONES: Record<number, string> = { 1: 'nofap.streak.day1', 7: 'nofap.streak.week', 30: 'nofap.streak.month' };

/**
 * Days that went the user's way in the window. The streak is off unless the user asked for it, small, and never
 * animated; after a lapse the sentence says the count restarts and the practice does not.
 */
export function ProgressCard({ progress }: { progress: ProgressView }) {
  const t = useT();
  const milestone = progress.streak !== null ? MILESTONES[progress.streak] : undefined;
  return (
    <Card data-testid="progress-card">
      <CardHeader title={t('nofap.progress.title')} />
      <p className="text-[15px] leading-6 text-ink">{t(progress.after_reset ? 'nofap.streak.after_reset' : 'nofap.progress.days', { days: progress.days })}</p>
      {progress.streak !== null && (
        <p className="num mt-2 text-xs text-ink-3" data-testid="progress-streak">
          {t('nofap.streak.label', { count: progress.streak })}
        </p>
      )}
      {milestone && <p className="mt-1 text-xs text-ink-2">{t(milestone)}</p>}
    </Card>
  );
}
