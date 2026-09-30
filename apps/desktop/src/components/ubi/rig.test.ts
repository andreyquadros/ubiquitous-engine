import { describe, expect, it } from 'vitest';
import * as THREE from 'three';
import {
  baseClipFor,
  BUBBLE_REACH,
  Cooldown,
  CROSSFADE,
  easeLook,
  findBone,
  formatLook,
  GLANCE_ON_SPEECH,
  GLANCE_PERIOD,
  GLANCE_SHORT,
  GlanceScheduler,
  HEAD_SHARE,
  headOnScreen,
  lookAtBubble,
  lookAtPoint,
  lookTowards,
  resolveLook,
  Rig,
  rotateInWorld,
  tapClipFor,
} from './rig';

const FULL = ['Idle', 'Yes', 'No', 'Wave', 'Jump', 'Excited', 'Worried', 'Sleep'];
/** The three.js RobotExpressive stand-in: the contract's one-shots but none of the mood loops. */
const STAND_IN = ['Idle', 'Yes', 'No', 'Wave', 'Jump', 'Dance', 'ThumbsUp'];

describe('baseClipFor', () => {
  it('maps every mood to its clip and tempo when the export has them all', () => {
    expect(baseClipFor('calm', FULL)).toEqual({ name: 'Idle', timeScale: 1 });
    expect(baseClipFor('focused', FULL)).toEqual({ name: 'Idle', timeScale: 0.85 });
    expect(baseClipFor('excited', FULL)).toEqual({ name: 'Excited', timeScale: 1 });
    expect(baseClipFor('worried', FULL)).toEqual({ name: 'Worried', timeScale: 1 });
    expect(baseClipFor('sleeping', FULL)).toEqual({ name: 'Sleep', timeScale: 1 });
  });

  it('degrades a missing mood clip to Idle at the mood tempo', () => {
    expect(baseClipFor('excited', STAND_IN)).toEqual({ name: 'Idle', timeScale: 1.3 });
    expect(baseClipFor('worried', STAND_IN)).toEqual({ name: 'Idle', timeScale: 1 });
    expect(baseClipFor('sleeping', STAND_IN)).toEqual({ name: 'Idle', timeScale: 0.5 });
    expect(baseClipFor('calm', STAND_IN)).toEqual({ name: 'Idle', timeScale: 1 });
  });

  it('takes the first clip without an Idle and null without clips', () => {
    expect(baseClipFor('calm', ['Breathe'])).toEqual({ name: 'Breathe', timeScale: 1 });
    expect(baseClipFor('sleeping', ['Breathe'])).toEqual({ name: 'Breathe', timeScale: 0.5 });
    expect(baseClipFor('calm', [])).toBeNull();
  });

  it('waves on a tap, shakes the head when worried', () => {
    expect(tapClipFor('calm')).toBe('Wave');
    expect(tapClipFor('excited')).toBe('Wave');
    expect(tapClipFor('worried')).toBe('No');
  });
});

describe('look-at maths', () => {
  const box = { left: 100, top: 50, width: 200, height: 240 };

  it('puts the head near the top centre of the box', () => {
    expect(headOnScreen(box)).toEqual({ x: 200, y: 50 + 240 * 0.22 });
  });

  it('turns towards the pointer side and reaches the clamp at 1.5× the mascot size', () => {
    const head = headOnScreen(box);
    const right = lookAtPoint(box, 200, head.x + 150, head.y);
    expect(right.yaw).toBeCloseTo(22.5, 5);
    expect(right.pitch).toBeCloseTo(0, 5);
    const left = lookAtPoint(box, 200, head.x - 150, head.y);
    expect(left.yaw).toBeCloseTo(-22.5, 5);
    expect(lookAtPoint(box, 200, head.x + 300, head.y).yaw).toBe(45);
    expect(lookAtPoint(box, 200, head.x + 3000, head.y).yaw).toBe(45);
    expect(lookAtPoint(box, 200, head.x - 3000, head.y).yaw).toBe(-45);
  });

  it('clamps the pitch to −25° down and +30° up (screen y grows downwards)', () => {
    const head = headOnScreen(box);
    expect(lookAtPoint(box, 200, head.x, head.y - 100).pitch).toBeCloseTo(15, 5);
    expect(lookAtPoint(box, 200, head.x, head.y - 5000).pitch).toBe(30);
    expect(lookAtPoint(box, 200, head.x, head.y + 5000).pitch).toBe(-25);
    expect(lookTowards(0, 0, 300)).toEqual({ yaw: 0, pitch: 0 });
  });

  it('looks up at the bubble above the head with the shorter reach', () => {
    const bubble = { left: 150, top: 0, width: 100, height: 40 };
    const l = lookAtBubble(box, 200, bubble);
    expect(l.yaw).toBeCloseTo(0, 5);
    const dy = 20 - headOnScreen(box).y;
    expect(l.pitch).toBeCloseTo(Math.min(30, (-dy * 45) / (200 * BUBBLE_REACH)), 5);
    expect(l.pitch).toBeGreaterThan(20);
  });

  it('prefers the bubble glance, then the pointer, then rest', () => {
    const glance = { yaw: 1, pitch: 20 };
    const pointer = { yaw: -30, pitch: 2 };
    expect(resolveLook(glance, pointer)).toBe(glance);
    expect(resolveLook(null, pointer)).toBe(pointer);
    expect(resolveLook(null, null)).toEqual({ yaw: 0, pitch: 0 });
  });

  it('eases with damping and snaps when motion is reduced', () => {
    const cur = { yaw: 0, pitch: 0 };
    easeLook(cur, { yaw: 40, pitch: -20 }, 1 / 60);
    expect(cur.yaw).toBeGreaterThan(0);
    expect(cur.yaw).toBeLessThan(40);
    expect(cur.pitch).toBeLessThan(0);
    for (let i = 0; i < 120; i++) easeLook(cur, { yaw: 40, pitch: -20 }, 1 / 60);
    expect(cur.yaw).toBeCloseTo(40, 3);
    expect(cur.pitch).toBeCloseTo(-20, 3);
    easeLook(cur, { yaw: -10, pitch: 5 }, 1 / 60, true);
    expect(cur).toEqual({ yaw: -10, pitch: 5 });
    expect(formatLook({ yaw: -12.345, pitch: 3 })).toBe('-12.3,3.0');
  });
});

describe('GlanceScheduler', () => {
  it('glances at the bubble for 2.5 s after new speech, then briefly every 10 s while a bubble is visible', () => {
    const g = new GlanceScheduler();
    expect(g.active(0, true)).toBe(false);
    g.speech(1);
    expect(g.active(1, true)).toBe(true);
    expect(g.active(1 + GLANCE_ON_SPEECH - 0.01, true)).toBe(true);
    expect(g.active(1 + GLANCE_ON_SPEECH, true)).toBe(false);
    expect(g.nextChange(1)).toBe(1 + GLANCE_ON_SPEECH);
    expect(g.active(1 + GLANCE_PERIOD - 0.01, true)).toBe(false);
    expect(g.active(1 + GLANCE_PERIOD, true)).toBe(true);
    expect(g.active(1 + GLANCE_PERIOD + GLANCE_SHORT, true)).toBe(false);
    expect(g.active(1 + 2 * GLANCE_PERIOD + 0.5, true)).toBe(true);
  });

  it('never glances without a bubble and restarts the period when one appears', () => {
    const g = new GlanceScheduler();
    g.speech(0);
    expect(g.active(0.5, false)).toBe(false);
    expect(g.nextChange(0.5)).toBe(Infinity);
    expect(g.active(3, true)).toBe(false);
    expect(g.active(3 + GLANCE_PERIOD - 0.5, true)).toBe(false);
    expect(g.active(3 + GLANCE_PERIOD, true)).toBe(true);
  });
});

describe('Cooldown', () => {
  it('lets one shot through per period', () => {
    const c = new Cooldown(6);
    expect(c.take(10)).toBe(true);
    expect(c.take(12)).toBe(false);
    expect(c.take(15.9)).toBe(false);
    expect(c.take(16)).toBe(true);
  });
});

/** Root > Hips > Spine > Neck > Head, with a mesh also called Head under the bone (like the stand-in). */
function skeleton() {
  const root = new THREE.Group();
  root.name = 'Scene';
  const hips = new THREE.Bone();
  hips.name = 'Hips';
  const spine = new THREE.Bone();
  spine.name = 'Spine';
  const neck = new THREE.Bone();
  neck.name = 'Neck';
  const head = new THREE.Bone();
  head.name = 'Head';
  const headMesh = new THREE.Mesh(new THREE.BoxGeometry());
  headMesh.name = 'Head';
  const armL = new THREE.Bone();
  armL.name = 'UpperArmL';
  root.add(hips.add(spine.add(neck.add(head.add(headMesh)), armL)));
  root.updateMatrixWorld(true);
  return { root, hips, spine, neck, head, headMesh, armL };
}

const quatTrack = (name: string, times: number[], eulers: Array<[number, number, number]>) =>
  new THREE.QuaternionKeyframeTrack(
    `${name}.quaternion`,
    times,
    eulers.flatMap(([x, y, z]) => new THREE.Quaternion().setFromEuler(new THREE.Euler(x, y, z)).toArray()),
  );

const clips = () => [
  new THREE.AnimationClip('Idle', 4, [quatTrack('Head', [0, 2, 4], [[0, 0, 0], [0, 0.05, 0], [0, 0, 0]])]),
  new THREE.AnimationClip('Yes', 1.2, [quatTrack('Neck', [0, 0.6, 1.2], [[0, 0, 0], [0.3, 0, 0], [0, 0, 0]])]),
  new THREE.AnimationClip('Wave', 2, [quatTrack('UpperArmL', [0, 1, 2], [[0, 0, 0], [0, 0, 1], [0, 0, 0]])]),
];

/** World-space direction of the node's local +Z axis. */
const forward = (node: THREE.Object3D) => {
  node.updateWorldMatrix(true, false);
  return new THREE.Vector3(0, 0, 1).applyQuaternion(node.getWorldQuaternion(new THREE.Quaternion()));
};

describe('findBone / rotateInWorld', () => {
  it('prefers the bone over a mesh of the same name and copes with the dots three strips', () => {
    const s = skeleton();
    expect(findBone(s.root, 'Head')).toBe(s.head);
    expect(findBone(s.root, 'UpperArm.L')).toBe(s.armL);
    expect(findBone(s.root, 'Orb')).toBeNull();
    expect(findBone(s.headMesh, 'Head')).toBe(s.headMesh);
  });

  it('turns about the world axes whatever the parent orientation', () => {
    const s = skeleton();
    // the neck is twisted 90° about Y and tilted: the head still turns to the viewer's right, about world Y
    s.neck.rotation.set(0.4, Math.PI / 2, 0);
    s.root.updateMatrixWorld(true);
    const before = forward(s.head);
    rotateInWorld(s.head, 30, 0);
    const after = forward(s.head);
    const expected = before.clone().applyAxisAngle(new THREE.Vector3(0, 1, 0), THREE.MathUtils.degToRad(30));
    expect(after.distanceTo(expected)).toBeLessThan(1e-6);
    // pitch up: the forward vector of an upright head gains +Y
    const t = skeleton();
    rotateInWorld(t.head, 0, 30);
    expect(forward(t.head).y).toBeCloseTo(Math.sin(THREE.MathUtils.degToRad(30)), 6);
    expect(forward(t.head).x).toBeCloseTo(0, 6);
  });
});

describe('Rig', () => {
  it('is rigged only with a Head bone and clips; a bare mesh keeps the whole-body motion', () => {
    const s = skeleton();
    expect(new Rig(s.root, clips()).rigged).toBe(true);
    expect(new Rig(s.root, []).rigged).toBe(false);
    const mesh = new THREE.Mesh(new THREE.BoxGeometry());
    expect(new Rig(mesh, clips()).rigged).toBe(false);
    expect(new Rig(mesh, clips()).head).toBeNull();
  });

  it('plays the mood clip, runs one-shots to the end and returns to the base clip', () => {
    const s = skeleton();
    const rig = new Rig(s.root, clips());
    rig.attach();
    expect(rig.setMood('excited')).toEqual({ name: 'Idle', timeScale: 1.3 });
    expect(rig.baseName).toBe('Idle');
    rig.update(0.5);
    expect(rig.mixer.time).toBeCloseTo(0.5, 6);
    expect(rig.playOnce('Yes')).toBe(true);
    expect(rig.playOnce('Missing')).toBe(false);
    expect(rig.busy).toBe(true);
    // a mood change while the nod plays waits for it
    rig.setMood('focused');
    expect(rig.baseName).toBe('Idle');
    let nodded = false;
    for (let i = 0; i < 40; i++) {
      rig.update(0.05);
      if (Math.abs(s.neck.rotation.x) > 0.1) nodded = true;
    }
    expect(nodded).toBe(true);
    expect(rig.busy).toBe(false);
    // after the cross-fade back the neck rests again
    for (let i = 0; i < 20; i++) rig.update(CROSSFADE / 10);
    expect(Math.abs(s.neck.rotation.x)).toBeLessThan(1e-3);
    rig.detach();
  });

  it('skips one-shots and freezes the clip at its first frame under reduced motion', () => {
    const s = skeleton();
    const rig = new Rig(s.root, clips());
    rig.attach();
    rig.setMood('calm', true);
    expect(rig.playOnce('Wave', true)).toBe(false);
    for (let i = 0; i < 10; i++) rig.update(0);
    expect(rig.mixer.time).toBe(0);
    expect(s.head.rotation.y).toBeCloseTo(0, 6);
    rig.detach();
  });

  it('layers the look-at over the clip pose without accumulating frame after frame', () => {
    const s = skeleton();
    const rig = new Rig(s.root, clips());
    rig.attach();
    rig.setMood('calm');
    const look = { yaw: 40, pitch: 0 };
    for (let i = 0; i < 30; i++) {
      rig.update(0.01);
      rig.look(look);
    }
    // 70 % on the head + 30 % on the neck = the full angle in world space
    const dir = forward(s.head);
    const yaw = THREE.MathUtils.radToDeg(Math.atan2(dir.x, dir.z));
    // the Idle clip adds a few degrees of its own sway (≤ 0.05 rad) on the head
    expect(Math.abs(yaw - 40)).toBeLessThan(4);
    const neckDir = forward(s.neck);
    expect(THREE.MathUtils.radToDeg(Math.atan2(neckDir.x, neckDir.z))).toBeCloseTo(40 * (1 - HEAD_SHARE), 3);
    // back to rest: no residue
    for (let i = 0; i < 3; i++) {
      rig.update(0.01);
      rig.look({ yaw: 0, pitch: 0 });
    }
    expect(Math.abs(forward(s.neck).x)).toBeLessThan(1e-6);
    rig.detach();
  });

  it('puts the whole look on the head when there is no neck bone', () => {
    const root = new THREE.Group();
    const head = new THREE.Bone();
    head.name = 'Head';
    root.add(head);
    const rig = new Rig(root, [new THREE.AnimationClip('Idle', 1, [quatTrack('Head', [0, 1], [[0, 0, 0], [0, 0, 0]])])]);
    rig.setMood('calm');
    rig.update(0.1);
    rig.look({ yaw: 20, pitch: 0 });
    const dir = forward(head);
    expect(THREE.MathUtils.radToDeg(Math.atan2(dir.x, dir.z))).toBeCloseTo(20, 4);
  });
});
