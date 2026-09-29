// Deterministic UBI renderer (page side). Driven by render.mjs through window.UBI.
//
// Mirrors the product's 3D UBI (apps/desktop/src/components/ubi/Ubi3d.tsx and site/src/components/UbiHero3d.tsx):
//  - GLTFLoader, model normalised like `normalise()`: largest dimension → FIT (2.4) units, centred on x/z, feet on y = 0
//  - renderer: alpha + antialias + premultipliedAlpha (MSAA edges resolve to premultiplied coverage → no dark fringe),
//    transparent clear, sRGB output, ACES Filmic tone mapping, exposure 1.05
//  - lights: RoomEnvironment IBL (PMREM sigma 0.04, environmentIntensity 0.8), white key directional (1.6) from the
//    top right, volt (#4d8dff, the "calm" mood glow) point fill from below (7, distance 7, decay 2) and a faint volt
//    rim directional (0.5) from behind
//  - look-at: the rig's rotateInWorld() layered on top of the clip pose, 30 % Neck / 70 % Head, world yaw then pitch
//  - camera: fov 30 like the product, but framed for a square frame (see fitCamera)
// Time never comes from requestAnimationFrame: every frame sets the clip time explicitly.
import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';

const GLB_URL = '/apps/desktop/public/ubi/Ubi.glb';
const FIT = 2.4;
const GLOW = '#4d8dff';
const HEAD_SHARE = 0.7;
const FOV = 30;
/** Product camera: (0, 1.55, 5.6) looking at (0, 1.2, 0) → ~3.6° downward pitch. Kept for the same perspective. */
const PITCH = Math.atan2(1.55 - FIT * 0.5, 5.6);

let renderer, scene, camera, model, holder, clips, rest, restOf, mesh, glCanvas, outCanvas, outCtx;
let head = null;
let neck = null;
const cam = { ty: FIT * 0.5, d: 5.6 };

function findBone(root, name) {
  const wanted = new Set([name, THREE.PropertyBinding.sanitizeNodeName(name)]);
  let bone = null;
  let any = null;
  root.traverse((o) => {
    if (!wanted.has(o.name)) return;
    if (o.isBone) bone ??= o;
    else any ??= o;
  });
  return bone ?? any;
}

const _parent = new THREE.Quaternion();
const _rot = new THREE.Quaternion();
const _delta = new THREE.Quaternion();
const _euler = new THREE.Euler();
/** Same as rig.ts rotateInWorld: turn `node` by yaw/pitch degrees about the WORLD axes, keeping the clip pose. */
function rotateInWorld(node, yaw, pitch) {
  if (yaw === 0 && pitch === 0) return;
  _euler.set(-THREE.MathUtils.degToRad(pitch), THREE.MathUtils.degToRad(yaw), 0, 'YXZ');
  _rot.setFromEuler(_euler);
  if (node.parent) node.parent.getWorldQuaternion(_parent);
  else _parent.identity();
  _delta.copy(_parent).invert().multiply(_rot).multiply(_parent);
  node.quaternion.premultiply(_delta);
}

function normalise(object) {
  object.rotation.set(0, 0, 0);
  object.position.set(0, 0, 0);
  object.scale.setScalar(1);
  object.traverse((node) => {
    if (node.isMesh) node.frustumCulled = false;
  });
  object.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(object);
  const size = box.getSize(new THREE.Vector3());
  const center = box.getCenter(new THREE.Vector3());
  const s = FIT / Math.max(size.x, size.y, size.z, 1e-3);
  object.scale.setScalar(s);
  object.position.set(-center.x * s, -box.min.y * s, -center.z * s);
  object.updateMatrixWorld(true);
  return { size: size.multiplyScalar(s).toArray() };
}

function setSize(w, h, ss) {
  renderer.setPixelRatio(1);
  renderer.setSize(w * ss, h * ss, false);
  outCanvas.width = w;
  outCanvas.height = h;
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}

function placeCamera() {
  const dir = new THREE.Vector3(0, Math.sin(PITCH), Math.cos(PITCH));
  camera.position.set(0, cam.ty, 0).addScaledVector(dir, cam.d);
  camera.lookAt(0, cam.ty, 0);
  camera.updateMatrixWorld(true);
}

function restoreRest() {
  for (const r of rest) {
    r.node.position.copy(r.p);
    r.node.quaternion.copy(r.q);
    r.node.scale.copy(r.s);
  }
}

/**
 * Stateless clip sampler: every track of the clip is evaluated with its own interpolant (LINEAR → slerp/lerp, STEP →
 * discrete, exactly what AnimationMixer does for one action at weight 1) and written straight to the node. No mixer:
 * its PropertyMixer only writes a binding when the value changed since its previous update, so with the rest pose
 * restored every frame, constant channels (the STEP holds on Neck / Shoulder.L / Shoulder.R in Worried and Sleep)
 * would fall back to rest after the first frame.
 */
const samplers = new Map();
function samplerFor(name) {
  let list = samplers.get(name);
  if (list) return list;
  list = [];
  for (const track of clips.get(name).tracks) {
    const parsed = THREE.PropertyBinding.parseTrackName(track.name);
    const node = THREE.PropertyBinding.findNode(model, parsed.nodeName);
    const prop = parsed.propertyName;
    if (!node || !['position', 'quaternion', 'scale'].includes(prop)) {
      console.warn(`unsupported track ${track.name}`);
      continue;
    }
    list.push({ node, prop, interp: track.createInterpolant(), track });
  }
  samplers.set(name, list);
  return list;
}

/** Evaluates `name` at `time` (clamped to the clip) into a Map node -> { position?, quaternion?, scale? }. */
function evalClip(name, time) {
  const out = new Map();
  const duration = clips.get(name).duration;
  const t = Math.min(Math.max(time, 0), duration);
  for (const { node, prop, interp } of samplerFor(name)) {
    const v = interp.evaluate(t);
    let rec = out.get(node);
    if (!rec) out.set(node, (rec = {}));
    rec[prop] = prop === 'quaternion' ? new THREE.Quaternion().fromArray(v).normalize() : new THREE.Vector3().fromArray(v);
  }
  return out;
}

/**
 * Poses the model: clip `clip` at `time` seconds (none → rest pose), optionally cross-faded with `mix` = { clip,
 * time, w } (w = weight of the mix clip; a channel missing from one side uses the rest pose, as AnimationMixer does),
 * model yaw `rotY` radians about the vertical axis, head look {yaw, pitch} in degrees (yaw > 0 = viewer's right,
 * pitch > 0 = up).
 */
function pose({ clip = null, time = 0, mix = null, rotY = 0, look = null } = {}) {
  restoreRest();
  if (mix && mix.w > 0) {
    const A = clip ? evalClip(clip, time) : new Map();
    const B = evalClip(mix.clip, mix.time);
    const w = mix.w;
    for (const node of new Set([...A.keys(), ...B.keys()])) {
      const r = restOf.get(node);
      const a = A.get(node) ?? {};
      const b = B.get(node) ?? {};
      if (a.quaternion || b.quaternion) node.quaternion.copy(a.quaternion ?? r.q).slerp(b.quaternion ?? r.q, w);
      if (a.position || b.position) node.position.copy(a.position ?? r.p).lerp(b.position ?? r.p, w);
      if (a.scale || b.scale) node.scale.copy(a.scale ?? r.s).lerp(b.scale ?? r.s, w);
    }
  } else if (clip) {
    for (const [node, rec] of evalClip(clip, time)) {
      if (rec.quaternion) node.quaternion.copy(rec.quaternion);
      if (rec.position) node.position.copy(rec.position);
      if (rec.scale) node.scale.copy(rec.scale);
    }
  }
  holder.rotation.set(0, rotY, 0);
  holder.updateMatrixWorld(true);
  if (look && (look.yaw || look.pitch)) {
    const neckShare = head && neck ? 1 - HEAD_SHARE : 1;
    const headShare = head && neck ? HEAD_SHARE : 1;
    if (neck) rotateInWorld(neck, look.yaw * neckShare, look.pitch * neckShare);
    if (head) rotateInWorld(head, look.yaw * headShare, look.pitch * headShare);
  }
  holder.updateMatrixWorld(true);
}

/** NDC box of the union of `poses` with the current camera (every `stride`-th vertex). */
function measure(poses, stride = 2) {
  let u = EMPTY;
  for (const p of poses) {
    pose(p);
    u = union(u, ndcOf(worldPoints(stride)));
  }
  return u;
}

const _v = new THREE.Vector3();
/** World-space points of the skinned mesh (every `stride`-th vertex) in the current pose. */
function worldPoints(stride) {
  mesh.skeleton.update();
  const n = mesh.geometry.attributes.position.count;
  const out = new Float32Array(Math.ceil(n / stride) * 3);
  let k = 0;
  for (let i = 0; i < n; i += stride) {
    mesh.getVertexPosition(i, _v);
    _v.applyMatrix4(mesh.matrixWorld);
    out[k++] = _v.x;
    out[k++] = _v.y;
    out[k++] = _v.z;
  }
  return out;
}

function ndcOf(points) {
  const b = { x0: Infinity, x1: -Infinity, y0: Infinity, y1: -Infinity };
  for (let k = 0; k < points.length; k += 3) {
    _v.set(points[k], points[k + 1], points[k + 2]).project(camera);
    if (_v.x < b.x0) b.x0 = _v.x;
    if (_v.x > b.x1) b.x1 = _v.x;
    if (_v.y < b.y0) b.y0 = _v.y;
    if (_v.y > b.y1) b.y1 = _v.y;
  }
  return b;
}

const union = (a, b) => ({ x0: Math.min(a.x0, b.x0), x1: Math.max(a.x1, b.x1), y0: Math.min(a.y0, b.y0), y1: Math.max(a.y1, b.y1) });
const EMPTY = { x0: Infinity, x1: -Infinity, y0: Infinity, y1: -Infinity };

function solve(sets, room, iters) {
  for (let it = 0; it < iters; it++) {
    placeCamera();
    let u = EMPTY;
    for (const s of sets) u = union(u, ndcOf(s));
    const halfH = cam.d * Math.tan(THREE.MathUtils.degToRad(FOV / 2));
    cam.ty += ((u.y0 + u.y1) / 2) * halfH;
    cam.d *= Math.max((u.y1 - u.y0) / 2 / room, Math.max(Math.abs(u.x0), Math.abs(u.x1)) / room);
  }
  placeCamera();
}

/**
 * Frames every pose in `poses` with `padding` (fraction of the frame on each side): camera kept on x = 0 (so a
 * turntable spins about the frame centre) with the product's fov and downward pitch; target height and distance
 * are solved so the union of the silhouettes is vertically centred and its larger extent fills 1 - 2·padding.
 * Coarse pass on every 4th vertex of every pose, then the poses that touch the union's edges are redone with every
 * vertex (the spike tips are single vertices) and the camera is solved again on those.
 */
function fitCamera({ poses, padding = 0.08, stride = 4 }) {
  const room = 1 - 2 * padding;
  cam.ty = FIT * 0.5;
  cam.d = 5.6;
  const coarse = poses.map((p) => {
    pose(p);
    return worldPoints(stride);
  });
  solve(coarse, room, 8);
  // which poses define the edges?
  const boxes = coarse.map(ndcOf);
  const u = boxes.reduce(union, EMPTY);
  const tol = 0.03;
  const edge = [];
  boxes.forEach((b, i) => {
    if (b.x0 < u.x0 + tol || b.x1 > u.x1 - tol || b.y0 < u.y0 + tol || b.y1 > u.y1 - tol) edge.push(i);
  });
  const exact = edge.map((i) => {
    pose(poses[i]);
    return worldPoints(1);
  });
  solve([...exact], room, 6);
  let ndc = EMPTY;
  for (const s of exact) ndc = union(ndc, ndcOf(s));
  for (const b of coarse.map(ndcOf)) ndc = union(ndc, b);
  const g = new THREE.Vector3(0, 0, 0).project(camera);
  return {
    camera: { ty: cam.ty, d: cam.d, fov: FOV, pitchDeg: THREE.MathUtils.radToDeg(PITCH), position: camera.position.toArray(), target: [0, cam.ty, 0] },
    ndc,
    edgePoses: edge.length,
    groundNdcY: g.y,
  };
}

function headShell() {
  const bi = mesh.skeleton.bones.findIndex((b) => b.name === 'Head');
  if (bi < 0) return null;
  mesh.skeleton.update();
  const si = mesh.geometry.attributes.skinIndex;
  const sw = mesh.geometry.attributes.skinWeight;
  const box = new THREE.Box3();
  for (let i = 0; i < si.count; i++) {
    let w = 0;
    for (let c = 0; c < 4; c++) if (si.getComponent(i, c) === bi) w += sw.getComponent(i, c);
    if (w < 0.99) continue;
    mesh.getVertexPosition(i, _v);
    _v.applyMatrix4(mesh.matrixWorld);
    box.expandByPoint(_v);
  }
  if (box.isEmpty()) return null;
  const center = box.getCenter(new THREE.Vector3());
  return { center, top: new THREE.Vector3(center.x, box.max.y, center.z) };
}

/** NDC of a node's world position in `spec`'s pose (anchors for the compositor: head, orb, feet…). */
function anchors(spec, names) {
  pose(spec);
  const out = {};
  for (const n of names) {
    const node = findBone(model, n);
    if (!node) continue;
    node.getWorldPosition(_v).project(camera);
    out[n] = [_v.x, _v.y];
  }
  // centre and top of the head shell (vertices mostly weighted to Head): where a speech bubble should point
  const head = headShell();
  if (head) {
    _v.copy(head.center).project(camera);
    out.headCenter = [_v.x, _v.y];
    _v.copy(head.top).project(camera);
    out.headTop = [_v.x, _v.y];
  }
  out.ground = (() => {
    _v.set(0, 0, 0).project(camera);
    return [_v.x, _v.y];
  })();
  return out;
}

function capture() {
  renderer.render(scene, camera);
  // premultiplied WebGL canvas → 2D canvas (also premultiplied): the (super-sampled) resize filters colour and
  // coverage together; toDataURL un-premultiplies into a straight-alpha PNG.
  outCtx.clearRect(0, 0, outCanvas.width, outCanvas.height);
  outCtx.imageSmoothingEnabled = true;
  outCtx.imageSmoothingQuality = 'high';
  outCtx.drawImage(glCanvas, 0, 0, outCanvas.width, outCanvas.height);
  return outCanvas.toDataURL('image/png');
}

async function init({ width = 900, height = 900, ss = 1, fixStrays = true } = {}) {
  glCanvas = document.createElement('canvas');
  document.body.appendChild(glCanvas);
  outCanvas = document.createElement('canvas');
  outCtx = outCanvas.getContext('2d', { alpha: true });
  renderer = new THREE.WebGLRenderer({ canvas: glCanvas, alpha: true, antialias: true, premultipliedAlpha: true, preserveDrawingBuffer: true });
  renderer.setClearColor(0x000000, 0);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.05;

  scene = new THREE.Scene();
  camera = new THREE.PerspectiveCamera(FOV, width / height, 0.1, 40);
  setSize(width, height, ss);

  const pmrem = new THREE.PMREMGenerator(renderer);
  const room = new RoomEnvironment();
  const env = pmrem.fromScene(room, 0.04);
  room.dispose();
  pmrem.dispose();
  scene.environment = env.texture;
  scene.environmentIntensity = 0.8;

  const key = new THREE.DirectionalLight('#ffffff', 1.6);
  key.position.set(2.5, 5, 3.5);
  const fill = new THREE.PointLight(GLOW, 7, 7, 2);
  fill.position.set(0, -1.4, 2.2);
  const rim = new THREE.DirectionalLight(GLOW, 0.5);
  rim.position.set(-3, 2.5, -3);
  scene.add(key, fill, rim);

  const gltf = await new GLTFLoader().loadAsync(GLB_URL);
  model = gltf.scene;
  const info = normalise(model);
  // lights stay fixed; the turntable spins this holder (the model is centred on x/z, so it spins in place)
  holder = new THREE.Group();
  holder.add(model);
  scene.add(holder);
  model.traverse((o) => {
    if (o.isSkinnedMesh) mesh = o;
  });
  head = findBone(model, 'Head');
  neck = findBone(model, 'Neck');
  rest = [];
  model.traverse((o) => rest.push({ node: o, p: o.position.clone(), q: o.quaternion.clone(), s: o.scale.clone() }));
  restOf = new Map(rest.map((r) => [r.node, r]));
  clips = new Map(gltf.animations.map((c) => [c.name, c]));
  const bones = mesh ? mesh.skeleton.bones.map((b) => b.name) : [];
  const fixes = {};
  if (fixStrays && mesh) {
    // the Orb floats next to the left hand and is its own bone: it is allowed there
    fixes.HandR = claimStrays('HandR', ['LowerArmR', 'UpperArmR'], 0.22);
    fixes.HandL = claimStrays('HandL', ['LowerArmL', 'UpperArmL', 'Orb'], 0.22);
  }
  const gl = renderer.getContext();
  const dbg = gl.getExtension('WEBGL_debug_renderer_info');
  return {
    three: THREE.REVISION,
    gl: dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER),
    samples: gl.getParameter(gl.SAMPLES),
    size: info.size,
    clips: gltf.animations.map((c) => ({ name: c.name, duration: c.duration, tracks: c.tracks.map((t) => t.name) })),
    bones,
    fixes,
    head: head?.name ?? null,
    neck: neck?.name ?? null,
    materials: [...new Set((Array.isArray(mesh.material) ? mesh.material : [mesh.material]).map((m) => `${m.type}:${m.name}`))],
  };
}

// ---------------------------------------------------------------------------------------------------------------
// Skin diagnostics / repair
// ---------------------------------------------------------------------------------------------------------------

/**
 * Stray-weight repair (video-only; the product GLB is untouched). scripts/ubi-rig/rig_ubi.py splits the body into
 * regions with planar cuts and gives each vertex the nearest bone of its region; the tip of the right thumb crosses
 * the arm cut, so 250 of its vertices are 100 % `Shoulder.R` although they sit on the hand ~0.4 units in front of
 * the shoulder. When `Wave` turns the hand, that tip stays pinned to the shoulder and the thumb stretches into a long
 * white bar. Every vertex within `radius` of the hand bone whose dominant bone is not part of that arm is given to
 * the hand (100 %). The rest pose is unchanged (bind pose skinning is the identity for any weights).
 */
function claimStrays(handName, allowedNames, radius) {
  const bones = mesh.skeleton.bones;
  const hi = bones.findIndex((b) => b.name === handName);
  if (hi < 0) return null;
  const allowed = new Set(allowedNames.map((n) => bones.findIndex((b) => b.name === n)).filter((i) => i >= 0));
  allowed.add(hi);
  pose({});
  mesh.skeleton.update();
  const hand = bones[hi].getWorldPosition(new THREE.Vector3());
  const si = mesh.geometry.attributes.skinIndex;
  const sw = mesh.geometry.attributes.skinWeight;
  const from = {};
  for (let i = 0; i < si.count; i++) {
    let dom = -1;
    let dw = -1;
    for (let c = 0; c < 4; c++) {
      const w = sw.getComponent(i, c);
      if (w > dw) {
        dw = w;
        dom = si.getComponent(i, c);
      }
    }
    if (allowed.has(dom)) continue;
    mesh.getVertexPosition(i, _v);
    _v.applyMatrix4(mesh.matrixWorld);
    if (_v.distanceTo(hand) > radius) continue;
    for (let c = 0; c < 4; c++) {
      si.setComponent(i, c, c === 0 ? hi : 0);
      sw.setComponent(i, c, c === 0 ? 1 : 0);
    }
    const name = bones[dom].name;
    from[name] = (from[name] || 0) + 1;
  }
  si.needsUpdate = true;
  sw.needsUpdate = true;
  return from;
}

/** Debug: dominant bones of the vertices within `radius` of the segment from bone `a` to bone `b` (rest pose). */
function debugNear(aName, bName, radius) {
  const bones = mesh.skeleton.bones;
  const A = bones.find((x) => x.name === aName).getWorldPosition(new THREE.Vector3());
  const B = bName ? bones.find((x) => x.name === bName).getWorldPosition(new THREE.Vector3()) : A.clone();
  pose({});
  mesh.skeleton.update();
  const names = bones.map((x) => x.name);
  const seg = new THREE.Line3(A, B);
  const q = new THREE.Vector3();
  const out = {};
  const n = mesh.geometry.attributes.position.count;
  for (let i = 0; i < n; i++) {
    mesh.getVertexPosition(i, _v);
    _v.applyMatrix4(mesh.matrixWorld);
    seg.closestPointToPoint(_v, true, q);
    if (q.distanceTo(_v) > radius) continue;
    const bw = boneWeights(i);
    const key = Object.entries(bw)
      .sort((x, y) => y[1] - x[1])
      .map(([k, w]) => `${names[k]}:${w.toFixed(2)}`)
      .join(' ');
    const dom = names[Object.entries(bw).sort((x, y) => y[1] - x[1])[0][0]];
    out[dom] ??= { count: 0, box: new THREE.Box3(), samples: new Set() };
    out[dom].count++;
    out[dom].box.expandByPoint(_v);
    if (out[dom].samples.size < 4) out[dom].samples.add(key);
  }
  return { A: A.toArray(), B: B.toArray(), groups: Object.fromEntries(Object.entries(out).map(([k, v]) => [k, { count: v.count, min: v.box.min.toArray().map((x) => +x.toFixed(3)), max: v.box.max.toArray().map((x) => +x.toFixed(3)), samples: [...v.samples] }])) };
}

/** Connected components of the mesh after welding vertices by position (UV seams duplicate vertices). */
function islands() {
  const pos = mesh.geometry.attributes.position;
  const n = pos.count;
  const key = new Map();
  const weld = new Int32Array(n);
  const p = new THREE.Vector3();
  for (let i = 0; i < n; i++) {
    p.fromBufferAttribute(pos, i);
    const k = `${Math.round(p.x * 1e4)},${Math.round(p.y * 1e4)},${Math.round(p.z * 1e4)}`;
    let w = key.get(k);
    if (w === undefined) key.set(k, (w = i));
    weld[i] = w;
  }
  const parent = new Int32Array(n);
  for (let i = 0; i < n; i++) parent[i] = i;
  const find = (x) => {
    while (parent[x] !== x) x = parent[x] = parent[parent[x]];
    return x;
  };
  const join = (a, b) => {
    a = find(a);
    b = find(b);
    if (a !== b) parent[a] = b;
  };
  const idx = mesh.geometry.index;
  const tri = idx ? idx.count : n;
  for (let t = 0; t < tri; t += 3) {
    const a = weld[idx ? idx.getX(t) : t];
    const b = weld[idx ? idx.getX(t + 1) : t + 1];
    const c = weld[idx ? idx.getX(t + 2) : t + 2];
    join(a, b);
    join(b, c);
  }
  const comp = new Int32Array(n);
  for (let i = 0; i < n; i++) comp[i] = find(weld[i]);
  return comp;
}

function boneWeights(i) {
  const si = mesh.geometry.attributes.skinIndex;
  const sw = mesh.geometry.attributes.skinWeight;
  const out = {};
  for (let c = 0; c < 4; c++) {
    const w = sw.getComponent(i, c);
    if (w > 0) out[si.getComponent(i, c)] = (out[si.getComponent(i, c)] || 0) + w;
  }
  return out;
}

/**
 * Per island: vertex count, bones it is weighted to, and the worst edge strain (skinned edge length / rest edge
 * length) over `poses`. Hard-surface parts should stay at ~1.
 */
function skinReport(poses, { minStrain = 1.15 } = {}) {
  const comp = islands();
  const pos = mesh.geometry.attributes.position;
  const n = pos.count;
  const idx = mesh.geometry.index;
  const groups = new Map();
  for (let i = 0; i < n; i++) {
    let g = groups.get(comp[i]);
    if (!g) groups.set(comp[i], (g = { id: comp[i], verts: 0, bones: {}, strain: 1, at: null }));
    g.verts++;
    const bw = boneWeights(i);
    for (const b in bw) g.bones[b] = (g.bones[b] || 0) + bw[b];
  }
  // rest edge lengths (unique undirected edges, sampled)
  const edges = [];
  for (let t = 0; t < idx.count; t += 3) {
    const a = idx.getX(t);
    const b = idx.getX(t + 1);
    const c = idx.getX(t + 2);
    edges.push(a, b, b, c, c, a);
  }
  const vcount = n;
  const restP = new Float32Array(vcount * 3);
  pose({});
  mesh.skeleton.update();
  for (let i = 0; i < vcount; i++) {
    mesh.getVertexPosition(i, _v);
    restP[i * 3] = _v.x;
    restP[i * 3 + 1] = _v.y;
    restP[i * 3 + 2] = _v.z;
  }
  const P = new Float32Array(vcount * 3);
  for (const spec of poses) {
    pose(spec);
    mesh.skeleton.update();
    for (let i = 0; i < vcount; i++) {
      mesh.getVertexPosition(i, _v);
      P[i * 3] = _v.x;
      P[i * 3 + 1] = _v.y;
      P[i * 3 + 2] = _v.z;
    }
    for (let e = 0; e < edges.length; e += 2) {
      const a = edges[e] * 3;
      const b = edges[e + 1] * 3;
      const r = Math.hypot(restP[a] - restP[b], restP[a + 1] - restP[b + 1], restP[a + 2] - restP[b + 2]);
      if (r < 1e-3) continue;
      const d = Math.hypot(P[a] - P[b], P[a + 1] - P[b + 1], P[a + 2] - P[b + 2]);
      const s = d / r;
      const g = groups.get(comp[edges[e]]);
      if (s > g.strain) {
        g.strain = s;
        g.at = spec;
      }
    }
  }
  const names = mesh.skeleton.bones.map((b) => b.name);
  return [...groups.values()]
    .filter((g) => g.strain >= minStrain)
    .sort((a, b) => b.strain - a.strain)
    .map((g) => ({
      id: g.id,
      verts: g.verts,
      strain: +g.strain.toFixed(2),
      at: g.at,
      bones: Object.fromEntries(
        Object.entries(g.bones)
          .sort((a, b) => b[1] - a[1])
          .map(([k, v]) => [names[k], +(v / g.verts).toFixed(3)]),
      ),
    }));
}

/**
 * Self-test: the stateless sampler against a fresh AnimationMixer per sample (fresh PropertyMixer buffers, so its
 * change-detection cannot skip a write). Returns the largest difference over every node transform.
 */
function selfTest(samples = 12) {
  let worst = { d: 0 };
  const snap = () => rest.map((r) => [...r.node.position.toArray(), ...r.node.quaternion.toArray(), ...r.node.scale.toArray()]);
  for (const [name, clip] of clips) {
    for (let k = 0; k < samples; k++) {
      const t = (clip.duration * k) / samples; // < duration: LoopRepeat would wrap at the end
      pose({ clip: name, time: t });
      const a = snap();
      restoreRest();
      const m = new THREE.AnimationMixer(model);
      m.clipAction(clip).play();
      m.setTime(t);
      const b = snap();
      m.stopAllAction();
      m.uncacheRoot(model);
      a.forEach((row, i) =>
        row.forEach((v, j) => {
          // quaternion sign is free (q and -q are the same rotation)
          const d = j >= 3 && j < 7 ? Math.min(Math.abs(v - b[i][j]), Math.abs(v + b[i][j])) : Math.abs(v - b[i][j]);
          if (d > worst.d) worst = { d, clip: name, t, node: rest[i].node.name, j };
        }),
      );
    }
  }
  restoreRest();
  return worst;
}

function render(spec) {
  pose(spec);
  return capture();
}

window.UBI = {
  init,
  setSize: ({ width, height, ss = 1 }) => {
    setSize(width, height, ss);
    placeCamera();
  },
  fitCamera,
  measure,
  anchors,
  setCamera: (c) => {
    cam.ty = c.ty;
    cam.d = c.d;
    placeCamera();
  },
  render,
  skinReport,
  selfTest,
  debugNear,
};
window.UBI_READY = true;
