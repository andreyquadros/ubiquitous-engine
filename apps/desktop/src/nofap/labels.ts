// Small pure helpers shared by the module's screens: names, durations, and the sentences that are built from
// data the engine sends (never from a host or a title).

import { hasKey, type Locale, type Vars } from '../i18n';
import { PANIC_CRISIS } from '../i18n/messages/nofap';
import type { AiProvider, ConfigView, HelpResource, NofapCard, NofapStatus, PendingView, PlainTierState, PreviewKey, PreviewPlaceholder, SitesView } from './types';

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

/** A waiting time in words, for a sentence: "10 minutos", "4 horas". Zero reads "sem espera". */
export function fmtDelay(secs: number, t: Translate): string {
  if (secs <= 0) return t('nofap.delay.none').toLowerCase();
  if (secs % 3600 === 0) return t('nofap.delay.hours', { count: secs / 3600 });
  return t('nofap.delay.minutes', { count: Math.max(1, Math.ceil(secs / 60)) });
}

/** A waiting time on a chip: "10 min", "4 h". */
export function fmtDelayShort(secs: number, t: Translate): string {
  if (secs <= 0) return t('nofap.delay.none');
  return secs % 3600 === 0 ? `${secs / 3600} h` : `${Math.round(secs / 60)} min`;
}

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
    case 'help':
      return { width, height: 560 };
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
