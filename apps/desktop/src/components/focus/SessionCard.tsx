import { Shield, Square } from 'lucide-react';
import { useEffect, useState, type FormEvent } from 'react';
import { useT } from '../../i18n';
import { fmtCountdown, fmtTime } from '../../lib/format';
import { useAppStore } from '../../lib/store';
import type { FocusStatus } from '../../lib/types';
import { Button } from '../ui/Button';
import { Card, CardHeader } from '../ui/Card';
import { Field, Input } from '../ui/Field';
import { DurationChips } from './DurationChips';
import { useCountdown } from './useCountdown';

interface Props {
  status: FocusStatus | null;
  /** Settings.focus.session_minutes: the preselected length. */
  defaultMinutes: number;
  onStart: (task: string, minutes: number) => Promise<unknown>;
  onStop: () => Promise<unknown>;
  /** Called when the local countdown reaches zero (the page reloads the status). */
  onElapsed?: () => void;
}

/** Idle: the task input, the duration chips and "Focar". Active: task, mm:ss countdown, distractions held and "Encerrar". */
export function SessionCard({ status, defaultMinutes, onStart, onStop, onElapsed }: Props) {
  const t = useT();
  // macOS runs a Shortcut by name; Windows and Linux run a command line (same setting, other label).
  const platform = useAppStore((s) => s.settingsView?.platform ?? 'macos');
  const session = status?.session ?? null;
  const [task, setTask] = useState('');
  const [minutes, setMinutes] = useState(defaultMinutes);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const remaining = useCountdown(session, status?.remaining_secs ?? null, onElapsed);

  useEffect(() => setMinutes(defaultMinutes), [defaultMinutes]);

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = task.trim();
    if (!trimmed) {
      setError(t('focus.session.invalid_task'));
      return;
    }
    if (minutes < 5 || minutes > 240) {
      setError(t('focus.session.invalid_minutes'));
      return;
    }
    setError(null);
    setBusy(true);
    try {
      await onStart(trimmed, minutes);
      setTask('');
    } finally {
      setBusy(false);
    }
  };

  const stop = async () => {
    setBusy(true);
    try {
      await onStop();
    } finally {
      setBusy(false);
    }
  };

  if (session) {
    return (
      <Card data-testid="focus-session-card" className="relative overflow-hidden">
        <div className="pointer-events-none absolute inset-0" style={{ background: 'var(--hero-glow)' }} aria-hidden />
        <div className="relative">
          <CardHeader
            title={t('focus.session.title')}
            subtitle={t('focus.session.active_subtitle')}
            action={
              <Button variant="secondary" size="sm" icon={<Square className="size-3.5" strokeWidth={1.75} aria-hidden />} onClick={() => void stop()} loading={busy}>
                {t('focus.session.stop')}
              </Button>
            }
          />
          <div className="flex flex-wrap items-end justify-between gap-6">
            <div className="min-w-0 flex-1">
              <p className="display truncate text-[20px] leading-7 text-ink">{session.task}</p>
              <p className="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-ink-3">
                <span className="num">{t('focus.session.started_at', { time: fmtTime(session.started_at) })}</span>
                <span className="num text-ink-2">{t('focus.session.interventions', { count: session.interventions })}</span>
                {session.hid_windows && <span>{t('focus.session.hid_windows')}</span>}
                {session.ran_shortcut && <span>{t(platform === 'macos' ? 'focus.session.ran_shortcut' : 'focus.session.ran_shortcut.other')}</span>}
              </p>
            </div>
            <div className="text-right">
              <p className="text-xs text-ink-3">{t('focus.session.remaining')}</p>
              <p className="display num text-[40px] leading-10 text-volt" data-testid="focus-countdown" aria-live="off">
                {fmtCountdown(remaining ?? status?.remaining_secs ?? 0)}
              </p>
            </div>
          </div>
        </div>
      </Card>
    );
  }

  return (
    <Card data-testid="focus-session-card">
      <CardHeader title={t('focus.session.title')} subtitle={t('focus.session.idle_subtitle')} />
      <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-4">
        <Field label={t('focus.session.task_label')}>
          {(id) => (
            <Input
              id={id}
              value={task}
              onChange={(e) => {
                setTask(e.target.value);
                if (error) setError(null);
              }}
              placeholder={t('focus.session.task_placeholder')}
              autoComplete="off"
              maxLength={200}
            />
          )}
        </Field>
        <div className="flex flex-wrap items-end justify-between gap-4">
          <div className="flex flex-col gap-1.5">
            <span className="text-[13px] font-medium text-ink">{t('focus.session.duration')}</span>
            <DurationChips value={minutes} onChange={setMinutes} extra={[defaultMinutes]} label={t('focus.session.duration')} />
          </div>
          <Button type="submit" variant="primary" icon={<Shield className="size-4" strokeWidth={1.75} aria-hidden />} loading={busy}>
            {t('focus.session.start')}
          </Button>
        </div>
        {error && (
          <p className="text-xs text-rose" role="alert">
            {error}
          </p>
        )}
      </form>
    </Card>
  );
}
