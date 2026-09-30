import { Component, lazy, Suspense, useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'framer-motion';
import clsx from 'clsx';
import type { Mood } from '../../lib/types';
import { UbiSvg } from './UbiSvg';
import { probePng, UbiImage, __resetPngProbe } from './UbiImage';
import { setUbiStatus, type UbiMode } from './status';

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

/**
 * Whether WebGL is there at all. This is the only thing worth asking before trying the model: it
 * ships inside the app, so its presence is not in question, and every other way it can fail (a bad
 * export, a blocked decoder, a lost context) shows up as a load error that `onGlbError` catches.
 *
 * An earlier version also asked `fetch('/ubi/Ubi.glb', { method: 'HEAD' })` first. That probe was
 * itself a failure mode: over the desktop app's custom protocol a HEAD is not the plain request it
 * is over http, and when it answered wrong the mascot fell back to the flat drawing with the model
 * sitting right there, unused.
 */
export function webglAvailable(): boolean {
  try {
    const c = document.createElement('canvas');
    return !!(c.getContext('webgl2') || c.getContext('webgl'));
  } catch {
    return false;
  }
}

/** Test hook: reset the cached PNG probe. */
export function __resetProbe(): void {
  __resetPngProbe();
}

class Boundary extends Component<{ fallback: ReactNode; onError: (e: unknown) => void; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch(error: unknown) {
    this.props.onError(error);
  }
  render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

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
  const [mode, setMode] = useState<UbiMode>('svg');
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
  // Only the `auto` mascot speaks for the app's status: `flat` and `svg` are deliberate choices,
  // not failures, and the rail avatar must not report itself as a degraded 3D model.
  const report = useCallback(
    (m: UbiMode, reason: string | null) => {
      if (variant === 'auto') setUbiStatus({ mode: m, reason });
    },
    [variant],
  );
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
      const png = await probePng();
      if (!alive) return;
      setPngOk(png);
      if (variant === 'auto' && webglAvailable()) return setMode('3d');
      setMode(png ? 'png' : 'svg');
      if (variant === 'auto') report(png ? 'png' : 'svg', 'WebGL unavailable');
    })();
    return () => {
      alive = false;
    };
  }, [variant, report]);

  const onGlbReady = useCallback(() => report('3d', null), [report]);

  // The model could not be loaded or rendered (a bad export, a blocked decoder, a lost WebGL context):
  // fall back to the PNG when it exists, and keep the reason for Settings → Sobre.
  const onGlbError = useCallback(
    (e: unknown) => {
      const next = pngOk ? 'png' : 'svg';
      setMode(next);
      report(next, e instanceof Error ? e.message : String(e));
    },
    [pngOk, report],
  );

  const svg = <UbiSvg mood={mood} size={size} />;
  let art: ReactNode = svg;
  if (mode === 'png') art = <UbiImage mood={mood} size={size} crop={crop} />;
  else if (mode === '3d')
    art = (
      <Boundary fallback={pngOk ? <UbiImage mood={mood} size={size} crop={crop} /> : svg} onError={onGlbError}>
        <Suspense fallback={svg}>
          <Ubi3d mood={mood} size={size} fallback={svg} speaking={speaking} bubbleRef={bubble} onReady={onGlbReady} />
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
