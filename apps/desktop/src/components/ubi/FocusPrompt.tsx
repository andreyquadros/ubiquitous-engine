import { motion, useReducedMotion } from 'framer-motion';
import { Shield } from 'lucide-react';
import { useState, type FormEvent } from 'react';
import { useT } from '../../i18n';
import { ipc } from '../../lib/ipc';
import { useAppStore } from '../../lib/store';
import { useToast } from '../../lib/toast';
import type { Mood, Nudge } from '../../lib/types';
import { DurationChips } from '../focus/DurationChips';
import { Button } from '../ui/Button';
import { Ubi } from './Ubi';

interface Props {
  nudge: Nudge;
  mood: Mood;
  size?: number;
  /** Settings.focus.session_minutes. */
  defaultMinutes: number;
}

/**
 * UBI's bubble as a mini form, shown on Hoje when the latest nudge is a `focus_prompt` ("a lot of windows"):
 * the message, the task input, the duration chips and "Me ajude a focar". Submitting starts a focus session,
 * marks the nudges seen and leaves UBI saying what he is holding.
 */
export function FocusPrompt({ nudge, mood, size = 200, defaultMinutes }: Props) {
  const t = useT();
  const toast = useToast();
  const reduce = useReducedMotion();
  const startFocusSession = useAppStore((s) => s.startFocusSession);
  const clearUnseen = useAppStore((s) => s.clearUnseen);
  const setUbiSpeech = useAppStore((s) => s.setUbiSpeech);
  const [task, setTask] = useState('');
  const [minutes, setMinutes] = useState(defaultMinutes);
  const [busy, setBusy] = useState(false);

  const seen = async () => {
    clearUnseen();
    try {
      await ipc.markNudgesSeen();
    } catch {
      /* the nudge list is refreshed on the next dashboard load anyway */
    }
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    const trimmed = task.trim();
    if (!trimmed) return;
    setBusy(true);
    try {
      const session = await startFocusSession(trimmed, minutes);
      setUbiSpeech(t('focus.session.done', { minutes, task: session.task }));
      await seen();
    } catch (err) {
      toast.error(t('focus.session.start_failed'), err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const dismiss = async () => {
    setUbiSpeech(t(`ubi.tip.${mood}`));
    await seen();
  };

  return (
    <div className="relative inline-flex flex-col items-center" data-testid="focus-prompt">
      <motion.form
        onSubmit={(e) => void submit(e)}
        role="status"
        aria-label={nudge.title}
        initial={reduce ? { opacity: 0 } : { opacity: 0, y: 6, scale: 0.96 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={{ duration: reduce ? 0 : 0.2, ease: [0.16, 1, 0.3, 1] }}
        className="glass relative z-10 mb-2 flex w-[300px] flex-col gap-2.5 px-3 py-2.5 text-left text-xs leading-5 text-ink"
      >
        <p>{nudge.message}</p>
        <input
          aria-label={t('focus.prompt.task_label')}
          value={task}
          onChange={(e) => setTask(e.target.value)}
          placeholder={t('focus.session.task_placeholder')}
          autoComplete="off"
          maxLength={200}
          disabled={busy}
          className="h-8 w-full rounded-[8px] border border-line-2 bg-panel-2 px-2.5 text-xs text-ink placeholder:text-ink-4 focus:border-volt focus:outline-none focus:ring-2 focus:ring-volt/25 disabled:opacity-50"
        />
        <DurationChips value={minutes} onChange={setMinutes} extra={[defaultMinutes]} size="sm" disabled={busy} label={t('focus.session.duration')} />
        <div className="flex items-center justify-between gap-2">
          <button type="button" onClick={() => void dismiss()} className="text-[11px] text-ink-3 hover:text-ink-2" disabled={busy}>
            {t('focus.prompt.dismiss')}
          </button>
          <Button type="submit" variant="primary" size="sm" icon={<Shield className="size-3.5" strokeWidth={1.75} aria-hidden />} loading={busy} disabled={!task.trim()}>
            {t('focus.prompt.submit')}
          </Button>
        </div>
        <span className="absolute -bottom-1.5 left-1/2 size-3 -translate-x-1/2 rotate-45 border-r border-b border-line-2 bg-[color-mix(in_oklab,var(--panel)_86%,transparent)]" aria-hidden />
      </motion.form>
      <Ubi mood={mood} size={size} />
    </div>
  );
}
