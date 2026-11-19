import { isTauri } from '../lib/ipc';
import { NOFAP_EVENT } from './ipc';
import type { NofapEvent } from './types';

/** Subscribes to the module's events (Tauri event `nofap`, or the mock's). Returns an unsubscribe function. */
export async function onNofapEvent(handler: (event: NofapEvent) => void): Promise<() => void> {
  if (isTauri()) {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<NofapEvent>(NOFAP_EVENT, (ev) => handler(ev.payload));
    return () => {
      unlisten();
    };
  }
  const mock = await import('./mock');
  return mock.subscribe(handler);
}
