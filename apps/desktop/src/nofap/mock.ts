// In-memory implementation of every `nofap` command, used whenever the page is not running inside Tauri
// (`vite dev` in a browser, Vitest, Playwright screenshots). `lib/mock` delegates the command `nofap` here.
// It keeps the commitment timer (requests that loosen protection wait out the delay and then need a fresh
// confirmation) and can be put into scripted states: `?nofap=<scenario>` in the address, or `__nofapMock`.

import { views } from './fixtures/golden';
import { cardSize } from './labels';
import type {
  AiProvider,
  AiReadiness,
  ApplyOutcome,
  CardAction,
  ChangeOutcome,
  ConfigView,
  EnableOptions,
  ExportResult,
  HelpResource,
  Id,
  IsoDateTime,
  NofapCard,
  NofapChange,
  NofapCommand,
  NofapEvent,
  NofapStatus,
  NofapView,
  PendingChange,
  PendingView,
  PlannedChange,
  ProductId,
  SampleFrame,
  SitesView,
  SupportView,
  SystemCapabilities,
  Tier,
  TierCapability,
  TierState,
  UserEvent,
  VisionState,
} from './types';

const clone = <T>(value: T): T => structuredClone(value) as T;
/** The fixtures are deeply read-only literals; a copy is an ordinary mutable value of the wire type. */
const thaw = <T>(value: unknown): T => structuredClone(value) as T;

/** How the mock answers the operating system's administrator prompt. */
export type PromptAnswer = 'accept' | 'decline' | 'partial' | 'unavailable';

/** Ready-made states for the browser preview and the screenshots (`?nofap=<name>`). */
export const SCENARIOS = ['off', 'on', 'pending', 'ready', 'reduced', 'gated', 'drift', 'secondary', 'left', 'help', 'unreadable'] as const;
export type Scenario = (typeof SCENARIOS)[number];

const TIERS: Tier[] = ['hosts', 'resolver', 'policies'];
const DISPLAY_NAME: string = views.view.display_name;
const DAY_MS = 86_400_000;

/** The help resources of the design (5.7); the real list comes from the engine's `resources.json`. */
const RESOURCES: HelpResource[] = [
  { id: 'cvv', country: 'BR', name: 'CVV', phone: '188', url: 'https://cvv.org.br/chat', note_pt: 'Grátis, 24 horas.', note_en: 'Free, 24 hours.' },
  { id: 'samu', country: 'BR', name: 'SAMU', phone: '192', url: null, note_pt: 'Emergência médica.', note_en: 'Medical emergency.' },
  { id: 'caps', country: 'BR', name: 'CAPS (SUS)', phone: null, url: 'https://www.gov.br/saude/pt-br/caps', note_pt: 'Atendimento público, sem agendamento.', note_en: 'Public service, walk-in.' },
  { id: 'crp', country: 'BR', name: 'CRP', phone: null, url: null, note_pt: 'Procure um psicólogo com registro ativo no CRP.', note_en: 'Look for a psychologist with an active CRP registration.' },
  { id: 'findahelpline', country: '', name: 'Find A Helpline', phone: null, url: 'https://findahelpline.com', note_pt: 'Linhas de apoio em outros países.', note_en: 'Helplines in other countries.' },
];

interface State {
  config: ConfigView;
  sites: SitesView;
  capabilities: SystemCapabilities;
  /** Which system tiers are in place on the pretend machine. */
  applied: Record<Tier, boolean>;
  /** Tiers that are in place but no longer what the app wrote (drift). */
  drifted: Tier[];
  prompt: PromptAnswer;
  provider: AiProvider;
  ai: AiReadiness;
  today: { blocks: number; checks: number };
  support: Omit<SupportView, 'values' | 'plans'>;
  cards: Map<Id, NofapCard>;
  /** Whether "Ver um exemplo" has a window to show. */
  hasCandidate: boolean;
  secondary: boolean;
  /** The stored configuration cannot be read: defaults in memory, writes refused until a reset or the wizard. */
  unreadable: boolean;
  /** A whole status that replaces the computed one (the fixtures' statuses, for rendering every state). */
  scripted: NofapStatus | null;
  seq: number;
}

const freshConfig = (): ConfigView => ({
  v: 1,
  enabled: false,
  enabled_at: null,
  self_use_at: null,
  system: { hosts: false, resolver: null, policies: false, hosts_user_sites: true, network_restored: false },
  vision: { enabled: false, consent: null, unknown_sites: true, media_apps: true, messaging_apps: false, private_windows: true, max_per_hour: 20, max_per_day: 120, monthly_cap_usd: 2 },
  delay_secs: 600,
  pending: [],
  ui: { discreet: true, show_streak: false, daily_checkin: false },
  values: '',
  plans: [],
});

const available = (): TierCapability => ({ state: 'available' });

function freshState(): State {
  return {
    config: freshConfig(),
    sites: { blocked: [], allowed: [], learned_count: 0 },
    capabilities: { os: 'linux', hosts: available(), resolver: available(), policies: available(), unverified: [] },
    applied: { hosts: false, resolver: false, policies: false },
    drifted: [],
    prompt: 'accept',
    provider: 'anthropic',
    ai: 'ready',
    today: { blocks: 0, checks: 0 },
    support: { progress: { window_days: 30, days: 0, streak: null, after_reset: false }, checkin_today: null, help_due: false, resources: clone(RESOURCES) },
    cards: new Map(),
    hasCandidate: true,
    secondary: false,
    unreadable: false,
    scripted: null,
    seq: 0,
  };
}

let S: State = freshState();
/** Which product hosts the page: ubiqX, unless the standalone app's mock says otherwise. Survives the scenarios. */
let product: ProductId = 'ubiqx';

const nowIso = (): IsoDateTime => new Date().toISOString();
const plus = (iso: IsoDateTime, secs: number): IsoDateTime => new Date(new Date(iso).getTime() + secs * 1000).toISOString();
/** A fresh id: never one a site, a waiting request or a card of the state already has (a scenario starts from the fixtures, which bring their own). */
function nextId(prefix: string): Id {
  const taken = (id: Id): boolean => S.cards.has(id) || [...S.sites.blocked, ...S.sites.allowed, ...S.config.pending].some((entry) => entry.id === id);
  let id: Id;
  do id = `${prefix}-${(S.seq += 1)}`;
  while (taken(id));
  return id;
}

/* ------------------------------------------------------------------ */
/* events                                                              */
/* ------------------------------------------------------------------ */

type Handler = (event: NofapEvent) => void;
const subscribers = new Set<Handler>();

const emit = (event: NofapEvent): void => {
  for (const handler of subscribers) handler(clone(event));
};

export function subscribe(handler: Handler): () => void {
  subscribers.add(handler);
  return () => {
    subscribers.delete(handler);
  };
}

/* ------------------------------------------------------------------ */
/* status                                                              */
/* ------------------------------------------------------------------ */

const wants = (tier: Tier): boolean => (tier === 'resolver' ? S.config.system.resolver !== null && !S.config.system.network_restored : S.config.system[tier]);

function tierState(tier: Tier): TierState {
  const capability = S.capabilities[tier];
  if (capability.state === 'unsupported') return { state: 'unsupported', reason: capability.reason };
  if (capability.state === 'gated') return { state: 'gated' };
  if (S.secondary && S.applied[tier]) return { state: 'in_use_by_other' };
  if (!S.config.enabled || !wants(tier)) return { state: S.applied[tier] && !S.config.enabled ? 'applied' : 'off' };
  if (!S.applied[tier]) return { state: 'not_applied' };
  if (S.drifted.includes(tier)) return { state: 'changed' };
  return { state: 'applied' };
}

function visionState(): VisionState {
  const vision = S.config.vision;
  if (!S.config.enabled || !vision.enabled) return { state: 'off', reason: 'disabled' };
  if (!vision.consent || vision.consent.provider !== S.provider) return { state: 'off', reason: 'consent_outdated' };
  if (S.ai === 'no_key') return { state: 'off', reason: 'no_key' };
  if (S.ai === 'local_only') return { state: 'off', reason: 'local_only' };
  if (S.ai === 'not_onboarded') return { state: 'off', reason: 'not_onboarded' };
  if (S.ai === 'licence_required') return { state: 'off', reason: 'licence' };
  return { state: 'on' };
}

/** A request is ready once its delay has passed; a ready request lapses 24 hours later. */
function pendingView(pending: PendingChange): PendingView {
  const remaining = Math.max(0, Math.ceil((new Date(pending.ready_at).getTime() - Date.now()) / 1000));
  return {
    id: pending.id,
    kind: pending.change.kind,
    requested_at: pending.requested_at,
    ready_at: pending.ready_at,
    ready: remaining === 0,
    remaining_secs: remaining,
    expires_at: remaining === 0 ? plus(pending.ready_at, DAY_MS / 1000) : null,
  };
}

function status(): NofapStatus {
  if (S.scripted) return clone(S.scripted);
  if (S.unreadable) {
    // As the engine says it: not "off", nothing in force from the app, and the reason.
    const off: TierState = { state: 'off' };
    return { overall: 'reduced', guard: { state: 'off' }, hosts: off, resolver: off, policies: off, vision: { state: 'off', reason: 'disabled' }, issues: ['config_unreadable'], pending: [], hosts_waiting: 0, extras_left_out: 0 };
  }
  const pending = S.config.pending.map(pendingView);
  const hosts = tierState('hosts');
  const resolver = tierState('resolver');
  const policies = tierState('policies');
  const vision = visionState();
  const issues: NofapStatus['issues'] = [];
  if (S.config.enabled && S.config.vision.enabled && vision.state === 'off' && vision.reason === 'consent_outdated') issues.push('consent_outdated');
  if (S.secondary) issues.push('other_product_primary');
  const short = [hosts, resolver, policies].some((t) => t.state === 'not_applied' || t.state === 'changed' || t.state === 'missing');
  const visionShort = S.config.vision.enabled && vision.state !== 'on';
  const overall: NofapStatus['overall'] = !S.config.enabled ? 'off' : S.secondary ? 'secondary' : pending.some((p) => p.kind === 'disable') ? 'pending_off' : short || visionShort ? 'reduced' : 'protected';
  return {
    overall,
    guard: S.config.enabled && !S.secondary ? { state: 'on' } : { state: 'off' },
    hosts,
    resolver,
    policies,
    vision,
    issues,
    pending,
    hosts_waiting: 0,
    extras_left_out: 0,
  };
}

function view(): NofapView {
  return {
    status: status(),
    config: clone(S.config),
    capabilities: clone(S.capabilities),
    today: { ...S.today },
    display_name: DISPLAY_NAME,
    product,
    provider: S.provider,
    ai: S.ai,
    cost_line: { month_usd: S.today.checks ? 0.21 : 0, month_checks: S.today.checks ? 300 : 0, cap_usd: S.config.vision.monthly_cap_usd },
  };
}

const announce = (): void => emit({ type: 'status', status: status() });

/* ------------------------------------------------------------------ */
/* the commitment rules (design 6.1)                                   */
/* ------------------------------------------------------------------ */

/** Whether a change takes effect now or waits out the delay. Before the first enable everything is immediate. */
export function loosens(config: ConfigView, change: NofapChange): boolean {
  if (!config.enabled) return false;
  switch (change.kind) {
    case 'disable':
    case 'remove_block':
    case 'add_allow':
    case 'forget_learned':
      return true;
    case 'add_block':
    case 'remove_allow':
    case 'set_prefs':
    case 'set_values':
    case 'set_plans':
      return false;
    case 'set_hosts':
    case 'set_policies':
    case 'set_vision':
    case 'set_hosts_user_sites':
      return !change.on;
    case 'set_resolver':
      return change.resolver === null;
    case 'set_vision_scope':
      return (['unknown_sites', 'media_apps', 'messaging_apps', 'private_windows'] as const).some((flag) => config.vision[flag] && !change[flag]);
    case 'set_caps':
      return change.max_per_hour < config.vision.max_per_hour || change.max_per_day < config.vision.max_per_day || change.monthly_cap_usd < config.vision.monthly_cap_usd;
    case 'set_delay':
      return change.secs < config.delay_secs;
  }
}

const normalizeHost = (raw: string): string =>
  raw
    .trim()
    .toLowerCase()
    .replace(/^[a-z]+:\/\//, '')
    .replace(/^www\./, '')
    .replace(/[/?#].*$/, '');

function applySystem(): void {
  for (const tier of TIERS) {
    if (S.capabilities[tier].state === 'available') S.applied[tier] = S.config.enabled && wants(tier);
  }
  S.drifted = [];
}

function applyChange(change: NofapChange): void {
  const config = S.config;
  switch (change.kind) {
    case 'disable':
      config.enabled = false;
      config.pending = [];
      // A declined prompt leaves the system tiers in place: the page then offers "Remover do sistema".
      if (S.prompt === 'accept') applySystem();
      break;
    case 'add_block': {
      const host = normalizeHost(change.host);
      if (!host || !host.includes('.')) throw new Error('invalid: host');
      if (!S.sites.blocked.some((s) => s.host === host)) S.sites.blocked.push({ id: nextId('site'), host, created_at: nowIso() });
      emit({ type: 'sites_changed' });
      break;
    }
    case 'remove_block':
      S.sites.blocked = S.sites.blocked.filter((s) => s.id !== change.id);
      emit({ type: 'sites_changed' });
      break;
    case 'add_allow': {
      const host = normalizeHost(change.host);
      if (!host || !host.includes('.')) throw new Error('invalid: host');
      if (!S.sites.allowed.some((s) => s.host === host)) S.sites.allowed.push({ id: nextId('site'), host, created_at: nowIso() });
      emit({ type: 'sites_changed' });
      break;
    }
    case 'remove_allow':
      S.sites.allowed = S.sites.allowed.filter((s) => s.id !== change.id);
      emit({ type: 'sites_changed' });
      break;
    case 'forget_learned':
      S.sites.learned_count = 0;
      emit({ type: 'sites_changed' });
      break;
    case 'set_hosts':
      config.system.hosts = change.on;
      break;
    case 'set_resolver':
      config.system.resolver = change.resolver;
      config.system.network_restored = false;
      break;
    case 'set_policies':
      config.system.policies = change.on;
      break;
    case 'set_hosts_user_sites':
      config.system.hosts_user_sites = change.on;
      break;
    case 'set_vision':
      config.vision.enabled = change.on;
      break;
    case 'set_vision_scope':
      config.vision.unknown_sites = change.unknown_sites;
      config.vision.media_apps = change.media_apps;
      config.vision.messaging_apps = change.messaging_apps;
      config.vision.private_windows = change.private_windows;
      break;
    case 'set_caps':
      config.vision.max_per_hour = change.max_per_hour;
      config.vision.max_per_day = change.max_per_day;
      config.vision.monthly_cap_usd = change.monthly_cap_usd;
      break;
    case 'set_delay':
      config.delay_secs = Math.max(0, Math.min(86_400, Math.round(change.secs)));
      break;
    case 'set_prefs':
      config.ui = { ...change.prefs };
      S.support.progress.streak = change.prefs.show_streak ? (S.support.progress.streak ?? 0) : null;
      break;
    case 'set_values':
      config.values = [...change.text.trim()].slice(0, 280).join('');
      break;
    case 'set_plans':
      config.plans = change.plans.slice(0, 3).map((p) => ({ when: p.when.trim(), then: p.then.trim() }));
      break;
  }
}

/** A change as text that does not depend on the order its fields were written in. */
const canonical = (change: NofapChange): string =>
  JSON.stringify(change, (_, value: unknown) => (value && typeof value === 'object' && !Array.isArray(value) ? Object.fromEntries(Object.entries(value).sort(([a], [b]) => a.localeCompare(b))) : value));

/** The engine's answer to a write while the stored configuration cannot be read. */
function refuseUnreadable(): void {
  if (S.unreadable) throw new Error('storage: the stored configuration of the module cannot be read; run the set-up again');
}

function request(change: NofapChange): ChangeOutcome {
  refuseUnreadable();
  if (loosens(S.config, change)) {
    // The same change asked for again is the request that is already waiting, with its clock as it stands.
    const waiting = S.config.pending.find((p) => canonical(p.change) === canonical(change));
    if (waiting) return { pending: pendingView(waiting) };
    if (S.config.pending.length >= 20) throw new Error('invalid: too many pending requests');
    const requested_at = nowIso();
    const pending: PendingChange = { id: nextId('pending'), change: clone(change), requested_at, ready_at: plus(requested_at, S.config.delay_secs), observed_secs: 0, ready_since: null };
    S.config.pending.push(pending);
    emit({ type: 'pending_changed' });
    announce();
    return { pending: pendingView(pending) };
  }
  applyChange(change);
  announce();
  return { applied: view() };
}

function findPending(id: Id): PendingChange {
  const pending = S.config.pending.find((p) => p.id === id);
  if (!pending) throw new Error('not_found: pending request');
  return pending;
}

function confirmPending(id: Id): NofapView {
  refuseUnreadable();
  const pending = findPending(id);
  if (!pendingView(pending).ready) throw new Error('invalid: the request is still waiting');
  S.config.pending = S.config.pending.filter((p) => p.id !== id);
  applyChange(pending.change);
  emit({ type: 'pending_changed' });
  announce();
  return view();
}

function cancelPending(id: Id): NofapView {
  refuseUnreadable();
  findPending(id);
  S.config.pending = S.config.pending.filter((p) => p.id !== id);
  emit({ type: 'pending_changed' });
  announce();
  return view();
}

/* ------------------------------------------------------------------ */
/* system layer                                                        */
/* ------------------------------------------------------------------ */

function enable(options: EnableOptions): NofapView {
  if (!options.self_use) throw new Error('invalid: the self-use tick is required');
  if (options.vision && !options.vision_consent) throw new Error('invalid: the AI check needs consent');
  const at = nowIso();
  S.config = {
    ...S.config,
    enabled: true,
    enabled_at: at,
    self_use_at: at,
    system: { hosts: options.hosts, resolver: options.resolver, policies: options.policies, hosts_user_sites: S.config.system.hosts_user_sites, network_restored: false },
    vision: { ...S.config.vision, ...options.vision_scope, enabled: options.vision, consent: options.vision_consent ? { ...options.vision_consent, at } : null },
    delay_secs: Math.max(0, Math.min(86_400, options.delay_secs)),
    values: options.values.trim(),
    ui: { ...S.config.ui, discreet: options.discreet },
    pending: [],
  };
  // The wizard writes a fresh configuration: one way out of an unreadable one.
  S.unreadable = false;
  if (S.prompt === 'accept') applySystem();
  announce();
  return view();
}

function systemPreview(): PlannedChange[] {
  const lines: PlannedChange[] = [];
  const enabled = S.config.enabled;
  if (S.capabilities.hosts.state === 'available') {
    if (enabled && wants('hosts') && !S.applied.hosts) {
      lines.push({ tier: 'hosts', text_key: 'preview.hosts_add', detail: String(8003 + S.sites.blocked.length) });
      if (S.config.system.hosts_user_sites && S.sites.blocked.length) lines.push({ tier: 'hosts', text_key: 'hosts.readable', detail: null });
    } else if (enabled && wants('hosts') && S.drifted.includes('hosts')) {
      lines.push({ tier: 'hosts', text_key: 'preview.hosts_update', detail: String(8003 + S.sites.blocked.length) });
    } else if (S.applied.hosts && !(enabled && wants('hosts'))) {
      lines.push({ tier: 'hosts', text_key: 'preview.hosts_remove', detail: null });
    }
  }
  if (S.capabilities.resolver.state === 'available') {
    if (enabled && wants('resolver') && !S.applied.resolver) {
      lines.push({ tier: 'resolver', text_key: 'preview.resolver_set', detail: 'Cloudflare for Families' });
      lines.push({ tier: 'resolver', text_key: 'dns.previous', detail: '192.168.0.2' });
    } else if (S.applied.resolver && !(enabled && wants('resolver'))) {
      lines.push({ tier: 'resolver', text_key: 'preview.resolver_reset', detail: null });
    }
  }
  if (S.capabilities.policies.state === 'available') {
    if (enabled && wants('policies') && !S.applied.policies) {
      lines.push({ tier: 'policies', text_key: 'preview.policies_set', detail: 'Chrome, Firefox' });
      lines.push({ tier: 'policies', text_key: 'policies.disclosure', detail: null });
    } else if (S.applied.policies && !(enabled && wants('policies'))) {
      lines.push({ tier: 'policies', text_key: 'preview.policies_remove', detail: null });
    }
  }
  return lines;
}

function prompted(act: () => void): ApplyOutcome {
  switch (S.prompt) {
    case 'decline':
      return { outcome: 'declined' };
    case 'unavailable':
      return { outcome: 'unavailable', detail: 'no_polkit_agent' };
    case 'partial':
      act();
      S.applied.resolver = false;
      announce();
      return {
        outcome: 'partial',
        detail: [
          { tier: 'hosts', ok: true, reason: null },
          { tier: 'resolver', ok: false, reason: 'unreachable' },
        ],
      };
    case 'accept':
      act();
      announce();
      return { outcome: 'applied' };
  }
}

function systemApply(): ApplyOutcome {
  refuseUnreadable();
  return prompted(() => {
    S.config.system.network_restored = false;
    applySystem();
  });
}

function systemRemove(): ApplyOutcome {
  if (S.config.enabled) throw new Error('invalid: only while the module is off');
  return prompted(() => {
    S.applied = { hosts: false, resolver: false, policies: false };
    S.drifted = [];
  });
}

function restoreNetwork(): ApplyOutcome {
  refuseUnreadable();
  return prompted(() => {
    S.applied.resolver = false;
    S.config.system.network_restored = true;
  });
}

/* ------------------------------------------------------------------ */
/* support, cards, data                                                */
/* ------------------------------------------------------------------ */

function record(event: UserEvent): null {
  switch (event.event) {
    case 'lapse':
      S.support.progress.after_reset = true;
      S.support.progress.days = Math.max(0, S.support.progress.days - 1);
      if (S.support.progress.streak !== null) S.support.progress.streak = 0;
      break;
    case 'checkin':
      S.support.checkin_today = event.mood;
      break;
    case 'surf_started':
    case 'surf_done':
    case 'panic':
    case 'step_away':
      break;
  }
  return null;
}

/** What the engine answers for an id it does not know: the neutral discreet card, never a sample. */
export const neutralCard = (id: Id): NofapCard => ({
  id,
  kind: 'blocked',
  message_key: 'block.discreet',
  note_key: null,
  discreet: true,
  actions: ['support', 'ok'],
  auto_close_secs: 12,
  expires_at: plus(nowIso(), 600),
});

function cardAction(id: Id, action: CardAction): null {
  const card = S.cards.get(id);
  switch (action) {
    case 'not_it':
      // A hit on a list becomes a delayed allow request; a vision card only buys a second look.
      if (card && (card.kind === 'blocked' || card.kind === 'blocked_repeat')) request({ kind: 'add_allow', host: 'example.net' });
      break;
    case 'request_exception':
      request({ kind: 'add_allow', host: 'example.net' });
      break;
    case 'add_to_list':
      request({ kind: 'add_block', host: 'example.net' });
      break;
    case 'restore':
      systemApply();
      break;
    case 'restore_network':
      restoreNetwork();
      break;
    case 'keep_protection':
      for (const pending of S.config.pending.filter((p) => p.change.kind === 'disable')) cancelPending(pending.id);
      break;
    case 'turn_off': {
      const ready = S.config.pending.find((p) => p.change.kind === 'disable' && pendingView(p).ready);
      if (ready) confirmPending(ready.id);
      break;
    }
    case 'ok':
    case 'support':
    case 'surf':
    case 'close_window':
    case 'later':
    case 'help':
      break;
  }
  // The window stays open for the flows it hosts itself.
  if (action !== 'support' && action !== 'surf' && action !== 'help' && action !== 'not_it' && action !== 'request_exception' && action !== 'add_to_list') {
    S.cards.delete(id);
    emit({ type: 'card_closed', id });
  }
  return null;
}

const toBase64 = (text: string): string => {
  let binary = '';
  for (const byte of new TextEncoder().encode(text)) binary += String.fromCharCode(byte);
  return btoa(binary);
};

/** A drawn stand-in for the reduced frame: a browser window with grey boxes, nothing real. */
function sampleFrame(): SampleFrame | null {
  if (!S.hasCandidate) return null;
  const svg =
    '<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400" viewBox="0 0 640 400">' +
    '<rect width="640" height="400" fill="#0c1220"/><rect width="640" height="36" fill="#121a2b"/>' +
    '<rect x="96" y="9" width="448" height="18" rx="9" fill="#18213a"/>' +
    '<rect x="24" y="60" width="280" height="150" rx="8" fill="#18213a"/><rect x="336" y="60" width="280" height="150" rx="8" fill="#18213a"/>' +
    '<rect x="24" y="234" width="592" height="14" rx="7" fill="#18213a"/><rect x="24" y="262" width="440" height="14" rx="7" fill="#18213a"/>' +
    '<rect x="24" y="290" width="520" height="14" rx="7" fill="#18213a"/></svg>';
  return { mime: 'image/svg+xml', base64: toBase64(svg), width: 640, height: 400 };
}

/** "Apagar meu histórico": the counts, and what the person wrote (the sentence and the plans). Protection stays. */
function eraseHistory(): null {
  S.today = { blocks: 0, checks: 0 };
  S.config.values = '';
  S.config.plans = [];
  S.support.progress = { window_days: 30, days: 0, streak: S.config.ui.show_streak ? 0 : null, after_reset: false };
  S.support.checkin_today = null;
  S.support.help_due = false;
  return null;
}

/** "Apagar tudo", and "Restaurar padrões" while the stored configuration cannot be read (a fresh state clears it). */
function reset(): null {
  if (S.config.enabled && !S.unreadable) throw new Error('invalid: only while the module is off');
  const { capabilities, applied, prompt, provider, ai } = S;
  S = { ...freshState(), capabilities, applied, prompt, provider, ai };
  emit({ type: 'sites_changed' });
  announce();
  return null;
}

/** Shows `card` the way the shell does: remembers it, emits the event and, in a browser, opens the window. */
function present(card: NofapCard): NofapCard {
  S.cards.set(card.id, clone(card));
  emit({ type: 'card', card });
  if (typeof window !== 'undefined' && import.meta.env.MODE !== 'test') {
    const { width, height } = cardSize(card, false);
    window.open(`${window.location.pathname}${window.location.search}#/nofap-card?id=${encodeURIComponent(card.id)}`, 'ubiqx-shield-card', `popup=yes,width=${width},height=${height},top=24`);
  }
  return card;
}

/* ------------------------------------------------------------------ */
/* dispatch                                                            */
/* ------------------------------------------------------------------ */

/** Answers one request the way `ubiqx_engine::nofap::ipc::handle` does. */
export function handle(command: NofapCommand): unknown {
  calls.push(clone(command));
  switch (command.cmd) {
    case 'view':
      return view();
    case 'enable':
      return enable(command.args);
    case 'consent_vision': {
      refuseUnreadable();
      S.config.vision.consent = { ...command.args, at: nowIso() };
      announce();
      return view();
    }
    case 'request':
      return request(command.args);
    case 'confirm_pending':
      return confirmPending(command.args.id);
    case 'cancel_pending':
      return cancelPending(command.args.id);
    case 'system_preview':
      return systemPreview();
    case 'system_apply':
      return systemApply();
    case 'system_remove':
      return systemRemove();
    case 'restore_network':
      return restoreNetwork();
    case 'sites':
      return clone(S.sites);
    case 'support':
      return { ...clone(S.support), values: S.config.values, plans: clone(S.config.plans) } satisfies SupportView;
    case 'record':
      return record(command.args);
    case 'card':
      return clone(S.cards.get(command.args.id) ?? neutralCard(command.args.id));
    case 'card_action':
      return cardAction(command.args.id, command.args.action);
    case 'sample_frame':
      return sampleFrame();
    case 'erase_history':
      return eraseHistory();
    case 'reset':
      return reset();
    case 'export':
      refuseUnreadable();
      return { path: views.export.path } satisfies ExportResult;
    case 'test_card':
      present({ ...neutralCard(nextId('card')), discreet: S.config.ui.discreet, message_key: S.config.ui.discreet ? 'block.discreet' : 'block.breathe' });
      return null;
  }
}

/* ------------------------------------------------------------------ */
/* scenarios and hooks for tests and screenshots                       */
/* ------------------------------------------------------------------ */

/** Every request the mock received since the last reset, oldest first. */
let calls: NofapCommand[] = [];

function enabledState(): void {
  const sample = thaw<NofapView>(views.view);
  S.config = { ...sample.config, pending: [], enabled_at: new Date(Date.now() - 12 * DAY_MS).toISOString(), self_use_at: new Date(Date.now() - 12 * DAY_MS).toISOString() };
  S.config.system = { hosts: true, resolver: 'cloudflare_family', policies: true, hosts_user_sites: true, network_restored: false };
  S.config.vision.consent = { provider: 'anthropic', managed: false, at: S.config.enabled_at ?? nowIso() };
  S.applied = { hosts: true, resolver: true, policies: true };
  S.sites = thaw<SitesView>(views.sites);
  S.today = { blocks: 2, checks: 14 };
  S.support = { progress: { window_days: 30, days: 26, streak: null, after_reset: false }, checkin_today: null, help_due: false, resources: clone(RESOURCES) };
}

function applyScenario(name: Scenario): void {
  S = freshState();
  if (name === 'off') return;
  enabledState();
  const requested_at = nowIso();
  switch (name) {
    case 'on':
      break;
    case 'pending':
      S.config.pending = [{ id: nextId('pending'), change: { kind: 'disable' }, requested_at, ready_at: plus(requested_at, 480), observed_secs: 120, ready_since: null }];
      break;
    case 'ready':
      S.config.pending = [{ id: nextId('pending'), change: { kind: 'disable' }, requested_at: plus(requested_at, -700), ready_at: plus(requested_at, -100), observed_secs: 700, ready_since: plus(requested_at, -100) }];
      break;
    case 'reduced':
      S.applied = { hosts: false, resolver: false, policies: false };
      S.prompt = 'decline';
      break;
    case 'gated':
      S.capabilities = { os: 'macos', hosts: { state: 'gated' }, resolver: { state: 'gated' }, policies: { state: 'unsupported', reason: 'not_built' }, unverified: [] };
      S.applied = { hosts: false, resolver: false, policies: false };
      break;
    case 'drift':
      S.drifted = ['hosts'];
      break;
    case 'secondary':
      S.secondary = true;
      break;
    case 'left':
      S.config.enabled = false;
      break;
    case 'unreadable':
      // What the engine holds when the stored row does not parse: the defaults, and the flag.
      S.config = freshConfig();
      S.sites = { blocked: [], allowed: [], learned_count: 0 };
      S.unreadable = true;
      break;
    case 'help':
      S.support.help_due = true;
      S.support.progress = { window_days: 30, days: 19, streak: 0, after_reset: true };
      S.config.ui = { discreet: true, show_streak: true, daily_checkin: true };
      break;
  }
}

const isScenario = (v: unknown): v is Scenario => typeof v === 'string' && (SCENARIOS as readonly string[]).includes(v);

/** `?nofap=<scenario>` is read once at module load, like the app's other preview switches. */
const SCENARIO_FROM_QUERY: Scenario = (() => {
  try {
    if (typeof window === 'undefined') return 'off';
    const q = new URLSearchParams(window.location.search).get('nofap');
    return isScenario(q) ? q : 'off';
  } catch {
    return 'off';
  }
})();

applyScenario(SCENARIO_FROM_QUERY);

export const __nofapMock = {
  /** Back to the scenario of the address (a fresh, switched-off module without one). */
  reset(scenario: Scenario = SCENARIO_FROM_QUERY): void {
    applyScenario(scenario);
    calls = [];
  },
  /** Replaces the computed status with a whole one (`null` goes back to computing it). */
  setStatus(status: NofapStatus | null): void {
    S.scripted = status ? clone(status) : null;
    announce();
  },
  /** How the pretend operating system answers the administrator prompt. */
  setPrompt(answer: PromptAnswer): void {
    S.prompt = answer;
  },
  setCapabilities(patch: Partial<SystemCapabilities>): void {
    S.capabilities = { ...S.capabilities, ...clone(patch) };
  },
  setAi(provider: AiProvider, ai: AiReadiness = 'ready'): void {
    S.provider = provider;
    S.ai = ai;
  },
  setSupport(patch: Partial<Omit<SupportView, 'values' | 'plans'>>): void {
    S.support = { ...S.support, ...clone(patch) };
  },
  setCandidate(present: boolean): void {
    S.hasCandidate = present;
  },
  /** Which product hosts the page (`view.product`); the standalone app's mock says `shield`. */
  setProduct(next: ProductId): void {
    product = next;
  },
  /** Shows a card the way the engine would. */
  present,
  /** Sends an event the way the engine would. */
  emit,
  /** The fixture card of `kind`, under a fresh id. */
  sampleCard(kind: NofapCard['kind']): NofapCard {
    const sample = views.cards.find((c) => c.kind === kind) ?? views.cards[0];
    return { ...thaw<NofapCard>(sample), id: nextId('card') };
  },
  calls: (): NofapCommand[] => clone(calls),
  state: (): State => S,
};

declare global {
  interface Window {
    __nofapMock?: typeof __nofapMock;
  }
}

if (typeof window !== 'undefined') window.__nofapMock = __nofapMock;
