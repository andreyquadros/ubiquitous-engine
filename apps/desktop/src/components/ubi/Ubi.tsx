import { Component, lazy, Suspense, useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import clsx from 'clsx';
import type { Mood } from '../../lib/types';
import { UbiSvg } from './UbiSvg';
import { probePng, UbiImage, __resetPngProbe } from './UbiImage';

// three and the loaders stay out of the initial bundle: the chunk is fetched only when the model exists.
const Ubi3d = lazy(() => import('./Ubi3d'));

export interface UbiProps {
  mood: Mood;
  size?: number;
  speaking?: string;
  /**
   * `auto` (default): 3D model → PNG → SVG. `flat`: PNG → SVG (no WebGL; rail/head avatar).
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

/** Checks once whether the 3D model exists and WebGL works; otherwise the PNG or the SVG UBI is used. */
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

class Boundary extends Component<{ fallback: ReactNode; onError: () => void; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch() {
    this.props.onError();
  }
  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

type Mode = 'svg' | 'png' | '3d';

/** Dev-only overrides (`window.__ubiqxUbi.set({ mood, speaking })`) so the rig can be driven from the browser. */
interface Override {
  mood?: Mood;
  speaking?: string | null;
}
const overrideListeners = new Set<(o: Override) => void>();
declare global {
  interface Window {
    __ubiqxUbi?: { set: (o: Override) => void; reset: () => void };
  }
}
if (import.meta.env.DEV && typeof window !== 'undefined') {
  window.__ubiqxUbi = {
    set: (o) => overrideListeners.forEach((l) => l(o)),
    reset: () => overrideListeners.forEach((l) => l({})),
  };
}

/** The mascot with an optional speech bubble. Picks the richest available presentation. */
export function Ubi({ mood: moodProp, size = 160, speaking: speakingProp, variant = 'auto', crop = 'full', className }: UbiProps) {
  const [mode, setMode] = useState<Mode>('svg');
  const [pngOk, setPngOk] = useState(false);
  const [override, setOverride] = useState<Override>({});
  const reduce = useReducedMotion();
  const bubble = useRef<HTMLDivElement>(null);
  // Callback ref for the bubble: with `key={speaking}` the exiting and the entering bubble would share one object ref,
  // and the old bubble's unmount (0.2 s later) would null it out from under the new one. Each bubble only clears itself.
  const setBubble = useCallback((el: HTMLDivElement | null) => {
    if (el) bubble.current = el;
    return () => {
      if (bubble.current === el) bubble.current = null;
    };
  }, []);
  const mood = override.mood ?? moodProp;
  const speaking = override.speaking === undefined ? speakingProp : (override.speaking ?? undefined);

  useEffect(() => {
    if (!import.meta.env.DEV || variant !== 'auto') return;
    overrideListeners.add(setOverride);
    return () => {
      overrideListeners.delete(setOverride);
    };
  }, [variant]);

  useEffect(() => {
    if (variant === 'svg') return;
    let alive = true;
    void (async () => {
      const [png, glb] = await Promise.all([probePng(), variant === 'auto' ? probeGlb() : Promise.resolve(false)]);
      if (!alive) return;
      setPngOk(png);
      setMode(glb ? '3d' : png ? 'png' : 'svg');
    })();
    return () => {
      alive = false;
    };
  }, [variant]);

  // The model exists but could not be loaded/rendered (bad export, WebGL context lost…): fall back to the PNG when it exists.
  const onGlbError = useCallback(() => setMode(pngOk ? 'png' : 'svg'), [pngOk]);

  const svg = <UbiSvg mood={mood} size={size} />;
  let art: ReactNode = svg;
  if (mode === 'png') art = <UbiImage mood={mood} size={size} crop={crop} />;
  else if (mode === '3d')
    art = (
      <Boundary fallback={pngOk ? <UbiImage mood={mood} size={size} crop={crop} /> : svg} onError={onGlbError}>
        <Suspense fallback={svg}>
          <Ubi3d mood={mood} size={size} fallback={svg} speaking={speaking} bubbleRef={bubble} />
        </Suspense>
      </Boundary>
    );

  return (
    <div className={clsx('relative inline-flex flex-col items-center', className)} data-testid="ubi" data-ubi-mode={mode}>
      <AnimatePresence>
        {speaking && (
          <motion.div
            key={speaking}
            ref={setBubble}
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
