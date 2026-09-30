import { lazy, Suspense, useEffect, useRef, useState } from 'react';
import { useCopy } from '../i18n';
import { cx } from './cx';
import { hasWebGL, prefersReducedMotion } from './webgl';

const UbiHero3d = lazy(() => import('./UbiHero3d'));
const HERO_PNG = `${import.meta.env.BASE_URL}ubi-hero.png`;
/** Seconds each line stays in the bubble. */
const BUBBLE_SECONDS = 6.5;
/** Delay before the first bubble, so the page settles first. */
const BUBBLE_START_MS = 2200;

/**
 * The hero mascot: PNG first (fast first paint), then the rigged 3D cross-fades in once loaded, with a speech
 * bubble it glances at. Reduced motion keeps the PNG and offers an explicit opt-in button, so the preference is
 * respected by default but a visitor who wants the animation can still have it. Without WebGL, or when the model
 * fails, the PNG stays. `capture` renders the 3D canvas alone, still, for the scripts that produce the PNG.
 */
export function UbiHero({ size = 440, capture = false, className }: { size?: number; capture?: boolean; className?: string }) {
  // `size` is the layout width in px; `className` may override it responsively (the box keeps a 1:1.2 ratio)
  const { copy } = useCopy();
  const [mode, setMode] = useState<'png' | '3d'>('png');
  const [ready, setReady] = useState(false);
  const [reduced, setReduced] = useState(false);
  const [optIn, setOptIn] = useState(false);
  const [line, setLine] = useState(-1);
  const bubble = useRef<HTMLDivElement>(null);
  const lines = copy.hero.bubbles;

  useEffect(() => {
    if (capture) {
      setMode('3d');
      return;
    }
    if (!hasWebGL()) return;
    if (prefersReducedMotion() && !optIn) {
      setReduced(true);
      return;
    }
    setReduced(false);
    // let the first paint and the fonts settle before three (its own chunk) is fetched
    const w = window as Window & { requestIdleCallback?: (cb: () => void, o?: { timeout: number }) => number };
    if (w.requestIdleCallback) w.requestIdleCallback(() => setMode('3d'), { timeout: 1500 });
    else setTimeout(() => setMode('3d'), 300);
  }, [capture, optIn]);

  // the bubble only runs with the 3D on screen: it is the mascot speaking, not a banner
  useEffect(() => {
    if (capture || mode !== '3d' || !ready || !lines.length) return;
    const first = setTimeout(() => setLine(0), BUBBLE_START_MS);
    const loop = setInterval(() => setLine((i) => (i + 1) % lines.length), BUBBLE_SECONDS * 1000);
    return () => {
      clearTimeout(first);
      clearInterval(loop);
    };
  }, [capture, mode, ready, lines.length]);

  const speech = line >= 0 ? lines[line] : undefined;
  const boxH = size * 1.2;
  return (
    <div
      className={cx('relative max-w-full select-none', className)}
      style={{ width: className ? undefined : size, aspectRatio: '1 / 1.2' }}
      role="img"
      aria-label={copy.hero.mascotAlt}
      data-testid="ubi-hero"
      data-mode={mode}
      data-ready={ready ? '1' : '0'}
    >
      {!capture && (
        <span
          className={cx('pointer-events-none absolute bottom-[2%] left-1/2 rounded-[50%]', !ready && 'ubi-glow')}
          style={{ width: '62%', height: '9%', transform: 'translateX(-50%)', background: 'radial-gradient(closest-side, #4d8dff, transparent)', filter: 'blur(8px)', opacity: 0.7 }}
          aria-hidden="true"
        />
      )}
      {!capture && (
        <img
          src={HERO_PNG}
          alt=""
          width={size}
          height={boxH}
          decoding="async"
          fetchPriority="high"
          className={cx(
            'absolute inset-0 h-full w-full object-contain object-bottom transition-opacity duration-300',
            ready ? 'opacity-0' : 'opacity-100',
            mode === 'png' && !reduced && 'ubi-float',
          )}
          aria-hidden="true"
        />
      )}
      {mode === '3d' && (
        <div className={cx('absolute inset-0 transition-opacity duration-300', ready ? 'opacity-100' : 'opacity-0')}>
          <Suspense fallback={null}>
            <UbiHero3d still={capture} speech={speech} bubbleRef={bubble} onReady={() => setReady(true)} onError={() => setMode('png')} />
          </Suspense>
        </div>
      )}

      {/* The speech bubble: the mascot turns its head towards it whenever the line changes. */}
      {!capture && (
        <div
          ref={bubble}
          aria-live="polite"
          className={cx(
            'pointer-events-none absolute -left-2 top-[2%] z-10 w-[190px] rounded-2xl border border-line-2 bg-panel-2/95 px-3 py-2 text-[12.5px] leading-snug text-ink-2 shadow-lg backdrop-blur transition-all duration-500 sm:-left-10 sm:w-[210px] sm:text-[13px]',
            speech ? 'translate-y-0 opacity-100' : 'translate-y-1 opacity-0',
          )}
        >
          {speech}
          <span className="absolute -bottom-1.5 right-6 h-3 w-3 rotate-45 border-b border-r border-line-2 bg-panel-2/95" aria-hidden="true" />
        </div>
      )}

      {/* Reduced motion: respected by default, with an explicit way in. */}
      {!capture && reduced && (
        <button
          type="button"
          onClick={() => setOptIn(true)}
          className="absolute bottom-0 left-1/2 z-10 -translate-x-1/2 rounded-pill border border-line-2 bg-panel-2/90 px-3 py-1.5 text-xs font-semibold text-ink-2 backdrop-blur transition-colors hover:text-ink"
        >
          {copy.hero.enableMotion}
        </button>
      )}
    </div>
  );
}
