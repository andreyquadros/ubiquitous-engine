import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import * as THREE from 'three';

// jsdom has no WebGL: the Canvas becomes a plain box that renders the scene graph through React (so
// effects run), the loader returns a hand-made scene and the PMREM step is a stub.
const fiber = vi.hoisted(() => ({
  frames: [] as Array<(state: unknown, dt: number) => void>,
  canvasProps: [] as Array<Record<string, unknown>>,
  invalidate: vi.fn(),
  scene: null as null | { environment: unknown; environmentIntensity: number },
  reduce: false,
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
  useLoader: Object.assign(
    () => {
      const scene = new THREE.Group();
      const visor = new THREE.Mesh(new THREE.BoxGeometry(1, 0.4, 0.2), new THREE.MeshStandardMaterial({ name: 'Visor_Glow', emissive: 0x000000 }));
      visor.position.y = 1.6;
      const body = new THREE.Mesh(new THREE.BoxGeometry(1.2, 2, 1), new THREE.MeshStandardMaterial({ name: 'Shell' }));
      body.position.set(0.5, 1, 0);
      scene.add(visor, body);
      return { scene };
    },
    { preload: vi.fn() },
  ),
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

import Ubi3d, { configureLoader, FIT, normalise } from './Ubi3d';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { MOOD_GLOW } from './moods';

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
});

describe('Ubi3d', () => {
  beforeAll(() => {
    // the mocked Canvas puts the scene graph (<group>, <pointLight>…) in the DOM; react-dom warns about the tags
    const error = console.error;
    vi.spyOn(console, 'error').mockImplementation((...args: unknown[]) => {
      const msg = String(args[0]);
      if (msg.includes('incorrect casing') || msg.includes('unrecognized in this browser')) return;
      error(...args);
    });
  });
  afterEach(() => {
    fiber.reduce = false;
    fiber.frames.length = 0;
    fiber.canvasProps.length = 0;
    fiber.invalidate.mockClear();
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
    // one idle-motion frame: the visor's emissive follows MOOD_GLOW and the group bobs
    expect(fiber.frames.length).toBeGreaterThan(0);
  });

  it('honors prefers-reduced-motion with a static pose and an on-demand frameloop', () => {
    fiber.reduce = true;
    render(<Ubi3d mood="sleeping" size={80} />);
    const props = fiber.canvasProps[0] as { frameloop: string };
    expect(props.frameloop).toBe('demand');
  });

  it('attaches the local Draco decoder to the loader', () => {
    const loader = new GLTFLoader();
    const spy = vi.spyOn(loader, 'setDRACOLoader');
    configureLoader(loader);
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('exposes one glow colour per mood', () => {
    for (const mood of ['sleeping', 'calm', 'focused', 'excited', 'worried'] as const) expect(MOOD_GLOW[mood]).toMatch(/^#[0-9a-f]{6}$/i);
  });
});
