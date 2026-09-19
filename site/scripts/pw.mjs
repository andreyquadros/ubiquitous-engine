// Shared Playwright launcher: software WebGL so the 3D mascot renders in headless Chromium.
import { chromium } from 'playwright';

export const GL_ARGS = ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'];

export async function launch() {
  try {
    return await chromium.launch({ args: GL_ARGS });
  } catch (e) {
    console.warn('default chromium launch failed, retrying with executablePath:', e.message.split('\n')[0]);
    return chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? '/opt/pw-browsers/chromium-1194/chrome-linux/chrome', args: GL_ARGS });
  }
}

/** Parses `--name value` and `--name=value`. */
export function argParser(argv) {
  return (name, def) => {
    const eq = argv.find((a) => a.startsWith(`${name}=`));
    if (eq) return eq.slice(name.length + 1) || def;
    const i = argv.indexOf(name);
    return i >= 0 && argv[i + 1] && !argv[i + 1].startsWith('--') ? argv[i + 1] : def;
  };
}

/** Polls `url` until it answers. */
export async function waitFor(url, ms = 60000) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) {
    try {
      const r = await fetch(url);
      if (r.ok) return;
    } catch {
      /* not yet */
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  throw new Error(`timed out waiting for ${url}`);
}
