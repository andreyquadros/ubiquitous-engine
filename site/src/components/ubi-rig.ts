import * as THREE from 'three';

/** Copiado de apps/desktop/src/components/ubi/rig.ts — mesma lógica de rig do app. */
export type Mood = 'sleeping' | 'calm' | 'focused' | 'excited' | 'worried';

/*
 * Pure logic behind the rigged UBI (no WebGL, no React): clip choice per mood, the head look-at maths, the
 * glance/one-shot scheduling and the small controller that layers the look-at over an AnimationMixer.
 * Everything here is unit-tested in jsdom; Ubi3d.tsx only wires it to the render loop.
 */

/** glTF clip names the export is expected to carry (the app degrades to Idle for any missing one). */
export const CLIPS = {
  idle: 'Idle',
  yes: 'Yes',
  no: 'No',
  wave: 'Wave',
  jump: 'Jump',
  excited: 'Excited',
  worried: 'Worried',
  sleep: 'Sleep',
} as const;

/** Bone names looked up in the export (three strips the dots of `UpperArm.L`-style names: `findBone` copes). */
export const BONES = { head: 'Head', neck: 'Neck' } as const;

/** Cross-fade between clips, seconds. */
export const CROSSFADE = 0.35;
/** Share of the look rotation carried by the Head bone; the Neck takes the rest. */
export const HEAD_SHARE = 0.7;
/** Look-at limits, degrees. */
export const LOOK_LIMITS = { yaw: 45, pitchDown: -25, pitchUp: 30 } as const;
/** Easing rate of the look-at (per second). */
export const LOOK_DAMPING = 8;
/** Longest frame the look-at easing integrates (a slow frame still lands close to the target in wall time). */
export const LOOK_MAX_DT = 0.2;
/** The pointer at this many mascot widths from the head already reaches the yaw/pitch clamp. */
export const POINTER_REACH = 1.5;
/** Same for the speech bubble (it sits right above the head, so the reach is shorter to make the glance visible). */
export const BUBBLE_REACH = 0.6;
/** The head sits about this far down the canvas box (fraction of its height). */
export const HEAD_Y_FRACTION = 0.22;
/** Seconds the mascot looks at the bubble when the speech appears or changes. */
export const GLANCE_ON_SPEECH = 2.5;
/** Seconds between the periodic glances at a visible bubble, and their length. */
export const GLANCE_PERIOD = 10;
export const GLANCE_SHORT = 1.2;
/** Minimum seconds between two `Yes` nods triggered by speech changes. */
export const YES_COOLDOWN = 6;
/** Whole-body float/parallax multiplier while a rig drives the pose (the clips do the work). */
export const BODY_MOTION_WITH_RIG = 1 / 3;

export interface BaseClip {
  name: string;
  timeScale: number;
}

/**
 * Base clip and tempo per mood. A missing mood clip degrades to Idle at the mood's tempo; a missing Idle takes
 * the first clip of the export; no clips at all → `null` (the whole-body motion of the unrigged model applies).
 */
export function baseClipFor(mood: Mood, available: Iterable<string>): BaseClip | null {
  const names = [...available];
  if (!names.length) return null;
  const has = new Set(names);
  const idle = has.has(CLIPS.idle) ? CLIPS.idle : (names[0] as string);
  const pick = (name: string, timeScale: number, idleScale: number): BaseClip => (has.has(name) ? { name, timeScale } : { name: idle, timeScale: idleScale });
  switch (mood) {
    case 'focused':
      return pick(CLIPS.idle, 0.85, 0.85);
    case 'excited':
      return pick(CLIPS.excited, 1, 1.3);
    case 'worried':
      return pick(CLIPS.worried, 1, 1);
    case 'sleeping':
      return pick(CLIPS.sleep, 1, 0.5);
    default:
      return pick(CLIPS.idle, 1, 1);
  }
}

/** The one-shot a click/tap on the mascot triggers. */
export const tapClipFor = (mood: Mood): string => (mood === 'worried' ? CLIPS.no : CLIPS.wave);

/** Look angles in degrees: `yaw > 0` = towards the viewer's right, `pitch > 0` = up. */
export interface Look {
  yaw: number;
  pitch: number;
}

export const REST_LOOK: Readonly<Look> = Object.freeze({ yaw: 0, pitch: 0 });

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/**
 * Angles that make the head face a screen point `dx`/`dy` px away from the head (screen y grows downwards).
 * Linear until `reachPx`, where the yaw clamp is reached, then clamped.
 */
export function lookTowards(dx: number, dy: number, reachPx: number): Look {
  const k = LOOK_LIMITS.yaw / Math.max(reachPx, 1);
  // `+ 0` folds a -0 into 0 so the debugging attribute never reads "-0.0"
  return { yaw: clamp(dx * k + 0, -LOOK_LIMITS.yaw, LOOK_LIMITS.yaw), pitch: clamp(-dy * k + 0, LOOK_LIMITS.pitchDown, LOOK_LIMITS.pitchUp) };
}

export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Where the head is on screen for a canvas box `rect`. */
export const headOnScreen = (rect: Rect) => ({ x: rect.left + rect.width / 2, y: rect.top + rect.height * HEAD_Y_FRACTION });

/** Look angles towards a screen point from the head of the mascot drawn in `box` (`size` = mascot width in px). */
export function lookAtPoint(box: Rect, size: number, x: number, y: number, reach = POINTER_REACH): Look {
  const head = headOnScreen(box);
  return lookTowards(x - head.x, y - head.y, size * reach);
}

/** Look angles towards the centre of the speech bubble. */
export const lookAtBubble = (box: Rect, size: number, bubble: Rect): Look =>
  lookAtPoint(box, size, bubble.left + bubble.width / 2, bubble.top + bubble.height / 2, BUBBLE_REACH);

/** Look target by priority: the bubble glance while it is active, else the pointer, else rest (the clip alone). */
export const resolveLook = (glance: Look | null, pointer: Look | null): Look => glance ?? pointer ?? REST_LOOK;

/** Frame-rate independent exponential easing (three's `MathUtils.damp`). */
export const damp = (current: number, target: number, lambda: number, dt: number): number => target + (current - target) * Math.exp(-lambda * dt);

/** Eases `current` towards `target` in place; snaps when `instant` (reduced motion). */
export function easeLook(current: Look, target: Look, dt: number, instant = false): Look {
  if (instant) {
    current.yaw = target.yaw;
    current.pitch = target.pitch;
  } else {
    current.yaw = damp(current.yaw, target.yaw, LOOK_DAMPING, dt);
    current.pitch = damp(current.pitch, target.pitch, LOOK_DAMPING, dt);
  }
  return current;
}

/** `yaw,pitch` with one decimal each — the `data-ubi-look` debugging attribute. */
export const formatLook = (l: Look): string => `${deg(l.yaw)},${deg(l.pitch)}`;
/** One decimal, and the damping tail (-0.04 → "-0.0") folded into "0.0". */
const deg = (v: number) => (Math.round(v * 10) / 10 + 0).toFixed(1);

/**
 * When the mascot looks at the bubble: `GLANCE_ON_SPEECH` seconds after the speech appears or changes, then a
 * short glance every `GLANCE_PERIOD` seconds while a bubble is visible. Times are seconds from any monotonic clock.
 */
export class GlanceScheduler {
  private until = -Infinity;
  private next = Infinity;

  /** The speech text appeared or changed. */
  speech(now: number): void {
    this.until = now + GLANCE_ON_SPEECH;
    this.next = now + GLANCE_PERIOD;
  }

  /** Whether the glance is active now; call once per frame. */
  active(now: number, bubbleVisible: boolean): boolean {
    if (!bubbleVisible) {
      this.until = -Infinity;
      this.next = Infinity;
      return false;
    }
    if (this.next === Infinity) this.next = now + GLANCE_PERIOD;
    if (now >= this.next) {
      this.until = now + GLANCE_SHORT;
      this.next = now + GLANCE_PERIOD;
    }
    return now < this.until;
  }

  /** When the active state next changes (end of the current glance or start of the next one); `Infinity` when never. */
  nextChange(now: number): number {
    return now < this.until ? this.until : this.next;
  }
}

/** Rate limit for a one-shot: `take` succeeds at most once per `seconds`. */
export class Cooldown {
  private last = -Infinity;
  constructor(private readonly seconds: number) {}
  take(now: number): boolean {
    if (now - this.last < this.seconds) return false;
    this.last = now;
    return true;
  }
}

/** Finds a bone (preferred) or any node called `name`, tolerating the `.`-stripping three applies to glTF names. */
export function findBone(root: THREE.Object3D, name: string): THREE.Object3D | null {
  const wanted = new Set([name, THREE.PropertyBinding.sanitizeNodeName(name)]);
  let bone: THREE.Object3D | null = null;
  let any: THREE.Object3D | null = null;
  root.traverse((o) => {
    if (!wanted.has(o.name)) return;
    if ((o as THREE.Bone).isBone) bone ??= o;
    else any ??= o;
  });
  return bone ?? any;
}

const _parent = new THREE.Quaternion();
const _rot = new THREE.Quaternion();
const _delta = new THREE.Quaternion();
const _euler = new THREE.Euler();

/**
 * Turns `node` by `yaw`/`pitch` degrees about the WORLD axes (Y then X), whatever the bone's own axes are, and
 * keeps the clip's pose underneath. With the camera on +Z a positive yaw faces the viewer's right, a positive pitch
 * looks up. World: W' = R·W = R·P·L = P·(P⁻¹·R·P)·L, so the local delta is P⁻¹·R·P.
 */
export function rotateInWorld(node: THREE.Object3D, yaw: number, pitch: number): void {
  if (yaw === 0 && pitch === 0) return;
  _euler.set(-THREE.MathUtils.degToRad(pitch), THREE.MathUtils.degToRad(yaw), 0, 'YXZ');
  _rot.setFromEuler(_euler);
  if (node.parent) node.parent.getWorldQuaternion(_parent);
  else _parent.identity();
  _delta.copy(_parent).invert().multiply(_rot).multiply(_parent);
  node.quaternion.premultiply(_delta);
}

interface LookBone {
  node: THREE.Object3D;
  /** The bone's local rotation as the mixer last left it (the look-at is undone before every mixer step). */
  clip: THREE.Quaternion;
}

/**
 * The mixer + bones of one model instance: base clip per mood with cross-fades, one-shots that return to the base
 * clip, and the procedural look-at layered on Head (70 %) and Neck (30 %) after every mixer step.
 */
export class Rig {
  readonly mixer: THREE.AnimationMixer;
  readonly clips: string[];
  readonly head: THREE.Object3D | null;
  readonly neck: THREE.Object3D | null;
  private readonly actions = new Map<string, THREE.AnimationAction>();
  private readonly bones: LookBone[] = [];
  private current: THREE.AnimationAction | null = null;
  private base: BaseClip | null = null;
  private oneShot: THREE.AnimationAction | null = null;
  private readonly onFinished = (e: { action: THREE.AnimationAction }) => {
    if (e.action !== this.oneShot) return;
    this.oneShot = null;
    this.fadeTo(this.baseAction(), this.base?.timeScale ?? 1);
  };

  constructor(
    readonly root: THREE.Object3D,
    animations: THREE.AnimationClip[],
  ) {
    this.head = findBone(root, BONES.head);
    this.neck = findBone(root, BONES.neck);
    for (const node of [this.neck, this.head]) if (node) this.bones.push({ node, clip: node.quaternion.clone() });
    this.mixer = new THREE.AnimationMixer(root);
    this.clips = animations.map((c) => c.name);
    for (const clip of animations) this.actions.set(clip.name, this.mixer.clipAction(clip));
  }

  /** Listens for the end of one-shots. Call from the mount effect; `detach` undoes it (StrictMode-safe). */
  attach(): void {
    this.mixer.removeEventListener('finished', this.onFinished);
    this.mixer.addEventListener('finished', this.onFinished);
  }

  /** A skeleton with a Head bone and at least one clip: the contract's "rigged" export. */
  get rigged(): boolean {
    return !!this.head && this.actions.size > 0;
  }

  /** Name of the base (mood) clip, `null` without clips. */
  get baseName(): string | null {
    return this.base?.name ?? null;
  }

  private baseAction(): THREE.AnimationAction | null {
    return this.base ? (this.actions.get(this.base.name) ?? null) : null;
  }

  /** Puts the look-at bones back where the mixer left them (call before the mixer reads or saves their state). */
  private undoLook(): void {
    for (const b of this.bones) b.node.quaternion.copy(b.clip);
  }

  private fadeTo(next: THREE.AnimationAction | null, timeScale: number, duration = CROSSFADE): void {
    if (!next || next === this.current) {
      next?.setEffectiveTimeScale(timeScale);
      return;
    }
    this.undoLook();
    this.current?.fadeOut(duration);
    next.reset().setEffectiveTimeScale(timeScale).setEffectiveWeight(1).fadeIn(duration).play();
    this.current = next;
  }

  /**
   * Selects the base clip for `mood`. While a one-shot plays the change waits for it to finish. With `reduce`
   * the clip is shown frozen at its first frame.
   */
  setMood(mood: Mood, reduce = false): BaseClip | null {
    const next = baseClipFor(mood, this.clips);
    this.base = next;
    if (!next) return null;
    const action = this.actions.get(next.name);
    if (!action) return next;
    if (reduce) {
      this.undoLook();
      this.mixer.stopAllAction();
      this.oneShot = null;
      action.reset().setEffectiveTimeScale(0).setEffectiveWeight(1).play();
      this.current = action;
      this.mixer.setTime(0);
      this.captureClipPose();
    } else if (!this.oneShot) this.fadeTo(action, next.timeScale);
    return next;
  }

  /** Plays `name` once and returns to the base clip. `false` when the clip is missing or motion is reduced. */
  playOnce(name: string, reduce = false): boolean {
    const action = this.actions.get(name);
    if (!action || reduce) return false;
    if (this.oneShot === action) return true;
    action.setLoop(THREE.LoopOnce, 1);
    action.clampWhenFinished = true;
    this.oneShot = action;
    this.fadeTo(action, 1);
    return true;
  }

  /** Whether a one-shot is playing right now. */
  get busy(): boolean {
    return this.oneShot !== null;
  }

  private captureClipPose(): void {
    for (const b of this.bones) b.clip.copy(b.node.quaternion);
  }

  /** One frame: advances the clips (not when `dt` is 0) and remembers the resulting pose of the look-at bones. */
  update(dt: number): void {
    this.undoLook();
    if (dt > 0) this.mixer.update(dt);
    this.captureClipPose();
  }

  /** Layers the look-at over the current pose: 30 % on the Neck, 70 % on the Head (all of it when one is missing). */
  look(l: Look): void {
    const neckShare = this.head && this.neck ? 1 - HEAD_SHARE : 1;
    const headShare = this.head && this.neck ? HEAD_SHARE : 1;
    if (this.neck) rotateInWorld(this.neck, l.yaw * neckShare, l.pitch * neckShare);
    if (this.head) rotateInWorld(this.head, l.yaw * headShare, l.pitch * headShare);
  }

  /** Stops every clip and the listener; the instance can be attached again (the effects re-run in StrictMode). */
  detach(): void {
    this.mixer.removeEventListener('finished', this.onFinished);
    this.undoLook();
    this.mixer.stopAllAction();
    this.current = null;
    this.oneShot = null;
  }
}
