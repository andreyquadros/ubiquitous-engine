// Provider-related helpers shared by the pages and the mock backend. Everything that is a
// *fact* about a provider (label, console URL, key prefix, default models) comes from the
// backend as `ProviderInfo`; this file only holds UI copy and pure model-id arithmetic.

import { hasKey, t } from '../i18n';
import type { AiModels, AiProvider, ApiKeyStatus, LicenseStatus, Plan, ProviderInfo, SettingsView } from './types';

/** Every provider, in the backend's order: the three "bring your own key" vendors, then the managed "IA do Ubi". */
export const PROVIDER_IDS: AiProvider[] = ['anthropic', 'openai', 'xai', 'ubi'];

/** The vendors that take the user's own API key (the annual plan); the managed provider is `ubi`. */
export const OWN_KEY_PROVIDER_IDS: AiProvider[] = ['anthropic', 'openai', 'xai'];

/** The managed provider: calls go through the Ubi proxy, paid by the monthly plan, no vendor key. */
export const MANAGED_PROVIDER: AiProvider = 'ubi';
export const isManagedProvider = (id: AiProvider): boolean => id === MANAGED_PROVIDER;

/** Fixed server-side model aliases of the managed provider (mirrors `UBI_MODEL_FAST` / `UBI_MODEL_SMART`). */
export const UBI_MODELS: AiModels = { classify: 'ubi-fast', vision: 'ubi-fast', report: 'ubi-smart' };

/** Where the plans are sold (the landing page's pricing section). Also the "console" of the managed provider. */
export const SITE_URL = 'https://andreyquadros.github.io/ubiquitous-engine/#planos';

/** Typical prefix of a license key (`UBIQX-<claims>-<signature>`). */
export const LICENSE_KEY_PREFIX = 'UBIQX-';

/** Both plans, in the order the UI lists them. */
export const PLAN_IDS: Plan[] = ['annual_own_key', 'monthly_managed'];

/** Whether the managed provider may be selected: a valid `monthly_managed` license. */
export const licenseAllowsManaged = (license: LicenseStatus | null | undefined): boolean => !!license && license.state === 'valid' && license.plan === 'monthly_managed';

/** Whether the license policy blocks AI features right now (only ever true under hard enforcement). */
export const licenseBlocksAi = (license: LicenseStatus | null | undefined): boolean => !!license && license.enforcement === 'hard' && license.state !== 'valid';

/** Which vendor a model id belongs to, judged by its family prefix (null = unknown/custom). */
export function vendorOfModel(id: string): AiProvider | null {
  const m = id.trim().toLowerCase();
  if (m.startsWith('claude')) return 'anthropic';
  if (m.startsWith('gpt-') || m.startsWith('chatgpt') || /^o\d/.test(m)) return 'openai';
  if (m.startsWith('grok')) return 'xai';
  if (m.startsWith('ubi-')) return 'ubi';
  return null;
}

export const sameModels = (a: AiModels, b: AiModels): boolean => a.classify === b.classify && a.vision === b.vision && a.report === b.report;

/**
 * Model ids to use after switching to `provider`: ids that belong to another vendor are
 * replaced by that provider's defaults; ids of the same vendor (or unknown ones) are kept.
 * Mirrors `AiModels::reconciled_with` in ubiqx-core.
 */
export function reconcileModels(models: AiModels, provider: ProviderInfo): AiModels {
  // The managed provider has no custom ids: its aliases are fixed.
  if (isManagedProvider(provider.id)) return { ...provider.default_models };
  const pick = (slot: keyof AiModels): string => {
    const current = models[slot];
    const vendor = vendorOfModel(current);
    return !current.trim() || (vendor !== null && vendor !== provider.id) ? provider.default_models[slot] : current;
  };
  return { classify: pick('classify'), vision: pick('vision'), report: pick('report') };
}

export const providerInfo = (view: SettingsView, id: AiProvider): ProviderInfo | undefined => view.providers.find((p) => p.id === id);

/**
 * The provider's display name. Vendor labels are brand names and come from the backend as they are; the managed
 * provider's label follows the UI language (`settings.provider.ubi.short`: "IA do Ubi" / "Ubi AI").
 */
export const providerLabel = (info: Pick<ProviderInfo, 'id' | 'label'>, tr: Translate = t): string => (isManagedProvider(info.id) ? tr('settings.provider.ubi.short') : info.label);

export const keyStatus = (view: SettingsView, id: AiProvider): ApiKeyStatus => view.api_keys.find((k) => k.provider === id) ?? { provider: id, configured: false, hint: null };

/** Marketing-free one-liners shown on the provider cards. Copy lives in i18n ('settings.provider.<id>.*'); costs are rough estimates for ~8 h/day. */
export interface ProviderPitch {
  /** Short product name used in cards and summaries ("Claude", "OpenAI", "Grok"). */
  short: string;
  pitch: string;
  cost: string;
  /** Extra factual note shown under the card, when any. */
  note?: string;
}

type Translate = (key: string) => string;

/**
 * The pitch of one provider in the current language. Inside a component pass the `t` from `useT()` so the
 * component re-renders when the language changes: `providerPitch(id, t)`.
 */
export function providerPitch(id: AiProvider, tr: Translate = t): ProviderPitch {
  const key = (field: string) => `settings.provider.${id}.${field}`;
  const pitch: ProviderPitch = { short: tr(key('short')), pitch: tr(key('pitch')), cost: tr(key('cost')) };
  if (hasKey(key('note'))) pitch.note = tr(key('note'));
  return pitch;
}

const PITCH_FIELDS = ['short', 'pitch', 'cost', 'note'] as const;

/** A pitch whose fields are read from i18n on every access, so `PROVIDER_PITCH[id].short` follows the locale. */
const livePitch = (id: AiProvider): ProviderPitch => {
  const o = {} as ProviderPitch;
  for (const field of PITCH_FIELDS) Object.defineProperty(o, field, { get: () => providerPitch(id)[field], enumerable: true });
  return o;
};

/**
 * Live (locale-following) pitches keyed by provider. Reading a field returns the text for the locale at that
 * moment; a component that only reads this map will not re-render on a language switch unless it also calls
 * `useT()`/`useLocale()`. Prefer `providerPitch(id, t)` in components.
 */
export const PROVIDER_PITCH: Record<AiProvider, ProviderPitch> = {
  anthropic: livePitch('anthropic'),
  openai: livePitch('openai'),
  xai: livePitch('xai'),
  ubi: livePitch('ubi'),
};
