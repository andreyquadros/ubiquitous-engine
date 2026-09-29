/**
 * Remotion CLI configuration for the ubiqX AI launch video.
 *
 * Only applies to the CLI (`npx remotion studio|still|render`). If you render
 * programmatically via @remotion/renderer you must pass the same options.
 *
 * Browser selection (first that exists wins):
 *   1. $REMOTION_BROWSER (absolute path) — manual override
 *   2. Playwright's Chromium headless shell (fast, "old" headless)
 *   3. Playwright's full Chromium (uses --headless=new)
 *   4. nothing → Remotion downloads its own chrome-headless-shell
 */
import fs from 'node:fs';
import {Config} from '@remotion/cli/config';

const HEADLESS_SHELL =
	'/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell';
const FULL_CHROME = '/opt/pw-browsers/chromium-1194/chrome-linux/chrome';

const override = process.env.REMOTION_BROWSER;
if (override && fs.existsSync(override)) {
	Config.setBrowserExecutable(override);
	Config.setChromeMode(
		/headless_shell|chrome-headless-shell/.test(override)
			? 'headless-shell'
			: 'chrome-for-testing',
	);
} else if (fs.existsSync(HEADLESS_SHELL)) {
	Config.setBrowserExecutable(HEADLESS_SHELL);
	Config.setChromeMode('headless-shell');
} else if (fs.existsSync(FULL_CHROME)) {
	Config.setBrowserExecutable(FULL_CHROME);
	Config.setChromeMode('chrome-for-testing');
}

// GPU-less box: `swiftshader` measured ~2.5x faster per frame than `swangle`
// on this machine with pixel-identical output (mean abs diff < 1/255), and
// WebGL/WebGL2 + CSS 3D transforms still work (ANGLE→SwiftShader Vulkan).
// Override with REMOTION_GL=swangle|angle|egl|vulkan if needed.
const GL = (process.env.REMOTION_GL ?? 'swiftshader') as 'swiftshader' | 'swangle' | 'angle' | 'egl' | 'vulkan' | 'angle-egl';
Config.setChromiumOpenGlRenderer(GL);

// The machine has 4 cores shared with other jobs; 3 tabs is the sweet spot.
// Override per run with `--concurrency=N`.
Config.setConcurrency(Number(process.env.REMOTION_CONCURRENCY ?? 3));

// Intermediate frames: high-quality JPEG (PNG only needed for transparency).
Config.setVideoImageFormat('jpeg');
Config.setJpegQuality(95);

// Output: H.264 / yuv420p (plays everywhere: X/Twitter, LinkedIn, QuickTime).
Config.setCodec('h264');
Config.setPixelFormat('yuv420p');
Config.setCrf(18);
// Tag + convert to BT.709 limited range (what X/Twitter, YouTube and QuickTime expect).
Config.setColorSpace('bt709');
Config.setX264Preset('medium');

Config.setOverwriteOutput(true);
Config.setDelayRenderTimeoutInMilliseconds(60_000);
Config.setEntryPoint('src/index.ts');
Config.setStudioPort(1460);
