// Small pure helpers shared by the module's screens: names, durations, and the sentences that are built from
// data the engine sends (never from a host or a title).

import { hasKey, type Locale, type Vars } from '../i18n';
import { PANIC_CRISIS } from '../i18n/messages/nofap';
import type { AiProvider, ConfigView, HelpResource, NofapCard, NofapStatus, PendingView, PlainTierState, PreviewKey, PreviewPlaceholder, ProductId, SitesView } from './types';

export type Translate = (key: string, vars?: Vars) => string;

/** The five layers, in the order the status header lists them. */
export const LAYERS = ['guard', 'hosts', 'resolver', 'policies', 'vision'] as const;
export type Layer = (typeof LAYERS)[number];

/** The waiting times the wizard and the page offer (design 5.2), in seconds. */
export const DELAY_CHOICES = [0, 300, 600, 1800, 3600, 14_400, 86_400] as const;
export const VALUES_MAX_CHARS = 280;
export const MAX_PLANS = 3;
export const PLAN_TEXT_MAX_CHARS = 140;

/** Vendor names are proper nouns; only the managed provider's name follows the language. */
export function providerName(provider: AiProvider, t: Translate): string {
  switch (provider) {
    case 'anthropic':
      return 'Anthropic';
    case 'openai':
      return 'OpenAI';
    case 'xai':
      return 'xAI';
    case 'ubi':
      return t('nofap.provider.ubi');
  }
}

/**
 * The consent sentence for a provider. The managed one has its own: its name takes an article, and the rules and
 * terms that apply to the image are those of the vendor it sends the image on to.
 */
export const consentKey = (provider: AiProvider): string => (provider === 'ubi' ? 'nofap.vision.consent_ubi' : 'nofap.vision.consent');

/**
 * A sentence of the module followed by the line that is true only in ubiqX (integrator decision 32), which the
 * standalone app never shows: `vision.pause_note` after the consent, `helptab.ai.local_only` after the AI exit.
 */
export function withUbiqxLine(text: string, ubiqxKey: string, product: ProductId, t: Translate): string {
  return product === 'ubiqx' ? `${text} ${t(ubiqxKey)}` : text;
}

/** The consent sentence as the page shows it in `product`. */
export const consentText = (provider: AiProvider, name: string, product: ProductId, t: Translate): string =>
  withUbiqxLine(t(consentKey(provider), { name, provider: providerName(provider, t) }), 'nofap.vision.pause_note', product, t);

/** How the AI check stops at once, as the page shows it in `product`. */
export const aiExitText = (product: ProductId, t: Translate): string => withUbiqxLine(t('nofap.helptab.ai.body'), 'nofap.helptab.ai.local_only', product, t);

/** A waiting time in words: "10 minutos", "4 horas". Zero reads "sem espera", which fits a label and no sentence (`delayVariant`). */
export function fmtDelay(secs: number, t: Translate): string {
  if (secs <= 0) return t('nofap.delay.none').toLowerCase();
  if (secs % 3600 === 0) return t('nofap.delay.hours', { count: secs / 3600 });
  return t('nofap.delay.minutes', { count: Math.max(1, Math.ceil(secs / 60)) });
}

/**
 * The key of a sentence that names the waiting time. "Desligar espera sem espera" is not a sentence, so with no
 * waiting time the sentence is its `_now` twin, which says what happens then: a confirmation is asked.
 */
export const delayVariant = (key: string, secs: number): string => (secs > 0 ? key : `${key}_now`);

/** A waiting time on a chip: "10 min", "4 h". */
export function fmtDelayShort(secs: number, t: Translate): string {
  if (secs <= 0) return t('nofap.delay.none');
  return secs % 3600 === 0 ? `${secs / 3600} h` : `${Math.round(secs / 60)} min`;
}

/** A dollar amount in the language's own number format: "2,00", "2.00". */
export const fmtMoney = (value: number, locale: Locale): string => value.toLocaleString(locale === 'en' ? 'en-US' : 'pt-BR', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

/** A running clock: "7:59", "1:02:03". */
export function fmtClock(secs: number): string {
  const s = Math.max(0, Math.round(secs));
  const pad = (n: number) => String(n).padStart(2, '0');
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}:${pad(m)}:${pad(s % 60)}` : `${m}:${pad(s % 60)}`;
}

/** i18n key suffix of a tier state. `in_use_by_other` cannot be a key: `_other` is the plural suffix. */
export const tierKey = (state: PlainTierState | 'unsupported'): string => (state === 'in_use_by_other' ? 'other_app' : state);

/** The sentence for a reason code; a code this build does not know gets the neutral one. */
export const reasonText = (code: string | null, t: Translate): string => (code && hasKey(`nofap.reason.${code}`) ? t(`nofap.reason.${code}`) : t('nofap.reason.unknown'));

/** The placeholder each preview line fills with its `detail` (`ubiqx_core::nofap::preview::LINES`). */
export const PREVIEW_PLACEHOLDER: Record<PreviewKey, PreviewPlaceholder | null> = {
  'preview.hosts_add': 'n',
  'preview.hosts_update': 'n',
  'preview.hosts_remove': null,
  'hosts.readable': null,
  'preview.resolver_set': 'resolver',
  'dns.previous': 'servers',
  'preview.resolver_reset': null,
  'preview.policies_set': 'browsers',
  'policies.disclosure': null,
  'preview.policies_conflict': 'browsers',
  'preview.policies_remove': null,
};

/** Which layers are doing something right now; "protected" is only ever shown next to this list. */
export function layersOn(status: NofapStatus): Layer[] {
  const tierOn = (state: string) => state === 'applied' || state === 'update_pending' || state === 'in_use_by_other';
  const on: Layer[] = [];
  if (status.guard.state !== 'off') on.push('guard');
  if (tierOn(status.hosts.state)) on.push('hosts');
  if (tierOn(status.resolver.state)) on.push('resolver');
  if (tierOn(status.policies.state)) on.push('policies');
  if (status.vision.state === 'on') on.push('vision');
  return on;
}

/** What a waiting request is about. The page may name the site (it reads the configuration); a card never does. */
export function pendingLabel(pending: PendingView, config: ConfigView, sites: SitesView | null, name: string, t: Translate): string {
  const change = config.pending.find((p) => p.id === pending.id)?.change;
  if (change?.kind === 'add_allow') return t('nofap.pending.allow_host', { host: change.host });
  if (change?.kind === 'remove_block') {
    const host = sites?.blocked.find((s) => s.id === change.id)?.host;
    if (host) return t('nofap.pending.remove_host', { host });
  }
  return t(`nofap.pending.kind.${pending.kind}`, { name });
}

export const SCOPE_FLAGS = ['unknown_sites', 'media_apps', 'messaging_apps', 'private_windows'] as const;
export type ScopeFlag = (typeof SCOPE_FLAGS)[number];

/** The switches of the page whose change is waiting out the delay. */
export interface WaitingSwitches {
  disable: boolean;
  hosts: boolean;
  resolver: boolean;
  policies: boolean;
  hostsUserSites: boolean;
  vision: boolean;
  scope: Record<ScopeFlag, boolean>;
}

/**
 * Which switches have their change waiting: the thing is on, and a request to turn it off is among the waiting
 * ones. Such a switch is shown disabled: the same request again would change nothing. A switch that is off is
 * never waiting (turning something on applies at once).
 */
export function waitingSwitches(config: ConfigView): WaitingSwitches {
  const changes = config.pending.map((p) => p.change);
  const off = (kind: 'set_hosts' | 'set_policies' | 'set_hosts_user_sites' | 'set_vision'): boolean => changes.some((c) => c.kind === kind && !c.on);
  const narrowed = (flag: ScopeFlag): boolean => config.vision[flag] && changes.some((c) => c.kind === 'set_vision_scope' && !c[flag]);
  return {
    disable: config.enabled && changes.some((c) => c.kind === 'disable'),
    hosts: config.system.hosts && off('set_hosts'),
    resolver: config.system.resolver !== null && changes.some((c) => c.kind === 'set_resolver' && c.resolver === null),
    policies: config.system.policies && off('set_policies'),
    hostsUserSites: config.system.hosts_user_sites && off('set_hosts_user_sites'),
    vision: config.vision.enabled && off('set_vision'),
    scope: { unknown_sites: narrowed('unknown_sites'), media_apps: narrowed('media_apps'), messaging_apps: narrowed('messaging_apps'), private_windows: narrowed('private_windows') },
  };
}

/** The user's sentence as it goes inside the quotes of `panic.reason`: without its own closing full stop. */
export const quoted = (values: string): string => values.trim().replace(/[.…\s]+$/u, '');

const bare = (url: string | null | undefined): string => (url ?? '').replace(/^https?:\/\//, '').replace(/\/$/, '');

/**
 * `panic.crisis` with its numbers and addresses filled from the help resources (`cvv`, `samu`, `findahelpline`).
 * `null` when the crisis line itself is not in the payload: a sentence with a hole in it helps nobody.
 */
export function crisisText(locale: Locale, resources: HelpResource[]): string | null {
  const find = (id: string) => resources.find((r) => r.id === id);
  const crisis = find('cvv');
  if (!crisis?.phone) return null;
  const vars: Record<string, string> = {
    crisis_phone: crisis.phone,
    crisis_name: crisis.name,
    crisis_chat: bare(crisis.url),
    emergency_phone: find('samu')?.phone ?? '',
    intl_directory: bare(find('findahelpline')?.url),
  };
  const text = PANIC_CRISIS[locale].replace(/\{(\w+)\}/g, (_, key: string) => vars[key] ?? '');
  // A missing secondary resource would leave "ligue ." behind: say nothing rather than half of it.
  const needed = [...PANIC_CRISIS[locale].matchAll(/\{(\w+)\}/g)].map((m) => m[1] ?? '');
  return needed.every((key) => vars[key]) ? text : null;
}

export const resourceNote = (resource: HelpResource, locale: Locale): string => (locale === 'en' ? resource.note_en : resource.note_pt);

/** Logical size of the card window per card (design 5.3); `expanded` is the support, surf and help view. */
export function cardSize(card: Pick<NofapCard, 'kind' | 'discreet'>, expanded: boolean): { width: number; height: number } {
  const width = 420;
  if (expanded) return { width, height: 560 };
  switch (card.kind) {
    case 'blocked':
      return { width, height: card.discreet ? 132 : 560 };
    case 'blocked_repeat':
      return { width, height: 560 };
    // The help card is a sentence and two buttons until "Ver opções de ajuda" opens the list.
    case 'help':
    case 'vision_closed':
    case 'pending_ready':
    case 'resolver_unreachable':
      return { width, height: 220 };
    case 'vision_ask':
    case 'drift':
      return { width, height: 200 };
    case 'reminder':
      return { width, height: 160 };
  }
}
