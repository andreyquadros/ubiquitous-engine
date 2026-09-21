import { afterEach, describe, expect, it, beforeEach, vi } from 'vitest';
import { __mock, handle } from './mock';
import type { CorrectionOutcome, DashboardData, BlockGroup, ApiKeyResult, DailyReport, LicenseStatus, Settings, SettingsView } from './types';

const today = new Date().toISOString().slice(0, 10);

const view = () => handle<SettingsView>('get_settings', {});
const update = (patch: Partial<Settings>) => view().then((v) => handle<SettingsView>('update_settings', { settings: { ...v.settings, ...patch } }));

describe('mock backend', () => {
  beforeEach(() => __mock.reset());

  it('builds a realistic day', async () => {
    const d = await handle<DashboardData>('get_dashboard', { date: today });
    expect(d.timeline.length).toBeGreaterThanOrEqual(38);
    expect(d.stats.focus_score).toBeGreaterThan(0);
    expect(d.hourly_focus).toHaveLength(24);
    expect(d.needs_review).toBeGreaterThan(0);
    expect(d.unseen_nudges).toHaveLength(1);
    expect(d.usage_month.cost_usd).toBeCloseTo(1.37);
  });

  it('reclassifies a group and mutates state', async () => {
    const groups = await handle<BlockGroup[]>('get_review_groups', { date: today });
    const g = groups.find((x) => x.needs_review)!;
    const out = await handle<CorrectionOutcome>('reclassify_group', { date: today, key: g.key, categoryId: 'cat-ifro' });
    expect(out.block_ids.length).toBe(g.block_ids.length);
    const after = await handle<BlockGroup[]>('get_review_groups', { date: today });
    expect(after.find((x) => x.key === g.key)?.category_id).toBe('cat-ifro');
  });

  it('generates reports', async () => {
    const r = await handle<DailyReport>('generate_report', { date: today, categoryId: 'cat-cidades' });
    expect(r.items.length).toBeGreaterThan(0);
    const list = await handle<DailyReport[]>('get_reports', { date: today });
    expect(list.some((x) => x.category_id === 'cat-cidades')).toBe(true);
  });
});

describe('mock backend · AI providers', () => {
  beforeEach(() => __mock.reset());

  it('serves the provider contract: anthropic selected and configured, the others not', async () => {
    const v = await view();
    expect(v.settings.ai_provider).toBe('anthropic');
    expect(v.settings.models).toEqual({ classify: 'claude-haiku-4-5', vision: 'claude-haiku-4-5', report: 'claude-sonnet-5' });
    expect(v.api_key_configured).toBe(true);
    expect(v.api_key_hint).toBe('…f3a9');
    expect(v.api_keys).toEqual([
      { provider: 'anthropic', configured: true, hint: '…f3a9' },
      { provider: 'openai', configured: false, hint: null },
      { provider: 'xai', configured: false, hint: null },
      { provider: 'ubi', configured: false, hint: null },
    ]);
    expect(v.providers.map((p) => [p.id, p.label, p.key_prefix])).toEqual([
      ['anthropic', 'Anthropic Claude', 'sk-ant-'],
      ['openai', 'OpenAI', 'sk-'],
      ['xai', 'xAI Grok', 'xai-'],
      ['ubi', 'IA do Ubi', 'UBIQX-'],
    ]);
    expect(v.providers.map((p) => p.console_url)).toEqual(['https://console.anthropic.com/settings/keys', 'https://platform.openai.com/api-keys', 'https://console.x.ai', 'https://andreyquadros.github.io/ubiquitous-engine/#planos']);
    expect(v.providers[3]?.default_models).toEqual({ classify: 'ubi-fast', vision: 'ubi-fast', report: 'ubi-smart' });
    expect(v.license).toEqual({ state: 'unlicensed', plan: null, expires_at: null, days_left: null, key_hint: null, enforcement: 'soft', managed_usage: null });
    expect(v.ubi_api_base).toBe('https://api.ubiqx.ai');
    expect(v.providers[1]?.default_models).toEqual({ classify: 'gpt-5-mini', vision: 'gpt-5-mini', report: 'gpt-5' });
    expect(v.providers[2]?.default_models).toEqual({ classify: 'grok-4-1-fast-non-reasoning', vision: 'grok-4-1-fast-non-reasoning', report: 'grok-4-1-fast-reasoning' });
    expect(v.ai_health).toEqual({ state: 'ok' });
  });

  it('validates api keys per provider', async () => {
    const set = (provider: string, key: string | null) => handle<ApiKeyResult>('set_api_key', { provider, key });
    expect((await set('anthropic', 'sk-ant-api03-abcdefghijklmnop')).valid).toBe(true);
    expect((await set('anthropic', 'sk-proj-abcdefghijklmnopqrstuvwxyz')).valid).toBe(false);

    expect((await set('openai', 'sk-proj-abcdefghijklmnopqrstuvwxyz')).valid).toBe(true);
    expect((await set('openai', 'sk-abcdefghijklmnopqrstuvwxyz')).valid).toBe(true);
    expect((await set('openai', 'sk-ant-api03-abcdefghijklmnop')).valid).toBe(false);
    expect((await set('openai', 'sk-short')).valid).toBe(false);

    const xai = await set('xai', 'xai-abcdefghijklmnopqrstuvwxyz1234');
    expect(xai.valid).toBe(true);
    expect(xai.message).toContain('xAI Grok');
    const bad = await set('xai', 'sk-abcdefghijklmnopqrstuvwxyz');
    expect(bad.valid).toBe(false);
    expect(bad.message).toContain('xAI Grok');

    const v = await view();
    expect(v.api_keys.find((k) => k.provider === 'xai')).toEqual({ provider: 'xai', configured: true, hint: '…1234' });
    expect(v.api_keys.find((k) => k.provider === 'openai')?.configured).toBe(true);
    // the selected provider is still anthropic, so the flat fields describe its key
    expect(v.api_key_hint).toBe('…mnop');

    expect((await set('anthropic', null)).valid).toBe(true);
    const after = await view();
    expect(after.api_key_configured).toBe(false);
    expect(after.ai_health).toEqual({ state: 'not_configured' });
  });

  it('switching provider reconciles the models and recomputes health', async () => {
    const openai = await update({ ai_provider: 'openai' });
    expect(openai.settings.ai_provider).toBe('openai');
    expect(openai.settings.models).toEqual({ classify: 'gpt-5-mini', vision: 'gpt-5-mini', report: 'gpt-5' });
    expect(openai.api_key_configured).toBe(false);
    expect(openai.api_key_hint).toBeNull();
    expect(openai.ai_health).toEqual({ state: 'not_configured' });

    // a key for the new provider brings the AI back
    await handle<ApiKeyResult>('set_api_key', { provider: 'openai', key: 'sk-proj-abcdefghijklmnopqrstuvwxyz' });
    expect((await view()).ai_health).toEqual({ state: 'ok' });

    // custom ids of the same vendor survive; foreign ones are replaced slot by slot
    const custom = await update({ models: { classify: 'gpt-5-nano', vision: 'gpt-5-mini', report: 'gpt-5.4' } });
    expect(custom.settings.models.classify).toBe('gpt-5-nano');
    const xai = await update({ ai_provider: 'xai', models: { classify: 'grok-3-mini', vision: 'gpt-5-mini', report: 'gpt-5.4' } });
    expect(xai.settings.models).toEqual({ classify: 'grok-3-mini', vision: 'grok-4-1-fast-non-reasoning', report: 'grok-4-1-fast-reasoning' });
    expect(xai.ai_health).toEqual({ state: 'not_configured' });

    // back to anthropic, whose key is still stored
    const back = await update({ ai_provider: 'anthropic' });
    expect(back.settings.models).toEqual({ classify: 'claude-haiku-4-5', vision: 'claude-haiku-4-5', report: 'claude-sonnet-5' });
    expect(back.api_key_hint).toBe('…f3a9');
    expect(back.ai_health).toEqual({ state: 'ok' });
  });

  it('lists the account models only when that provider has a key', async () => {
    const claude = await handle<string[]>('list_models', { provider: 'anthropic' });
    expect(claude).toEqual(['claude-haiku-4-5', 'claude-opus-5', 'claude-sonnet-5']);
    await expect(handle<string[]>('list_models', { provider: 'openai' })).rejects.toThrow(/chave não configurada/);
    await handle<ApiKeyResult>('set_api_key', { provider: 'openai', key: 'sk-proj-abcdefghijklmnopqrstuvwxyz' });
    const gpt = await handle<string[]>('list_models', { provider: 'openai' });
    expect(gpt).toContain('gpt-5-mini');
    expect(gpt).toEqual([...gpt].sort((a, b) => a.localeCompare(b)));
    await expect(handle<string[]>('list_models', { provider: 'xai' })).rejects.toThrow(/xAI Grok/);
  });

  it('usage strings name the active provider model', async () => {
    await update({ ai_provider: 'xai' });
    await handle<ApiKeyResult>('set_api_key', { provider: 'xai', key: 'xai-abcdefghijklmnopqrstuvwxyz1234' });
    const md = await handle<string>('get_monthly_report', { categoryId: 'cat-ifro', year: 2026, month: 9 });
    expect(md).toContain('xAI Grok · grok-4-1-fast-reasoning');
    const report = await handle<DailyReport>('generate_report', { date: today, categoryId: 'cat-cidades' });
    expect(report.model).toBe('grok-4-1-fast-reasoning');
  });
});

describe('mock backend · license', () => {
  beforeEach(() => __mock.reset());

  const status = () => handle<LicenseStatus>('get_license_status', {});
  const setKey = (key: string | null) => handle<LicenseStatus>('set_license_key', { key });

  it('round-trips the sample keys: annual, managed (with usage), expired, invalid, removed', async () => {
    expect((await status()).state).toBe('unlicensed');

    const annual = await setKey(__mock.sampleLicenseKeys.annual);
    expect(annual.state).toBe('valid');
    expect(annual.plan).toBe('annual_own_key');
    expect(annual.days_left).toBeGreaterThanOrEqual(334);
    expect(annual.key_hint).toBe(__mock.sampleLicenseKeys.annual.slice(-4));
    expect(annual.managed_usage).toBeNull();
    expect(annual.enforcement).toBe('soft');

    const managed = await setKey(__mock.sampleLicenseKeys.managed);
    expect(managed.state).toBe('valid');
    expect(managed.plan).toBe('monthly_managed');
    expect(managed.managed_usage).toEqual({ month: expect.stringMatching(/^\d{4}-\d{2}$/), spent_usd: 2.35, budget_usd: 6 });
    // the license also shows up as the managed provider's "key"
    const v = await view();
    expect(v.license.state).toBe('valid');
    expect(v.api_keys.find((k) => k.provider === 'ubi')).toEqual({ provider: 'ubi', configured: true, hint: `…${__mock.sampleLicenseKeys.managed.slice(-4)}` });

    const expired = await setKey(__mock.sampleLicenseKeys.expired);
    expect(expired.state).toBe('expired');
    expect(expired.plan).toBe('annual_own_key');
    expect(expired.days_left).toBe(0);

    const invalid = await setKey('UBIQX-NOTAKEY-NOPE');
    expect(invalid.state).toBe('invalid');
    expect(invalid.plan).toBeNull();
    expect(invalid.key_hint).toBe('NOPE');

    expect((await setKey(null)).state).toBe('unlicensed');
  });

  it('gates the managed provider on a valid monthly license and falls back when it goes away', async () => {
    await expect(update({ ai_provider: 'ubi' })).rejects.toThrow(/^license_required:/);
    __mock.setLicense('annual');
    await expect(update({ ai_provider: 'ubi' })).rejects.toThrow(/^license_required:/);

    __mock.setLicense('managed');
    const v = await update({ ai_provider: 'ubi', models: { classify: 'claude-haiku-4-5', vision: 'x', report: 'y' } });
    expect(v.settings.ai_provider).toBe('ubi');
    expect(v.settings.models).toEqual({ classify: 'ubi-fast', vision: 'ubi-fast', report: 'ubi-smart' });
    expect(v.ai_health).toEqual({ state: 'ok' });
    expect(await handle<string[]>('list_models', { provider: 'ubi' })).toEqual(['ubi-fast', 'ubi-smart']);
    await expect(handle<ApiKeyResult>('set_api_key', { provider: 'ubi', key: 'UBIQX-x' })).rejects.toThrow(/^invalid:/);

    // proxy offline: the local verdict stays, the usage is unknown
    expect(__mock.setLicense('managed', { proxyReachable: false }).managed_usage).toBeNull();
    expect((await view()).license.state).toBe('valid');

    // removing the key keeps the managed provider selected but not configured (like the engine)
    const after = await setKey(null);
    expect(after.state).toBe('unlicensed');
    expect((await view()).settings.ai_provider).toBe('ubi');
    expect((await view()).ai_health).toEqual({ state: 'not_configured' });
  });
});

describe('mock backend · the focus seed stays inside the current day', () => {
  afterEach(() => vi.useRealTimers());

  // The Focus page shows "4 hoje", counted by local date. Seeding "190 minutes ago" put the
  // oldest intervention on the previous day whenever the app was opened before 03:10.
  it.each(['2026-09-19T00:04:00', '2026-09-19T02:30:00', '2026-09-19T13:00:00'])('at %s', async (local) => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(local));
    __mock.reset();
    const today = new Date(local).toLocaleDateString('en-CA');
    const interventions = await handle<{ at: string }[]>('list_interventions', {});
    expect(interventions).toHaveLength(4);
    expect(interventions.map((i) => new Date(i.at).toLocaleDateString('en-CA'))).toEqual([today, today, today, today]);
    const status = await handle<{ interventions_today: number }>('get_focus_status', {});
    expect(status.interventions_today).toBe(4);
  });
});
