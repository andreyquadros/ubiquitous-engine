import { Canvas, useFrame, useLoader, useThree, type RootState } from '@react-three/fiber';
import { useReducedMotion } from 'framer-motion';
import { Suspense, useEffect, useMemo, useRef, useState, type PointerEvent, type ReactNode, type RefObject } from 'react';
import * as THREE from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { DRACOLoader } from 'three/examples/jsm/loaders/DRACOLoader.js';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { clone as cloneSkeleton } from 'three/examples/jsm/utils/SkeletonUtils.js';
import { useT } from '../../i18n';
import type { Mood } from '../../lib/types';
import { FLOAT_PERIOD, FloorGlow } from './FloorGlow';
import { MOOD_GLOW } from './moods';

/** The user's model, installed by scripts/install-ubi-model.sh and committed with the app. */
export const GLB_URL = '/ubi/Ubi.glb';
/** Local Draco decoder (public/draco) — keeps compressed models offline and CSP-safe. */
export const DRACO_PATH = '/draco/';
/**
 * Yaw applied to the export, in radians. `0` assumes UBI was exported facing +Z (towards the camera); use
 * `Math.PI` for an export that faces away, `±Math.PI / 2` for one that faces sideways.
 */
export const ROTATION_Y = 0;
/** The model's largest dimension is scaled to this many scene units; the camera is framed around it. */
export const FIT = 2.4;
/** Materials (or the meshes wearing them) named like this get the mood colour as emissive tint and a blink. */
const EMISSIVE_RE = /eye|visor|crest|glow|emiss/i;

const CAMERA = { position: [0, 1.55, 5.6] as [number, number, number], fov: 30, near: 0.1, far: 40 };
const LOOK_AT = new THREE.Vector3(0, FIT * 0.5, 0);
/**
 * `premultipliedAlpha: true` + MSAA: an anti-aliased edge pixel comes out of the resolve as
 * (colour × coverage, coverage), which is exactly what the compositor expects from a premultiplied canvas, so
 * the silhouette blends over the panel without the dark fringe a straight-alpha canvas shows.
 */
const GL = { alpha: true, antialias: true, premultipliedAlpha: true, powerPreference: 'low-power' as const };
const DPR: [number, number] = [1, 2];

let draco: DRACOLoader | null = null;
/** Attaches the local Draco decoder; used by `useLoader` and its preload. */
export function configureLoader(loader: GLTFLoader): void {
  if (!draco) {
    draco = new DRACOLoader();
    draco.setDecoderPath(DRACO_PATH);
  }
  loader.setDRACOLoader(draco);
}

type EmissiveMaterial = THREE.Material & { emissive: THREE.Color; emissiveIntensity: number };
const hasEmissive = (m: THREE.Material): m is EmissiveMaterial => 'emissive' in m && (m as EmissiveMaterial).emissive instanceof THREE.Color;

export interface Normalised {
  object: THREE.Object3D;
  /** Cloned materials that follow the mood colour (empty when the export has no eye/visor/crest materials). */
  emissives: EmissiveMaterial[];
}

/**
 * Prepares one instance of the loaded scene: skeleton-safe clone, largest dimension → FIT units, centred on
 * x/z, feet on y = 0, facing +Z (after ROTATION_Y). Emissive-looking materials are cloned so tinting them does
 * not leak into the shared loader cache.
 */
export function normalise(gltf: Pick<GLTF, 'scene'>): Normalised {
  const object = cloneSkeleton(gltf.scene);
  object.rotation.set(0, ROTATION_Y, 0);
  object.position.set(0, 0, 0);
  object.scale.setScalar(1);
  const emissives: EmissiveMaterial[] = [];
  object.traverse((node) => {
    const mesh = node as THREE.Mesh;
    if (!mesh.isMesh) return;
    mesh.frustumCulled = false;
    const mats = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    const next = mats.map((m) => {
      if (!hasEmissive(m) || !(EMISSIVE_RE.test(m.name) || EMISSIVE_RE.test(mesh.name))) return m;
      const c = m.clone() as EmissiveMaterial;
      emissives.push(c);
      return c;
    });
    mesh.material = Array.isArray(mesh.material) ? next : (next[0] as THREE.Material);
  });
  object.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(object);
  const size = box.getSize(new THREE.Vector3());
  const center = box.getCenter(new THREE.Vector3());
  const s = FIT / Math.max(size.x, size.y, size.z, 1e-3);
  object.scale.setScalar(s);
  object.position.set(-center.x * s, -box.min.y * s, -center.z * s);
  object.updateMatrixWorld(true);
  return { object, emissives };
}

/** Image-based lighting from three's RoomEnvironment (no network), built once per renderer and disposed with it. */
function Environment() {
  const gl = useThree((s) => s.gl);
  const scene = useThree((s) => s.scene);
  const invalidate = useThree((s) => s.invalidate);
  useEffect(() => {
    const pmrem = new THREE.PMREMGenerator(gl);
    const room = new RoomEnvironment();
    const target = pmrem.fromScene(room, 0.04);
    room.dispose();
    pmrem.dispose();
    scene.environment = target.texture;
    scene.environmentIntensity = 0.8;
    invalidate();
    return () => {
      if (scene.environment === target.texture) scene.environment = null;
      target.dispose();
    };
  }, [gl, scene, invalidate]);
  return null;
}

interface Pointer {
  x: number;
  y: number;
}

/** Static pose per mood, used when motion is reduced and as the resting pose the idle motion swings around. */
const REST_PITCH: Record<Mood, number> = { sleeping: 0.16, calm: 0, focused: 0, excited: 0, worried: 0.05 };

function Model({ mood, pointer, reduce, onReady }: { mood: Mood; pointer: RefObject<Pointer>; reduce: boolean; onReady: () => void }) {
  const gltf = useLoader(GLTFLoader, GLB_URL, configureLoader);
  const invalidate = useThree((s) => s.invalidate);
  const group = useRef<THREE.Group>(null);
  const eased = useRef({ yaw: 0, pitch: 0 });
  const { object, emissives } = useMemo(() => normalise(gltf), [gltf]);

  useEffect(() => {
    onReady();
    return () => emissives.forEach((m) => m.dispose());
  }, [emissives, onReady]);

  useEffect(() => {
    const c = new THREE.Color(MOOD_GLOW[mood]);
    for (const m of emissives) m.emissive.copy(c);
    // with frameloop 'demand' (reduced motion) this schedules the one frame that applies the static pose below
    invalidate();
  }, [mood, emissives, reduce, invalidate]);

  useFrame((state: RootState, dt: number) => {
    const g = group.current;
    if (!g) return;
    if (reduce) {
      g.position.y = 0;
      g.rotation.set(REST_PITCH[mood], 0, 0);
      for (const m of emissives) m.emissiveIntensity = mood === 'sleeping' ? 0.35 : 1.1;
      return;
    }
    const t = state.clock.elapsedTime;
    const d = Math.min(dt, 0.05);
    const w = (Math.PI * 2) / FLOAT_PERIOD[mood];
    // pointer parallax: a few degrees towards the cursor, eased
    const p = pointer.current ?? { x: 0, y: 0 };
    eased.current.yaw = THREE.MathUtils.damp(eased.current.yaw, p.x * 0.35, 6, d);
    eased.current.pitch = THREE.MathUtils.damp(eased.current.pitch, p.y * 0.18, 6, d);
    let y = 0;
    let yaw = Math.sin(t * w * 0.5) * 0.07;
    let pitch = REST_PITCH[mood];
    let roll = 0;
    switch (mood) {
      case 'sleeping':
        y = Math.sin(t * w) * 0.04;
        yaw = Math.sin(t * w * 0.5) * 0.03;
        pitch += Math.sin(t * w) * 0.012;
        break;
      case 'excited':
        y = Math.abs(Math.sin(t * w)) * 0.16;
        yaw = Math.sin(t * w * 0.5) * 0.12;
        roll = Math.sin(t * w) * 0.03;
        break;
      case 'worried':
        y = Math.sin(t * w) * 0.05;
        yaw = Math.sin(t * w * 0.5) * 0.05 + Math.sin(t * 38) * 0.008;
        roll = Math.sin(t * 41) * 0.006;
        break;
      default:
        y = Math.sin(t * w) * 0.08;
        roll = Math.sin(t * w) * 0.02;
    }
    g.position.y = y;
    g.rotation.set(pitch + eased.current.pitch, yaw + eased.current.yaw, roll);
    if (emissives.length) {
      let k = 1;
      if (mood === 'sleeping') k = 0.8 + Math.sin(t * w) * 0.2;
      else {
        const ph = t % 4.6;
        if (ph < 0.16) k = 1 - Math.sin((ph / 0.16) * Math.PI) * 0.85; // blink
        if (mood === 'excited') k *= 1 + Math.sin(t * w) * 0.15;
      }
      const base = mood === 'sleeping' ? 0.35 : mood === 'excited' ? 1.5 : 1.1;
      for (const m of emissives) m.emissiveIntensity = base * k;
    }
  });

  return (
    <group ref={group}>
      <primitive object={object} />
    </group>
  );
}

/** Whether `ref` is on screen and the document is visible — the render loop stops otherwise. */
function useActive(ref: RefObject<HTMLElement | null>): boolean {
  const [onScreen, setOnScreen] = useState(true);
  const [visible, setVisible] = useState(() => typeof document === 'undefined' || document.visibilityState !== 'hidden');
  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === 'undefined') return;
    const io = new IntersectionObserver((entries) => setOnScreen(entries.some((e) => e.isIntersecting)), { threshold: 0 });
    io.observe(el);
    return () => io.disconnect();
  }, [ref]);
  useEffect(() => {
    const on = () => setVisible(document.visibilityState !== 'hidden');
    document.addEventListener('visibilitychange', on);
    return () => document.removeEventListener('visibilitychange', on);
  }, []);
  return onScreen && visible;
}

export interface Ubi3dProps {
  mood: Mood;
  /** Width in px; the box is size × 1.2 like the other UBI variants. */
  size: number;
  /** Shown in the same box until the model is on screen (the SVG UBI), then cross-faded out. */
  fallback?: ReactNode;
}

/**
 * UBI from the user's glTF: direct GLTFLoader (+ local Draco), transparent premultiplied canvas over the UI,
 * RoomEnvironment IBL, mood-coloured fill light and floor glow, idle float with a mood tempo, pointer parallax
 * and a blink on emissive eye/visor materials. Honors prefers-reduced-motion and stops rendering off screen.
 */
export default function Ubi3d({ mood, size, fallback }: Ubi3dProps) {
  const t = useT();
  const reduce = !!useReducedMotion();
  const glow = MOOD_GLOW[mood];
  const box = useRef<HTMLDivElement>(null);
  const active = useActive(box);
  const pointer = useRef<Pointer>({ x: 0, y: 0 });
  const [ready, setReady] = useState(false);
  const onReady = useRef(() => setReady(true)).current;
  const frameloop = !active ? 'never' : reduce ? 'demand' : 'always';
  const boxH = size * 1.2;

  const onMove = (e: PointerEvent<HTMLDivElement>) => {
    if (reduce) return;
    const b = e.currentTarget.getBoundingClientRect();
    pointer.current = { x: (e.clientX - b.left) / b.width - 0.5, y: (e.clientY - b.top) / b.height - 0.5 };
  };
  const onLeave = () => {
    pointer.current = { x: 0, y: 0 };
  };

  return (
    <div
      ref={box}
      className="relative select-none"
      style={{ width: size, height: boxH }}
      data-testid="ubi-3d"
      data-mood={mood}
      data-ready={ready ? '1' : '0'}
      role="img"
      aria-label={t('ubi.mascot', { mood: t(`common.mood.${mood}`) })}
      onPointerMove={onMove}
      onPointerLeave={onLeave}
    >
      <FloorGlow mood={mood} size={size} reduce={reduce} />
      {!ready && fallback && (
        <div className="absolute inset-0 flex items-end justify-center" aria-hidden>
          {fallback}
        </div>
      )}
      <Canvas
        frameloop={frameloop}
        dpr={DPR}
        gl={GL}
        camera={CAMERA}
        flat={false}
        style={{ position: 'absolute', inset: 0, background: 'transparent', opacity: ready ? 1 : 0, transition: 'opacity 240ms ease-out' }}
        onCreated={({ gl, camera }) => {
          gl.setClearColor(0x000000, 0);
          gl.outputColorSpace = THREE.SRGBColorSpace;
          gl.toneMapping = THREE.ACESFilmicToneMapping;
          gl.toneMappingExposure = 1.05;
          camera.lookAt(LOOK_AT);
        }}
      >
        <Environment />
        {/* soft key from the top right, a mood-coloured fill from below and a faint rim behind */}
        <directionalLight position={[2.5, 5, 3.5]} intensity={1.6} color="#ffffff" />
        <pointLight position={[0, -1.4, 2.2]} color={glow} intensity={mood === 'sleeping' ? 2 : 7} distance={7} decay={2} />
        <directionalLight position={[-3, 2.5, -3]} intensity={0.5} color={glow} />
        <Suspense fallback={null}>
          <Model mood={mood} pointer={pointer} reduce={reduce} onReady={onReady} />
        </Suspense>
      </Canvas>
    </div>
  );
}

useLoader.preload(GLTFLoader, GLB_URL, configureLoader);
