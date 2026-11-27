// The mascot's model is the app's one binary asset with a hard requirement: it must load with
// nothing but a plain GLTFLoader.
//
// The app shipped a Draco-compressed model once. three.js decodes Draco in a Worker built from a
// `blob:` URL, which WebKit — the engine of the macOS app — refuses under the app's content policy,
// so the 3D mascot silently degraded to the flat drawing on every Mac. The model now uses
// KHR_mesh_quantization, which three.js reads natively, and this test keeps it that way.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const MODEL = fileURLToPath(new URL('../apps/desktop/public/ubi/Ubi.glb', import.meta.url));

/** The JSON chunk of a binary glTF. */
function gltfJson(path) {
  const buf = readFileSync(path);
  assert.equal(buf.toString('utf8', 0, 4), 'glTF', 'not a .glb');
  let off = 12;
  while (off < buf.length) {
    const len = buf.readUInt32LE(off);
    const type = buf.readUInt32LE(off + 4);
    if (type === 0x4e4f534a) return JSON.parse(buf.toString('utf8', off + 8, off + 8 + len));
    off += 8 + len + (len % 4 ? 4 - (len % 4) : 0);
  }
  throw new Error('no JSON chunk');
}

const model = gltfJson(MODEL);

test('needs no decoder the app does not ship', () => {
  const required = model.extensionsRequired ?? [];
  assert.ok(!required.includes('KHR_draco_mesh_compression'), `Draco is back: ${required.join(', ')}`);
  assert.ok(!required.includes('EXT_meshopt_compression'), `meshopt needs a decoder: ${required.join(', ')}`);
  // Everything left has to be an extension three.js reads on its own.
  const native = ['KHR_mesh_quantization', 'KHR_materials_unlit', 'KHR_texture_transform'];
  for (const ext of required) assert.ok(native.includes(ext), `unknown required extension: ${ext}`);
});

test('keeps the rig and the animation clips', () => {
  assert.equal(model.skins?.length, 1, 'the skeleton is gone');
  assert.equal(model.skins[0].joints.length, 21, 'the joint count changed');
  assert.equal(model.nodes.filter((n) => 'skin' in n).length, 1, 'no node wears the skin');
  const clips = (model.animations ?? []).map((a) => a.name).sort();
  assert.deepEqual(clips, ['Excited', 'Idle', 'Jump', 'No', 'Sleep', 'Wave', 'Worried', 'Yes']);
});
