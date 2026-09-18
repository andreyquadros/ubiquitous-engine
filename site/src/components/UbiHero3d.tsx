// The 3D UBI for the hero. Lazy-loaded (three lives in its own chunk) and only mounted when WebGL exists.
// Mirrors the desktop app's Ubi3d: GLTFLoader + Draco decoder (three's own copy, bundled as hashed assets), RoomEnvironment IBL, a key light, a volt
// fill from below, normalised to FIT units with the feet on y = 0, slow turn, pointer parallax and a blink on
// emissive materials. Stops rendering when off screen or when the tab is hidden.
import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { RoomEnvironment } from 'three/examples/jsm/environments/RoomEnvironment.js';
import { DRACOLoader } from 'three/examples/jsm/loaders/DRACOLoader.js';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';

const BASE = import.meta.env.BASE_URL;
const GLB_URL = `${BASE}ubi/Ubi.glb`;
const FIT = 2.4;
const GLOW = '#4d8dff';
const EMISSIVE_RE = /eye|visor|crest|glow|emiss/i;

type EmissiveMaterial = THREE.Material & { emissive: THREE.Color; emissiveIntensity: number };
const hasEmissive = (m: THREE.Material): m is EmissiveMaterial => 'emissive' in m && (m as EmissiveMaterial).emissive instanceof THREE.Color;

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
  /** Reduced motion or the capture mode: one static frame, no turn, no float. */
  still: boolean;
  onReady: () => void;
  onError: () => void;
}

export default function UbiHero3d({ still, onReady, onError }: UbiHero3dProps) {
  const host = useRef<HTMLDivElement>(null);
  const cb = useRef({ onReady, onError });
  cb.current = { onReady, onError };

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

    const pointer = { x: 0, y: 0 };
    const eased = { yaw: 0, pitch: 0 };
    const onMove = (e: PointerEvent) => {
      if (still) return;
      pointer.x = (e.clientX / window.innerWidth - 0.5) * 2;
      pointer.y = (e.clientY / window.innerHeight - 0.5) * 2;
    };
    window.addEventListener('pointermove', onMove, { passive: true });

    let emissives: EmissiveMaterial[] = [];
    const timer = new THREE.Timer();

    const frame = (now?: number) => {
      raf = 0;
      if (disposed) return;
      timer.update(now);
      const dt = Math.min(timer.getDelta(), 0.05);
      const t = timer.getElapsed();
      if (!still) {
        eased.yaw = THREE.MathUtils.damp(eased.yaw, pointer.x * 0.28, 5, dt);
        eased.pitch = THREE.MathUtils.damp(eased.pitch, pointer.y * 0.12, 5, dt);
        group.rotation.set(eased.pitch + Math.sin(t * 0.45) * 0.03, Math.sin(t * 0.32) * 0.42 + eased.yaw, Math.sin(t * 0.6) * 0.015);
        group.position.y = Math.sin((t * Math.PI * 2) / 4.2) * 0.07;
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
      document.removeEventListener('visibilitychange', onVis);
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
