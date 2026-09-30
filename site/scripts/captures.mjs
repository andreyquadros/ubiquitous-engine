#!/usr/bin/env node
// QA captures of the landing page: 1440x900 and 390x844, both languages, full page, plus a console-error report.
//
//   pnpm captures [--url http://localhost:4173] [--out ../../../tmp/site-captures]
import { mkdirSync } from 'node:fs';
import sharp from 'sharp';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { argParser, launch } from './pw.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const arg = argParser(process.argv.slice(2));
const base = arg('--url', 'http://localhost:4173').replace(/\/$/, '');
const out = resolve(here, arg('--out', '../captures'));
mkdirSync(out, { recursive: true });

const browser = await launch();
let errors = 0;
try {
  for (const [w, h, tag] of [[1440, 900, 'desktop'], [390, 844, 'phone']]) {
    for (const lang of ['pt-BR', 'en']) {
      const context = await browser.newContext({ viewport: { width: w, height: h }, deviceScaleFactor: 1, colorScheme: 'dark', locale: lang === 'en' ? 'en-US' : 'pt-BR' });
      const page = await context.newPage();
      page.on('console', (m) => {
        if (m.type() === 'error') {
          errors += 1;
          console.error(`[${tag} ${lang}] console error: ${m.text()}`);
        }
      });
      page.on('pageerror', (err) => {
        errors += 1;
        console.error(`[${tag} ${lang}] page error: ${err.message}`);
      });
      await page.goto(`${base}/?lang=${lang}`, { waitUntil: 'networkidle' });
      await page.waitForSelector('[data-testid="ubi-hero"][data-ready="1"]', { timeout: 30000 }).catch(() => console.warn(`[${tag} ${lang}] 3D mascot not ready, capturing the PNG fallback`));
      await page.evaluate(() => document.fonts.ready);
      // smooth scrolling breaks Playwright's full-page stitching
      await page.addStyleTag({ content: 'html { scroll-behavior: auto !important; }' });
      // scroll through so lazy images load, then back to the top
      await page.evaluate(async () => {
        for (let y = 0; y < document.body.scrollHeight; y += 600) {
          window.scrollTo(0, y);
          await new Promise((r) => setTimeout(r, 60));
        }
        window.scrollTo(0, 0);
      });
      // force the lazy gallery images and wait for them (at most 8 s)
      await page.evaluate(
        () =>
          new Promise((done) => {
            const imgs = [...document.images];
            for (const i of imgs) i.loading = 'eager';
            const t = setTimeout(done, 8000);
            Promise.all(
              imgs.map((i) => (i.complete ? null : new Promise((r) => { i.addEventListener('load', r, { once: true }); i.addEventListener('error', r, { once: true }); }))),
            ).then(() => { clearTimeout(t); done(); });
          }),
      );
      await page.waitForTimeout(800);
      const scrollW = await page.evaluate(() => document.documentElement.scrollWidth);
      if (scrollW > w) {
        errors += 1;
        console.error(`[${tag} ${lang}] horizontal overflow: scrollWidth ${scrollW} > ${w}`);
      }
      const file = resolve(out, `${tag}-${lang}.png`);
      // software GL caps a single capture near 8192 px, so tall pages are captured in clips and stitched
      const total = await page.evaluate(() => document.documentElement.scrollHeight);
      const STEP = 4000;
      if (total <= 7000) {
        await page.screenshot({ path: file, fullPage: true });
      } else {
        const parts = [];
        for (let y = 0; y < total; y += STEP) {
          const height = Math.min(STEP, total - y);
          parts.push({ input: await page.screenshot({ fullPage: true, clip: { x: 0, y, width: w, height } }), top: y, left: 0 });
        }
        await sharp({ create: { width: w, height: total, channels: 3, background: '#060a14' } }).composite(parts).png().toFile(file);
      }
      console.log(`${file} (${w}x${total})`);
      await context.close();
    }
  }
} finally {
  await browser.close();
}
if (errors) {
  console.error(`${errors} problem(s) found`);
  process.exit(1);
}
