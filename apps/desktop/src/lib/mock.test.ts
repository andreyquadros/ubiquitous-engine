import { describe, expect, it, beforeEach } from 'vitest';
import { __mock, handle } from './mock';
import type { CorrectionOutcome, DashboardData, BlockGroup, ApiKeyResult, DailyReport } from './types';

const today = new Date().toISOString().slice(0, 10);

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

  it('validates api keys', async () => {
    expect((await handle<ApiKeyResult>('set_api_key', { key: 'sk-ant-api03-abcdefghijklmnop' })).valid).toBe(true);
    expect((await handle<ApiKeyResult>('set_api_key', { key: 'nope' })).valid).toBe(false);
  });

  it('generates reports', async () => {
    const r = await handle<DailyReport>('generate_report', { date: today, categoryId: 'cat-cidades' });
    expect(r.items.length).toBeGreaterThan(0);
    const list = await handle<DailyReport[]>('get_reports', { date: today });
    expect(list.some((x) => x.category_id === 'cat-cidades')).toBe(true);
  });
});
