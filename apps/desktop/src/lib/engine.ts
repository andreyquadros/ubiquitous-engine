import { useEffect } from 'react';
import { onEngineEvent } from './ipc';
import { useAppStore } from './store';
import { useToastStore } from './toast';
import type { EngineEvent } from './types';

const DATA_EVENTS = new Set<EngineEvent['type']>(['block_opened', 'block_closed', 'blocks_classified']);

/** Subscribes to engine events once (in the app shell) and forwards them to the store, debouncing data refreshes. */
export function useEngineEvents(): void {
  const applyEvent = useAppStore((s) => s.applyEvent);
  const push = useToastStore((s) => s.push);

  useEffect(() => {
    let unsub: (() => void) | null = null;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let disposed = false;

    void onEngineEvent((e) => {
      if (DATA_EVENTS.has(e.type)) {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => applyEvent(e), 400);
        return;
      }
      applyEvent(e);
      if (e.type === 'nudge') push({ kind: 'info', title: e.nudge.title, message: e.nudge.message });
      if (e.type === 'report_ready') push({ kind: 'success', title: 'Relatório pronto', message: `Relatório de ${e.report.date} gerado.` });
      if (e.type === 'permission_required') push({ kind: 'error', title: 'Permissão necessária', message: `O ubiqX precisa da permissão: ${e.permission}.` });
    }).then((u) => {
      if (disposed) u();
      else unsub = u;
    });

    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      unsub?.();
    };
  }, [applyEvent, push]);
}
