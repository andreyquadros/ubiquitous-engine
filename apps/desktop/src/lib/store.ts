import { create } from 'zustand';
import { ipc } from './ipc';
import { applyTheme, initialTheme, type Theme } from './theme';
import { todayIso } from './format';
import { hasUrlLocaleOverride, normaliseLocale, setLocale, type Locale } from '../i18n';
import type {
  AiHealth,
  Category,
  DashboardData,
  EngineEvent,
  IsoDate,
  Nudge,
  Settings,
  SettingsView,
  TrackerState,
  UpdateStatus,
} from './types';

interface AppState {
  // theme
  theme: Theme;
  setTheme: (t: Theme) => void;
  toggleTheme: () => void;

  // selected day (shared by Hoje / Timeline / Revisão / Relatórios)
  date: IsoDate;
  setDate: (d: IsoDate) => void;

  // settings
  settingsView: SettingsView | null;
  settingsError: string | null;
  loadSettings: () => Promise<SettingsView | null>;
  saveSettings: (patch: Partial<Settings>) => Promise<SettingsView | null>;
  applySettingsView: (v: SettingsView) => void;
  /** Switches the UI language now and persists it as Settings.language. */
  setLanguage: (locale: Locale) => Promise<SettingsView | null>;

  // categories
  categories: Category[];
  loadCategories: (includeArchived?: boolean) => Promise<Category[]>;

  // dashboards cache (by date)
  dashboards: Record<IsoDate, DashboardData>;
  loadDashboard: (date: IsoDate, force?: boolean) => Promise<DashboardData>;

  // live engine state
  trackerState: TrackerState;
  aiHealth: AiHealth;
  setTrackerState: (s: TrackerState) => void;
  latestNudge: Nudge | null;
  unseenNudges: Nudge[];
  clearUnseen: () => void;

  // change counters bumped by engine events so pages refetch
  dataVersion: number;
  reportsVersion: number;
  bumpData: () => void;
  applyEvent: (e: EngineEvent) => void;

  // UBI speech (what the mascot is currently saying)
  ubiSpeech: string | null;
  setUbiSpeech: (s: string | null) => void;

  // updates (rolling "continuous" release)
  updateStatus: UpdateStatus | null;
  updateChecking: boolean;
  loadUpdateStatus: () => Promise<UpdateStatus | null>;
  /** Runs a check now (regardless of Settings.check_updates) and returns the fresh status. */
  checkForUpdates: () => Promise<UpdateStatus>;
  /** Hides the banner for the currently available build. */
  dismissUpdate: () => Promise<UpdateStatus | null>;
  /** Opens the DMG download of the available release. */
  openUpdate: () => Promise<void>;
}

const inflight = new Map<string, Promise<DashboardData>>();

export const useAppStore = create<AppState>((set, get) => ({
  theme: initialTheme(),
  setTheme: (t) => {
    applyTheme(t);
    set({ theme: t });
  },
  toggleTheme: () => get().setTheme(get().theme === 'dark' ? 'light' : 'dark'),

  date: todayIso(),
  setDate: (d) => set({ date: d }),

  settingsView: null,
  settingsError: null,
  loadSettings: async () => {
    try {
      const v = await ipc.getSettings();
      get().applySettingsView(v);
      return v;
    } catch (e) {
      set({ settingsError: e instanceof Error ? e.message : String(e) });
      return null;
    }
  },
  saveSettings: async (patch) => {
    const current = get().settingsView;
    if (!current) return null;
    const next = { ...current.settings, ...patch };
    // optimistic
    set({ settingsView: { ...current, settings: next } });
    const v = await ipc.updateSettings(next);
    get().applySettingsView(v);
    return v;
  },
  applySettingsView: (v) => {
    set({ settingsView: v, trackerState: v.tracker_state, aiHealth: v.ai_health, settingsError: null });
    // Settings.language is the persisted source of truth, unless `?lang=` overrides it (dev / screenshots).
    if (!hasUrlLocaleOverride()) setLocale(normaliseLocale(v.settings.language));
  },
  setLanguage: async (locale) => {
    setLocale(locale);
    return get().saveSettings({ language: locale });
  },

  categories: [],
  loadCategories: async (includeArchived = false) => {
    const cats = await ipc.listCategories(includeArchived);
    set({ categories: cats });
    return cats;
  },

  dashboards: {},
  loadDashboard: async (date, force = false) => {
    const cached = get().dashboards[date];
    if (cached && !force) return cached;
    const pending = inflight.get(date);
    if (pending && !force) return pending;
    const p = ipc.getDashboard(date).then((d) => {
      set((s) => ({
        dashboards: { ...s.dashboards, [date]: d },
        categories: d.categories.length ? d.categories : s.categories,
        trackerState: d.tracker_state,
        aiHealth: d.ai_health,
        unseenNudges: d.unseen_nudges,
        latestNudge: d.unseen_nudges[0] ?? s.latestNudge,
      }));
      inflight.delete(date);
      return d;
    });
    inflight.set(date, p);
    return p;
  },

  trackerState: 'running',
  aiHealth: { state: 'ok' },
  setTrackerState: (s) => set({ trackerState: s }),
  latestNudge: null,
  unseenNudges: [],
  clearUnseen: () => set({ unseenNudges: [] }),

  dataVersion: 0,
  reportsVersion: 0,
  bumpData: () => set((s) => ({ dataVersion: s.dataVersion + 1, dashboards: {} })),
  applyEvent: (e) => {
    switch (e.type) {
      case 'block_opened':
      case 'block_closed':
      case 'blocks_classified':
        get().bumpData();
        break;
      case 'nudge':
        set((s) => ({ latestNudge: e.nudge, unseenNudges: [e.nudge, ...s.unseenNudges], ubiSpeech: e.nudge.message }));
        break;
      case 'report_ready':
        set((s) => ({ reportsVersion: s.reportsVersion + 1 }));
        break;
      case 'tracker_state':
        set({ trackerState: e.state });
        break;
      case 'ai_health':
        set({ aiHealth: e.health });
        break;
      case 'update_available':
        // The engine updates its status before emitting, and that status is the only place
        // that knows whether the user already dismissed this build (the event is re-sent once
        // per process and on every manual check). Reloading it, instead of showing the release
        // optimistically, keeps a dismissed banner from flashing back in.
        void get().loadUpdateStatus();
        break;
      default:
        break;
    }
  },

  ubiSpeech: null,
  setUbiSpeech: (s) => set({ ubiSpeech: s }),

  updateStatus: null,
  updateChecking: false,
  loadUpdateStatus: async () => {
    try {
      const status = await ipc.getUpdateStatus();
      set({ updateStatus: status });
      return status;
    } catch {
      // An old engine without the command, or a transient IPC error: the UI simply shows nothing.
      return null;
    }
  },
  checkForUpdates: async () => {
    set({ updateChecking: true });
    try {
      const status = await ipc.checkForUpdates();
      set({ updateStatus: status });
      return status;
    } finally {
      set({ updateChecking: false });
    }
  },
  dismissUpdate: async () => {
    const rel = get().updateStatus?.available;
    if (!rel) return get().updateStatus;
    // optimistic
    set((s) => (s.updateStatus ? { updateStatus: { ...s.updateStatus, dismissed: true } } : {}));
    const status = await ipc.dismissUpdate(rel.build.epoch);
    set({ updateStatus: status });
    return status;
  },
  openUpdate: () => ipc.openUpdate(),
}));

// Apply initial theme on module load (browser only).
if (typeof document !== 'undefined') applyTheme(useAppStore.getState().theme);
