// Provider-related helpers shared by the pages and the mock backend. Everything that is a
// *fact* about a provider (label, console URL, key prefix, default models) comes from the
// backend as `ProviderInfo`; this file only holds UI copy and pure model-id arithmetic.

import type { AiModels, AiProvider, ApiKeyStatus, ProviderInfo, SettingsView } from './types';

export const PROVIDER_IDS: AiProvider[] = ['anthropic', 'openai', 'xai'];

/** Which vendor a model id belongs to, judged by its family prefix (null = unknown/custom). */
export function vendorOfModel(id: string): AiProvider | null {
  const m = id.trim().toLowerCase();
  if (m.startsWith('claude')) return 'anthropic';
  if (m.startsWith('gpt-') || m.startsWith('chatgpt') || /^o\d/.test(m)) return 'openai';
  if (m.startsWith('grok')) return 'xai';
  return null;
}

export const sameModels = (a: AiModels, b: AiModels): boolean => a.classify === b.classify && a.vision === b.vision && a.report === b.report;

/**
 * Model ids to use after switching to `provider`: ids that belong to another vendor are
 * replaced by that provider's defaults; ids of the same vendor (or unknown ones) are kept.
 * Mirrors `AiModels::reconciled_with` in ubiqx-core.
 */
export function reconcileModels(models: AiModels, provider: ProviderInfo): AiModels {
  const pick = (slot: keyof AiModels): string => {
    const current = models[slot];
    const vendor = vendorOfModel(current);
    return !current.trim() || (vendor !== null && vendor !== provider.id) ? provider.default_models[slot] : current;
  };
  return { classify: pick('classify'), vision: pick('vision'), report: pick('report') };
}

export const providerInfo = (view: SettingsView, id: AiProvider): ProviderInfo | undefined => view.providers.find((p) => p.id === id);

export const keyStatus = (view: SettingsView, id: AiProvider): ApiKeyStatus => view.api_keys.find((k) => k.provider === id) ?? { provider: id, configured: false, hint: null };

/** Marketing-free one-liners shown on the provider cards (pt-BR). Costs are rough estimates for ~8 h/day. */
export interface ProviderPitch {
  /** Short product name used in cards and summaries ("Claude", "OpenAI", "Grok"). */
  short: string;
  pitch: string;
  cost: string;
  /** Extra factual note shown under the card, when any. */
  note?: string;
}

export const PROVIDER_PITCH: Record<AiProvider, ProviderPitch> = {
  anthropic: {
    short: 'Claude',
    pitch: 'Melhor leitura de contexto em português e relatórios com a sua voz.',
    cost: 'US$ 3–6/mês',
  },
  openai: {
    short: 'OpenAI',
    pitch: 'Modelos GPT-5 rápidos e baratos, com visão e saída estruturada.',
    cost: 'US$ 1–3/mês',
    note: 'Entrar com ChatGPT (OAuth) hoje só identifica o usuário; para usar os modelos em apps de terceiros a OpenAI exige chave de API.',
  },
  xai: {
    short: 'Grok',
    pitch: 'Grok 4.1 Fast: o mais barato dos três, com visão e contexto longo.',
    cost: 'US$ 0,50–2/mês',
  },
};
