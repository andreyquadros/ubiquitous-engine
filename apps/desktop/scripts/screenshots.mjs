#!/usr/bin/env node
// Captures the README screenshots in mock mode (no Tauri): light and dark, 1440x900.
//
//   pnpm dev --port 1420   (in another terminal)
//   node scripts/screenshots.mjs [--url http://localhost:1420] [--out ../../docs/screenshots]
//
// Uses the Playwright Chromium from PLAYWRIGHT_BROWSERS_PATH (falls back to a fixed executablePath).

import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const arg = (name, def) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : def;
};
const base = arg('--url', process.env.UBIQX_URL ?? 'http://localhost:1420');
const out = resolve(here, arg('--out', '../../../docs/screenshots'));
const themes = (arg('--themes', 'light,dark')).split(',');

const PAGES = [
  ['dashboard', '/', 'page-dashboard'],
  ['timeline', '/timeline', 'page-timeline'],
  ['review', '/review', 'page-review'],
  ['reports', '/reports', 'page-reports'],
  ['categories', '/categories', 'page-categories'],
  ['insights', '/insights', 'page-insights'],
  ['settings', '/settings', 'page-settings'],
  ['onboarding', '/onboarding', 'page-onboarding'],
];

async function launch() {
  try {
    return await chromium.launch();
  } catch (e) {
    console.warn('default chromium launch failed, retrying with executablePath:', e.message.split('\n')[0]);
    return chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? '/opt/pw-browsers/chromium' });
  }
}

mkdirSync(out, { recursive: true });
const browser = await launch();
try {
  for (const theme of themes) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, colorScheme: theme, reducedMotion: 'reduce', locale: 'pt-BR' });
    const page = await context.newPage();
    page.on('pageerror', (err) => console.error(`[${theme}] page error:`, err.message));
    for (const [name, route, testId] of PAGES) {
      // the onboarding capture lands on step 2 (provider chooser); `?step=` is optional, step 1 is the default
      const query = `?theme=${theme}${name === 'onboarding' ? '&onboarding=1&step=2' : ''}`;
      await page.goto(`${base}/${query}#${route}`, { waitUntil: 'networkidle' });
      await page.waitForSelector(`[data-testid="${testId}"]`, { timeout: 15000 });
      // let data, charts and the UBI probe settle
      await page.waitForSelector('[data-testid="ubi-svg"], [data-testid="ubi-3d"]', { timeout: 15000 }).catch(() => {});
      if (name === 'insights' || name === 'dashboard') await page.waitForSelector('.recharts-surface', { timeout: 15000 }).catch(() => {});
      if (name === 'insights') {
        await page.getByRole('button', { name: /^Gerar$/ }).click().catch(() => {});
        await page.waitForTimeout(1800);
      }
      if (name === 'review') {
        await page.locator('[data-testid="review-row"]').first().click().catch(() => {});
      }
      await page.waitForTimeout(700);
      const file = resolve(out, `${name}${theme === 'dark' ? '-dark' : ''}.png`);
      await page.screenshot({ path: file, fullPage: false });
      console.log(`✓ ${file}`);
    }
    await context.close();
  }
} finally {
  await browser.close();
}
