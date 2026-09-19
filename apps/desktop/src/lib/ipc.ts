// Typed wrappers around Tauri commands. When the page is not running inside Tauri
// (plain `vite dev` in a browser, tests, screenshots), every call is served by `./mock`.

import type {
  ActivityBlock,
  Advice,
  ApiKeyResult,
  BlockGroup,
  Category,
  ClassifyReport,
  CorrectionOutcome,
  DailyReport,
  DashboardData,
  EngineEvent,
  FocusSession,
  FocusStatus,
  FocusTarget,
  FocusTargetKind,
  Id,
  InstalledApp,
  Intervention,
  KnownDomain,
  IsoDate,
  IsoDateTime,
  LicenseStatus,
  Nudge,
  PermissionKind,
  PrivateModeDuration,
  ReclassifyScope,
  Rule,
  RuleSuggestion,
  ScreenshotData,
  Settings,
  SettingsView,
  AiProvider,
  UpdateStatus,
} from './types';

export const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/**
 * Error code of a failed command. Tauri rejects with the serialised `IpcError { code, message }`; the mock throws
 * an `Error` whose message starts with the code (`"license_required: …"`). Unknown shapes give `"other"`.
 */
export function ipcErrorCode(e: unknown): string {
  if (e && typeof e === 'object' && 'code' in e && typeof (e as { code: unknown }).code === 'string') return (e as { code: string }).code;
  const message = e instanceof Error ? e.message : typeof e === 'string' ? e : '';
  const m = /^([a-z_]+):\s/.exec(message);
  return m?.[1] ?? 'other';
}

/** Human message of a failed command, whatever its shape. */
export function ipcErrorMessage(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (e && typeof e === 'object' && 'message' in e && typeof (e as { message: unknown }).message === 'string') return (e as { message: string }).message;
  return String(e);
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (isTauri()) {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
  }
  const mock = await import('./mock');
  return mock.handle<T>(cmd, args ?? {});
}

export const ipc = {
  // Dashboard & timeline
  getDashboard: (date: IsoDate) => call<DashboardData>('get_dashboard', { date }),
  getTimeline: (date: IsoDate) => call<ActivityBlock[]>('get_timeline', { date }),
  getReviewGroups: (date: IsoDate) => call<BlockGroup[]>('get_review_groups', { date }),
  getAiSent: (date: IsoDate) => call<ActivityBlock[]>('get_ai_sent', { date }),
  /** Screenshot still stored for a block; `null` when none was taken or it has already been deleted. */
  getScreenshot: (blockId: Id) => call<ScreenshotData | null>('get_screenshot', { blockId }),

  // Corrections (learning)
  reclassify: (blockId: Id, categoryId: Id, scope: ReclassifyScope, note?: string) =>
    call<CorrectionOutcome>('reclassify', { blockId, categoryId, scope, note: note ?? null }),
  reclassifyGroup: (date: IsoDate, key: string, categoryId: Id) =>
    call<CorrectionOutcome>('reclassify_group', { date, key, categoryId }),
  acceptRuleSuggestion: (suggestion: RuleSuggestion) =>
    call<Rule>('accept_rule_suggestion', { suggestion }),
  splitBlock: (blockId: Id, at: IsoDateTime) => call<Id>('split_block', { blockId, at }),
  addManualEntry: (startedAt: IsoDateTime, endedAt: IsoDateTime, categoryId: Id, note?: string) =>
    call<ActivityBlock>('add_manual_entry', { startedAt, endedAt, categoryId, note: note ?? null }),
  classifyNow: () => call<ClassifyReport>('classify_now'),

  // Categories & rules
  listCategories: (includeArchived = false) =>
    call<Category[]>('list_categories', { includeArchived }),
  saveCategory: (category: Category) => call<Category>('save_category', { category }),
  deleteCategory: (id: Id) => call<void>('delete_category', { id }),
  listRules: () => call<Rule[]>('list_rules'),
  saveRule: (rule: Rule) => call<Rule>('save_rule', { rule }),
  deleteRule: (id: Id) => call<void>('delete_rule', { id }),

  // Reports
  getReports: (date: IsoDate) => call<DailyReport[]>('get_reports', { date }),
  listReportsBetween: (from: IsoDate, to: IsoDate) =>
    call<DailyReport[]>('list_reports_between', { from, to }),
  generateReport: (date: IsoDate, categoryId: Id) =>
    call<DailyReport>('generate_report', { date, categoryId }),
  updateReport: (report: DailyReport) => call<DailyReport>('update_report', { report }),
  getMonthlyReport: (categoryId: Id, year: number, month: number) =>
    call<string>('get_monthly_report', { categoryId, year, month }),

  // Nudges & advice
  getNudges: (limit = 30) => call<Nudge[]>('get_nudges', { limit }),
  markNudgesSeen: () => call<void>('mark_nudges_seen'),
  snoozeNudges: (minutes: number) => call<void>('snooze_nudges', { minutes }),
  getAdvice: () => call<Advice>('get_advice'),

  // Settings, tracking, permissions
  getSettings: () => call<SettingsView>('get_settings'),
  updateSettings: (settings: Settings) => call<SettingsView>('update_settings', { settings }),
  /** Validates the key against `provider` and stores it (null removes that provider's key). */
  setApiKey: (provider: AiProvider, key: string | null) => call<ApiKeyResult>('set_api_key', { provider, key }),
  /** Chat-capable model ids the provider's account can use (needs that provider's key). */
  listModels: (provider: AiProvider) => call<string[]>('list_models', { provider }),
  setTracking: (enabled: boolean) => call<void>('set_tracking', { enabled }),
  setPrivateMode: (duration: PrivateModeDuration) =>
    call<void>('set_private_mode', { duration }),
  requestPermission: (kind: PermissionKind) => call<void>('request_permission', { kind }),
  restartApp: () => call<void>('restart_app'),
  deleteAllData: () => call<void>('delete_all_data'),
  exportData: () => call<string>('export_data'),
  openExternal: (url: string) => call<void>('open_external', { url }),

  // License (ubiqX Anual / Mensal)
  /** The stored license key's verdict, re-verified now. */
  getLicenseStatus: () => call<LicenseStatus>('get_license_status'),
  /**
   * Stores the key (null removes it), verifies the signature and expiry locally and, for `monthly_managed`, asks the
   * proxy for the month's usage (a proxy that cannot be reached keeps the local verdict with `managed_usage: null`).
   */
  setLicenseKey: (key: string | null) => call<LicenseStatus>('set_license_key', { key }),

  // Updates (rolling "continuous" GitHub release)
  getUpdateStatus: () => call<UpdateStatus>('get_update_status'),
  /** Runs a check now, regardless of Settings.check_updates, and returns the fresh status. */
  checkForUpdates: () => call<UpdateStatus>('check_for_updates'),
  /** Hides the banner for this build epoch until a newer build shows up. */
  dismissUpdate: (epoch: number) => call<UpdateStatus>('dismiss_update', { epoch }),
  /** Opens the DMG download of the available release in the browser. */
  openUpdate: () => call<void>('open_update'),

  // Focus guard (blocked apps and sites) and focus sessions
  /** Apps found on this Mac, sorted by name (cached for 60 s on the Rust side). */
  listInstalledApps: () => call<InstalledApp[]>('list_installed_apps'),
  /** Domains seen in the user's own blocks, most time first. */
  listKnownDomains: (limit = 30) => call<KnownDomain[]>('list_known_domains', { limit }),
  /** Enabled first, then by name. */
  listFocusTargets: () => call<FocusTarget[]>('list_focus_targets'),
  /** kind+key is unique: adding an existing target re-enables and returns it. Domains are normalised on the Rust side. */
  addFocusTarget: (kind: FocusTargetKind, name: string, key: string) => call<FocusTarget>('add_focus_target', { kind, name, key }),
  setFocusTargetEnabled: (id: Id, enabled: boolean) => call<FocusTarget>('set_focus_target_enabled', { id, enabled }),
  removeFocusTarget: (id: Id) => call<void>('remove_focus_target', { id }),
  /** Newest first. */
  listInterventions: (limit = 30) => call<Intervention[]>('list_interventions', { limit }),
  getFocusStatus: () => call<FocusStatus>('get_focus_status'),
  /** IpcError "invalid" when `task` is blank or `minutes` is outside 5..=240; a running session is ended first. */
  startFocusSession: (task: string, minutes: number) => call<FocusSession>('start_focus_session', { task, minutes }),
  stopFocusSession: () => call<FocusSession | null>('stop_focus_session'),
  /** Shows the intervention window with a sample message; records nothing. */
  testIntervention: () => call<void>('test_intervention'),
};

/** Subscribes to engine events. Returns an unsubscribe function. */
export async function onEngineEvent(handler: (e: EngineEvent) => void): Promise<() => void> {
  if (isTauri()) {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await listen<EngineEvent>('engine', (ev) => handler(ev.payload));
    return () => {
      unlisten();
    };
  }
  const mock = await import('./mock');
  return mock.subscribe(handler);
}
