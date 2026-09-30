// The 3D UBI for the hero. Lazy-loaded (three lives in its own chunk) and only mounted when WebGL exists.
// Same rig as the desktop app (./ubi-rig.ts, copied from apps/desktop): the glTF carries the Idle/Wave/Yes/Jump/
// Excited clips and a Head/Neck skeleton, so an AnimationMixer drives the body while a procedural look-at is
// layered on the head. The mascot follows the pointer, glances at the speech bubble and plays a one-shot now and
// then. Stops rendering when off screen or when the tab is hidden.
import { useEffect, useRef, type RefObject } from 'react';
import * as THREE from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { DRACOLoader } from 'three/examples/jsm/loaders/DRACOLoader.js';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';
import {
  BODY_MOTION_WITH_RIG,
  CLIPS,
  Cooldown,
  GlanceScheduler,
  REST_LOOK,
  Rig,
  easeLook,
  lookAtBubble,
  lookAtPoint,
  resolveLook,
  tapClipFor,
  type Look,
  type Mood,
} from './ubi-rig';

const BASE = import.meta.env.BASE_URL;
const GLB_URL = `${BASE}ubi/Ubi.glb`;
const FIT = 2.4;
const GLOW = '#4d8dff';
const EMISSIVE_RE = /eye|visor|crest|glow|emiss/i;

/** Moods the landing page cycles through; the negative ones belong in the app, not in a hero. */
const MOODS: Mood[] = ['calm', 'focused', 'excited'];
/** Seconds between mood changes and between the idle one-shots. */
const MOOD_EVERY = 18;
const ONE_SHOT_EVERY = 11;
/** One-shots that read as friendly on a landing page. */
const IDLE_ONE_SHOTS: string[] = [CLIPS.wave, CLIPS.yes, CLIPS.jump, CLIPS.excited];

type EmissiveMaterial = THREE.Material & { emissive: THREE.Color; emissiveIntensity: number };
const hasEmissive = (m: THREE.Material): m is EmissiveMaterial => 'emissive' in m && (m as EmissiveMaterial).emissive instanceof THREE.Color;

const pick = <T,>(list: readonly T[]): T => list[Math.floor(Math.random() * list.length)] as T;

function normalise(gltf: Pick<GLTF, 'scene'>): { object: THREE.Object3D; emissives: EmissiveMaterial[] } {
  const object = gltf.scene;
  object.rotation.set(0, 0, 0);
  object.position.set(0, 0, 0);
  object.scale.setScalar(1);
  const emissives: EmissiveMaterial[] = [];
  const tint = new THREE.Color(GLOW);
  object.traverse((node) => {
    const mesh = node as THREE.Mesh;
    if (!mesh.isMesh) return;
    mesh.frustumCulled = false;
    const mats = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    for (const m of mats) {
      if (hasEmissive(m) && (EMISSIVE_RE.test(m.name) || EMISSIVE_RE.test(mesh.name))) {
        m.emissive.copy(tint);
        emissives.push(m);
      }
    }
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

export interface UbiHero3dProps {
  /** Capture mode: one static frame, no clips, no look-at. */
  still: boolean;
  /** Text currently in the speech bubble; a change makes the mascot glance at it (and nod). */
  speech?: string;
  /** The bubble element, so the glance aims at where it actually is. */
  bubbleRef?: RefObject<HTMLElement | null>;
  onReady: () => void;
  onError: () => void;
}

export default function UbiHero3d({ still, speech, bubbleRef, onReady, onError }: UbiHero3dProps) {
  const host = useRef<HTMLDivElement>(null);
  const cb = useRef({ onReady, onError });
  cb.current = { onReady, onError };
  // read by the render loop without re-running the effect
  const live = useRef({ speech, bubbleRef });
  live.current = { speech, bubbleRef };

  useEffect(() => {
    const el = host.current;
    if (!el) return;
    let disposed = false;
    let raf = 0;
    let active = true;
    let visible = document.visibilityState !== 'hidden';

    const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true, premultipliedAlpha: true, powerPreference: 'low-power' });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    renderer.setClearColor(0x000000, 0);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    renderer.domElement.style.position = 'absolute';
    renderer.domElement.style.inset = '0';
    renderer.domElement.style.width = '100%';
    renderer.domElement.style.height = '100%';
    el.appendChild(renderer.domElement);

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(30, 1, 0.1, 40);
    camera.position.set(0, 1.55, 5.6);
    camera.lookAt(new THREE.Vector3(0, FIT * 0.5, 0));

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

    const group = new THREE.Group();
    scene.add(group);

    const resize = () => {
      const w = el.clientWidth || 1;
      const h = el.clientHeight || 1;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h, false);
    };
    resize();
    const ro = new ResizeObserver(() => {
      resize();
      if (still) renderer.render(scene, camera);
    });
    ro.observe(el);

    // pointer in viewport coordinates, so the look-at can aim at it from where the head is drawn
    const pointer = { x: 0, y: 0, seen: false };
    const onMove = (e: PointerEvent) => {
      if (still) return;
      pointer.x = e.clientX;
      pointer.y = e.clientY;
      pointer.seen = true;
    };
    window.addEventListener('pointermove', onMove, { passive: true });

    let rig: Rig | null = null;
    let emissives: EmissiveMaterial[] = [];
    let mood: Mood = 'calm';
    let moodAt = 0;
    let shotAt = 0;
    const eased: Look = { ...REST_LOOK };
    const glances = new GlanceScheduler();
    const nod = new Cooldown(6);
    let lastSpeech = live.current.speech;

    const timer = new THREE.Timer();

    const frame = (now?: number) => {
      raf = 0;
      if (disposed) return;
      timer.update(now);
      const dt = Math.min(timer.getDelta(), 0.05);
      const t = timer.getElapsed();

      if (!still) {
        // the clips drive the body; the group only adds a slow drift on top of them
        const amp = rig?.rigged ? BODY_MOTION_WITH_RIG : 1;
        group.rotation.set(Math.sin(t * 0.45) * 0.03 * amp, Math.sin(t * 0.32) * 0.42 * amp, Math.sin(t * 0.6) * 0.015 * amp);
        group.position.y = Math.sin((t * Math.PI * 2) / 4.2) * 0.07 * amp;

        if (rig) {
          // a new bubble text: glance at it and, at most once every few seconds, nod
          const speechNow = live.current.speech;
          if (speechNow !== lastSpeech) {
            lastSpeech = speechNow;
            if (speechNow) {
              glances.speech(t);
              if (nod.take(t)) rig.playOnce(CLIPS.yes);
            }
          }
          // mood changes and idle one-shots keep it alive while nobody interacts
          if (t - moodAt > MOOD_EVERY) {
            moodAt = t;
            mood = pick(MOODS);
            rig.setMood(mood);
          }
          if (t - shotAt > ONE_SHOT_EVERY && !rig.busy) {
            shotAt = t;
            rig.playOnce(pick(IDLE_ONE_SHOTS));
          }

          rig.update(dt);

          const box = el.getBoundingClientRect();
          const bubble = live.current.bubbleRef?.current?.getBoundingClientRect() ?? null;
          const glancing = glances.active(t, Boolean(bubble && live.current.speech));
          const target = resolveLook(
            glancing && bubble ? lookAtBubble(box, box.width, bubble) : null,
            pointer.seen ? lookAtPoint(box, box.width, pointer.x, pointer.y) : null,
          );
          rig.look(easeLook(eased, target, dt));
        }

        if (emissives.length) {
          const ph = t % 4.6;
          const k = ph < 0.16 ? 1 - Math.sin((ph / 0.16) * Math.PI) * 0.85 : 1;
          for (const m of emissives) m.emissiveIntensity = 1.1 * k;
        }
      }
      renderer.render(scene, camera);
      if (!still && active && visible) raf = requestAnimationFrame(frame);
    };
    const kick = () => {
      if (!raf && !disposed) raf = requestAnimationFrame(frame);
    };
    const onTap = () => {
      if (still || !rig) return;
      rig.playOnce(tapClipFor(mood));
      kick();
    };
    el.addEventListener('pointerdown', onTap);

    const io = new IntersectionObserver((entries) => {
      active = entries.some((e) => e.isIntersecting);
      if (active) kick();
    });
    io.observe(el);
    const onVis = () => {
      visible = document.visibilityState !== 'hidden';
      if (visible) kick();
    };
    document.addEventListener('visibilitychange', onVis);

    // no setDecoderPath: DRACOLoader resolves three's bundled decoder through import.meta.url, which Vite emits
    // next to the three chunk, so nothing is fetched from a CDN
    const draco = new DRACOLoader();
    const loader = new GLTFLoader();
    loader.setDRACOLoader(draco);
    loader.load(
      GLB_URL,
      (gltf) => {
        if (disposed) return;
        const n = normalise(gltf);
        emissives = n.emissives;
        for (const m of emissives) m.emissiveIntensity = 1.1;
        group.add(n.object);
        rig = new Rig(n.object, gltf.animations ?? []);
        rig.attach();
        rig.setMood(mood, still);
        if (still) group.rotation.set(0, -0.18, 0);
        renderer.render(scene, camera);
        cb.current.onReady();
        kick();
      },
      undefined,
      () => {
        if (!disposed) cb.current.onError();
      },
    );

    return () => {
      disposed = true;
      if (raf) cancelAnimationFrame(raf);
      ro.disconnect();
      io.disconnect();
      window.removeEventListener('pointermove', onMove);
      el.removeEventListener('pointerdown', onTap);
      document.removeEventListener('visibilitychange', onVis);
      rig?.detach();
      timer.dispose();
      draco.dispose();
      env.dispose();
      scene.traverse((node) => {
        const mesh = node as THREE.Mesh;
        if (!mesh.isMesh) return;
        mesh.geometry?.dispose();
        const mats = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
        for (const m of mats) m?.dispose();
      });
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, [still]);

  return <div ref={host} className="absolute inset-0" aria-hidden="true" />;
}
