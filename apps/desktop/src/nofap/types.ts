// TypeScript mirror of the module's wire contract (`crates/ubiqx-core/src/nofap.rs` and `nofap_ipc.rs`).
// The Rust side is the source: `contract.ts` checks these types against the generated fixtures in both
// directions, so `pnpm typecheck` fails when the two drift apart.

export type Id = string;
/** RFC 3339 timestamp in UTC. */
export type IsoDateTime = string;

export type AiProvider = 'anthropic' | 'openai' | 'xai' | 'ubi';
export type ProductId = 'ubiqx' | 'shield';
export type Os = 'macos' | 'windows' | 'linux';
/** The three system tiers (L2, L3, L4). */
export type Tier = 'hosts' | 'resolver' | 'policies';
export type ResolverId = 'cloudflare_family';

/* ------------------------------------------------------------------ */
/* status                                                              */
/* ------------------------------------------------------------------ */

export type Overall = 'off' | 'protected' | 'reduced' | 'pending_off' | 'secondary';
export type GuardLimit = 'automation_denied' | 'browser_without_url' | 'title_only' | 'wayland';
export type GuardState = { state: 'on' } | { state: 'off' } | { state: 'limited'; reason: GuardLimit };

export type PlainTierState = 'off' | 'applied' | 'update_pending' | 'changed' | 'missing' | 'not_applied' | 'gated' | 'in_use_by_other';
/** `reason` is one of the codes of `ReasonCode`; a newer build may send one this build does not know. */
export type TierState = { state: PlainTierState } | { state: 'unsupported'; reason: string };

export type VisionOff = 'disabled' | 'consent_outdated' | 'no_provider' | 'no_key' | 'local_only' | 'not_onboarded' | 'licence' | 'screen_recording' | 'wayland' | 'budget_spent' | 'cap_reached';
export type VisionState = { state: 'off'; reason: VisionOff } | { state: 'on' } | { state: 'degraded'; until: IsoDateTime };

export type Issue =
  | 'hosts_not_honoured'
  | 'dns_bypass_suspected'
  | 'resolver_unreachable'
  | 'new_connection_unfiltered'
  | 'policy_conflict'
  | 'policy_advisory_only'
  | 'automation_denied'
  | 'screen_recording_denied'
  | 'vision_budget_spent'
  | 'vision_cap_reached'
  | 'vision_no_provider'
  | 'consent_outdated'
  | 'other_product_primary'
  | 'config_unreadable';

export type ChangeKind =
  | 'disable'
  | 'add_block'
  | 'remove_block'
  | 'add_allow'
  | 'remove_allow'
  | 'forget_learned'
  | 'set_hosts'
  | 'set_resolver'
  | 'set_policies'
  | 'set_hosts_user_sites'
  | 'set_vision'
  | 'set_vision_scope'
  | 'set_caps'
  | 'set_delay'
  | 'set_prefs'
  | 'set_values'
  | 'set_plans';

/** A waiting request as the status shows it: its kind and its clock, never its payload. */
export interface PendingView {
  id: Id;
  kind: ChangeKind;
  requested_at: IsoDateTime;
  ready_at: IsoDateTime;
  /** The delay has passed and enough running time was observed: it waits for a confirmation. */
  ready: boolean;
  /** Seconds until it can be confirmed (0 once ready). */
  remaining_secs: number;
  /** When a ready request lapses without a confirmation. */
  expires_at: IsoDateTime | null;
}

export interface NofapStatus {
  overall: Overall;
  guard: GuardState;
  hosts: TierState;
  resolver: TierState;
  policies: TierState;
  vision: VisionState;
  issues: Issue[];
  pending: PendingView[];
  /** Names waiting to enter the hosts block at the next apply the user starts. */
  hosts_waiting: number;
  /** User sites that do not fit the system list and stay in the in-app block only. */
  extras_left_out: number;
}

/* ------------------------------------------------------------------ */
/* configuration and changes                                           */
/* ------------------------------------------------------------------ */

export interface SystemPrefs {
  hosts: boolean;
  resolver: ResolverId | null;
  policies: boolean;
  hosts_user_sites: boolean;
  network_restored: boolean;
}

export interface VisionConsent {
  provider: AiProvider;
  managed: boolean;
  at: IsoDateTime;
}

export interface VisionScope {
  unknown_sites: boolean;
  media_apps: boolean;
  messaging_apps: boolean;
  private_windows: boolean;
}

export interface VisionPrefs extends VisionScope {
  enabled: boolean;
  consent: VisionConsent | null;
  max_per_hour: number;
  max_per_day: number;
  monthly_cap_usd: number;
}

export interface UiPrefs {
  discreet: boolean;
  show_streak: boolean;
  daily_checkin: boolean;
}

export interface IfThenPlan {
  when: string;
  then: string;
}

/** Every way the configuration or the lists can change. Wire form: `{ kind, ...fields }`. */
export type NofapChange =
  | { kind: 'disable' }
  | { kind: 'add_block'; host: string }
  | { kind: 'remove_block'; id: Id }
  | { kind: 'add_allow'; host: string }
  | { kind: 'remove_allow'; id: Id }
  | { kind: 'forget_learned' }
  | { kind: 'set_hosts'; on: boolean }
  | { kind: 'set_resolver'; resolver: ResolverId | null }
  | { kind: 'set_policies'; on: boolean }
  | { kind: 'set_hosts_user_sites'; on: boolean }
  | { kind: 'set_vision'; on: boolean }
  | ({ kind: 'set_vision_scope' } & VisionScope)
  | { kind: 'set_caps'; max_per_hour: number; max_per_day: number; monthly_cap_usd: number }
  | { kind: 'set_delay'; secs: number }
  | { kind: 'set_prefs'; prefs: UiPrefs }
  | { kind: 'set_values'; text: string }
  | { kind: 'set_plans'; plans: IfThenPlan[] };

/** A request that weakens protection, waiting out the delay. */
export interface PendingChange {
  id: Id;
  change: NofapChange;
  requested_at: IsoDateTime;
  ready_at: IsoDateTime;
  observed_secs: number;
  ready_since: IsoDateTime | null;
}

/** The stored configuration without its internal bookkeeping. */
export interface ConfigView {
  v: number;
  enabled: boolean;
  enabled_at: IsoDateTime | null;
  self_use_at: IsoDateTime | null;
  system: SystemPrefs;
  vision: VisionPrefs;
  /** How long a request that weakens protection waits, in seconds (0 to 86,400). */
  delay_secs: number;
  pending: PendingChange[];
  ui: UiPrefs;
  /** The user's own sentence (at most 280 characters). */
  values: string;
  /** "Se… então…" plans (at most three). */
  plans: IfThenPlan[];
}

/* ------------------------------------------------------------------ */
/* the view                                                            */
/* ------------------------------------------------------------------ */

export type TierCapability = { state: 'available' } | { state: 'gated' } | { state: 'unsupported'; reason: string };

export interface SystemCapabilities {
  /** `null` on a system without a back-end. */
  os: Os | null;
  hosts: TierCapability;
  resolver: TierCapability;
  policies: TierCapability;
  /** Tiers a tester opened; the page labels them "em teste". */
  unverified: Tier[];
}

export type AiReadiness = 'ready' | 'no_key' | 'local_only' | 'not_onboarded' | 'licence_required';

export interface TodayCounts {
  blocks: number;
  checks: number;
}

export interface CostLine {
  month_usd: number;
  month_checks: number;
  cap_usd: number;
}

/** Everything the module's page shows. */
export interface NofapView {
  status: NofapStatus;
  config: ConfigView;
  capabilities: SystemCapabilities;
  today: TodayCounts;
  /** The display name, for the `{name}` placeholder. */
  display_name: string;
  product: ProductId;
  /** The AI provider now selected, for the `{provider}` placeholder. */
  provider: AiProvider;
  ai: AiReadiness;
  cost_line: CostLine;
}

export type ChangeOutcome = { applied: NofapView } | { pending: PendingView };

/** One line of the preview shown before the OS prompt: `t(text_key, { [placeholder]: detail })`. */
export interface PlannedChange {
  tier: Tier;
  text_key: PreviewKey;
  detail: string | null;
}

export type PreviewKey =
  | 'preview.hosts_add'
  | 'preview.hosts_update'
  | 'preview.hosts_remove'
  | 'hosts.readable'
  | 'preview.resolver_set'
  | 'dns.previous'
  | 'preview.resolver_reset'
  | 'preview.policies_set'
  | 'policies.disclosure'
  | 'preview.policies_conflict'
  | 'preview.policies_remove';

export type PreviewPlaceholder = 'n' | 'resolver' | 'servers' | 'browsers';

export interface PreviewLine {
  key: PreviewKey;
  tier: Tier;
  placeholder: PreviewPlaceholder | null;
}

/** The closed vocabulary of "why not" codes. They travel as plain strings: an unknown one gets the neutral sentence. */
export type ReasonCode =
  | 'write_refused'
  | 'unreadable'
  | 'appimage'
  | 'no_resolved'
  | 'unreachable'
  | 'flatpak'
  | 'conflict'
  | 'gated'
  | 'not_built'
  | 'unsupported_os'
  | 'no_system_layer'
  | 'no_polkit_agent'
  | 'failed';

export interface TierReport {
  tier: Tier;
  ok: boolean;
  reason: string | null;
}

export type ApplyOutcome = { outcome: 'applied' } | { outcome: 'partial'; detail: TierReport[] } | { outcome: 'declined' } | { outcome: 'unavailable'; detail: string };

export interface SiteEntry {
  id: Id;
  host: string;
  created_at: IsoDateTime;
}

/** The lists the user can read and edit. Learned sites are a count, never a name. */
export interface SitesView {
  blocked: SiteEntry[];
  allowed: SiteEntry[];
  learned_count: number;
}

export type Mood = 'ok' | 'meh' | 'hard' | 'very_bad';
export type Trigger = 'tired' | 'bored' | 'stressed' | 'lonely' | 'hungry' | 'angry' | 'other';

export interface ProgressView {
  window_days: number;
  /** Days of the window that went the way the user wanted. */
  days: number;
  /** Current streak in days; `null` while the streak is switched off. */
  streak: number | null;
  /** A lapse was recorded inside the window. */
  after_reset: boolean;
}

/** One help resource. Phone numbers and addresses come from here, never from a message string. */
export interface HelpResource {
  id: string;
  /** ISO 3166-1 alpha-2, or empty for a resource that covers several countries. */
  country: string;
  name: string;
  phone: string | null;
  url: string | null;
  note_pt: string;
  note_en: string;
}

export interface SupportView {
  values: string;
  plans: IfThenPlan[];
  progress: ProgressView;
  checkin_today: Mood | null;
  help_due: boolean;
  resources: HelpResource[];
}

/** Something the user did in a flow. Counted per day and hour, never stored as an event. */
export type UserEvent =
  | { event: 'lapse'; triggers: Trigger[] }
  | { event: 'checkin'; mood: Mood; triggers: Trigger[] }
  | { event: 'surf_started' }
  | { event: 'surf_done' }
  | { event: 'panic' }
  | { event: 'step_away' };

export interface SampleFrame {
  mime: string;
  base64: string;
  width: number;
  height: number;
}

export interface ExportResult {
  path: string;
}

/* ------------------------------------------------------------------ */
/* cards and events                                                    */
/* ------------------------------------------------------------------ */

export type CardKind = 'blocked' | 'blocked_repeat' | 'vision_closed' | 'vision_ask' | 'drift' | 'pending_ready' | 'resolver_unreachable' | 'reminder' | 'help';

export type CardAction =
  | 'ok'
  | 'support'
  | 'surf'
  | 'close_window'
  | 'not_it'
  | 'request_exception'
  | 'add_to_list'
  | 'restore'
  | 'keep_protection'
  | 'turn_off'
  | 'restore_network'
  | 'later'
  | 'help';

/** What the card window shows. It has no host, title, URL or application field, by contract. */
export interface NofapCard {
  id: Id;
  kind: CardKind;
  /** Key in the `nofap` i18n namespace. */
  message_key: string;
  /** A second line shown under the message. */
  note_key: string | null;
  discreet: boolean;
  actions: CardAction[];
  /** The window closes by itself after this many seconds; `null` stays until answered. */
  auto_close_secs: number | null;
  expires_at: IsoDateTime;
}

/** What the module tells its shell (Tauri event `nofap`). No variant carries a host. */
export type NofapEvent =
  | { type: 'status'; status: NofapStatus }
  | { type: 'card'; card: NofapCard }
  | { type: 'card_closed'; id: Id }
  | { type: 'pending_changed' }
  | { type: 'sites_changed' };

/* ------------------------------------------------------------------ */
/* commands                                                            */
/* ------------------------------------------------------------------ */

export interface ConsentArgs {
  provider: AiProvider;
  managed: boolean;
}

/** What the wizard collected. Nothing is applied before this request. */
export interface EnableOptions {
  self_use: boolean;
  hosts: boolean;
  resolver: ResolverId | null;
  policies: boolean;
  vision: boolean;
  vision_consent: ConsentArgs | null;
  vision_scope: VisionScope;
  delay_secs: number;
  values: string;
  discreet: boolean;
}

/** A request to the one Tauri command, `nofap`. `args` is absent for commands without arguments. */
export type NofapCommand =
  | { cmd: 'view' }
  | { cmd: 'enable'; args: EnableOptions }
  | { cmd: 'consent_vision'; args: ConsentArgs }
  | { cmd: 'request'; args: NofapChange }
  | { cmd: 'confirm_pending'; args: { id: Id } }
  | { cmd: 'cancel_pending'; args: { id: Id } }
  | { cmd: 'system_preview' }
  | { cmd: 'system_apply' }
  | { cmd: 'system_remove' }
  | { cmd: 'restore_network' }
  | { cmd: 'sites' }
  | { cmd: 'support' }
  | { cmd: 'record'; args: UserEvent }
  | { cmd: 'card'; args: { id: Id } }
  | { cmd: 'card_action'; args: { id: Id; action: CardAction } }
  | { cmd: 'sample_frame' }
  | { cmd: 'erase_history' }
  | { cmd: 'reset' }
  | { cmd: 'export' }
  | { cmd: 'test_card' };

export type NofapCmd = NofapCommand['cmd'];

/* ------------------------------------------------------------------ */
/* the generated fixtures                                              */
/* ------------------------------------------------------------------ */

/** Every value of every closed set that crosses the boundary as a bare string. */
export interface GoldenEnums {
  overall: Overall[];
  guard_limit: GuardLimit[];
  vision_off: VisionOff[];
  issue: Issue[];
  change_kind: ChangeKind[];
  card_kind: CardKind[];
  card_action: CardAction[];
  mood: Mood[];
  trigger: Trigger[];
  ai_readiness: AiReadiness[];
  ai_provider: AiProvider[];
  product: ProductId[];
  os: Os[];
  tier: Tier[];
  resolver: ResolverId[];
  reason: ReasonCode[];
}

/** Shape of `fixtures/views.json`. */
export interface GoldenViews {
  view: NofapView;
  change_outcomes: ChangeOutcome[];
  planned_changes: PlannedChange[];
  apply_outcomes: ApplyOutcome[];
  sites: SitesView;
  support: SupportView;
  cards: NofapCard[];
  sample_frames: (SampleFrame | null)[];
  export: ExportResult;
  events: NofapEvent[];
  statuses: NofapStatus[];
  preview_lines: PreviewLine[];
  enums: GoldenEnums;
}
