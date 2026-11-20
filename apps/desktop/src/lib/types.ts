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
  | 'attention'
  | 'focus_prompt';

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

/**
 * Hosted LLM vendor answering the remote calls. Ids match the Rust `AiProvider` enum. `ubi` is the managed option
 * ("IA do Ubi"): the calls go through the Ubi proxy, paid by the monthly plan, authenticated with the license key.
 */
export type AiProvider = 'anthropic' | 'openai' | 'xai' | 'ubi';

/* ------------------------------------------------------------------ */
/* License (crates/ubiqx-core/src/license.rs)                          */
/* ------------------------------------------------------------------ */

/** `annual_own_key` = "ubiqX Anual, com a sua IA"; `monthly_managed` = "ubiqX Mensal, com a IA do Ubi". */
export type Plan = 'annual_own_key' | 'monthly_managed';

export type LicenseState = 'unlicensed' | 'valid' | 'expired' | 'invalid';

/** `soft` = reminders only; `hard` = AI features blocked without a valid license (tracking, timeline and manual categorisation always work). A compile-time constant on the Rust side. */
export type LicenseEnforcement = 'soft' | 'hard';

/** The current month's spend of a `monthly_managed` subscriber, as reported by the proxy. */
export interface ManagedUsage {
  /** `YYYY-MM` (UTC). */
  month: string;
  spent_usd: number;
  budget_usd: number;
}

/** The stored license key's verdict (the key itself never leaves the secret store). */
export interface LicenseStatus {
  state: LicenseState;
  plan: Plan | null;
  /** RFC 3339 (UTC). */
  expires_at: IsoDateTime | null;
  days_left: number | null;
  /** Last 4 characters of the stored key. */
  key_hint: string | null;
  enforcement: LicenseEnforcement;
  /** Only for a valid `monthly_managed` license, and only when the proxy answered. */
  managed_usage: ManagedUsage | null;
}

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

/** Focus guard and focus sessions (`Settings.focus`; serde defaults on the Rust side, so old rows load). */
export interface FocusSettings {
  /** Enforce the block list (the guard loop runs only while this is on). Default true. */
  guard_enabled: boolean;
  /** During a focus session, also hold whatever the rules map to the built-in Distraction category. Default true. */
  block_distraction_in_session: boolean;
  /** Hide the other windows when a session starts (macOS: System Events). Default true. */
  hide_others_on_start: boolean;
  /** Default session length offered by the UI (5..=240). Default 45. */
  session_minutes: number;
  /** Seconds between two interventions on the same app or site. Default 20. */
  intervention_cooldown_secs: number;
  /** macOS: name of a Shortcut run when a session starts (`shortcuts run "<name>"`); Windows/Linux: a command line (cmd /C, sh -c). Null = nothing. */
  macos_focus_shortcut_on: string | null;
  /** Same for the end of a session. */
  macos_focus_shortcut_off: string | null;
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
  /** Automatic update checks (45 s after start, then every 6 h). Defaults to true on the Rust side. */
  check_updates: boolean;
  /** Focus guard and focus sessions. */
  focus: FocusSettings;
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
  /** Blocks flagged for review *on this day* -- the same day the rest of this payload describes. */
  needs_review: number;
  /** What is still flagged on other days, so a clean day does not hide it. */
  review_backlog: ReviewBacklog | null;
  hourly_focus: (number | null)[];
}

/** Blocks still waiting for review outside the day on screen (every review surface is day-scoped). */
export interface ReviewBacklog {
  count: number;
  /** The most recent day holding one, so the UI can send the user straight there. */
  date: IsoDate;
}

/** A stored screenshot, inlined for the UI (`get_screenshot`). `null` from the command means it is gone or was never taken. */
export interface ScreenshotData {
  mime: string;
  data_base64: string;
  width: number | null;
  height: number | null;
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

/** Operating system the engine runs on (`std::env::consts::OS`). Decides which install hints, permissions and labels the UI shows. */
export type Platform = 'macos' | 'windows' | 'linux';

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
  /** The stored license key's verdict (the key never sits in `settings`; it lives in the secret store). */
  license: LicenseStatus;
  /** Base URL of the Ubi proxy this build talks to (`UBIQX_API_BASE`). */
  ubi_api_base: string;
  data_dir: string;
  platform: Platform;
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

/** Identity of a build, stamped into the binary at compile time (crates: BuildInfo). `epoch` 0 = development build. */
export interface BuildInfo {
  version: string;
  /** Unix seconds of the built commit; 0 in development builds. */
  epoch: number;
  /** `git rev-list --count HEAD`; 0 in development builds. */
  number: number;
  /** Short 7-char sha, or "dev". */
  sha: string;
  /** Branch name, "" when unknown. */
  branch: string;
}

/** Installer kinds the update feed publishes (scripts/publish-release.mjs); the download button names the extension. */
export type AssetKind = 'dmg' | 'msi' | 'exe' | 'appimage' | 'deb';

/** One platform entry of latest.json (`platforms["<os>-<arch>"]`), as the feed publishes it. */
export interface PlatformAsset {
  url: string;
  kind: AssetKind;
  size: number;
  /** macOS only: the ditto-zipped .app next to the DMG. */
  app_zip_url: string | null;
  /** Other installers of the same platform (Windows: the .msi next to the setup .exe; Linux: the .deb next to the AppImage). */
  alternates?: { url: string; kind: AssetKind; size: number }[];
}

/** A release entry of the update feed (latest.json of the rolling "continuous" GitHub release), already picked for this platform. */
export interface ReleaseInfo {
  version: string;
  build: BuildInfo;
  published_at: IsoDateTime | null;
  notes: string;
  download_url: string;
  app_zip_url: string | null;
  release_url: string | null;
  /** Mirrors `PlatformAsset.kind` of the entry picked for this platform. */
  kind: AssetKind;
}

/** What the engine knows about updates right now (`get_update_status`). */
export interface UpdateStatus {
  current: BuildInfo;
  feed_url: string;
  /** False on development builds: no automatic checks (a manual check still runs). */
  enabled: boolean;
  available: ReleaseInfo | null;
  /** True when the user dismissed exactly this `available` build. */
  dismissed: boolean;
  last_check: IsoDateTime | null;
  last_error: string | null;
  checking: boolean;
}

/* ------------------------------------------------------------------ */
/* Focus guard (blocked apps and sites, interventions, focus sessions) */
/* ------------------------------------------------------------------ */

export type FocusTargetKind = 'app' | 'site';

/** An app or site the user asked ubiqX to hold. `key` = bundle id for apps, lower-case registrable domain for sites (matches any subdomain). */
export interface FocusTarget {
  id: Id;
  kind: FocusTargetKind;
  name: string;
  key: string;
  enabled: boolean;
  created_at: IsoDateTime;
  last_blocked_at: IsoDateTime | null;
  blocked_count: number;
}

/** An application found on this computer (`list_installed_apps`). `bundle_id` is the app id: bundle id on macOS, the executable stem on Windows/Linux. */
export interface InstalledApp {
  name: string;
  bundle_id: string;
  path: string;
}

/** What the guard did when a blocked app or site came to the front. */
export type InterventionAction = 'app_quit' | 'tab_closed' | 'tab_blanked' | 'notified';

export interface Intervention {
  id: Id;
  at: IsoDateTime;
  target_id: Id | null;
  kind: FocusTargetKind;
  name: string;
  key: string;
  action: InterventionAction;
  session_id: Id | null;
  /** The line UBI showed, in the UI language. */
  message: string;
}

export interface FocusSession {
  id: Id;
  task: string;
  started_at: IsoDateTime;
  ends_at: IsoDateTime;
  ended_at: IsoDateTime | null;
  interventions: number;
  hid_windows: boolean;
  ran_shortcut: boolean;
}

export interface FocusStatus {
  session: FocusSession | null;
  remaining_secs: number | null;
  targets_enabled: number;
  interventions_today: number;
  guard_enabled: boolean;
}

/** A domain seen in the user's own blocks, with the time spent there (`list_known_domains`, most time first). */
export interface KnownDomain {
  domain: string;
  seconds: number;
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
  | { type: 'screenshot_taken'; screenshot_id: Id; block_id: Id | null }
  | { type: 'update_available'; release: ReleaseInfo }
  | { type: 'intervention'; intervention: Intervention }
  /** Emitted on start, on end (session with `ended_at` set) and whenever `interventions` increments. */
  | { type: 'focus_session'; session: FocusSession | null };
