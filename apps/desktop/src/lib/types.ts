// Mirrors of the Rust domain types (crates/ubiqx-core/src/model.rs) and engine DTOs
// (crates/ubiqx-engine/src/service.rs). Field names are snake_case exactly as serde emits them.

export type Id = string;
export type IsoDateTime = string; // RFC 3339 UTC
export type IsoDate = string; // YYYY-MM-DD (local day)
export type HHMM = string; // "18:00:00" — chrono NaiveTime serialises as HH:MM:SS

export type ClassificationSource = 'rule' | 'memory' | 'llm' | 'vision' | 'user';

export interface ActivityBlock {
  id: Id;
  started_at: IsoDateTime;
  ended_at: IsoDateTime;
  app_name: string;
  app_id: string;
  title: string;
  title_key: string;
  url: string | null;
  domain: string | null;
  category_id: Id | null;
  confidence: number;
  source: ClassificationSource | null;
  description: string | null;
  screenshot_id: Id | null;
  sample_count: number;
  is_open: boolean;
  classify_attempts: number;
  next_attempt_at: IsoDateTime | null;
  needs_review: boolean;
  ai_payload: string | null;
  ai_sent_at: IsoDateTime | null;
  is_manual: boolean;
  note: string | null;
}

export interface Category {
  id: Id;
  name: string;
  color: string;
  icon: string;
  description: string;
  keywords: string[];
  report_time: HHMM | null;
  report_template: string | null;
  is_productive: boolean;
  is_system: boolean;
  archived: boolean;
  sort_order: number;
  created_at: IsoDateTime;
}

export const SYSTEM_CATEGORIES = {
  uncategorized: 'sys-uncategorized',
  distraction: 'sys-distraction',
  break: 'sys-break',
  private: 'sys-private',
} as const;

export type RuleMatcher = 'app' | 'domain' | 'title_contains' | 'regex';
export type RuleOrigin = 'user' | 'learned';

export interface Rule {
  id: Id;
  category_id: Id;
  matcher: RuleMatcher;
  pattern: string;
  priority: number;
  origin: RuleOrigin;
  enabled: boolean;
  created_at: IsoDateTime;
  hit_count: number;
  miss_count: number;
  last_contradicted_at: IsoDateTime | null;
}

export interface RuleSuggestion {
  category_id: Id;
  matcher: RuleMatcher;
  pattern: string;
  support: number;
  rationale: string;
  auto_apply_safe: boolean;
}

export type ActivityKind =
  | 'desenvolvimento'
  | 'reuniao'
  | 'comunicacao'
  | 'documentacao'
  | 'ensino'
  | 'pesquisa'
  | 'extensao'
  | 'gestao'
  | 'outro';

export interface ReportItem {
  activity: string;
  kind: ActivityKind;
  minutes: number;
  evidence: string[];
  time_range: string;
  continuation_of: string | null;
}

export interface DailyReport {
  id: Id;
  date: IsoDate;
  category_id: Id;
  generated_at: IsoDateTime;
  summary_md: string;
  items: ReportItem[];
  highlights: string[];
  total_secs: number;
  model: string;
  input_tokens: number;
  output_tokens: number;
  stale: boolean;
  edited: boolean;
}

export type NudgeKind =
  | 'unproductive'
  | 'distracted'
  | 'break_suggested'
  | 'praise'
  | 'idle'
  | 'report_ready'
  | 'attention';

export interface Nudge {
  id: Id;
  at: IsoDateTime;
  kind: NudgeKind;
  title: string;
  message: string;
  seen: boolean;
}

export type Mood = 'sleeping' | 'calm' | 'focused' | 'excited' | 'worried';

export interface FocusStats {
  focus_score: number;
  productive_secs: number;
  distraction_secs: number;
  uncategorized_secs: number;
  idle_secs: number;
  total_secs: number;
  switches_per_hour: number;
  longest_focus_secs: number;
  mood: Mood;
}

/** Hosted LLM vendor answering the remote calls. Ids match the Rust `AiProvider` enum. */
export type AiProvider = 'anthropic' | 'openai' | 'xai';

export interface AiModels {
  classify: string;
  vision: string;
  report: string;
}

/** Static facts about a provider, served by the backend so the UI never hardcodes them. */
export interface ProviderInfo {
  id: AiProvider;
  label: string;
  console_url: string;
  key_prefix: string;
  default_models: AiModels;
}

/** Whether a key is stored for a provider (hint = last 4 chars). */
export interface ApiKeyStatus {
  provider: AiProvider;
  configured: boolean;
  hint: string | null;
}

export interface QuietHours {
  enabled: boolean;
  start: HHMM;
  end: HHMM;
}

export type VisionPolicy =
  | { mode: 'never' }
  | { mode: 'only_apps'; apps: string[] }
  | { mode: 'all_except_blocked' };

export interface NudgeSettings {
  enabled: boolean;
  unproductive: boolean;
  distracted: boolean;
  break_suggested: boolean;
  praise: boolean;
  idle: boolean;
  max_per_day: number;
  cooldown_mins: number;
  silent_apps: string[];
  snoozed_until: IsoDateTime | null;
}

export interface Settings {
  tracking_enabled: boolean;
  sample_interval_secs: number;
  idle_threshold_secs: number;
  min_block_secs: number;
  screenshot_interval_secs: number;
  screenshot_max_edge: number;
  screenshot_retention_hours: number;
  keep_screenshots_for_review: boolean;
  vision_policy: VisionPolicy;
  vision_denied_apps: string[];
  blocked_apps: string[];
  blocked_domains: string[];
  private_mode: boolean;
  private_until: IsoDateTime | null;
  ai_provider: AiProvider;
  models: AiModels;
  max_vision_per_hour: number;
  ai_monthly_budget_usd: number;
  classify_batch_min: number;
  classify_max_wait_secs: number;
  local_only: boolean;
  min_confidence: number;
  report_default_time: HHMM;
  language: string;
  user_profile: string | null;
  quiet_hours: QuietHours;
  nudges: NudgeSettings;
  launch_at_login: boolean;
  onboarding_done: boolean;
}

export type TrackerState = 'running' | 'paused' | 'private' | 'idle' | 'blocked';

export type AiHealth =
  | { state: 'ok' }
  | { state: 'not_configured' }
  | { state: 'degraded'; reason: string; until: IsoDateTime }
  | { state: 'paused'; reason: string };

export type PermissionState = 'granted' | 'denied' | 'unknown' | 'not_applicable';
export type PermissionKind = 'screen_recording' | 'automation' | 'accessibility';

export interface PermissionStatus {
  screen_recording: PermissionState;
  automation: PermissionState;
  accessibility: PermissionState;
}

export interface CategoryTotal {
  category_id: Id | null;
  secs: number;
  block_count: number;
}

export interface AppTotal {
  app_id: string;
  app_name: string;
  secs: number;
}

export interface AiUsageTotals {
  calls: number;
  input_tokens: number;
  output_tokens: number;
  cost_usd: number;
}

export interface DashboardData {
  date: IsoDate;
  stats: FocusStats;
  totals: CategoryTotal[];
  top_apps: AppTotal[];
  timeline: ActivityBlock[];
  open_block: ActivityBlock | null;
  categories: Category[];
  unseen_nudges: Nudge[];
  tracker_state: TrackerState;
  ai_health: AiHealth;
  usage_month: AiUsageTotals;
  budget_usd: number;
  needs_review: number;
  hourly_focus: (number | null)[];
}

export interface BlockGroup {
  key: string;
  app_id: string;
  app_name: string;
  domain: string | null;
  title: string;
  total_secs: number;
  block_ids: Id[];
  category_id: Id | null;
  min_confidence: number;
  source: ClassificationSource | null;
  needs_review: boolean;
  description: string | null;
  first_started_at: IsoDateTime;
}

export type ReclassifyScope = 'block' | 'day' | 'month';
export type PrivateModeDuration = 'off' | 'minutes30' | 'hour1' | 'until_tomorrow' | 'indefinite';

export interface CorrectionOutcome {
  block_ids: Id[];
  backfilled: number;
  suggestions: RuleSuggestion[];
  auto_rules: Rule[];
  disabled_rules: Rule[];
}

export interface ClassifyReport {
  local: number;
  remote: number;
  vision: number;
  needs_review: number;
  skipped_remote: boolean;
}

export interface SettingsView {
  settings: Settings;
  /** Key status of the *selected* provider (`settings.ai_provider`). */
  api_key_configured: boolean;
  api_key_hint: string | null;
  /** Key status of every provider, so switching shows what is already stored. */
  api_keys: ApiKeyStatus[];
  providers: ProviderInfo[];
  permissions: PermissionStatus;
  ai_health: AiHealth;
  tracker_state: TrackerState;
  data_dir: string;
  platform: 'macos' | 'other';
  version: string;
}

export interface ApiKeyResult {
  valid: boolean;
  message: string;
}

export interface Advice {
  headline: string;
  recommendations: string[];
  model: string;
}

/** Events pushed by the engine on the `engine` channel. */
export type EngineEvent =
  | { type: 'block_opened'; block: ActivityBlock }
  | { type: 'block_closed'; block: ActivityBlock }
  | { type: 'blocks_classified'; block_ids: Id[] }
  | { type: 'report_ready'; report: DailyReport }
  | { type: 'nudge'; nudge: Nudge }
  | { type: 'tracker_state'; state: TrackerState }
  | { type: 'ai_health'; health: AiHealth }
  | { type: 'permission_required'; permission: string }
  | { type: 'screenshot_taken'; screenshot_id: Id; block_id: Id | null };
