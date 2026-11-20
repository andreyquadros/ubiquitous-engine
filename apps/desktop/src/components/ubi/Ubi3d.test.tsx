import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';

// jsdom has no WebGL: the Canvas becomes a plain box that renders the scene graph through React (so
// effects run), the loader returns a hand-made scene and the PMREM step is a stub.
const fiber = vi.hoisted(() => ({
  frames: [] as Array<(state: unknown, dt: number) => void>,
  canvasProps: [] as Array<Record<string, unknown>>,
  invalidate: vi.fn(),
  scene: null as null | { environment: unknown; environmentIntensity: number },
  reduce: false,
  /** What `useLoader` hands back: an unrigged pair of meshes by default, a skeleton + clips when `rigged`. */
  rigged: false,
}));
vi.mock('framer-motion', async (orig) => ({ ...(await orig<typeof import('framer-motion')>()), useReducedMotion: () => fiber.reduce }));
vi.mock('@react-three/fiber', () => ({
  Canvas: (props: Record<string, unknown> & { children?: React.ReactNode; style?: React.CSSProperties }) => {
    fiber.canvasProps.push(props);
    return (
      <div data-testid="r3f-canvas" style={props.style}>
        {props.children}
      </div>
    );
  },
  useFrame: (cb: (state: unknown, dt: number) => void) => {
    fiber.frames.push(cb);
  },
  useLoader: Object.assign(() => (fiber.rigged ? riggedGltf() : plainGltf()), { preload: vi.fn() }),
  useThree: (sel: (s: Record<string, unknown>) => unknown) => {
    fiber.scene ??= { environment: null, environmentIntensity: 1 };
    return sel({ gl: {}, scene: fiber.scene, invalidate: fiber.invalidate });
  },
}));
vi.mock('three', async (orig) => {
  const three = await orig<typeof import('three')>();
  class PMREMGenerator {
    fromScene() {
      return { texture: { isEnvTexture: true }, dispose: vi.fn() };
    }
    dispose() {}
  }
  return { ...three, PMREMGenerator };
});
vi.mock('three/examples/jsm/environments/RoomEnvironment.js', () => ({
  RoomEnvironment: class {
    dispose() {}
  },
}));
vi.mock('three/examples/jsm/loaders/DRACOLoader.js', () => ({
  DRACOLoader: class {
    setDecoderPath() {}
  },
}));
vi.mock('three/examples/jsm/loaders/GLTFLoader.js', () => ({
  GLTFLoader: class {
    setDRACOLoader() {}
  },
}));

function plainGltf() {
  const scene = new THREE.Group();
  const visor = new THREE.Mesh(new THREE.BoxGeometry(1, 0.4, 0.2), new THREE.MeshStandardMaterial({ name: 'Visor_Glow', emissive: 0x000000 }));
  visor.position.y = 1.6;
  const body = new THREE.Mesh(new THREE.BoxGeometry(1.2, 2, 1), new THREE.MeshStandardMaterial({ name: 'Shell' }));
  body.position.set(0.5, 1, 0);
  scene.add(visor, body);
  return { scene, animations: [] as THREE.AnimationClip[] };
}

const quatTrack = (name: string, times: number[], eulers: Array<[number, number, number]>) =>
  new THREE.QuaternionKeyframeTrack(
    `${name}.quaternion`,
    times,
    eulers.flatMap(([x, y, z]) => new THREE.Quaternion().setFromEuler(new THREE.Euler(x, y, z)).toArray()),
  );

/** Hips > Spine > Neck > Head with a visor mesh, and the stand-in's clip set (Idle + one-shots, no mood loops). */
function riggedGltf() {
  const scene = new THREE.Group();
  const hips = new THREE.Bone();
  hips.name = 'Hips';
  const spine = new THREE.Bone();
  spine.name = 'Spine';
  spine.position.y = 1;
  const neck = new THREE.Bone();
  neck.name = 'Neck';
  neck.position.y = 0.8;
  const head = new THREE.Bone();
  head.name = 'Head';
  head.position.y = 0.2;
  const visor = new THREE.Mesh(new THREE.BoxGeometry(0.6, 0.3, 0.2), new THREE.MeshStandardMaterial({ name: 'Visor', emissive: 0x000000 }));
  visor.position.set(0, 0.2, 0.3);
  const body = new THREE.Mesh(new THREE.BoxGeometry(1, 2, 0.8));
  body.position.y = 1;
  scene.add(hips.add(spine.add(neck.add(head.add(visor)))), body);
  const animations = [
    new THREE.AnimationClip('Idle', 4, [quatTrack('Head', [0, 2, 4], [[0, 0, 0], [0, 0.05, 0], [0, 0, 0]])]),
    new THREE.AnimationClip('Yes', 1.2, [quatTrack('Neck', [0, 0.6, 1.2], [[0, 0, 0], [0.3, 0, 0], [0, 0, 0]])]),
    new THREE.AnimationClip('No', 1.2, [quatTrack('Head', [0, 0.6, 1.2], [[0, 0, 0], [0, 0.2, 0], [0, 0, 0]])]),
    new THREE.AnimationClip('Wave', 2, [quatTrack('Spine', [0, 1, 2], [[0, 0, 0], [0, 0, 0.1], [0, 0, 0]])]),
  ];
  return { scene, animations };
}

import Ubi3d, { FIT, normalise } from './Ubi3d';
import { MOOD_GLOW } from './moods';
import { LOOK_MAX_DT } from './rig';

/** Runs every registered useFrame callback once. */
const frame = (dt = 1 / 60, elapsed = 1) => {
  for (const cb of fiber.frames) cb({ clock: { elapsedTime: elapsed } }, dt);
};

/** The box sits at (100, 50) 200 × 240 px; a bubble, when asked for, is centred 40 px above it. */
const BOX = { left: 100, top: 50, width: 200, height: 240 };
const rectOf = (el: Element): DOMRect => {
  const r = el.getAttribute('data-testid') === 'ubi-3d' ? BOX : el.getAttribute('data-bubble') ? { left: 150, top: 0, width: 100, height: 40 } : { left: 0, top: 0, width: 0, height: 0 };
  return { ...r, right: r.left + r.width, bottom: r.top + r.height, x: r.left, y: r.top, toJSON: () => r } as DOMRect;
};

/** A fake monotonic clock for the glance/attribute throttling; `tick` advances it by one frame. */
let perfSpy: ReturnType<typeof vi.spyOn> | null = null;
const fakeClock = () => {
  let t = 1000;
  perfSpy = vi.spyOn(performance, 'now').mockImplementation(() => t * 1000);
  return {
    frames(n: number) {
      for (let i = 0; i < n; i++) {
        t += 1 / 60;
        frame();
      }
    },
    skip(seconds: number) {
      t += seconds;
    },
  };
};

const look = (box: HTMLElement) => {
  const [yaw, pitch] = (box.getAttribute('data-ubi-look') ?? '0,0').split(',').map(Number) as [number, number];
  return { yaw, pitch };
};

describe('normalise', () => {
  it('fits the largest dimension to FIT units, centres x/z and stands the model on y = 0', () => {
    const scene = new THREE.Group();
    const m = new THREE.Mesh(new THREE.BoxGeometry(1, 4, 2));
    m.position.set(3, 7, -1);
    scene.add(m);
    const { object, emissives } = normalise({ scene });
    const box = new THREE.Box3().setFromObject(object);
    expect(box.max.y - box.min.y).toBeCloseTo(FIT, 5);
    expect(box.min.y).toBeCloseTo(0, 5);
    expect((box.min.x + box.max.x) / 2).toBeCloseTo(0, 5);
    expect((box.min.z + box.max.z) / 2).toBeCloseTo(0, 5);
    expect(emissives).toHaveLength(0);
    // the loader's cached scene is untouched
    expect(scene.scale.x).toBe(1);
    expect(m.position.x).toBe(3);
  });

  it('clones only the eye/visor/crest/glow materials so tinting never leaks into the cache', () => {
    const scene = new THREE.Group();
    const eye = new THREE.MeshStandardMaterial({ name: 'EyeL' });
    const shell = new THREE.MeshStandardMaterial({ name: 'Shell' });
    const crest = new THREE.Mesh(new THREE.BoxGeometry(), new THREE.MeshStandardMaterial({ name: 'Mat.001' }));
    crest.name = 'Crest';
    scene.add(new THREE.Mesh(new THREE.BoxGeometry(), eye), new THREE.Mesh(new THREE.BoxGeometry(), shell), crest);
    const { object, emissives } = normalise({ scene });
    expect(emissives).toHaveLength(2);
    expect(emissives).not.toContain(eye);
    const mats: THREE.Material[] = [];
    object.traverse((n) => {
      if ((n as THREE.Mesh).isMesh) mats.push((n as THREE.Mesh).material as THREE.Material);
    });
    expect(mats).toContain(shell);
  });

  it('keeps the skeleton (bones cloned with the meshes)', () => {
    const { object } = normalise(riggedGltf());
    expect(object.getObjectByName('Head')).toBeInstanceOf(THREE.Bone);
  });
});

describe('Ubi3d', () => {
  const realRect = Element.prototype.getBoundingClientRect;
  beforeAll(() => {
    // the mocked Canvas puts the scene graph (<primitive>, <pointLight>…) in the DOM; react-dom warns about the tags
    const error = console.error;
    vi.spyOn(console, 'error').mockImplementation((...args: unknown[]) => {
      const msg = String(args[0]);
      if (msg.includes('incorrect casing') || msg.includes('unrecognized in this browser') || msg.includes('non-boolean attribute')) return;
      error(...args);
    });
    Element.prototype.getBoundingClientRect = function () {
      return rectOf(this);
    };
  });
  afterEach(() => {
    fiber.reduce = false;
    fiber.rigged = false;
    fiber.frames.length = 0;
    fiber.canvasProps.length = 0;
    fiber.invalidate.mockClear();
    perfSpy?.mockRestore();
    perfSpy = null;
  });
  afterAll(() => {
    Element.prototype.getBoundingClientRect = realRect;
  });

  it('renders a transparent, premultiplied canvas box with the floor glow and tints the visor with the mood colour', async () => {
    render(<Ubi3d mood="excited" size={100} fallback={<span data-testid="fb" />} />);
    const box = screen.getByTestId('ubi-3d');
    expect(box).toHaveAttribute('role', 'img');
    expect(box).toHaveAttribute('aria-label', 'UBI, o mascote (Empolgado)');
    expect(box).toHaveStyle({ width: '100px', height: '120px' });
    expect(screen.getByTestId('ubi-floor-glow')).toBeInTheDocument();
    const props = fiber.canvasProps[0] as { gl: Record<string, unknown>; frameloop: string; dpr: number[] };
    expect(props.gl).toMatchObject({ alpha: true, antialias: true, premultipliedAlpha: true, powerPreference: 'low-power' });
    expect(props.dpr).toEqual([1, 2]);
    expect(props.frameloop).toBe('always');
    expect(screen.getByTestId('r3f-canvas')).toHaveStyle({ background: 'transparent' });
    // the SVG stand-in is shown until the model reports ready, then the canvas fades in
    await waitFor(() => expect(box).toHaveAttribute('data-ready', '1'));
    expect(screen.queryByTestId('fb')).not.toBeInTheDocument();
    expect(fiber.scene?.environment).toEqual({ isEnvTexture: true });
    expect(fiber.invalidate).toHaveBeenCalled();
    expect(fiber.frames.length).toBeGreaterThan(0);
  });

  it('keeps the whole-body motion and no clip for an unrigged export (data-ubi-rig="0")', async () => {
    render(<Ubi3d mood="calm" size={100} speaking="Oi" />);
    const box = screen.getByTestId('ubi-3d');
    await waitFor(() => expect(box).toHaveAttribute('data-ubi-rig', '0'));
    expect(box).not.toHaveAttribute('data-ubi-clip');
    // the idle float runs without throwing and never writes the look attribute
    frame(1 / 60, 1.3);
    frame(1 / 60, 1.32);
    expect(box).not.toHaveAttribute('data-ubi-look');
    fireEvent.pointerMove(window, { clientX: 900, clientY: 100 });
    frame();
    expect(box).not.toHaveAttribute('data-ubi-look');
  });

  it('drives the rig: mood → base clip with the Idle fallbacks, data-ubi-rig="1"', async () => {
    fiber.rigged = true;
    const { rerender } = render(<Ubi3d mood="calm" size={200} />);
    const box = screen.getByTestId('ubi-3d');
    await waitFor(() => expect(box).toHaveAttribute('data-ubi-rig', '1'));
    expect(box).toHaveAttribute('data-ubi-clip', 'Idle');
    rerender(<Ubi3d mood="excited" size={200} />);
    expect(box).toHaveAttribute('data-ubi-clip', 'Idle'); // Excited is missing from this export
    rerender(<Ubi3d mood="sleeping" size={200} />);
    expect(box).toHaveAttribute('data-ubi-clip', 'Idle');
    frame();
    expect(box).toHaveAttribute('data-ubi-look', '0.0,0.0');
  });

  it('turns the head towards the pointer anywhere in the window, eased and clamped', async () => {
    fiber.rigged = true;
    const { frames } = fakeClock();
    render(<Ubi3d mood="calm" size={200} />);
    const box = screen.getByTestId('ubi-3d');
    await waitFor(() => expect(box).toHaveAttribute('data-ubi-rig', '1'));
    // far to the right of the head (head at x = 200): eased, then the yaw clamps at +45
    fireEvent.pointerMove(window, { clientX: 1200, clientY: 103 });
    frames(1);
    const first = look(box);
    expect(first.yaw).toBeGreaterThan(0);
    expect(first.yaw).toBeLessThan(45);
    frames(90);
    expect(look(box).yaw).toBeCloseTo(45, 0);
    expect(look(box).pitch).toBeCloseTo(0, 0);
    // to the left and above: negative yaw, positive pitch
    fireEvent.pointerMove(window, { clientX: 0, clientY: 0 });
    frames(90);
    expect(look(box).yaw).toBeCloseTo(-30, 0);
    expect(look(box).pitch).toBeGreaterThan(10);
    // the pointer leaves the window: back to rest
    fireEvent.pointerOut(document, { relatedTarget: null });
    frames(90);
    expect(look(box).yaw).toBeCloseTo(0, 0);
  });

  it('glances at the bubble when the speech changes, over the pointer, then goes back to the pointer', async () => {
    fiber.rigged = true;
    const bubble = document.createElement('div');
    bubble.setAttribute('data-bubble', '1');
    document.body.appendChild(bubble);
    const bubbleRef = { current: bubble };
    const clock = fakeClock();
    const { rerender } = render(<Ubi3d mood="calm" size={200} speaking="Olá" bubbleRef={bubbleRef} />);
    const box = screen.getByTestId('ubi-3d');
    await waitFor(() => expect(box).toHaveAttribute('data-ubi-rig', '1'));
    fireEvent.pointerMove(window, { clientX: 1200, clientY: 103 });
    // the speech is on screen from the start: a glance up at the bubble wins over the pointer for 2.5 s
    clock.frames(60);
    expect(look(box).pitch).toBeGreaterThan(20);
    expect(Math.abs(look(box).yaw)).toBeLessThan(2);
    // after the glance the pointer takes over again
    clock.skip(3);
    clock.frames(120);
    expect(look(box).yaw).toBeCloseTo(45, 0);
    // a new line → a new glance
    rerender(<Ubi3d mood="calm" size={200} speaking="Outra frase" bubbleRef={bubbleRef} />);
    clock.frames(60);
    expect(look(box).pitch).toBeGreaterThan(20);
    bubble.remove();
  });

  it('honors prefers-reduced-motion: on-demand frameloop, frozen clip, look-at without easing', async () => {
    fiber.reduce = true;
    fiber.rigged = true;
    render(<Ubi3d mood="sleeping" size={200} />);
    const props = fiber.canvasProps[0] as { frameloop: string };
    expect(props.frameloop).toBe('demand');
    const box = screen.getByTestId('ubi-3d');
    await waitFor(() => expect(box).toHaveAttribute('data-ubi-rig', '1'));
    expect(box).toHaveAttribute('data-ubi-clip', 'Idle');
    fiber.invalidate.mockClear();
    fireEvent.pointerMove(window, { clientX: 1200, clientY: 103 });
    // the pointer move asks for a frame, and that single frame snaps the head to the clamp
    expect(fiber.invalidate).toHaveBeenCalled();
    frame();
    expect(look(box).yaw).toBe(45);
  });

  it('refreshes data-ubi-look on every reduced-motion frame (no throttling between on-demand frames)', async () => {
    fiber.reduce = true;
    fiber.rigged = true;
    render(<Ubi3d mood="calm" size={200} />);
    const box = screen.getByTestId('ubi-3d');
    await waitFor(() => expect(box).toHaveAttribute('data-ubi-rig', '1'));
    // two pointer events well inside the 0.1 s throttle window, one frame each: the attribute follows the last one
    fireEvent.pointerMove(window, { clientX: 1200, clientY: 103 });
    frame();
    expect(look(box).yaw).toBe(45);
    fireEvent.pointerMove(window, { clientX: 0, clientY: 103 });
    frame();
    expect(look(box).yaw).toBe(-30);
  });

  it('feeds the mixer wall time up to LOOK_MAX_DT so a slow frame does not play the clips in slow motion', async () => {
    fiber.rigged = true;
    const update = vi.spyOn(THREE.AnimationMixer.prototype, 'update');
    render(<Ubi3d mood="calm" size={200} />);
    const box = screen.getByTestId('ubi-3d');
    await waitFor(() => expect(box).toHaveAttribute('data-ubi-rig', '1'));
    update.mockClear();
    frame(0.15);
    expect(update).toHaveBeenLastCalledWith(0.15);
    frame(0.5);
    expect(update).toHaveBeenLastCalledWith(LOOK_MAX_DT);
    update.mockRestore();
  });

  it('honors prefers-reduced-motion with a static pose for an unrigged export', () => {
    fiber.reduce = true;
    render(<Ubi3d mood="sleeping" size={80} />);
    const props = fiber.canvasProps[0] as { frameloop: string };
    expect(props.frameloop).toBe('demand');
    frame();
  });

  it('plays a one-shot on a tap without changing the base clip', async () => {
    fiber.rigged = true;
    render(<Ubi3d mood="calm" size={200} />);
    const box = screen.getByTestId('ubi-3d');
    await waitFor(() => expect(box).toHaveAttribute('data-ubi-rig', '1'));
    act(() => {
      fireEvent.pointerDown(box);
    });
    frame();
    expect(box).toHaveAttribute('data-ubi-clip', 'Idle');
  });

  it('exposes one glow colour per mood', () => {
    for (const mood of ['sleeping', 'calm', 'focused', 'excited', 'worried'] as const) expect(MOOD_GLOW[mood]).toMatch(/^#[0-9a-f]{6}$/i);
  });
});
