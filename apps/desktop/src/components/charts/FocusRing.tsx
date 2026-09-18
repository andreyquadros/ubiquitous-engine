import { animate, motion, useMotionValue, useReducedMotion, useTransform } from 'framer-motion';
import { useEffect, useId } from 'react';
import { focusColor, MOOD_LABEL } from '../../lib/format';
import type { Mood } from '../../lib/types';

interface Props {
  score: number;
  mood: Mood;
  /** Outer diameter in px. */
  size?: number;
  stroke?: number;
  /** Text under the number. */
  label?: string;
  /** Draw-in animation on mount (the Hoje load moment). Off under reduced motion. */
  animate?: boolean;
  className?: string;
}

const EASE = [0.16, 1, 0.3, 1] as const;

/**
 * The Focus Dial — the one glowing instrument. A conic arc with a blurred copy underneath as glow,
 * the score in Sora inside, the mood beneath. Draw-in ~900 ms, number counts up alongside.
 */
export function FocusDial({ score, mood, size = 176, stroke = 12, label = 'de foco', animate: shouldAnimate = true, className }: Props) {
  const id = useId().replace(/:/g, '');
  const reduce = useReducedMotion();
  const live = shouldAnimate && !reduce;
  const r = (size - stroke) / 2 - 6;
  const c = 2 * Math.PI * r;
  const pct = Math.max(0, Math.min(100, score)) / 100;
  const color = focusColor(score);

  const progress = useMotionValue(live ? 0 : pct);
  const offset = useTransform(progress, (p) => c * (1 - p));
  const count = useTransform(progress, (p) => Math.round((p / Math.max(pct, 0.0001)) * score));

  useEffect(() => {
    if (!live) {
      progress.set(pct);
      return;
    }
    const controls = animate(progress, pct, { duration: 0.9, ease: EASE, delay: 0.1 });
    return () => controls.stop();
  }, [live, pct, progress]);

  return (
    <div className={className} style={{ width: size, height: size, position: 'relative' }} role="img" aria-label={`Score de foco ${score} de 100, ${MOOD_LABEL[mood]}`}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="-rotate-90" style={{ overflow: 'visible' }}>
        <defs>
          <filter id={`dial-glow-${id}`} x="-30%" y="-30%" width="160%" height="160%">
            <feGaussianBlur stdDeviation={stroke * 0.9} />
          </filter>
        </defs>
        {/* track */}
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--panel-3)" strokeWidth={stroke} />
        {/* tick marks every 10 */}
        {Array.from({ length: 10 }, (_, i) => {
          const a = (i / 10) * Math.PI * 2;
          const r1 = r + stroke / 2 + 3;
          const r2 = r1 + 3;
          return <line key={i} x1={size / 2 + r1 * Math.cos(a)} y1={size / 2 + r1 * Math.sin(a)} x2={size / 2 + r2 * Math.cos(a)} y2={size / 2 + r2 * Math.sin(a)} stroke="var(--ink-4)" strokeWidth={1} opacity={0.6} />;
        })}
        {/* glow copy */}
        <motion.circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke={color} strokeWidth={stroke} strokeLinecap="round" strokeDasharray={c} style={{ strokeDashoffset: offset }} filter={`url(#dial-glow-${id})`} opacity={0.55} />
        {/* arc */}
        <motion.circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke={color} strokeWidth={stroke} strokeLinecap="round" strokeDasharray={c} style={{ strokeDashoffset: offset }} />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <motion.span className="display num leading-none" style={{ fontSize: Math.round(size * 0.32), letterSpacing: '-0.03em', color }}>
          {count}
        </motion.span>
        <span className="mt-1 text-xs font-medium text-ink-2">{label}</span>
        <span className="mt-0.5 text-[11px] text-ink-3">{MOOD_LABEL[mood]}</span>
      </div>
    </div>
  );
}

/** Backwards-compatible name. */
export const FocusRing = FocusDial;
