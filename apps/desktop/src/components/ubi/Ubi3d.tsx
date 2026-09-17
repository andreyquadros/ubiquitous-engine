import { Canvas, useFrame } from '@react-three/fiber';
import { useGLTF, Float, Center } from '@react-three/drei';
import { Suspense, useMemo, useRef } from 'react';
import * as THREE from 'three';
import type { Mood } from '../../lib/types';
import { MOOD_GLOW } from './moods';

export const GLB_URL = '/ubi/Ubi.glb';
/** Local Draco decoder (public/draco) — keeps compressed models offline and CSP-safe. */
export const DRACO_PATH = '/draco/';

function Model({ mood }: { mood: Mood }) {
  const { scene } = useGLTF(GLB_URL, DRACO_PATH);
  const group = useRef<THREE.Group>(null);
  const normalized = useMemo(() => {
    const clone = scene.clone(true);
    const box = new THREE.Box3().setFromObject(clone);
    const size = new THREE.Vector3();
    box.getSize(size);
    const scale = 2.4 / Math.max(size.x, size.y, size.z, 0.001);
    clone.scale.setScalar(scale);
    return clone;
  }, [scene]);

  useFrame((state) => {
    if (!group.current) return;
    const t = state.clock.elapsedTime;
    const speed = mood === 'sleeping' ? 0.25 : mood === 'excited' ? 1.4 : 0.6;
    group.current.rotation.y = Math.sin(t * speed) * 0.25;
    group.current.rotation.z = mood === 'worried' ? Math.sin(t * 2) * 0.03 : 0;
  });

  return (
    <group ref={group}>
      <primitive object={normalized} />
    </group>
  );
}

export default function Ubi3d({ mood, size }: { mood: Mood; size: number }) {
  const glow = MOOD_GLOW[mood];
  return (
    <div style={{ width: size, height: size * 1.2 }} data-testid="ubi-3d">
      <Canvas dpr={[1, 2]} camera={{ position: [0, 0.6, 4.6], fov: 32 }} gl={{ alpha: true, antialias: true }} style={{ background: 'transparent' }}>
        <ambientLight intensity={0.9} />
        <directionalLight position={[3, 5, 3]} intensity={1.4} />
        <pointLight position={[0, -1.5, 2]} intensity={mood === 'sleeping' ? 0.4 : 2.5} color={glow} distance={8} />
        <Suspense fallback={null}>
          <Float speed={mood === 'sleeping' ? 1 : 2} rotationIntensity={0.15} floatIntensity={mood === 'excited' ? 1.2 : 0.6}>
            <Center>
              <Model mood={mood} />
            </Center>
          </Float>
        </Suspense>
      </Canvas>
    </div>
  );
}

useGLTF.preload(GLB_URL, DRACO_PATH);
