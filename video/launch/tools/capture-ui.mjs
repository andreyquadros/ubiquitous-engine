#!/usr/bin/env node
// Captures the real ubiqX desktop UI (mock mode, no Tauri) at 2x for the launch video, with DOM hotspots.
//
//   (cd apps/desktop && npx vite --port 1420 --strictPort)      # in another terminal
//   node video/launch/tools/capture-ui.mjs [name ...]            # no names = every capture
//
// Output: video/launch/public/ui/<name>.png + ui-manifest.json ([{file, route, theme, width, height, dpr,
// description_pt, description_en, hotspots:[{name, x, y, w, h}]}]). Hotspots are in image pixels
// (CSS rect x DPR, clipped to the viewport). A partial run (names given) merges into the existing manifest.
//
// Hi-res twins (for camera zooms past bitmap scale 1.0):
//
//   node video/launch/tools/capture-ui.mjs --hires review-queue-selected settings-ai ...
//
// Names are PNG stems (a capture name such as `review` takes all of its files). Same viewport, clock, license,
// theme and state; only the device scale factor goes x1.5 (DPR 2 -> 3), so the layout is the 2x layout and
// the file is public/ui/<stem>@3x.png. The 2x PNGs and ui-manifest.json are NOT written: hotspots stay in the
// 2x space (multiply by `scale` for the twin). Before a twin is written the privacy check below runs as for
// any capture, and its hotspots (measured at DPR 3, divided by 1.5) must match the 2x manifest within
// HIRES_TOLERANCE px, else nothing is written. Twins merge into public/ui/hires.json:
// [{file, hires, scale, width, height}] (width/height = the twin's own pixel size = 2x size x scale).
//
// State: dark theme, pt-BR, a valid "ubiqX Mensal" license (?license=managed, so no license banner and the
// "IA do Ubi" provider is selectable), clock fixed at Tue 2026-09-29 18:40 America/Sao_Paulo so the mock
// day (08:00-17:55) reads as a finished work day.
//
// Privacy: the desktop mock (apps/desktop/src/lib/mock.ts) seeds the product owner's real data: his Gmail
// address in the inbox window titles, his user name in the data/export folders, his GitHub handle in the
// site/repo/release URLs, plus a realistic SEI process number. The file on disk is product code and stays
// untouched: every /src/ module Vite serves is rewritten on the fly (context.route) so the mock is born
// with fictional equivalents (see SCRUB). Before each screenshot the DOM (text, HTML, form values, inline
// SVG "screenshots") and the mock state are scanned for e-mail addresses and the owner's name; any hit
// aborts that capture (no PNG is written) and the run exits non-zero.

import pw from '/home/user/ubiquitous-engine/site/node_modules/playwright/index.js';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const { chromium } = pw;
const BASE = process.env.UI_URL ?? 'http://localhost:1420';
const OUT = fileURLToPath(new URL('../public/ui/', import.meta.url));
const EXEC = process.env.CHROMIUM_PATH ?? '/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell';
const NOW = new Date('2026-09-29T21:40:00Z'); // 18:40 in São Paulo
const ARGS = process.argv.slice(2);
/** --hires: shoot twins at 1.5x the capture's DPR into <stem>@<dpr>x.png (see the header). */
const HIRES = ARGS.includes('--hires');
const HIRES_FACTOR = HIRES ? 1.5 : 1;
/** Max |2x hotspot - hi-res hotspot / factor| (2x image px) for a twin to count as the same layout. */
const HIRES_TOLERANCE = 2;
const ONLY = ARGS.filter((a) => !a.startsWith('--')).map((a) => a.replace(/\.png$/, ''));
const unknownFlags = ARGS.filter((a) => a.startsWith('--') && a !== '--hires');
if (unknownFlags.length) throw new Error(`unknown option(s): ${unknownFlags.join(', ')}`);
if (HIRES && !ONLY.length) throw new Error('--hires needs the names of the captures to re-shoot');

/* ------------------------------------------------------------------ */
/* privacy: fictional stand-ins for the owner's data, and the scan      */
/* ------------------------------------------------------------------ */

const FICTIONAL = {
  email: 'ana.souza@example.com', // RFC 2606 reserved domain: cannot belong to anyone
  user: 'ana', // home-folder user name
  githubOwner: 'example',
  site: 'https://ubiqx.com.br/', // the brand site (brief/facts.md)
  seiProcess: '12345.000042/2026-00', // same shape as a real NUP, clearly not one
};
const FICTIONAL_EMAILS = new Set([FICTIONAL.email]);
/** The owner's name as it appears in the mock (e-mail local part, user name, GitHub handle). */
const OWNER_RE = /andrey|quadros/gi;
const EMAIL_RE = /[A-Za-z0-9._%+-]+@(?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,}/g;
/** `ubi@2x.png` and friends look like e-mail addresses to EMAIL_RE; they are asset names. */
const ASSET_TLD = /\.(?:png|jpe?g|webp|avif|gif|svg|glb|gltf|hdr|m?js|tsx?|css|json|woff2?|ttf|otf|wasm)$/i;

/** Applied, in order, to the text of every JS/TS module Vite serves from /src/ (the mock, providers.ts, Settings.tsx). */
const SCRUB = [
  // Gmail window titles: "Caixa de entrada (12) - <owner address> - Gmail"
  [EMAIL_RE, (m) => (ASSET_TLD.test(m) || FICTIONAL_EMAILS.has(m.toLowerCase()) ? m : FICTIONAL.email)],
  // data_dir per OS and the export path: /Users/<u>/…, /home/<u>/…, C:\\Users\\<u>\\…
  [/(Users|home)((?:\\{1,4}|\/))andrey\b/g, `$1$2${FICTIONAL.user}`],
  // SITE_URL (providers.ts), REPO_URL (Settings.tsx), RELEASE_BASE (mock.ts)
  [/https:\/\/andreyquadros\.github\.io\/ubiquitous-engine\/?/g, FICTIONAL.site],
  [/github\.com\/andreyquadros\/ubiquitous-engine/g, `github.com/${FICTIONAL.githubOwner}/ubiqx`],
  // the SEI process in the timeline title and the IFRO report
  [/23243\.001234\/2026-11/g, FICTIONAL.seiProcess],
];
const SCRUB_NEEDLE = /@|andrey|quadros|23243\.001234/i;
/** module path → replacements made (logged once at the end). */
const scrubbed = new Map();

function scrubSource(text) {
  let count = 0;
  for (const [re, to] of SCRUB) {
    text = text.replace(re, (...args) => {
      const out = typeof to === 'function' ? to(args[0]) : args[0].replace(new RegExp(re.source, re.flags.replace('g', '')), to);
      if (out !== args[0]) count += 1;
      return out;
    });
  }
  return { text, count };
}

const SRC_MODULE = /^\/src\/.+\.(?:[cm]?[jt]sx?)$/;

/** Rewrites the served source of /src/ modules so the mock never holds the owner's data, not even for the first render. */
async function installScrub(context) {
  const origin = new URL(BASE).origin;
  await context.route(
    (url) => url.origin === origin && SRC_MODULE.test(url.pathname),
    async (route) => {
      // Never throw from here (an unhandled rejection kills the whole run); a module that cannot be fetched
      // is aborted, so the page fails loudly instead of rendering unscrubbed source.
      let response;
      let body;
      for (let attempt = 1; !response; attempt++) {
        try {
          response = await route.fetch();
          body = await response.text();
        } catch (e) {
          response = undefined;
          if (attempt >= 4) {
            console.warn(`  ! ${route.request().url()}: ${e.message.split('\n')[0]} (gave up)`);
            return route.abort().catch(() => {});
          }
          await new Promise((r) => setTimeout(r, 250 * attempt));
        }
      }
      if (!SCRUB_NEEDLE.test(body)) return route.fulfill({ response, body }).catch(() => {});
      const scrubResult = scrubSource(body);
      const { count } = scrubResult;
      // the inline source map embeds the original file (base64): drop it from the rewritten modules
      const text = count ? scrubResult.text.replace(/\n\/\/# sourceMappingURL=data:[^\n]*/g, '') : scrubResult.text;
      const path = new URL(route.request().url()).pathname;
      if (count) scrubbed.set(path, count);
      const left = [...new Set([...text.matchAll(OWNER_RE)].map((m) => m[0]))];
      if (left.length) console.warn(`  ! ${path} still mentions ${left.join(', ')} after the scrub`);
      return route.fulfill({ response, body: text }).catch(() => {});
    },
  );
}

/** Everything a viewer (or a zoom) could read, plus the whole HTML and the mock state. Runs in the page. */
async function privacyCorpusInPage() {
  const html = document.documentElement.outerHTML;
  const parts = [document.title, document.body?.innerText ?? '', html];
  for (const el of document.querySelectorAll('input, textarea, select')) parts.push(el.value ?? '');
  // placeholder "screenshots" are SVGs inlined as base64 data URIs
  for (const m of html.matchAll(/data:image\/svg\+xml;base64,([A-Za-z0-9+/=]+)/g)) {
    try {
      parts.push(new TextDecoder().decode(Uint8Array.from(atob(m[1]), (c) => c.charCodeAt(0))));
    } catch {
      /* not decodable: the HTML scan still covers it */
    }
  }
  let state = '';
  try {
    const mock = await import('/src/lib/mock.ts');
    state = JSON.stringify(mock.__mock.state(), (_k, v) => (v instanceof Map ? Object.fromEntries(v) : v));
  } catch (e) {
    state = `!mock state unavailable: ${e}`;
  }
  return { dom: parts.join('\n'), state };
}

/** E-mail addresses that are not the fictional ones, and any trace of the owner's name. */
function privacyHits(text) {
  const hits = new Set();
  for (const [m] of text.matchAll(EMAIL_RE)) if (!ASSET_TLD.test(m) && !FICTIONAL_EMAILS.has(m.toLowerCase())) hits.add(`e-mail "${m}"`);
  for (const m of text.matchAll(OWNER_RE)) hits.add(`name "${text.slice(Math.max(0, m.index - 30), m.index + 40).replace(/\s+/g, ' ')}"`);
  return [...hits];
}

const privacyLog = [];

async function privacyCheck(page, file) {
  const { dom, state } = await page.evaluate(privacyCorpusInPage);
  const hits = [...privacyHits(dom).map((h) => `DOM ${h}`), ...privacyHits(state).map((h) => `state ${h}`)];
  if (state.startsWith('!')) hits.push(state.slice(1));
  const fictional = [...new Set([...dom.matchAll(EMAIL_RE)].map(([m]) => m).filter((m) => FICTIONAL_EMAILS.has(m.toLowerCase())))];
  privacyLog.push({ file, hits, fictional });
  if (hits.length) throw new Error(`privacy: ${file} shows real personal data, not saved: ${hits.join('; ')}`);
  return fictional;
}

/* ------------------------------------------------------------------ */
/* hotspot specs                                                       */
/* ------------------------------------------------------------------ */
// {name, css, text?: regex on normalized textContent, leaf?: keep innermost matches, nth?, up?: closest(), pad?}

const nav = (name, href) => ({ name, css: `aside nav a[href="#${href}"]` });
const cardByTitle = (name, title) => ({ name, css: 'main h2', text: `^${title}`, up: 'div.p-5' });

const SHELL = [
  { name: 'sidebar', css: 'aside' },
  { name: 'brand-wordmark', css: 'aside span.display', up: 'div' },
  nav('nav-hoje', '/'),
  nav('nav-timeline', '/timeline'),
  nav('nav-revisao', '/review'),
  { name: 'nav-revisao-badge', css: 'aside nav a[href="#/review"] span.num' },
  nav('nav-relatorios', '/reports'),
  nav('nav-categorias', '/categories'),
  nav('nav-insights', '/insights'),
  nav('nav-configuracoes', '/settings'),
  nav('nav-foco', '/focus'),
  { name: 'nav-foco-dot', css: '[data-testid="nav-focus-dot"]' },
  { name: 'tracker-pill', css: '[data-testid="tracker-pill"]' },
  { name: 'page-title', css: 'main header h1' },
  { name: 'page-subtitle', css: 'main header h1 + p' },
];
const DAYNAV = { name: 'daynav', css: 'main header div.p-0\\.5' };
const TOAST = { name: 'toast', css: 'div.fixed[aria-live="polite"] > *' };

const HERO = '[data-testid="ubi-hero"]';
const DASH = [
  DAYNAV,
  { name: 'hero', css: HERO },
  { name: 'focus-dial', css: `${HERO} div[role="img"]` },
  { name: 'focus-score', css: `${HERO} div[role="img"] span.display` },
  { name: 'headline', css: `${HERO} h2` },
  { name: 'review-cta', css: `${HERO} a[href="#/review"]` },
  { name: 'uncategorized-note', css: `${HERO} span.num.text-xs` },
  { name: 'stats-row', css: `${HERO} div.grid-cols-2` },
  { name: 'stat-tempo-produtivo', css: `${HERO} div.grid-cols-2 > div`, nth: 0 },
  { name: 'stat-distracoes', css: `${HERO} div.grid-cols-2 > div`, nth: 1 },
  { name: 'stat-maior-foco', css: `${HERO} div.grid-cols-2 > div`, nth: 2 },
  { name: 'stat-trocas-hora', css: `${HERO} div.grid-cols-2 > div`, nth: 3 },
  { name: 'ubi', css: `${HERO} [data-testid="ubi"], ${HERO} [data-testid="focus-prompt"]` },
  { name: 'ubi-robot', css: `${HERO} [data-testid="ubi-3d"], ${HERO} [data-testid="ubi-png"], ${HERO} [data-testid="ubi-svg"]` },
  { name: 'ubi-speech-bubble', css: `${HERO} [data-testid="ubi"] > [role="status"]` },
  { name: 'session-line', css: '[data-testid="dashboard-session-line"]' },
  { name: 'session-countdown', css: '[data-testid="dashboard-countdown"]' },
  { name: 'focus-prompt', css: '[data-testid="focus-prompt"]' },
  { name: 'focus-prompt-bubble', css: '[data-testid="focus-prompt"] > [role="status"]' },
  { name: 'focus-prompt-input', css: '[data-testid="focus-prompt"] input' },
  { name: 'focus-prompt-submit', css: '[data-testid="focus-prompt"] button[type="submit"]' },
  cardByTitle('day-track-card', 'Linha do tempo'),
  { name: 'day-track', css: 'main div[role="list"]' },
  { name: 'day-track-legend', css: 'main ul[aria-label]' },
  cardByTitle('card-tempo-por-categoria', 'Tempo por categoria'),
  cardByTitle('card-foco-por-hora', 'Foco por hora'),
  cardByTitle('card-apps-mais-usados', 'Apps mais usados'),
  { name: 'ubi-card', css: '[data-testid="ubi-card"]' },
];

const PR = '[data-testid="page-review"]';
const ROW = '[data-testid="review-row"]';
const REVIEW = [
  DAYNAV,
  { name: 'classify-now-button', css: 'main header button', text: 'Classificar agora' },
  { name: 'queue-card', css: `${PR} div[class*="col-span-8"]` },
  { name: 'queue-list', css: `${PR} div[role="list"]` },
  { name: 'queue-row-1', css: ROW, nth: 0 },
  { name: 'queue-row-2', css: ROW, nth: 1 },
  { name: 'queue-row-3', css: ROW, nth: 2 },
  { name: 'queue-row-1-app', css: `${ROW} p.font-medium`, nth: 0 },
  { name: 'queue-row-1-category', css: `${ROW} [data-testid="group-category"]`, nth: 0 },
  { name: 'queue-row-active', css: `${ROW}[aria-current="true"]` },
  { name: 'queue-row-active-hint', css: `${ROW}[aria-current="true"] kbd`, up: 'p' },
  { name: 'assign-card', css: `${PR} div[class*="col-span-4"]` },
  { name: 'assign-card-title', css: `${PR} div[class*="col-span-4"] h2` },
  { name: 'assign-picker', css: '[role="listbox"]' },
  ...Array.from({ length: 9 }, (_, i) => ({ name: `assign-option-${i + 1}`, css: '[role="listbox"] [role="option"]', nth: i })),
  ...Array.from({ length: 9 }, (_, i) => ({ name: `assign-key-${i + 1}`, css: '[role="listbox"] kbd', nth: i })),
  { name: 'keys-legend', css: `${PR} div[class*="col-span-4"] > div.border-t` },
  { name: 'settled-header', css: `${PR} section > button[aria-expanded]` },
  { name: 'confirm-bar', css: '[data-testid="confirm-all"]', up: 'div.flex' },
  { name: 'confirm-all-button', css: '[data-testid="confirm-all"]' },
  { name: 'reviewed-list', css: '[data-testid="reviewed-list"]' },
  { name: 'reviewed-row-1', css: `[data-testid="reviewed-list"] ${ROW}`, nth: 0 },
  { name: 'last-suggestions', css: '[data-testid="last-suggestions"]' },
  { name: 'review-done', css: '[data-testid="review-done"]' },
  { name: 'review-done-ubi', css: '[data-testid="review-done"] [data-testid="ubi"]' },
  { name: 'block-details', css: '[data-testid="block-details"]' },
  { name: 'block-card-1', css: '[data-testid="block-card"]', nth: 0 },
  TOAST,
];

const TIMELINE = [
  DAYNAV,
  { name: 'add-manual-button', css: 'main header button', text: 'Adicionar' },
  { name: 'filters-row', css: 'main label[for="filter-cat"]', up: 'div.flex' },
  { name: 'timeline-list', css: 'main ol[aria-label]' },
  { name: 'block-1', css: 'main ol article', nth: 0 },
  { name: 'block-2', css: 'main ol article', nth: 1 },
  { name: 'block-3', css: 'main ol article', nth: 2 },
  { name: 'selected-block', css: 'main ol article[data-selected]' },
  { name: 'selected-block-rail', css: 'main ol article[data-selected] > span' },
  { name: 'selected-block-title', css: 'main ol article[data-selected] p.text-sm.font-medium' },
  { name: 'selected-block-time', css: 'main ol article[data-selected] > div.num' },
  { name: 'selected-block-description', css: 'main ol article[data-selected] p.text-ink-2' },
  { name: 'selected-block-category', css: 'main ol article[data-selected] [aria-haspopup="listbox"]' },
  { name: 'selected-block-badges', css: 'main ol article[data-selected] [aria-haspopup="listbox"]', up: 'div.flex-wrap' },
  { name: 'reclassify-popover', css: '[role="listbox"]', up: 'div[class*="shadow"], div.glass, div.panel-raised' },
  { name: 'reclassify-list', css: '[role="listbox"]' },
  TOAST,
];

const REPORTS = [
  { name: 'tabs', css: 'main header [role="tablist"]' },
  { name: 'report-card-1', css: '[data-testid="page-reports"] div.overflow-hidden', nth: 0 },
  { name: 'report-card-2', css: '[data-testid="page-reports"] div.overflow-hidden', nth: 1 },
  { name: 'report-1-title', css: '[data-testid="page-reports"] h3', nth: 0 },
  { name: 'report-1-meta', css: '[data-testid="page-reports"] h3 + p', nth: 0 },
  { name: 'report-1-summary', css: '[data-testid="page-reports"] .md', nth: 0 },
  { name: 'report-1-summary-heading', css: '[data-testid="page-reports"] .md h2', nth: 0 },
  { name: 'report-1-highlights', css: '[data-testid="page-reports"] .md + div', nth: 0 },
  { name: 'report-1-items', css: '[data-testid="page-reports"] ul[aria-label]', nth: 0 },
  { name: 'report-1-item-1', css: '[data-testid="page-reports"] ul[aria-label] > li', nth: 0 },
  { name: 'report-1-regenerate', css: '[data-testid="page-reports"] button', text: '^Regenerar' },
  { name: 'report-1-copy', css: '[data-testid="page-reports"] button', text: '^Copiar' },
  { name: 'report-2-title', css: '[data-testid="page-reports"] h3', nth: 1 },
  TOAST,
];

const CATEGORIES = [
  { name: 'categories-list-card', css: '[data-testid="page-categories"] div[class*="col-span-4"]' },
  { name: 'category-item-1', css: '[data-testid="page-categories"] li.group', nth: 0 },
  { name: 'category-item-2', css: '[data-testid="page-categories"] li.group', nth: 1 },
  { name: 'category-item-3', css: '[data-testid="page-categories"] li.group', nth: 2 },
  { name: 'category-editor', css: '[data-testid="page-categories"] div[class*="col-span-8"]' },
  { name: 'category-editor-title', css: '[data-testid="page-categories"] div[class*="col-span-8"] h2', nth: 0 },
];

const INSIGHTS = [
  ...['stat-tempo-produtivo', 'stat-score-medio', 'stat-maior-foco', 'stat-trocas-hora'].map((name, i) => ({ name, css: '[data-testid="page-insights"] > div.mb-5 > *', nth: i })),
  cardByTitle('card-horas-semana', 'Horas'),
  { name: 'weekly-chart', css: '.recharts-surface', nth: 0 },
  { name: 'card-pontuacao', css: '[data-testid="page-insights"] div[class*="col-span-5"]', nth: 0 },
  cardByTitle('card-score-foco', 'Score de foco'),
  { name: 'card-recomendacoes', css: '[data-testid="page-insights"] h2', text: '^Recomendações', up: 'div.h-full' },
  { name: 'advice-generate', css: '[data-testid="page-insights"] button', text: '^(Gerar|Gerar de novo|Gerar novamente)' },
  { name: 'advice-list', css: '[data-testid="page-insights"] h2', text: '^Recomendações', up: 'div.h-full', sub: 'ul, ol' },
  { name: 'card-o-que-o-ubi-disse', css: '[data-testid="page-insights"] h2', text: '^O que o UBI', up: 'div.h-full' },
  { name: 'ubi', css: '[data-testid="page-insights"] [data-testid="ubi"]' },
  { name: 'ubi-speech-bubble', css: '[data-testid="page-insights"] [data-testid="ubi"] > [role="status"]' },
];

const SETTINGS = [
  { name: 'sections-nav', css: 'main nav[aria-label]' },
  ...[
    ['geral', 'Geral'],
    ['licenca', 'Licença'],
    ['ia', 'IA'],
    ['rastreamento', 'Rastreamento'],
    ['privacidade', 'Privacidade'],
    ['relatorios', 'Relatórios'],
    ['ubi', 'UBI e notificações'],
    ['atualizacoes', 'Atualizações'],
    ['permissoes', 'Permissões do macOS'],
    ['sobre', 'Sobre'],
  ].map(([id, label]) => ({ name: `section-nav-${id}`, css: 'main nav[aria-label] button', text: `^${label}$` })),
  { name: 'section-geral', css: '#sec-geral' },
  { name: 'section-licenca', css: '#sec-licenca' },
  { name: 'license-status-pill', css: '#sec-licenca > div:first-child > div:last-child' },
  { name: 'license-plan', css: '[data-testid="license-plan"]' },
  { name: 'license-key-form', css: '[data-testid="license-key-form"]' },
  { name: 'license-usage-bar', css: '#sec-licenca [data-testid="usage-bar"]' },
  { name: 'license-managed-note', css: '[data-testid="license-managed-note"]' },
  { name: 'section-ia', css: '#sec-ia' },
  { name: 'ai-status-pill', css: '#sec-ia > div:first-child > div:last-child' },
  { name: 'provider-picker', css: '#sec-ia [role="radiogroup"]' },
  { name: 'provider-anthropic', css: '#sec-ia [role="radio"]', nth: 0 },
  { name: 'provider-openai', css: '#sec-ia [role="radio"]', nth: 1 },
  { name: 'provider-xai', css: '#sec-ia [role="radio"]', nth: 2 },
  { name: 'provider-ia-do-ubi', css: '#sec-ia [role="radio"]', nth: 3 },
  { name: 'provider-active', css: '#sec-ia [role="radio"][aria-checked="true"]' },
  { name: 'api-key-form', css: '#sec-ia [data-testid^="api-key-form"]' },
  { name: 'model-fields', css: '#sec-ia [data-testid^="model-fields"]' },
  TOAST,
];

const FOCUS = [
  { name: 'session-card', css: '[data-testid="focus-session-card"]' },
  { name: 'session-task', css: '[data-testid="focus-session-card"] p.display' },
  { name: 'session-countdown', css: '[data-testid="focus-countdown"]' },
  { name: 'session-start-button', css: '[data-testid="focus-session-card"] button[type="submit"]' },
  { name: 'session-task-input', css: '[data-testid="focus-session-card"] input' },
  { name: 'targets-card', css: '[data-testid="focus-targets"]', up: 'div.p-5' },
  { name: 'targets-list', css: '[data-testid="focus-targets"]' },
  { name: 'target-1', css: '[data-testid^="focus-target-"]', nth: 0 },
  { name: 'target-2', css: '[data-testid^="focus-target-"]', nth: 1 },
  { name: 'target-3', css: '[data-testid^="focus-target-"]', nth: 2 },
  { name: 'interventions-list', css: '[data-testid="focus-interventions"]' },
  { name: 'focus-options', css: '[data-testid="focus-options"]' },
];

const ONB = [
  { name: 'rail', css: 'aside' },
  { name: 'brand-wordmark', css: 'aside span.display', up: 'div' },
  { name: 'ubi', css: 'aside [data-testid="ubi"]' },
  { name: 'ubi-robot', css: 'aside [data-testid="ubi-3d"], aside [data-testid="ubi-png"], aside [data-testid="ubi-svg"]' },
  { name: 'ubi-speech-bubble', css: 'aside [data-testid="ubi"] > [role="status"]' },
  { name: 'steps-list', css: 'aside ol' },
  ...Array.from({ length: 7 }, (_, i) => ({ name: `step-${i + 1}`, css: 'aside ol > li', nth: i })),
  { name: 'step-active', css: 'aside ol button[aria-current="step"]' },
  { name: 'eyebrow', css: 'main p.eyebrow' },
  { name: 'title', css: 'main h1' },
  { name: 'lead', css: 'main h1 + p' },
  { name: 'content', css: 'main div.max-w-\\[680px\\]' },
  { name: 'footer-bar', css: 'main > div.border-t' },
  { name: 'continue-button', css: 'main > div.border-t button', text: '^(Continuar|Concluir)$' },
  { name: 'back-button', css: 'main > div.border-t button', text: '^Voltar$' },
  { name: 'plan-cards', css: '[data-testid="plan-cards"]' },
  { name: 'plan-card-managed', css: '[data-testid="plan-card-managed"]' },
  { name: 'plan-card-own', css: '[data-testid="plan-card-own"]' },
  { name: 'managed-panel', css: '[data-testid="managed-panel"]' },
  { name: 'provider-cards', css: '[data-testid="provider-cards"]' },
  { name: 'provider-card-claude', css: '[data-testid="provider-cards"] [role="radio"]', nth: 0 },
  { name: 'provider-card-gpt', css: '[data-testid="provider-cards"] [role="radio"]', nth: 1 },
  { name: 'provider-card-grok', css: '[data-testid="provider-cards"] [role="radio"]', nth: 2 },
  { name: 'annual-license-panel', css: '[data-testid="annual-license-panel"]' },
  { name: 'content-list', css: 'main ul.panel' },
  { name: 'finish-ai-summary', css: '[data-testid="finish-ai-summary"]' },
];

const INTERVENTION = [
  { name: 'panel', css: '[data-testid="page-intervention"] > section' },
  { name: 'ubi', css: '[data-testid="page-intervention"] [data-testid="ubi"]' },
  { name: 'message', css: '[data-testid="intervention-message"]' },
  { name: 'target-badge', css: '[data-testid="page-intervention"] span.h-5.rounded-md' },
  { name: 'ok-button', css: '[data-testid="page-intervention"] button' },
];

/* ------------------------------------------------------------------ */
/* helpers                                                             */
/* ------------------------------------------------------------------ */

const query = ({ theme = 'dark', license = 'managed', extra = '' } = {}) => `?theme=${theme}&lang=pt-BR${license ? `&license=${license}` : ''}${extra}`;

async function newPage(browser, { theme = 'dark', width = 1440, height = 900, dpr = 2 } = {}) {
  const context = await browser.newContext({
    viewport: { width, height },
    deviceScaleFactor: dpr * HIRES_FACTOR, // --hires: same CSS layout, more pixels
    colorScheme: theme,
    reducedMotion: 'reduce',
    locale: 'pt-BR',
    timezoneId: 'America/Sao_Paulo',
  });
  await context.clock.setFixedTime(NOW);
  await installScrub(context);
  const page = await context.newPage();
  page.on('pageerror', (err) => console.error('  page error:', err.message));
  return { context, page };
}

/** No skeletons, no spinners, fonts loaded, every 3D UBI has its model; then a beat for charts/poses. */
async function settle(page, extra = 1200) {
  await page
    .waitForFunction(() => !document.querySelector('.animate-pulse.bg-panel-3') && !document.querySelector('svg.animate-spin'), null, { timeout: 20000 })
    .catch(() => console.warn('  ! skeleton/spinner still visible'));
  await page.evaluate(() => document.fonts.ready);
  // An 'auto' UBI switches to mode 3d right away but shows the SVG (Suspense fallback) until the GLB is rigged.
  await page.waitForTimeout(300);
  await page
    .waitForFunction(
      () =>
        [...document.querySelectorAll('[data-testid="ubi"][data-ubi-mode="3d"]')].every(
          (u) => u.querySelector('[data-ubi-rig]') && !u.querySelector('[data-testid="ubi-svg"]'),
        ),
      null,
      { timeout: 30000 },
    )
    .catch(() => console.warn('  ! UBI 3D model not ready'));
  await page.waitForTimeout(extra);
}

async function open(page, route, q, testId) {
  await page.goto(`${BASE}/${q}#${route}`, { waitUntil: 'networkidle' });
  if (testId) await page.waitForSelector(`[data-testid="${testId}"]`, { timeout: 20000 });
  await settle(page);
}

function hotspotsInPage(specs) {
  const dpr = window.devicePixelRatio || 1;
  const W = window.innerWidth;
  const H = window.innerHeight;
  const norm = (s) => (s || '').replace(/\s+/g, ' ').trim();
  const out = [];
  const missing = [];
  for (const s of specs) {
    let els = [...document.querySelectorAll(s.css)];
    els = els.filter((e) => {
      const cs = getComputedStyle(e);
      return cs.visibility !== 'hidden' && cs.display !== 'none';
    });
    if (s.text) {
      const re = new RegExp(s.text, 'i');
      els = els.filter((e) => re.test(norm(e.textContent)));
    }
    if (s.leaf) els = els.filter((e) => !els.some((o) => o !== e && e.contains(o)));
    let el = els[s.nth ?? 0];
    if (el && s.up) el = el.closest(s.up);
    if (el && s.sub) el = el.querySelector(s.sub);
    if (!el) {
      missing.push(s.name);
      continue;
    }
    const r = el.getBoundingClientRect();
    const p = s.pad ?? 0;
    const x0 = Math.max(0, r.left - p);
    const y0 = Math.max(0, r.top - p);
    const x1 = Math.min(W, r.right + p);
    const y1 = Math.min(H, r.bottom + p);
    if (x1 - x0 < 2 || y1 - y0 < 2) {
      missing.push(`${s.name} (offscreen)`);
      continue;
    }
    const clipped = x0 > r.left - p || y0 > r.top - p || x1 < r.right + p || y1 < r.bottom + p;
    const h = { name: s.name, x: Math.round(x0 * dpr), y: Math.round(y0 * dpr), w: Math.round((x1 - x0) * dpr), h: Math.round((y1 - y0) * dpr) };
    if (clipped) h.clipped = true;
    out.push(h);
  }
  return { out, missing };
}

const manifest = [];

async function capture(page, meta, specs, { omitBackground = false } = {}) {
  if (HIRES) return captureHires(page, meta, specs, { omitBackground });
  const { out, missing } = await page.evaluate(hotspotsInPage, specs);
  const vp = page.viewportSize();
  const dpr = await page.evaluate(() => window.devicePixelRatio);
  const path = `${OUT}${meta.file}`;
  const fictional = await privacyCheck(page, meta.file);
  await page.screenshot({ path, fullPage: false, omitBackground });
  const entry = { file: meta.file, route: meta.route, theme: meta.theme ?? 'dark', width: Math.round(vp.width * dpr), height: Math.round(vp.height * dpr), dpr, description_pt: meta.pt, description_en: meta.en, hotspots: out };
  manifest.push(entry);
  console.log(`✓ ${meta.file}  ${entry.width}x${entry.height}  ${out.length} hotspots${missing.length ? `  (absent: ${missing.join(', ')})` : ''}  privacy: 0 hits${fictional.length ? ` (fictional: ${fictional.join(', ')})` : ''}`);
  return entry;
}

/* hi-res twins ------------------------------------------------------ */

const manifestPath = `${OUT}ui-manifest.json`;
const hiresPath = `${OUT}hires.json`;
/** The 2x manifest the twins must line up with (read-only in --hires mode). */
const baseManifest = HIRES && existsSync(manifestPath) ? JSON.parse(readFileSync(manifestPath, 'utf8')) : [];
const hiresEntries = [];

/** --hires: shoot one requested file at the raised DPR, after the privacy check and a layout check against the 2x hotspots. */
async function captureHires(page, meta, specs, { omitBackground }) {
  const stem = meta.file.replace(/\.png$/, '');
  if (!HIRES_FILES.has(stem)) {
    console.log(`  · ${meta.file}: not requested, skipped`);
    return null;
  }
  const base = baseManifest.find((m) => m.file === meta.file);
  if (!base) throw new Error(`hires: ${meta.file} has no 2x entry in ui-manifest.json (capture it without --hires first)`);
  const { out } = await page.evaluate(hotspotsInPage, specs);
  const vp = page.viewportSize();
  const dpr = await page.evaluate(() => window.devicePixelRatio);
  const scale = dpr / base.dpr;
  const file = `${stem}@${dpr}x.png`;
  const width = Math.round(vp.width * dpr);
  const height = Math.round(vp.height * dpr);
  if (Math.abs(width - base.width * scale) > 0.5 || Math.abs(height - base.height * scale) > 0.5)
    throw new Error(`hires: ${file} is ${width}x${height}, expected ${base.width * scale}x${base.height * scale}`);
  // same layout: every 2x hotspot is found again, at the same place (hi-res rect / scale)
  const byName = new Map(out.map((h) => [h.name, h]));
  const lost = base.hotspots.filter((h) => !byName.has(h.name)).map((h) => h.name);
  let drift = 0;
  let worst = '';
  for (const h of base.hotspots) {
    const t = byName.get(h.name);
    if (!t) continue;
    const d = Math.max(...['x', 'y', 'w', 'h'].map((k) => Math.abs(t[k] / scale - h[k])));
    if (d > drift) [drift, worst] = [d, h.name];
  }
  if (lost.length || drift > HIRES_TOLERANCE)
    throw new Error(`hires: ${file} layout differs from ${meta.file}: ${lost.length ? `hotspots lost: ${lost.join(', ')}; ` : ''}max drift ${drift.toFixed(2)} px (${worst})`);
  const fictional = await privacyCheck(page, file);
  await page.screenshot({ path: `${OUT}${file}`, fullPage: false, omitBackground });
  hiresEntries.push({ file: meta.file, hires: file, scale, width, height });
  console.log(`✓ ${file}  ${width}x${height}  DPR ${dpr} (x${scale} of ${meta.file})  layout: ${base.hotspots.length} hotspots, max drift ${drift.toFixed(2)} px (${worst || '-'})  privacy: 0 hits${fictional.length ? ` (fictional: ${fictional.join(', ')})` : ''}`);
  return null;
}

const scrollTo = (page, css, block = 'center') =>
  page.evaluate(
    ([sel, b]) => {
      const el = document.querySelector(sel);
      el?.scrollIntoView({ block: b, behavior: 'instant' });
      return !!el;
    },
    [css, block],
  );

/* ------------------------------------------------------------------ */
/* captures                                                            */
/* ------------------------------------------------------------------ */

const CAPTURES = [
  {
    name: 'dashboard',
    files: ['dashboard'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/', query(), 'ubi-hero');
      await capture(page, { file: 'dashboard.png', route: '/#/', pt: 'Hoje: nota de foco 88, frase do dia, estatísticas e o Ubi falando; linha do tempo do dia e gráficos.', en: 'Today: focus score dial, headline sentence, stats and UBI speaking; day track and charts.' }, [...SHELL, ...DASH]);
      await context.close();
    },
  },
  {
    name: 'dashboard-light',
    files: ['dashboard-light'],
    async run(b) {
      const { context, page } = await newPage(b, { theme: 'light' });
      await open(page, '/', query({ theme: 'light' }), 'ubi-hero');
      await capture(page, { file: 'dashboard-light.png', route: '/#/', theme: 'light', pt: 'Hoje no tema claro.', en: 'Today dashboard, light theme.' }, [...SHELL, ...DASH]);
      await context.close();
    },
  },
  {
    name: 'dashboard-nudge',
    files: ['dashboard-nudge'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/', query({ extra: '&nudge=focus_prompt' }), 'ubi-hero');
      await page.waitForSelector('[data-testid="focus-prompt"]', { timeout: 10000 }).catch(() => console.warn('  ! focus prompt absent'));
      await settle(page, 600);
      await capture(page, { file: 'dashboard-nudge.png', route: '/#/?nudge=focus_prompt', pt: 'Hoje com o Ubi sugerindo foco: "Muitas janelas" e o campo para a tarefa.', en: 'Today with UBI nudging: "A lot of windows" focus prompt with a task field.' }, [...SHELL, ...DASH]);
      await context.close();
    },
  },
  {
    name: 'dashboard-session',
    files: ['dashboard-session'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/', query({ extra: '&session=active' }), 'ubi-hero');
      await page.waitForSelector('[data-testid="dashboard-session-line"]', { timeout: 10000 }).catch(() => console.warn('  ! session line absent'));
      await settle(page, 400);
      await capture(page, { file: 'dashboard-session.png', route: '/#/?session=active', pt: 'Hoje com uma sessão de foco em andamento (contagem regressiva no herói).', en: 'Today with a running focus session (countdown line in the hero).' }, [...SHELL, ...DASH]);
      await context.close();
    },
  },
  {
    name: 'dashboard-full',
    files: ['dashboard-full'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/', query(), 'ubi-hero');
      const h = await page.evaluate(() => {
        const sc = document.querySelector('main > div.overflow-y-auto');
        return Math.ceil((sc?.scrollHeight ?? 900) + 38);
      });
      await page.setViewportSize({ width: 1440, height: Math.min(2400, h) });
      await settle(page, 1200);
      await capture(page, { file: 'dashboard-full.png', route: '/#/', pt: 'Hoje, página inteira (para panorâmica vertical): herói, linha do tempo, gráficos e cartão do Ubi.', en: 'Today, full page (for a vertical pan): hero, day track, charts and UBI card.' }, [...SHELL, ...DASH]);
      await context.close();
    },
  },
  {
    name: 'timeline',
    files: ['timeline'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/timeline', query(), 'page-timeline');
      await capture(page, { file: 'timeline.png', route: '/#/timeline', pt: 'Timeline: cada bloco do dia com app, título, categoria, confiança e origem.', en: 'Timeline: every block of the day with app, title, category, confidence and source.' }, [...SHELL, ...TIMELINE]);
      await context.close();
    },
  },
  {
    name: 'timeline-selected',
    files: ['timeline-selected', 'timeline-reclassify'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/timeline', query(), 'page-timeline');
      const target = page.locator('main ol article', { hasText: 'Conversa com a equipe da AgroTech' }).first();
      await target.scrollIntoViewIfNeeded();
      await page.evaluate(() => {
        const el = [...document.querySelectorAll('main ol article')].find((a) => a.textContent.includes('Conversa com a equipe da AgroTech'));
        el?.scrollIntoView({ block: 'center', behavior: 'instant' });
      });
      await target.click({ position: { x: 40, y: 12 } });
      await page.mouse.move(5, 5);
      await settle(page, 500);
      await capture(page, { file: 'timeline-selected.png', route: '/#/timeline', pt: 'Timeline com um bloco selecionado (WhatsApp: conversa com a equipe da AgroTech, precisa de revisão).', en: 'Timeline with one block selected (WhatsApp chat with the AgroTech team, needs review).' }, [...SHELL, ...TIMELINE]);
      await page.locator('main ol article[data-selected] [aria-haspopup="listbox"]').click();
      await page.waitForSelector('[role="listbox"]', { timeout: 5000 }).catch(() => console.warn('  ! reclassify popover absent'));
      await settle(page, 400);
      await capture(page, { file: 'timeline-reclassify.png', route: '/#/timeline', pt: 'Timeline: bloco selecionado com o seletor "Reclassificar" aberto.', en: 'Timeline: selected block with the reclassify category picker open.' }, [...SHELL, ...TIMELINE]);
      await context.close();
    },
  },
  {
    name: 'review',
    files: ['review-queue-selected', 'review-after-assign'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/review', query(), 'page-review');
      await page.locator(ROW).first().click({ position: { x: 30, y: 20 } });
      await page.mouse.move(5, 5);
      await settle(page, 500);
      await capture(page, { file: 'review-queue-selected.png', route: '/#/review', pt: 'Revisão: fila do que o Ubi não resolveu, primeira linha selecionada; à direita, categorias numeradas 1–9.', en: 'Review: the queue of what UBI could not settle, first row selected; numbered categories 1–9 on the right.' }, [...SHELL, ...REVIEW]);
      await page.keyboard.press('1');
      await page.waitForSelector('div.fixed[aria-live="polite"] > *', { timeout: 5000 }).catch(() => console.warn('  ! toast absent'));
      await settle(page, 700);
      await capture(page, { file: 'review-after-assign.png', route: '/#/review', pt: 'Revisão logo após apertar "1": o grupo saiu da fila, a próxima linha foi selecionada e o toast confirma.', en: 'Review right after pressing "1": the group left the queue, the next row is selected, a toast confirms.' }, [...SHELL, ...REVIEW]);
      await context.close();
    },
  },
  {
    name: 'review-details',
    files: ['review-details'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/review', query(), 'page-review');
      // the YouTube group (TEDx talk, 2 blocks) has a screenshot the vision model saw
      await page.locator(ROW, { hasText: 'youtube.com' }).first().click({ position: { x: 30, y: 20 } });
      await page.keyboard.press('Enter');
      await page.waitForSelector('[data-testid="block-details"]', { timeout: 5000 }).catch(() => console.warn('  ! details absent'));
      await page.mouse.move(5, 5);
      await settle(page, 900);
      await scrollTo(page, `${ROW}[aria-current="true"]`, 'start');
      await page.evaluate(() => document.querySelector('main > div.overflow-y-auto')?.scrollBy(0, -16));
      await settle(page, 600);
      await capture(page, { file: 'review-details.png', route: '/#/review', pt: 'Revisão com os detalhes do grupo abertos (blocos e captura).', en: 'Review with the group details open (blocks and screenshot).' }, [...SHELL, ...REVIEW]);
      await context.close();
    },
  },
  {
    name: 'review-confirm',
    files: ['review-settled-expanded', 'review-confirmed'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/review', query(), 'page-review');
      await page.locator(`${PR} section > button[aria-expanded]`).click();
      await page.waitForSelector('[data-testid="confirm-all"]', { timeout: 5000 });
      await scrollTo(page, `${PR} section > button[aria-expanded]`, 'start');
      await page.evaluate(() => document.querySelector('main > div.overflow-y-auto')?.scrollBy(0, -120));
      await page.mouse.move(5, 5);
      await settle(page, 500);
      await capture(page, { file: 'review-settled-expanded.png', route: '/#/review', pt: 'Revisão: "Classificados neste dia" aberto, com o botão "Confirmar os N" para aceitar tudo de uma vez.', en: 'Review: "Classified this day" expanded, with the "Confirm all N" button.' }, [...SHELL, ...REVIEW]);
      await page.locator('[data-testid="confirm-all"]').click();
      await page.waitForSelector('div.fixed[aria-live="polite"] > *', { timeout: 5000 }).catch(() => console.warn('  ! toast absent'));
      await page.mouse.move(5, 5);
      await settle(page, 700);
      await capture(page, { file: 'review-confirmed.png', route: '/#/review', pt: 'Revisão depois de "Confirmar os N": tudo confirmado, toast de sucesso.', en: 'Review after "Confirm all": everything confirmed, success toast.' }, [...SHELL, ...REVIEW]);
      await context.close();
    },
  },
  {
    name: 'review-done',
    files: ['review-done'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/review', query(), 'page-review');
      for (let i = 0; i < 20; i++) {
        if (await page.locator('[data-testid="review-done"]').count()) break;
        await page.keyboard.press('1');
        await page.waitForTimeout(450);
      }
      await page.waitForSelector('[data-testid="review-done"]', { timeout: 8000 }).catch(() => console.warn('  ! review-done absent'));
      await page.waitForTimeout(5200); // let the toasts leave
      await settle(page, 600);
      await capture(page, { file: 'review-done.png', route: '/#/review', pt: 'Revisão zerada: fila vazia, o Ubi comemora; o selo da Revisão some da barra lateral.', en: 'Review done: empty queue, UBI celebrates; the Review badge leaves the sidebar.' }, [...SHELL, ...REVIEW]);
      await context.close();
    },
  },
  {
    name: 'reports',
    files: ['reports', 'reports-generated'],
    async run(b) {
      const { context, page } = await newPage(b);
      // The seeded IFRO report carries stale=true ("Desatualizado" badge); a fresh report reads cleaner on video. Flip the flag in
      // the same mock module instance the app uses (Vite serves /src/lib/mock.ts once per page), then route in-app.
      await open(page, '/', query(), 'ubi-hero');
      const unstale = await page.evaluate(async () => {
        const m = await import('/src/lib/mock.ts');
        const reports = m.__mock.state().reports;
        for (const r of reports) r.stale = false;
        return reports.length;
      });
      console.log(`  reports un-staled: ${unstale}`);
      await page.evaluate(() => {
        location.hash = '#/reports';
      });
      await page.waitForSelector('[data-testid="page-reports"]', { timeout: 20000 });
      await settle(page);
      await capture(page, { file: 'reports.png', route: '/#/reports', pt: 'Relatórios: relatório diário gerado pela IA por categoria (resumo, destaques e itens com horário).', en: 'Reports: AI-written daily report per category (summary, highlights and timed items).' }, [...SHELL, ...REPORTS]);
      await page.locator('[data-testid="page-reports"] button', { hasText: /^Regenerar/ }).first().click();
      await page.waitForSelector('div.fixed[aria-live="polite"] > *', { timeout: 8000 }).catch(() => console.warn('  ! toast absent'));
      await page.mouse.move(5, 5);
      await settle(page, 600);
      await capture(page, { file: 'reports-generated.png', route: '/#/reports', pt: 'Relatórios logo após "Regenerar": relatório novo e toast "Relatório gerado".', en: 'Reports right after "Regenerate": fresh report and a "Report generated" toast.' }, [...SHELL, ...REPORTS]);
      await context.close();
    },
  },
  {
    name: 'categories',
    files: ['categories'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/categories', query(), 'page-categories');
      await capture(page, { file: 'categories.png', route: '/#/categories', pt: 'Categorias: lista à esquerda e editor (descrição, palavras-chave, relatório) à direita.', en: 'Categories: list on the left, editor (description, keywords, report) on the right.' }, [...SHELL, ...CATEGORIES]);
      await context.close();
    },
  },
  {
    name: 'insights',
    files: ['insights'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/insights', query(), 'page-insights');
      await page.waitForSelector('.recharts-surface', { timeout: 15000 }).catch(() => {});
      await page.getByRole('button', { name: /^Gerar/ }).first().click().catch(() => console.warn('  ! no Gerar button'));
      await page.waitForTimeout(1800);
      await page.mouse.move(5, 5);
      await settle(page, 900);
      await capture(page, { file: 'insights.png', route: '/#/insights', pt: 'Insights: horas da semana por categoria, nota de foco e o conselho do Ubi gerado por IA.', en: 'Insights: weekly hours by category, focus score and UBI’s AI advice.' }, [...SHELL, ...INSIGHTS]);
      await context.close();
    },
  },
  {
    name: 'settings',
    files: ['settings', 'settings-license', 'settings-ai', 'settings-ai-ubi'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/settings', query(), 'page-settings');
      await capture(page, { file: 'settings.png', route: '/#/settings', pt: 'Configurações: navegação por seções e o topo (Geral, Licença).', en: 'Settings: section navigation and the top (General, License).' }, [...SHELL, ...SETTINGS]);
      await page.locator('main nav[aria-label] button', { hasText: /^Licença$/ }).click();
      await page.mouse.move(5, 5);
      await page.waitForTimeout(900);
      await settle(page, 300);
      await capture(page, { file: 'settings-license.png', route: '/#/settings (Licença)', pt: 'Configurações › Licença: ubiqX Mensal com a IA do Ubi, validade e uso do mês.', en: 'Settings › License: ubiqX Monthly with UBI’s AI, expiry and monthly usage.' }, [...SHELL, ...SETTINGS]);
      await page.locator('main nav[aria-label] button', { hasText: /^IA$/ }).click();
      await page.mouse.move(5, 5);
      await page.waitForTimeout(900);
      await settle(page, 300);
      await capture(page, { file: 'settings-ai.png', route: '/#/settings (IA)', pt: 'Configurações › IA: escolha do provedor (Anthropic, OpenAI, xAI, IA do Ubi), chave e modelos.', en: 'Settings › AI: provider choice (Anthropic, OpenAI, xAI, UBI’s AI), key and models.' }, [...SHELL, ...SETTINGS]);
      await page.locator('#sec-ia [role="radio"]').nth(3).click();
      await page.waitForTimeout(1600);
      await page.mouse.move(5, 5);
      await settle(page, 300);
      await capture(page, { file: 'settings-ai-ubi.png', route: '/#/settings (IA)', pt: 'Configurações › IA com a "IA do Ubi" selecionada (sem chave: a licença é a credencial).', en: 'Settings › AI with "UBI’s AI" selected (no key: the license is the credential).' }, [...SHELL, ...SETTINGS]);
      await context.close();
    },
  },
  {
    name: 'focus',
    files: ['focus'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/focus', query(), 'page-focus');
      await capture(page, { file: 'focus.png', route: '/#/focus', pt: 'Foco: iniciar sessão, alvos bloqueados (YouTube, Instagram, Discord…) e intervenções de hoje.', en: 'Focus: start a session, blocked targets (YouTube, Instagram, Discord…) and today’s interventions.' }, [...SHELL, ...FOCUS]);
      await context.close();
    },
  },
  {
    name: 'focus-session',
    files: ['focus-session'],
    async run(b) {
      const { context, page } = await newPage(b);
      await open(page, '/focus', query({ extra: '&session=active' }), 'page-focus');
      await page.waitForSelector('[data-testid="focus-countdown"]', { timeout: 10000 }).catch(() => console.warn('  ! countdown absent'));
      await settle(page, 400);
      await capture(page, { file: 'focus-session.png', route: '/#/focus?session=active', pt: 'Foco com sessão em andamento: tarefa, contagem regressiva e alvos protegidos.', en: 'Focus with a running session: task, countdown and guarded targets.' }, [...SHELL, ...FOCUS]);
      await context.close();
    },
  },
  ...[1, 2, 3, 4, 5, 6, 7].map((step) => ({
    name: `onboarding-${step}`,
    files: [`onboarding-${step}-${['intro', 'ia', 'permissoes', 'categorias', 'horarios', 'visao', 'concluir'][step - 1]}`, ...(step === 2 ? ['onboarding-2-ia-managed'] : [])],
    async run(b) {
      const names = ['intro', 'ia', 'permissoes', 'categorias', 'horarios', 'visao', 'concluir'];
      const pt = ['boas-vindas e o que o ubiqX faz', 'como a IA é paga (IA do Ubi ou sua própria chave) e provedores', 'permissões do macOS', 'suas categorias', 'horários dos relatórios', 'política de capturas de tela (visão)', 'resumo e concluir'];
      const { context, page } = await newPage(b);
      await open(page, '/onboarding', query({ extra: `&onboarding=1&step=${step}` }), 'page-onboarding');
      await capture(page, { file: `onboarding-${step}-${names[step - 1]}.png`, route: `/#/onboarding?onboarding=1&step=${step}`, pt: `Onboarding, passo ${step} de 7: ${pt[step - 1]}.`, en: `Onboarding, step ${step} of 7.` }, ONB);
      if (step === 2) {
        await page.locator('[data-testid="plan-card-managed"] button[role="radio"]').click();
        await page.waitForSelector('[data-testid="managed-panel"]', { timeout: 5000 }).catch(() => {});
        await page.mouse.move(5, 5);
        await settle(page, 700);
        await capture(page, { file: 'onboarding-2-ia-managed.png', route: '/#/onboarding?onboarding=1&step=2', pt: 'Onboarding, passo 2: cartão "IA do Ubi" escolhido, licença mensal ativa.', en: 'Onboarding step 2: "UBI’s AI" card picked, monthly license active.' }, ONB);
      }
      await context.close();
    },
  })),
  {
    name: 'intervention',
    files: ['intervention'],
    async run(b) {
      const { context, page } = await newPage(b, { width: 460, height: 188, dpr: 4 });
      await page.addInitScript(() => {
        const s = document.createElement('style');
        s.textContent = 'html,body,#root{background:transparent !important}';
        document.addEventListener('DOMContentLoaded', () => document.head.appendChild(s));
      });
      await page.goto(`${BASE}/${query()}#/intervention?id=test`, { waitUntil: 'networkidle' });
      await page.waitForSelector('[data-testid="intervention-message"]', { timeout: 15000 });
      await settle(page, 700);
      await page.mouse.move(1, 1);
      await capture(page, { file: 'intervention.png', route: '/#/intervention?id=test', pt: 'Janela de intervenção (460×188, fundo transparente): o Ubi fechou uma distração e explica por quê.', en: 'Intervention window (460×188, transparent background): UBI closed a distraction and says why.' }, INTERVENTION, { omitBackground: true });
      await context.close();
    },
  },
];

/* ------------------------------------------------------------------ */

/** --hires: the PNG stems to re-shoot. A name that is a file stem means that file only (`reports` is both a capture
 * and a file: the file); any other capture name means all of its files. */
const HIRES_FILES = new Set(
  HIRES ? ONLY.flatMap((n) => (CAPTURES.some((c) => c.files.includes(n)) ? [n] : (CAPTURES.find((c) => c.name === n)?.files ?? []))) : [],
);

mkdirSync(OUT, { recursive: true });
const browser = await chromium.launch({ executablePath: EXEC, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--lang=pt-BR'],
  // On Linux the <input type="time"> format follows the process locale, not the context locale: 18:00, not 06:00 PM.
  env: { ...process.env, LANG: 'pt_BR.UTF-8', LANGUAGE: 'pt_BR:pt', LC_ALL: 'pt_BR.UTF-8' },
});
try {
  for (const c of CAPTURES) {
    if (HIRES ? !c.files.some((f) => HIRES_FILES.has(f)) : ONLY.length && !ONLY.includes(c.name)) continue;
    console.log(`→ ${c.name}${HIRES ? ` (hi-res, DPR x${HIRES_FACTOR})` : ''}`);
    try {
      await c.run(browser);
    } catch (e) {
      console.error(`✗ ${c.name}:`, e.message.split('\n')[0]);
    }
  }
} finally {
  await browser.close();
}

if (HIRES) {
  // the 2x PNGs and ui-manifest.json are left alone; the twins go to hires.json
  const prev = existsSync(hiresPath) ? JSON.parse(readFileSync(hiresPath, 'utf8')) : [];
  const fresh = new Set(hiresEntries.map((h) => h.file));
  const merged = [...prev.filter((h) => !fresh.has(h.file)), ...hiresEntries];
  if (hiresEntries.length) writeFileSync(hiresPath, `${JSON.stringify(merged, null, 2)}\n`);
  console.log(`hires: ${hiresEntries.length} twins written, ${merged.length} entries → ${hiresPath}`);
  const known = new Set(CAPTURES.flatMap((c) => [c.name, ...c.files]));
  const unknown = ONLY.filter((n) => !known.has(n));
  const shot = new Set(hiresEntries.map((h) => h.file.replace(/\.png$/, '')));
  const lostFiles = [...HIRES_FILES].filter((f) => !shot.has(f));
  if (unknown.length) console.error(`✗ unknown names: ${unknown.join(', ')}`);
  if (lostFiles.length) console.error(`✗ no twin written for: ${lostFiles.join(', ')}`);
  if (unknown.length || lostFiles.length) process.exitCode = 1;
} else {
  let merged = manifest;
  if (ONLY.length && existsSync(manifestPath)) {
    const prev = JSON.parse(readFileSync(manifestPath, 'utf8'));
    const fresh = new Set(manifest.map((m) => m.file));
    merged = [...prev.filter((m) => !fresh.has(m.file)), ...manifest];
  }
  writeFileSync(manifestPath, `${JSON.stringify(merged, null, 2)}\n`);
  console.log(`manifest: ${merged.length} entries → ${manifestPath}`);
}

console.log(`scrubbed modules: ${[...scrubbed].map(([p, n]) => `${p} (${n})`).join(', ') || 'none'}`);
const leaks = privacyLog.filter((l) => l.hits.length);
console.log(`privacy scan: ${privacyLog.length} captures scanned, ${leaks.reduce((n, l) => n + l.hits.length, 0)} hits${leaks.length ? ` in ${leaks.map((l) => l.file).join(', ')}` : ''}`);
if (leaks.length) process.exitCode = 1;
