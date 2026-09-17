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
  Id,
  IsoDate,
  IsoDateTime,
  Nudge,
  PermissionKind,
  PrivateModeDuration,
  ReclassifyScope,
  Rule,
  RuleSuggestion,
  Settings,
  SettingsView,
} from './types';

export const isTauri = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
  setApiKey: (key: string | null) => call<ApiKeyResult>('set_api_key', { key }),
  setTracking: (enabled: boolean) => call<void>('set_tracking', { enabled }),
  setPrivateMode: (duration: PrivateModeDuration) =>
    call<void>('set_private_mode', { duration }),
  requestPermission: (kind: PermissionKind) => call<void>('request_permission', { kind }),
  restartApp: () => call<void>('restart_app'),
  deleteAllData: () => call<void>('delete_all_data'),
  exportData: () => call<string>('export_data'),
  openExternal: (url: string) => call<void>('open_external', { url }),
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
