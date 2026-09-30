#!/usr/bin/env node
/**
 * Fast review stills: bundles once, opens ONE browser, renders many frames.
 * (`npx remotion still` re-bundles + relaunches Chrome per frame.)
 *
 * Usage (from video/launch/):
 *   node tools/stills.mjs <CompositionId> <frames> [--out=out/stills] [--scale=0.5] [--jpeg]
 *     frames: comma list and/or ranges with step, e.g. "0,30,90-150:15"
 *
 * Output: <out>/<CompositionId>-<frame>.png (or .jpg)
 */
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {bundle} from '@remotion/bundler';
import {openBrowser, renderStill, selectComposition} from '@remotion/renderer';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const flags = Object.fromEntries(args.filter((a) => a.startsWith('--')).map((a) => a.slice(2).split('=')));
const [compId, framesArg] = args.filter((a) => !a.startsWith('--'));
if (!compId || !framesArg) {
	console.error('usage: node tools/stills.mjs <CompositionId> <frames e.g. 0,30,60-120:15> [--out=dir] [--scale=0.5] [--jpeg]');
	process.exit(1);
}
const frames = framesArg.split(',').flatMap((part) => {
	const m = part.match(/^(\d+)-(\d+)(?::(\d+))?$/);
	if (!m) return [Number(part)];
	const out = [];
	for (let f = Number(m[1]); f <= Number(m[2]); f += Number(m[3] ?? 1)) out.push(f);
	return out;
});
const outDir = path.resolve(root, flags.out ?? 'out/stills');
const scale = Number(flags.scale ?? 1);
const jpeg = 'jpeg' in flags;
fs.mkdirSync(outDir, {recursive: true});

const HEADLESS_SHELL = '/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell';
const FULL_CHROME = '/opt/pw-browsers/chromium-1194/chrome-linux/chrome';
const browserExecutable = process.env.REMOTION_BROWSER ?? (fs.existsSync(HEADLESS_SHELL) ? HEADLESS_SHELL : fs.existsSync(FULL_CHROME) ? FULL_CHROME : null);
const chromeMode = browserExecutable && !/headless_shell/.test(browserExecutable) ? 'chrome-for-testing' : 'headless-shell';
const GL = process.env.REMOTION_GL ?? process.env.GL ?? 'swiftshader';
const chromiumOptions = {gl: GL === 'default' ? null : GL};

const t0 = Date.now();
const serveUrl = await bundle({entryPoint: path.join(root, 'src/index.ts'), publicDir: path.join(root, 'public')});
const t1 = Date.now();
const browser = await openBrowser('chrome', {browserExecutable, chromiumOptions, chromeMode});
const composition = await selectComposition({serveUrl, id: compId, puppeteerInstance: browser, browserExecutable, chromiumOptions, chromeMode});
for (const frame of frames) {
	if (frame >= composition.durationInFrames) {
		console.warn(`skip ${frame}: beyond ${composition.durationInFrames}`);
		continue;
	}
	const output = path.join(outDir, `${compId}-${String(frame).padStart(4, '0')}.${jpeg ? 'jpg' : 'png'}`);
	const s = Date.now();
	await renderStill({
		serveUrl,
		composition,
		frame,
		output,
		scale,
		imageFormat: jpeg ? 'jpeg' : 'png',
		...(jpeg ? {jpegQuality: 90} : {}),
		puppeteerInstance: browser,
		browserExecutable,
		chromiumOptions,
		chromeMode,
		overwrite: true,
	});
	console.log(`${path.relative(root, output)}  (${Date.now() - s} ms)`);
}
await browser.close({silent: true});
console.log(`bundle ${((t1 - t0) / 1000).toFixed(1)}s, total ${((Date.now() - t0) / 1000).toFixed(1)}s, browser ${browserExecutable ?? 'remotion-default'}`);
fs.rmSync(serveUrl, {recursive: true, force: true});
