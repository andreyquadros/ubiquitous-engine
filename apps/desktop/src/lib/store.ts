import { create } from 'zustand';
import { ipc } from './ipc';
import { applyTheme, initialTheme, type Theme } from './theme';
import { IDLE_INSTALL, checkInAppUpdate, isInstalling, relaunchApp, type UpdateInstall } from './updater';
import { todayIso } from './format';
import { hasUrlLocaleOverride, normaliseLocale, setLocale, type Locale } from '../i18n';
import type {
  AiHealth,
  Category,
  DashboardData,
  EngineEvent,
  FocusSession,
  FocusStatus,
  FocusTarget,
  FocusTargetKind,
  Id,
  InstalledApp,
  Intervention,
  IsoDate,
  KnownDomain,
  LicenseStatus,
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

  // license (ubiqX Anual / Mensal). Mirrors settingsView.license, refreshed by get_license_status / set_license_key.
  license: LicenseStatus | null;
  loadLicense: () => Promise<LicenseStatus | null>;
  /** Validates and stores the key (null removes it); the returned status is also the new `license`. */
  setLicenseKey: (key: string | null) => Promise<LicenseStatus>;

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
  /** Opens the DMG download of the available release (the manual fallback). */
  openUpdate: () => Promise<void>;

  // in-app install (tauri-plugin-updater + tauri-plugin-process)
  updateInstall: UpdateInstall;
  /**
   * Downloads the signed update with a progress bar, installs it and relaunches the app. Never throws:
   * the outcome lands in `updateInstall` (`failed` with a message, `unavailable` when the plugin has
   * nothing to install for this build) so the banner and Settings can show it and offer the manual path.
   */
  installUpdate: () => Promise<UpdateInstall>;
  /** Back to `idle` after a failure, so the user can try again or take the manual route. */
  resetUpdateInstall: () => void;

  // focus guard (blocked apps and sites) and focus sessions
  focusStatus: FocusStatus | null;
  focusTargets: FocusTarget[];
  interventions: Intervention[];
  installedApps: InstalledApp[];
  knownDomains: KnownDomain[];
  /** Session, counters and guard flag (cheap; the sidebar dot and the dashboard line depend on it). */
  loadFocusStatus: () => Promise<FocusStatus | null>;
  /** Status + targets + interventions, for the Focus page and after an intervention. */
  loadFocus: () => Promise<void>;
  /** Installed apps and known domains (the search catalogue). */
  loadFocusCatalog: () => Promise<void>;
  addFocusTarget: (kind: FocusTargetKind, name: string, key: string) => Promise<FocusTarget>;
  setFocusTargetEnabled: (id: Id, enabled: boolean) => Promise<FocusTarget>;
  removeFocusTarget: (id: Id) => Promise<void>;
  startFocusSession: (task: string, minutes: number) => Promise<FocusSession>;
  stopFocusSession: () => Promise<FocusSession | null>;
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
    set({ settingsView: v, trackerState: v.tracker_state, aiHealth: v.ai_health, license: v.license ?? null, settingsError: null });
    // Settings.language is the persisted source of truth, unless `?lang=` overrides it (dev / screenshots).
    if (!hasUrlLocaleOverride()) setLocale(normaliseLocale(v.settings.language));
  },
  setLanguage: async (locale) => {
    setLocale(locale);
    return get().saveSettings({ language: locale });
  },

  license: null,
  loadLicense: async () => {
    try {
      const license = await ipc.getLicenseStatus();
      set((s) => ({ license, settingsView: s.settingsView ? { ...s.settingsView, license } : s.settingsView }));
      return license;
    } catch {
      // An old engine without the command: the UI keeps what get_settings said.
      return null;
    }
  },
  setLicenseKey: async (key) => {
    const license = await ipc.setLicenseKey(key);
    set((s) => ({ license, settingsView: s.settingsView ? { ...s.settingsView, license } : s.settingsView }));
    // The provider, the key statuses and the AI health may all have moved with the license.
    await get().loadSettings();
    return license;
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
      case 'intervention':
        // Newest first, deduplicated (the event may arrive before the list was ever loaded).
        set((s) => ({ interventions: [e.intervention, ...s.interventions.filter((i) => i.id !== e.intervention.id)] }));
        void get().loadFocus();
        break;
      case 'focus_session': {
        // Show the session (or its end) right away; the status reload brings the counters.
        const session = e.session && !e.session.ended_at ? e.session : null;
        set((s) => ({
          focusStatus: s.focusStatus
            ? { ...s.focusStatus, session, remaining_secs: session ? Math.max(0, Math.round((new Date(session.ends_at).getTime() - Date.now()) / 1000)) : null }
            : s.focusStatus,
        }));
        void get().loadFocusStatus();
        break;
      }
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
    // Hiding the banner also clears a failed install: the next build starts from a clean slate.
    if (!isInstalling(get().updateInstall)) get().resetUpdateInstall();
    // optimistic
    set((s) => (s.updateStatus ? { updateStatus: { ...s.updateStatus, dismissed: true } } : {}));
    const status = await ipc.dismissUpdate(rel.build.epoch);
    set({ updateStatus: status });
    return status;
  },
  openUpdate: () => ipc.openUpdate(),

  updateInstall: IDLE_INSTALL,
  installUpdate: async () => {
    if (isInstalling(get().updateInstall)) return get().updateInstall;
    const finish = (install: UpdateInstall): UpdateInstall => {
      set({ updateInstall: install });
      return install;
    };
    set({ updateInstall: { ...IDLE_INSTALL, phase: 'checking' } });
    let update = null;
    try {
      update = await checkInAppUpdate();
      if (!update) return finish({ ...IDLE_INSTALL, phase: 'unavailable' });
      set({ updateInstall: { ...IDLE_INSTALL, phase: 'downloading' } });
      await update.downloadAndInstall(({ downloaded, total }) => {
        const phase = total !== null && downloaded >= total ? 'installing' : 'downloading';
        set((s) => (isInstalling(s.updateInstall) ? { updateInstall: { ...s.updateInstall, phase, downloaded, total } } : {}));
      });
      // Windows never gets here: its installer exits the app. macOS and Linux relaunch themselves.
      set((s) => ({ updateInstall: { ...s.updateInstall, phase: 'relaunching' } }));
      await relaunchApp();
      return get().updateInstall;
    } catch (e) {
      // The plugin keeps a handle per update: free it, or a retry piles them up on the Rust side.
      await update?.close().catch(() => {});
      return finish({ ...get().updateInstall, phase: 'failed', error: e instanceof Error ? e.message : String(e) });
    }
  },
  resetUpdateInstall: () => set({ updateInstall: IDLE_INSTALL }),

  focusStatus: null,
  focusTargets: [],
  interventions: [],
  installedApps: [],
  knownDomains: [],
  loadFocusStatus: async () => {
    try {
      const status = await ipc.getFocusStatus();
      set({ focusStatus: status });
      return status;
    } catch {
      // An engine without the focus commands: the page shows nothing rather than an error.
      return null;
    }
  },
  loadFocus: async () => {
    const [status, targets, interventions] = await Promise.all([ipc.getFocusStatus(), ipc.listFocusTargets(), ipc.listInterventions()]);
    set({ focusStatus: status, focusTargets: targets, interventions });
  },
  loadFocusCatalog: async () => {
    const [apps, domains] = await Promise.all([ipc.listInstalledApps().catch(() => [] as InstalledApp[]), ipc.listKnownDomains().catch(() => [] as KnownDomain[])]);
    set({ installedApps: apps, knownDomains: domains });
  },
  addFocusTarget: async (kind, name, key) => {
    const target = await ipc.addFocusTarget(kind, name, key);
    set((s) => ({ focusTargets: sortTargets([target, ...s.focusTargets.filter((x) => x.id !== target.id)]) }));
    void get().loadFocusStatus();
    return target;
  },
  setFocusTargetEnabled: async (id, enabled) => {
    // optimistic
    set((s) => ({ focusTargets: s.focusTargets.map((x) => (x.id === id ? { ...x, enabled } : x)) }));
    try {
      const target = await ipc.setFocusTargetEnabled(id, enabled);
      set((s) => ({ focusTargets: sortTargets(s.focusTargets.map((x) => (x.id === id ? target : x))) }));
      void get().loadFocusStatus();
      return target;
    } catch (e) {
      set((s) => ({ focusTargets: s.focusTargets.map((x) => (x.id === id ? { ...x, enabled: !enabled } : x)) }));
      throw e;
    }
  },
  removeFocusTarget: async (id) => {
    await ipc.removeFocusTarget(id);
    set((s) => ({ focusTargets: s.focusTargets.filter((x) => x.id !== id) }));
    void get().loadFocusStatus();
  },
  startFocusSession: async (task, minutes) => {
    const session = await ipc.startFocusSession(task, minutes);
    set((s) => ({
      focusStatus: {
        session,
        remaining_secs: Math.max(0, Math.round((new Date(session.ends_at).getTime() - Date.now()) / 1000)),
        targets_enabled: s.focusStatus?.targets_enabled ?? 0,
        interventions_today: s.focusStatus?.interventions_today ?? 0,
        guard_enabled: s.focusStatus?.guard_enabled ?? true,
      },
    }));
    void get().loadFocusStatus();
    return session;
  },
  stopFocusSession: async () => {
    const session = await ipc.stopFocusSession();
    set((s) => (s.focusStatus ? { focusStatus: { ...s.focusStatus, session: null, remaining_secs: null } } : {}));
    void get().loadFocusStatus();
    return session;
  },
}));

/** Enabled first, then by name (the order `list_focus_targets` returns). */
const sortTargets = (list: FocusTarget[]): FocusTarget[] => [...list].sort((a, b) => Number(b.enabled) - Number(a.enabled) || a.name.localeCompare(b.name));

// Apply initial theme on module load (browser only).
if (typeof document !== 'undefined') applyTheme(useAppStore.getState().theme);
