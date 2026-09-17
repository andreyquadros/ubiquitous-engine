import { Component, lazy, Suspense, useEffect, useState, type ReactNode } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import clsx from 'clsx';
import type { Mood } from '../../lib/types';
import { UbiSvg } from './UbiSvg';

const Ubi3d = lazy(() => import('./Ubi3d'));

export interface UbiProps {
  mood: Mood;
  size?: number;
  speaking?: string;
  /** Force the SVG version (tiny avatars, tests). */
  variant?: 'auto' | 'svg';
  className?: string;
}

const GLB_URL = '/ubi/Ubi.glb';
let probe: Promise<boolean> | null = null;

const webglAvailable = (): boolean => {
  try {
    const c = document.createElement('canvas');
    return !!(c.getContext('webgl2') || c.getContext('webgl'));
  } catch {
    return false;
  }
};

/** Checks once whether the 3D model exists and WebGL works; otherwise the SVG UBI is used. */
export function probeGlb(): Promise<boolean> {
  if (!probe) {
    probe = (async () => {
      try {
        if (typeof window === 'undefined' || !webglAvailable()) return false;
        const res = await fetch(GLB_URL, { method: 'HEAD' });
        if (!res.ok) return false;
        const ct = res.headers.get('content-type') ?? '';
        return !ct.includes('text/html');
      } catch {
        return false;
      }
    })();
  }
  return probe;
}

/** Test hook: reset the cached probe. */
export function __resetProbe(): void {
  probe = null;
}

class Boundary extends Component<{ fallback: ReactNode; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

export function Ubi({ mood, size = 160, speaking, variant = 'auto', className }: UbiProps) {
  const [use3d, setUse3d] = useState(false);

  useEffect(() => {
    if (variant === 'svg') return;
    let alive = true;
    void probeGlb().then((ok) => {
      if (alive) setUse3d(ok);
    });
    return () => {
      alive = false;
    };
  }, [variant]);

  const svg = <UbiSvg mood={mood} size={size} />;

  return (
    <div className={clsx('relative inline-flex flex-col items-center', className)} data-testid="ubi">
      <AnimatePresence>
        {speaking && (
          <motion.div
            key={speaking}
            role="status"
            initial={{ opacity: 0, y: 6, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 4, scale: 0.96 }}
            className="card relative mb-2 max-w-[260px] px-3 py-2 text-center text-xs leading-5 text-ink"
          >
            {speaking}
            <span className="absolute -bottom-1.5 left-1/2 size-3 -translate-x-1/2 rotate-45 border-r border-b border-line bg-surface" aria-hidden />
          </motion.div>
        )}
      </AnimatePresence>
      {use3d ? (
        <Boundary fallback={svg}>
          <Suspense fallback={svg}>
            <Ubi3d mood={mood} size={size} />
          </Suspense>
        </Boundary>
      ) : (
        svg
      )}
    </div>
  );
}
