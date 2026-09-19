import { describe, expect, it } from 'vitest';
// The very file Tauri ships with the app, read as text so this test breaks when the policy changes.
import tauriConf from '../../../src-tauri/tauri.conf.json?raw';

/**
 * The shipped Content-Security-Policy has to allow what the 3D UBI needs, or the app silently
 * falls back to the flat SVG (see Ubi.tsx). three.js uses `blob:` twice:
 *
 *  - DRACOLoader builds its decoder Worker from a Blob URL (DRACOLoader.js: `new Worker(URL.createObjectURL(...))`).
 *    Chrome and Firefox judge that by `worker-src`, but WebKit — the engine of the macOS app — has no
 *    `worker-src` and falls back to `child-src`, then `script-src`. Without `blob:` in those, the model
 *    cannot be decompressed at all and the mascot degrades to the SVG.
 *  - GLTFLoader turns the textures embedded in the .glb into Blob URLs and fetches them, which `connect-src`
 *    governs. Without `blob:` there the model renders as a white, untextured ghost.
 */
const csp: string = JSON.parse(tauriConf).app.security.csp;

const directive = (name: string): string[] => {
  const found = csp
    .split(';')
    .map((d) => d.trim())
    .find((d) => d === name || d.startsWith(`${name} `));
  return found ? found.split(/\s+/).slice(1) : [];
};

describe('the desktop Content-Security-Policy', () => {
  it.each(['connect-src', 'script-src', 'child-src', 'worker-src'])('lets the 3D UBI use blob: in %s', (name) => {
    expect(directive(name)).toContain('blob:');
  });

  it('keeps every other source local', () => {
    expect(directive('default-src')).toEqual(["'self'"]);
    for (const d of ['connect-src', 'script-src', 'child-src', 'worker-src', 'img-src', 'font-src', 'style-src']) {
      const sources = directive(d).filter((s) => !s.startsWith('http://asset.localhost') && !s.startsWith('http://ipc.localhost'));
      expect(sources.filter((s) => s.startsWith('http') || s === '*'), `${d} reaches the network`).toEqual([]);
    }
  });
});
