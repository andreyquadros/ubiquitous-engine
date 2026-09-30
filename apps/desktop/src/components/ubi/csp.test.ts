import { describe, expect, it } from 'vitest';
// The very file Tauri ships with the app, read as text so this test breaks when the policy changes.
import tauriConf from '../../../src-tauri/tauri.conf.json?raw';

/**
 * The shipped Content-Security-Policy has to allow what the 3D UBI needs, or the app silently falls
 * back to the flat SVG (see Ubi.tsx).
 *
 * GLTFLoader turns the textures embedded in the .glb into `blob:` URLs and fetches them back, which
 * `connect-src` governs. Without it the model renders as a white, untextured ghost — reproduced in a
 * browser serving the built app under this exact policy.
 *
 * Nothing here may need `blob:` for code any more. The model used to be Draco-compressed, and
 * three.js decodes Draco in a Worker built from a `blob:` URL; WebKit, the engine of the macOS app,
 * has no `worker-src` and falls back to `child-src` and then `script-src`, so that Worker was
 * refused and the mascot was flat on every Mac. The model now uses KHR_mesh_quantization, which
 * three.js reads natively, and `scripts/ubi-model.test.mjs` keeps a decoder from creeping back in.
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
  it('lets GLTFLoader fetch the textures it unpacks from the model', () => {
    expect(directive('connect-src')).toContain('blob:');
    expect(directive('img-src')).toContain('blob:');
  });

  it('runs no code from a blob: URL', () => {
    // If this ever has to change, the model has grown a decoder again — fix the model instead.
    for (const d of ['script-src', 'child-src', 'worker-src', 'default-src']) {
      expect(directive(d), `${d} would run blob: code`).not.toContain('blob:');
    }
  });

  it('keeps every other source local', () => {
    expect(directive('default-src')).toEqual(["'self'"]);
    for (const d of ['connect-src', 'script-src', 'worker-src', 'img-src', 'font-src', 'style-src']) {
      const sources = directive(d).filter((s) => !s.startsWith('http://asset.localhost') && !s.startsWith('http://ipc.localhost'));
      expect(sources.filter((s) => s.startsWith('http') || s === '*'), `${d} reaches the network`).toEqual([]);
    }
  });
});
