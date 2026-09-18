import { lazy, Suspense, useEffect, useState } from 'react';
import { useCopy } from '../i18n';
import { cx } from './cx';
import { hasWebGL, prefersReducedMotion } from './webgl';

const UbiHero3d = lazy(() => import('./UbiHero3d'));
const HERO_PNG = `${import.meta.env.BASE_URL}ubi-hero.png`;

/**
 * The hero mascot: PNG first (fast first paint), then the 3D model cross-fades in once loaded. Reduced motion
 * keeps the PNG (a still render of the same model); without WebGL, or when the model fails, the PNG stays.
 * `capture` renders the 3D canvas alone, still, for the scripts that produce the PNG.
 */
export function UbiHero({ size = 440, capture = false, className }: { size?: number; capture?: boolean; className?: string }) {
  // `size` is the layout width in px; `className` may override it responsively (the box keeps a 1:1.2 ratio)
  const { copy } = useCopy();
  const [mode, setMode] = useState<'png' | '3d'>('png');
  const [ready, setReady] = useState(false);

  useEffect(() => {
    if (capture) {
      setMode('3d');
      return;
    }
    if (prefersReducedMotion() || !hasWebGL()) return;
    // let the first paint and the fonts settle before three (its own chunk) is fetched
    const w = window as Window & { requestIdleCallback?: (cb: () => void, o?: { timeout: number }) => number };
    if (w.requestIdleCallback) w.requestIdleCallback(() => setMode('3d'), { timeout: 1500 });
    else setTimeout(() => setMode('3d'), 300);
  }, [capture]);

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
          className={cx('absolute inset-0 h-full w-full object-contain object-bottom transition-opacity duration-300', ready ? 'opacity-0' : 'opacity-100', mode === 'png' && 'ubi-float')}
          aria-hidden="true"
        />
      )}
      {mode === '3d' && (
        <div className={cx('absolute inset-0 transition-opacity duration-300', ready ? 'opacity-100' : 'opacity-0')}>
          <Suspense fallback={null}>
            <UbiHero3d still={capture} onReady={() => setReady(true)} onError={() => setMode('png')} />
          </Suspense>
        </div>
      )}
    </div>
  );
}
