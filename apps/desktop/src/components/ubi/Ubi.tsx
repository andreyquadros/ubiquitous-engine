import { Component, lazy, Suspense, useEffect, useState, type ReactNode } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import clsx from 'clsx';
import type { Mood } from '../../lib/types';
import { UbiSvg } from './UbiSvg';
import { probePng, UbiImage, __resetPngProbe } from './UbiImage';

const Ubi3d = lazy(() => import('./Ubi3d'));

export interface UbiProps {
  mood: Mood;
  size?: number;
  speaking?: string;
  /**
   * `auto` (default): PNG → 3D model → SVG. `flat`: PNG → SVG (no WebGL; rail avatar).
   * `svg`: always the inline drawing (tests, tiny sizes).
   */
  variant?: 'auto' | 'svg' | 'flat';
  /** `head` = circular head crop (PNG only; the SVG is simply drawn small). */
  crop?: 'full' | 'head';
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

/** Test hook: reset the cached probes (GLB and PNG). */
export function __resetProbe(): void {
  probe = null;
  __resetPngProbe();
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

type Mode = 'svg' | 'png' | '3d';

/** The mascot with an optional speech bubble. Picks the richest available presentation. */
export function Ubi({ mood, size = 160, speaking, variant = 'auto', crop = 'full', className }: UbiProps) {
  const [mode, setMode] = useState<Mode>('svg');
  const reduce = useReducedMotion();

  useEffect(() => {
    if (variant === 'svg') return;
    let alive = true;
    void (async () => {
      if (await probePng()) return alive && setMode('png');
      if (variant === 'auto' && (await probeGlb())) return alive && setMode('3d');
      return undefined;
    })();
    return () => {
      alive = false;
    };
  }, [variant]);

  const svg = <UbiSvg mood={mood} size={size} />;
  let art: ReactNode = svg;
  if (mode === 'png') art = <UbiImage mood={mood} size={size} crop={crop} />;
  else if (mode === '3d')
    art = (
      <Boundary fallback={svg}>
        <Suspense fallback={svg}>
          <Ubi3d mood={mood} size={size} />
        </Suspense>
      </Boundary>
    );

  return (
    <div className={clsx('relative inline-flex flex-col items-center', className)} data-testid="ubi" data-ubi-mode={mode}>
      <AnimatePresence>
        {speaking && (
          <motion.div
            key={speaking}
            role="status"
            initial={reduce ? { opacity: 0 } : { opacity: 0, y: 6, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={reduce ? { opacity: 0 } : { opacity: 0, y: 4, scale: 0.96 }}
            transition={{ duration: reduce ? 0 : 0.2, ease: [0.16, 1, 0.3, 1] }}
            className="glass relative z-10 mb-2 max-w-[260px] px-3 py-2 text-center text-xs leading-5 text-ink"
          >
            {speaking}
            <span className="absolute -bottom-1.5 left-1/2 size-3 -translate-x-1/2 rotate-45 border-r border-b border-line-2 bg-[color-mix(in_oklab,var(--panel)_86%,transparent)]" aria-hidden />
          </motion.div>
        )}
      </AnimatePresence>
      {art}
    </div>
  );
}
