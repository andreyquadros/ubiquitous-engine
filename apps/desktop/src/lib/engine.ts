import { useEffect } from 'react';
import { hasKey, t } from '../i18n';
import { fmtDateLong } from './format';
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
      // t() at event time, so the toast follows the language selected when the event arrives.
      if (e.type === 'report_ready') push({ kind: 'success', title: t('ui.report_ready_title'), message: t('ui.report_ready_body', { date: fmtDateLong(e.report.date) }) });
      if (e.type === 'permission_required') push({ kind: 'error', title: t('ui.permission_required_title'), message: t('ui.permission_required_body', { permission: hasKey(`settings.permissions.${e.permission}.label`) ? t(`settings.permissions.${e.permission}.label`) : e.permission }) });
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
