#!/usr/bin/env node
// Renders the 3D mascot UBI as transparent PNG sequences for the launch video.
//
//   node tools/ubi-render/render.mjs                 # everything (clips, turntable, look, stills, manifest)
//   node tools/ubi-render/render.mjs --only wave,jump # re-render some sequences (same camera, manifest merged)
//   node tools/ubi-render/render.mjs --stills-only   # only the hero stills
//   options: --size 900 --ss 2 --stills-size 1400 --padding 0.08 --no-stills --dry (fit + manifest, no frames)
//            --no-fix (keep the GLB's stray thumb weights; see skin_repair in the manifest)
//            --refit (with --only: refit the camera instead of reusing the manifest's — then re-render everything)
//
// Serves the repo root on 127.0.0.1:1450 (python3 -m http.server; reused when already up, stopped by PID when this
// script started it), opens tools/ubi-render/index.html in headless Chromium with SwiftShader WebGL and steps every
// frame explicitly (clip time = i / 30 s), so the output is deterministic. Frames: public/ubi/<dir>/NNNN.png.
import pw from '/home/user/ubiquitous-engine/site/node_modules/playwright/index.js';
import { spawn } from 'node:child_process';
import { mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const { chromium } = pw;
const HERE = dirname(fileURLToPath(import.meta.url));
const LAUNCH = resolve(HERE, '../..');
const REPO = resolve(LAUNCH, '../..');
const PUBLIC = join(LAUNCH, 'public');
const OUT = join(PUBLIC, 'ubi');
const PORT = 1450;
const ORIGIN = `http://127.0.0.1:${PORT}`;
const PAGE = `${ORIGIN}/video/launch/tools/ubi-render/index.html`;
const CHROME = process.env.UBI_CHROME || '/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell';
const FPS = 30;
/** Cross-fade length of the transition sequences, frames (the app's CROSSFADE is 0.35 s). */
const XF = 10;

const args = process.argv.slice(2);
const opt = (name, def) => {
  const i = args.indexOf(`--${name}`);
  return i >= 0 ? args[i + 1] : def;
};
const flag = (name) => args.includes(`--${name}`);
const SIZE = Number(opt('size', 900));
const SS = Number(opt('ss', 2));
const STILL_SIZE = Number(opt('stills-size', 1400));
const PADDING = Number(opt('padding', 0.08));
const ONLY = opt('only', '')
  .split(',')
  .map((s) => s.trim())
  .filter(Boolean);
const STILLS_ONLY = flag('stills-only');
const NO_STILLS = flag('no-stills');
const DRY = flag('dry');
const FIX = !flag('no-fix');

// ---------------------------------------------------------------------------------------------------------------
// Sequences
// ---------------------------------------------------------------------------------------------------------------

/** Clip facts from the GLB (durations are the glTF sampler ranges: 24 fps keys, 1.2 s clips are 29/24 s). */
const CLIP_INFO = {
  Idle: {
    loop: true,
    description: 'Breathing idle: chest rise, slight weight shift, micro head sway (<= 4 deg), orb bobbing and orbiting near the left hand.',
    best_use: 'Default presence. Hold UBI beside headlines, UI shots or the end card; loop it under any other element.',
  },
  Yes: {
    loop: false,
    description: 'Two approving nods (head pitch), starts and ends on the rest pose.',
    best_use: 'Agreement beat: a checkmark, "Confirmar os N", a positive stat or a feature that "just works".',
  },
  No: {
    loop: false,
    description: 'Two head shakes (yaw +/-12 deg), starts and ends on the rest pose.',
    best_use: 'Problem framing: "sem planilha", "sem cronometro manual", "sem digitar horas" — say no to the old way.',
  },
  Wave: {
    loop: false,
    description: 'Right forearm waves outwards, head tilts; starts and ends on the rest pose.',
    best_use: 'Intro "Oi, eu sou o UBI" or the goodbye on the end card / CTA.',
  },
  Jump: {
    loop: false,
    description: 'Crouch, jump (~0.2 units), land with squash; starts and ends on the rest pose.',
    best_use: 'Celebration hit on a downbeat: launch reveal, "relatorio pronto", price reveal.',
  },
  Excited: {
    loop: true,
    description: 'Small bounces, pumping arms, fast orbiting orb (the "Empolgado" mood).',
    best_use: 'High-energy loop: focus score >= 85, feature montage peaks, the CTA.',
  },
  Worried: {
    loop: true,
    description: 'Shoulders raised, quick 4 Hz tremble in head and chest, orb pulled close (the "Preocupado" mood).',
    best_use: 'Problem framing: context switching, distractions, a budget alert at 80 %.',
  },
  Sleep: {
    loop: true,
    description: 'Chest slumped ~12 deg, head down and to the side, slow breathing, orb low (the "Descansando" mood).',
    best_use: 'Idle time / no activity / after hours; a calm beat before the reveal ("acorda" -> Jump).',
  },
};

const lower = (s) => s.toLowerCase();

/** Frame plan for a glTF clip at 30 fps. */
function clipPlan(name, duration) {
  const info = CLIP_INFO[name] ?? { loop: false, description: '', best_use: '' };
  if (info.loop) {
    // a loop's last key equals its first: N frames cover [0, duration) and frame N would be frame 0 again.
    // A duration that is not a whole number of frames (Excited: 29/24 s = 36.25 frames) is retimed to the nearest
    // whole count so the loop closes exactly (36 frames, 0.7 % faster).
    const n = Math.max(1, Math.round(duration * FPS));
    const step = duration / n;
    return {
      frames: n,
      at: (i) => ({ clip: name, time: i * step }),
      loopable: true,
      // playback speed vs the original clip (1.0069 = 0.69 % faster)
      retime: Math.abs(step * FPS - 1) > 1e-6 ? +(step * FPS).toFixed(4) : 1,
    };
  }
  // one-shot: frames 0..ceil(duration*30), the last one clamped on the clip's end (the rest pose again)
  const n = Math.ceil(duration * FPS - 1e-6) + 1;
  return { frames: n, at: (i) => ({ clip: name, time: Math.min(i / FPS, duration) }), loopable: false, retime: 1 };
}

// head look (degrees; yaw > 0 = viewer's right, pitch > 0 = up), eased like the app (exponential, rate 8/s)
const LOOK_DAMPING = 8;
const LOOK_KEYS = [
  // [from second, yaw, pitch]
  [0.0, 0, 0],
  [0.2, -38, -3], // left
  [0.95, 0, 0], // center
  [1.45, 38, -3], // right
  [2.05, 28, 22], // up-right: where a speech bubble sits (above the head, to the viewer's right)
];
function lookTrack(frames) {
  const cur = { yaw: 0, pitch: 0 };
  const out = [];
  const dt = 1 / FPS;
  for (let i = 0; i < frames; i++) {
    const t = i / FPS;
    let key = LOOK_KEYS[0];
    for (const k of LOOK_KEYS) if (t >= k[0]) key = k;
    if (i > 0) {
      const f = Math.exp(-LOOK_DAMPING * dt);
      cur.yaw = key[1] + (cur.yaw - key[1]) * f;
      cur.pitch = key[2] + (cur.pitch - key[2]) * f;
    }
    out.push({ yaw: +cur.yaw.toFixed(4), pitch: +cur.pitch.toFixed(4) });
  }
  return out;
}

function sequences(clips) {
  const list = [];
  const order = ['Idle', 'Yes', 'No', 'Wave', 'Jump', 'Excited', 'Worried', 'Sleep'];
  const byName = new Map(clips.map((c) => [c.name, c]));
  for (const name of [...order.filter((n) => byName.has(n)), ...clips.map((c) => c.name).filter((n) => !order.includes(n))]) {
    const c = byName.get(name);
    const plan = clipPlan(name, c.duration);
    const info = CLIP_INFO[name] ?? {};
    list.push({
      name: lower(name),
      dir: `ubi/${lower(name)}`,
      source: { clip: name, clip_duration_s: +c.duration.toFixed(4), retime: plan.retime },
      frames: plan.frames,
      loopable: plan.loopable,
      at: plan.at,
      description: info.description ?? '',
      best_use: info.best_use ?? '',
    });
  }
  const idle = byName.get('Idle');
  const idleAt = (i) => (idle ? { clip: 'Idle', time: (i / FPS) % idle.duration } : {});
  // turntable: 4 s, one full turn, Idle playing underneath (Idle is 4 s too, so turn and clip both loop at 120)
  const TT = 4 * FPS;
  list.push({
    name: 'turntable',
    dir: 'ubi/turntable',
    source: { clip: 'Idle', rotation: '360 deg about the vertical axis, counter-clockwise seen from above (UBI turns to his left = viewer\'s right first)' },
    frames: TT,
    loopable: true,
    at: (i) => ({ ...idleAt(i), rotY: (2 * Math.PI * i) / TT }),
    description: 'Slow 360 deg turntable on the idle pose (Idle keeps breathing underneath), constant angular speed.',
    best_use: 'Mascot reveal / "conheca o UBI" showcase; slow it down or use a slice for a parallax-like turn.',
  });
  // look: 3 s head look-at layered over Idle (Head 70 % / Neck 30 %, like the app's rig)
  const LOOK = 3 * FPS;
  const track = lookTrack(LOOK);
  list.push({
    name: 'look',
    dir: 'ubi/look',
    source: { clip: 'Idle', look_keys: LOOK_KEYS.map(([t, yaw, pitch]) => ({ t, yaw, pitch })), easing: `exponential, rate ${LOOK_DAMPING}/s (app's LOOK_DAMPING)` },
    frames: LOOK,
    loopable: false,
    at: (i) => ({ ...idleAt(i), look: track[i] }),
    look: track,
    description: 'Head glances: center -> viewer\'s left (0.2 s) -> center (0.95 s) -> viewer\'s right (1.45 s) -> up-right toward a speech bubble (2.05 s, settled by ~2.5 s).',
    best_use: 'UBI "reads" the UI left and right, then looks up-right at a speech bubble — place the bubble above-right of the head (see anchors.Head). Frames 75-89 hold the bubble glance.',
  });
  // --- transitions, rendered the way the app plays clips (Rig.fadeTo: cross-fade 0.35 s, one-shot, then back to a
  // restarted Idle). Idle never passes through the rest pose the one-shots start and end on (closest: 2.0 mean diff
  // at Idle frame 60), so a hard cut Idle -> one-shot pops; these sequences start ON idle frame 0 and end ON an idle
  // frame, so they splice into the Idle loop without a jump.
  const smooth = (x) => x * x * (3 - 2 * x);
  if (idle) {
    for (const name of ['Wave', 'Yes', 'No', 'Jump']) {
      const c = byName.get(name);
      if (!c) continue;
      const D = c.duration;
      const E = Math.ceil(D * FPS - 1e-6); // first frame where the one-shot has reached its end (rest pose)
      const frames = E + XF + 1;
      const base = list.find((x) => x.name === lower(name));
      list.push({
        name: `${lower(name)}-from-idle`,
        dir: `ubi/${lower(name)}-from-idle`,
        source: { clip: name, base: 'Idle', crossfade_frames: XF, easing: 'smoothstep' },
        frames,
        loopable: false,
        first_frame_equals: { clip: 'idle', frame: 0 },
        last_frame_equals: { clip: 'idle', frame: XF },
        at: (i) => {
          if (i <= XF) return { clip: 'Idle', time: i / FPS, mix: { clip: name, time: i / FPS, w: smooth(i / XF) } };
          if (i <= E) return { clip: name, time: Math.min(i / FPS, D) };
          const j = i - E;
          return { clip: name, time: D, mix: { clip: 'Idle', time: j / FPS, w: smooth(j / XF) } };
        },
        description: `${name} spliced into Idle like the app plays it: ${XF}-frame cross-fade Idle -> ${name}, the full ${name}, then a ${XF}-frame cross-fade into a restarted Idle. ${base?.description ?? ''}`,
        best_use: `${base?.best_use ?? ''} Use this one inside a longer Idle shot: play idle up to its frame 119, then this, then continue idle from frame ${XF + 1}.`,
      });
    }
    for (const name of ['Excited', 'Worried', 'Sleep']) {
      const c = byName.get(name);
      if (!c) continue;
      const plan = clipPlan(name, c.duration);
      const target = lower(name);
      list.push({
        name: `idle-to-${target}`,
        dir: `ubi/idle-to-${target}`,
        source: { clip: name, base: 'Idle', crossfade_frames: XF, easing: 'smoothstep' },
        frames: XF + 1,
        loopable: false,
        first_frame_equals: { clip: 'idle', frame: 0 },
        last_frame_equals: { clip: target, frame: XF },
        at: (i) => ({ clip: 'Idle', time: i / FPS, mix: { ...plan.at(i), w: smooth(i / XF) } }),
        description: `${XF + 1}-frame cross-fade from Idle into the ${name} loop (mood change, as the app's Rig.setMood does).`,
        best_use: `Mood switch inside one shot: idle up to its frame 119, then this, then loop ${target} starting at its frame ${XF + 1}.`,
      });
    }
  }
  return list;
}

/** Poses the camera fit must contain: every 3rd frame of every sequence (+ each loop's last frame). */
function fitPoses(seqs) {
  const poses = [];
  for (const s of seqs) {
    for (let i = 0; i < s.frames; i += 3) poses.push(s.at(i));
    poses.push(s.at(s.frames - 1));
  }
  return poses;
}

const STILLS = [
  { name: 'front', file: 'ubi/stills/front.png', spec: { clip: 'Idle', time: 0, rotY: 0 }, description: 'Front view, rest/idle pose.' },
  {
    name: 'three-quarter-left',
    file: 'ubi/stills/three-quarter-left.png',
    spec: { clip: 'Idle', time: 0, rotY: -0.6109 },
    description: '3/4 view, body turned 35 deg so UBI faces the viewer\'s LEFT (put him on the right side of the frame, facing the content).',
  },
  {
    name: 'three-quarter-right',
    file: 'ubi/stills/three-quarter-right.png',
    spec: { clip: 'Idle', time: 0, rotY: 0.6109 },
    description: '3/4 view, body turned 35 deg so UBI faces the viewer\'s RIGHT (put him on the left side of the frame, facing the content).',
  },
  {
    name: 'wave-front',
    file: 'ubi/stills/wave-front.png',
    spec: { clip: 'Wave', time: 28 / 30, rotY: 0 },
    description: 'Front view at the peak of the Wave (= wave frame 28): forearm swung out, head tilted. Bonus for the greeting / end card.',
  },
];

// ---------------------------------------------------------------------------------------------------------------
// Server + browser
// ---------------------------------------------------------------------------------------------------------------

async function up() {
  try {
    const r = await fetch(`${ORIGIN}/video/launch/tools/ubi-render/index.html`, { signal: AbortSignal.timeout(1500) });
    return r.ok;
  } catch {
    return false;
  }
}

async function ensureServer() {
  if (await up()) return null;
  const child = spawn('python3', ['-m', 'http.server', String(PORT), '--bind', '127.0.0.1'], { cwd: REPO, stdio: 'ignore' });
  for (let i = 0; i < 50 && !(await up()); i++) await new Promise((r) => setTimeout(r, 100));
  if (!(await up())) throw new Error('static server did not start');
  console.log(`static server pid ${child.pid} on :${PORT}`);
  return child;
}

const pad = (i) => String(i).padStart(4, '0');
const dataToBuf = (url) => Buffer.from(url.slice(url.indexOf(',') + 1), 'base64');
const ndcToPx = ([x, y], w, h) => [+(((x + 1) / 2) * w).toFixed(1), +(((1 - y) / 2) * h).toFixed(1)];

function cleanDir(dir) {
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
}

function dirBytes(dir) {
  if (!existsSync(dir)) return 0;
  return readdirSync(dir).reduce((n, f) => n + statSync(join(dir, f)).size, 0);
}

async function main() {
  const server = await ensureServer();
  const browser = await chromium.launch({
    executablePath: CHROME,
    headless: true,
    args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--enable-webgl'],
  });
  const t0 = Date.now();
  try {
    const page = await browser.newPage();
    page.on('pageerror', (e) => console.error('[pageerror]', e.message));
    page.on('console', (m) => {
      if (m.type() === 'error' || m.type() === 'warning') console.error(`[page ${m.type()}]`, m.text());
    });
    await page.goto(PAGE);
    await page.waitForFunction(() => window.UBI_READY === true, null, { timeout: 60000 });
    const info = await page.evaluate((o) => window.UBI.init(o), { width: SIZE, height: SIZE, ss: SS, fixStrays: FIX });
    console.log(`skin repair: ${JSON.stringify(info.fixes)}`);
    console.log(`three r${info.three} · ${info.gl} · MSAA ${info.samples} · clips ${info.clips.map((c) => `${c.name} ${c.duration.toFixed(3)}s`).join(', ')}`);

    const seqs = sequences(info.clips);
    const manifestPath = join(OUT, 'ubi-manifest.json');
    const prev = existsSync(manifestPath) ? JSON.parse(readFileSync(manifestPath, 'utf8')) : null;
    let clipCamera = null;
    let fit = null;
    let anchors = null;

    if (!STILLS_ONLY) {
      // one camera for every sequence, so cuts between clips never jump. A partial run (--only) keeps the camera of
      // the existing manifest (unless --refit) and checks that the re-rendered sequences still fit in it.
      const tf = Date.now();
      if (ONLY.length && prev?.camera && !flag('refit')) {
        await page.evaluate((c) => window.UBI.setCamera(c), prev.camera);
        clipCamera = prev.camera;
        const m = await page.evaluate((o) => window.UBI.measure(o.poses), { poses: fitPoses(seqs.filter((x) => ONLY.includes(x.name))) });
        const margin = Math.min(1 + m.x0, 1 - m.x1, 1 + m.y0, 1 - m.y1) / 2;
        console.log(`reusing the manifest camera; --only sequences keep a ${(margin * 100).toFixed(1)} % margin`);
        if (margin < 0.02) throw new Error('the --only sequences leave the frame with the existing camera: run without --only (or --refit)');
      } else {
        fit = await page.evaluate((o) => window.UBI.fitCamera(o), { poses: fitPoses(seqs), padding: PADDING });
        clipCamera = fit.camera;
      }
      const nd = anchorsNdc(await page.evaluate((n) => window.UBI.anchors({ clip: 'Idle', time: 0 }, n), ['Head', 'Neck', 'Chest', 'Hips', 'Orb', 'HandR', 'HandL']));
      anchors = Object.fromEntries(Object.entries(nd).map(([k, v]) => [k, ndcToPx(v, SIZE, SIZE)]));
      if (fit) console.log(`camera fit in ${((Date.now() - tf) / 1000).toFixed(1)}s: d=${clipCamera.d.toFixed(3)} ty=${clipCamera.ty.toFixed(3)} ndc=${JSON.stringify(roundObj(fit.ndc))} edge poses=${fit.edgePoses}`);

      const todo = seqs.filter((s) => !ONLY.length || ONLY.includes(s.name));
      for (const s of todo) {
        if (DRY) continue;
        const dir = join(PUBLIC, s.dir);
        cleanDir(dir);
        const ts = Date.now();
        for (let i = 0; i < s.frames; i++) {
          const url = await page.evaluate((spec) => window.UBI.render(spec), s.at(i));
          writeFileSync(join(dir, `${pad(i)}.png`), dataToBuf(url));
          if (i % 20 === 0 || i === s.frames - 1) {
            const el = (Date.now() - ts) / 1000;
            process.stdout.write(`\r${s.name}: ${i + 1}/${s.frames} (${(el / (i + 1)).toFixed(2)} s/frame)   `);
          }
        }
        process.stdout.write(`\n${s.name}: ${s.frames} frames, ${(dirBytes(dir) / 1e6).toFixed(1)} MB in ${((Date.now() - ts) / 1000).toFixed(0)}s\n`);
      }
    }

    let stills = prev?.stills ?? [];
    let stillCamera = prev?.stills_camera ?? null;
    if (!NO_STILLS && !ONLY.length && !DRY) {
      await page.evaluate((o) => window.UBI.setSize(o), { width: STILL_SIZE, height: STILL_SIZE, ss: SS });
      const sf = await page.evaluate((o) => window.UBI.fitCamera(o), { poses: STILLS.map((s) => s.spec), padding: PADDING });
      stillCamera = sf.camera;
      mkdirSync(join(OUT, 'stills'), { recursive: true });
      stills = [];
      for (const s of STILLS) {
        const url = await page.evaluate((spec) => window.UBI.render(spec), s.spec);
        writeFileSync(join(PUBLIC, s.file), dataToBuf(url));
        stills.push({ name: s.name, file: s.file, size: [STILL_SIZE, STILL_SIZE], rotation_y_deg: +((s.spec.rotY * 180) / Math.PI).toFixed(1), pose: `${s.spec.clip} @ ${+s.spec.time.toFixed(3)}s`, description: s.description });
        console.log(`still ${s.file}`);
      }
      const g = ndcToPx([0, sf.groundNdcY], STILL_SIZE, STILL_SIZE)[1];
      stillCamera = { ...stillCamera, ground_y_px: g, silhouette_px: ndcBoxPx(sf.ndc, STILL_SIZE, STILL_SIZE) };
    }

    // manifest (merged with the previous one when only part was re-rendered)
    const prevClips = new Map((prev?.clips ?? []).map((c) => [c.name, c]));
    const clips = seqs.map((s) => {
      const dir = join(PUBLIC, s.dir);
      const frames = existsSync(dir) ? readdirSync(dir).filter((f) => /^\d{4}\.png$/.test(f)).length : 0;
      const entry = {
        name: s.name,
        dir: s.dir,
        pattern: `${s.dir}/{frame:04d}.png`,
        frames,
        duration_s: +(frames / FPS).toFixed(4),
        loopable: s.loopable,
        description: s.description,
        best_use: s.best_use,
        source: s.source,
        bytes: dirBytes(dir),
      };
      if (s.look) entry.look_deg = s.look;
      if (s.first_frame_equals) entry.first_frame_equals = s.first_frame_equals;
      if (s.last_frame_equals) entry.last_frame_equals = s.last_frame_equals;
      return prevClips.has(s.name) && ONLY.length && !ONLY.includes(s.name) ? { ...prevClips.get(s.name), ...entry, bytes: entry.bytes } : entry;
    });
    const cam = clipCamera ?? prev?.camera;
    const manifest = {
      fps: FPS,
      size: [SIZE, SIZE],
      format: 'PNG RGBA, straight (un-premultiplied) alpha, sRGB; transparent background',
      frame_naming: 'public/<dir>/NNNN.png, 4 digits from 0000; use staticFile(`${dir}/${String(i).padStart(4, "0")}.png`)',
      source_model: 'apps/desktop/public/ubi/Ubi.glb (== site/public/ubi/Ubi.glb), 21-bone rig, 8 clips',
      look: 'Matches the product (Ubi3d.tsx / UbiHero3d.tsx): MeshStandardMaterial from the GLB, RoomEnvironment IBL x0.8, white key 1.6, volt #4d8dff fill from below and rim, ACES Filmic, exposure 1.05, sRGB. No whole-body float or floor glow is baked in — add float (e.g. translateY sin, 4.2 s, ~8 px) and a volt floor glow in the composition if wanted.',
      rendering: `three r${info.three}, headless Chromium + SwiftShader WebGL, MSAA ${info.samples}x at ${SS}x supersampling, downsampled in a premultiplied 2D canvas; deterministic clip time i/${FPS} s`,
      skin_repair: FIX
        ? {
            applied: info.fixes,
            note: 'Video-only fix at load time (the product GLB is untouched): 250 vertices of the right thumb tip are weighted 100 % to Shoulder.R in Ubi.glb (they cross rig_ubi.py\'s arm-region cut), so whenever the right arm moves the thumb stretches into a white bar pinned to the shoulder (very visible in Wave). Vertices within 0.22 of a hand bone whose dominant bone is not part of that arm are given to the hand.',
          }
        : null,
      camera: cam
        ? {
            ...cam,
            note: 'One camera for every clip, turntable and look: cuts between sequences never jump. UBI stands on the same ground line in every frame; x = 0 is the frame centre.',
            padding: PADDING,
            ground_y_px: fit ? ndcToPx([0, fit.groundNdcY], SIZE, SIZE)[1] : (prev?.camera?.ground_y_px ?? null),
            silhouette_union_px: fit ? ndcBoxPx(fit.ndc, SIZE, SIZE) : (prev?.camera?.silhouette_union_px ?? null),
          }
        : null,
      anchors_px: anchors ?? prev?.anchors_px ?? null,
      anchors_note: 'Pixel positions [x, y] in the clip frame on Idle frame 0 (same camera for all sequences): headCenter/headTop = centre/top of the helmet shell (point speech bubbles here), Head/Neck/Chest/Hips/HandL/HandR/Orb = joint pivots (Head is the neck-top pivot), ground = floor point under the body centre. The soles reach ~7 px below ground (feet_bottom_y_px) because the camera looks slightly down.',
      compositing_tips: [
        'Everything shares one camera: keep the <Img> at the same position/scale and UBI stays planted (feet_bottom_y_px); x = 0 of the model is the frame centre. Jump is the only clip that leaves the ground.',
        'Loops (idle, excited, worried, sleep, turntable) wrap cleanly: frame = i % frames.',
        'Idle does NOT pass through the rest pose that yes/no/wave/jump start and end on (see continuity_vs_idle_frame0): a hard cut idle -> wave pops slightly. Either cut on a beat with a scale/position change (hides it), or use the *-from-idle / idle-to-* sequences, whose first_frame_equals / last_frame_equals say where they splice in.',
        'Splicing: an Idle segment of N frames that must END on idle frame 119 starts at idle frame (120 - N % 120) % 120. Then play e.g. wave-from-idle (its frame 0 == idle frame 0), then continue with idle from frame last_frame_equals.frame + 1 (idle-to-* continue the mood loop the same way).',
        'turntable[0] and look[0] equal idle frame 0: they splice after idle frame 119 too.',
        'Add life in the composition: slow float (translateY ~6-10 px, 4.2 s sine), a soft volt (#4d8dff) elliptical floor glow under the feet and a subtle shadow; the renders carry none of these.',
      ],
      clips,
      stills,
      stills_camera: stillCamera,
      total_bytes: clips.reduce((n, c) => n + c.bytes, 0) + stills.reduce((n, s) => n + (existsSync(join(PUBLIC, s.file)) ? statSync(join(PUBLIC, s.file)).size : 0), 0),
      generated_by: 'video/launch/tools/ubi-render/render.mjs',
    };
    if (!DRY || !existsSync(manifestPath)) writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    console.log(`manifest: ${clips.length} sequences, ${(manifest.total_bytes / 1e6).toFixed(1)} MB total, ${((Date.now() - t0) / 1000).toFixed(0)}s`);
  } finally {
    await browser.close();
    if (server) {
      server.kill('SIGTERM');
      console.log(`stopped static server pid ${server.pid}`);
    }
  }
}

function anchorsNdc(a) {
  return a;
}
function roundObj(o) {
  return Object.fromEntries(Object.entries(o).map(([k, v]) => [k, +v.toFixed(4)]));
}
function ndcBoxPx(b, w, h) {
  const [x0, y0] = ndcToPx([b.x0, b.y1], w, h);
  const [x1, y1] = ndcToPx([b.x1, b.y0], w, h);
  return { x0, y0, x1, y1 };
}

main().catch((e) => {
  console.error(e);
  process.exitCode = 1;
});
