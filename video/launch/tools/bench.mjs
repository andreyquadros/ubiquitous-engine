#!/usr/bin/env node
/**
 * Per-frame render cost probe: renders each listed frame N times in one
 * browser tab-set and prints the best time (ms). Use it to find expensive
 * effects before a full render.
 *
 *   node tools/bench.mjs <CompositionId> <frames e.g. 100,448,530> [--reps=3] [--scale=1]
 */
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {bundle} from '@remotion/bundler';
import {openBrowser, renderStill, selectComposition} from '@remotion/renderer';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const flags = Object.fromEntries(args.filter((a) => a.startsWith('--')).map((a) => a.slice(2).split('=')));
const [compId, framesArg] = args.filter((a) => !a.startsWith('--'));
const frames = framesArg.split(',').map(Number);
const reps = Number(flags.reps ?? 3);
const HEADLESS_SHELL = '/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell';
const browserExecutable = process.env.REMOTION_BROWSER ?? (fs.existsSync(HEADLESS_SHELL) ? HEADLESS_SHELL : null);
const chromeMode = browserExecutable && !/headless_shell/.test(browserExecutable) ? 'chrome-for-testing' : 'headless-shell';
const GL = process.env.REMOTION_GL ?? process.env.GL ?? 'swiftshader';
const chromiumOptions = {gl: GL === 'default' ? null : GL};
const serveUrl = await bundle({entryPoint: path.join(root, 'src/index.ts'), publicDir: path.join(root, 'public')});
const browser = await openBrowser('chrome', {browserExecutable, chromiumOptions, chromeMode});
const composition = await selectComposition({serveUrl, id: compId, puppeteerInstance: browser, browserExecutable, chromiumOptions, chromeMode});
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'bench-'));
console.log(`load avg ${os.loadavg().map((l) => l.toFixed(1)).join(' ')}`);
for (const frame of frames) {
	const times = [];
	for (let r = 0; r < reps; r++) {
		const s = performance.now();
		await renderStill({serveUrl, composition, frame, output: path.join(tmp, 'f.jpg'), imageFormat: 'jpeg', jpegQuality: 80, scale: Number(flags.scale ?? 1), puppeteerInstance: browser, browserExecutable, chromiumOptions, chromeMode, overwrite: true});
		times.push(performance.now() - s);
	}
	console.log(`frame ${String(frame).padStart(5)}  best ${Math.min(...times).toFixed(0)} ms  (${times.map((t) => t.toFixed(0)).join(', ')})`);
}
await browser.close({silent: true});
fs.rmSync(tmp, {recursive: true, force: true});
fs.rmSync(serveUrl, {recursive: true, force: true});
