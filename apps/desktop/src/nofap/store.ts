import { useEffect } from 'react';
import { create } from 'zustand';
import { ipcErrorMessage } from '../lib/ipc';
import { onNofapEvent } from './events';
import { nofapIpc } from './ipc';
import type { ChangeOutcome, Id, NofapChange, NofapEvent, NofapView, SitesView, SupportView } from './types';

/** The module's own store: it shares nothing with the host app's settings, by design (6.1). */
interface NofapState {
  view: NofapView | null;
  /** Message of the last failed load; `null` after a good one. */
  error: string | null;
  sites: SitesView | null;
  support: SupportView | null;
  load: () => Promise<NofapView | null>;
  loadSites: () => Promise<SitesView | null>;
  loadSupport: () => Promise<SupportView | null>;
  setView: (view: NofapView) => void;
  /** Sends a change; the view is fresh afterwards whether it applied or waits. */
  request: (change: NofapChange) => Promise<ChangeOutcome>;
  confirmPending: (id: Id) => Promise<void>;
  cancelPending: (id: Id) => Promise<void>;
  applyEvent: (event: NofapEvent) => void;
}

export const useNofapStore = create<NofapState>((set, get) => ({
  view: null,
  error: null,
  sites: null,
  support: null,

  load: async () => {
    try {
      const view = await nofapIpc.view();
      set({ view, error: null });
      return view;
    } catch (e) {
      set({ error: ipcErrorMessage(e) });
      return null;
    }
  },
  loadSites: async () => {
    try {
      const sites = await nofapIpc.sites();
      set({ sites });
      return sites;
    } catch {
      return null;
    }
  },
  loadSupport: async () => {
    try {
      const support = await nofapIpc.support();
      set({ support });
      return support;
    } catch {
      return null;
    }
  },
  setView: (view) => set({ view, error: null }),

  request: async (change) => {
    const outcome = await nofapIpc.request(change);
    if ('applied' in outcome) set({ view: outcome.applied, error: null });
    else await get().load();
    return outcome;
  },
  confirmPending: async (id) => {
    set({ view: await nofapIpc.confirmPending(id), error: null });
  },
  cancelPending: async (id) => {
    set({ view: await nofapIpc.cancelPending(id), error: null });
  },

  applyEvent: (event) => {
    switch (event.type) {
      case 'status': {
        const view = get().view;
        if (!view) break;
        set({ view: { ...view, status: event.status } });
        // The configuration behind a different overall state (switched off, switched on) is stale: read it again.
        if (view.status.overall !== event.status.overall) void get().load();
        break;
      }
      case 'pending_changed':
        void get().load();
        break;
      case 'sites_changed':
        void get().loadSites();
        break;
      case 'card':
      case 'card_closed':
        break;
    }
  },
}));

/** Subscribes to the module's events for as long as the calling component is mounted. */
export function useNofapEvents(): void {
  const applyEvent = useNofapStore((s) => s.applyEvent);
  useEffect(() => {
    let unsubscribe: (() => void) | null = null;
    let disposed = false;
    void onNofapEvent(applyEvent).then((u) => {
      if (disposed) u();
      else unsubscribe = u;
    });
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, [applyEvent]);
}
