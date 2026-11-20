import { useState } from 'react';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { useT } from '../../i18n';
import type { Mood, Trigger } from '../types';
import { TriggerChips } from './LapseFlow';

const MOODS: Mood[] = ['ok', 'meh', 'hard', 'very_bad'];
/** HALT, as a prompt and not as a fact. */
const HALT: Trigger[] = ['hungry', 'angry', 'lonely', 'tired'];

interface Props {
  today: Mood | null;
  onCheckin: (mood: Mood, triggers: Trigger[]) => void;
}

/** The daily check-in: a card on the page, never a notification. One tap; on a hard day, the HALT chips. */
export function CheckinCard({ today, onCheckin }: Props) {
  const t = useT();
  const [hard, setHard] = useState<Mood | null>(null);
  const [triggers, setTriggers] = useState<Trigger[]>([]);

  const pick = (mood: Mood) => {
    if (mood === 'hard' || mood === 'very_bad') setHard(mood);
    else onCheckin(mood, []);
  };

  return (
    <Card data-testid="checkin-card">
      <CardHeader title={t('nofap.checkin.title')} subtitle={today ? t('nofap.checkin.marked', { mood: t(`nofap.mood.${today}`) }) : t('nofap.checkin.daily')} />
      {!today && !hard && (
        <div className="flex flex-wrap gap-1.5">
          {MOODS.map((mood) => (
            <Button key={mood} size="sm" onClick={() => pick(mood)}>
              {t(`nofap.mood.${mood}`)}
            </Button>
          ))}
        </div>
      )}
      {!today && hard && (
        <div className="flex flex-col gap-3">
          <p className="text-[13px] leading-5 text-ink-2">{t('nofap.checkin.halt')}</p>
          <TriggerChips options={HALT} value={triggers} onChange={setTriggers} label={t('nofap.lapse.triggers')} />
          <div className="flex justify-end">
            <Button variant="primary" size="sm" onClick={() => onCheckin(hard, triggers)}>
              {t('nofap.btn.save')}
            </Button>
          </div>
        </div>
      )}
    </Card>
  );
}
