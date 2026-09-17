import { Pause, Play } from 'lucide-react';
import { useState } from 'react';
import { ipc } from '../../lib/ipc';
import { useAppStore } from '../../lib/store';
import { TRACKER_LABEL } from '../../lib/format';
import { useToast } from '../../lib/toast';
import type { TrackerState } from '../../lib/types';
import { Ubi } from '../ubi/Ubi';
import { IconButton } from '../ui/Button';

const COLORS: Record<TrackerState, string> = {
  running: '#10b981',
  paused: '#f59e0b',
  private: '#64748b',
  idle: '#94a3b8',
  blocked: '#ef4444',
};

export function TrackerPill() {
  const state = useAppStore((s) => s.trackerState);
  const setTrackerState = useAppStore((s) => s.setTrackerState);
  const date = useAppStore((s) => s.date);
  const mood = useAppStore((s) => s.dashboards[date]?.stats.mood ?? 'calm');
  const [busy, setBusy] = useState(false);
  const toast = useToast();

  const running = state === 'running' || state === 'idle';

  const toggle = async () => {
    setBusy(true);
    try {
      if (state === 'private') {
        await ipc.setPrivateMode('off');
        setTrackerState('running');
      } else {
        await ipc.setTracking(!running);
        setTrackerState(running ? 'paused' : 'running');
      }
    } catch (e) {
      toast.error('Não foi possível alterar o rastreamento', e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex items-center gap-2 rounded-xl border border-line bg-surface p-1.5 min-[1180px]:pr-2" data-testid="tracker-pill">
      <div className="hidden size-8 items-center justify-center overflow-hidden min-[1180px]:flex" aria-hidden>
        <Ubi mood={state === 'paused' ? 'sleeping' : mood} size={26} variant="svg" />
      </div>
      <div className="hidden min-w-0 flex-1 min-[1180px]:block">
        <p className="flex items-center gap-1.5 text-xs font-medium">
          <span className="size-2 rounded-full" style={{ background: COLORS[state], boxShadow: `0 0 0 3px ${COLORS[state]}26` }} />
          {TRACKER_LABEL[state]}
        </p>
      </div>
      <IconButton label={running ? 'Pausar rastreamento' : 'Retomar rastreamento'} size="sm" onClick={toggle} disabled={busy} title={TRACKER_LABEL[state]}>
        {running ? <Pause className="size-3.5" /> : <Play className="size-3.5" />}
      </IconButton>
    </div>
  );
}
