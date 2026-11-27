import { motion } from 'framer-motion';
import type { Mood } from '../../lib/types';
import { MOOD_GLOW } from './moods';

/** Idle float period per mood (seconds) — shared by the PNG and the 3D UBI so both breathe at the same tempo. */
export const FLOAT_PERIOD: Record<Mood, number> = {
  sleeping: 5.6,
  calm: 4.2,
  focused: 4.2,
  excited: 3.2,
  worried: 3.6,
};

/**
 * The soft pool of mood-coloured light under the mascot. Absolutely positioned at the bottom centre of a
 * `relative` box of `size` px width; pulses with the float unless motion is reduced.
 */
export function FloorGlow({ mood, size, reduce }: { mood: Mood; size: number; reduce: boolean }) {
  const glow = MOOD_GLOW[mood];
  const period = FLOAT_PERIOD[mood];
  return (
    <motion.span
      className="pointer-events-none absolute bottom-0 left-1/2 -translate-x-1/2 rounded-[50%]"
      style={{ width: size * 0.62, height: size * 0.11, background: `radial-gradient(closest-side, ${glow}, transparent)`, filter: 'blur(6px)', opacity: 0.7 }}
      animate={reduce ? undefined : { scaleX: [1, 0.86, 1], opacity: [0.7, 0.45, 0.7] }}
      transition={{ duration: period, repeat: Infinity, ease: 'easeInOut' }}
      aria-hidden
      data-testid="ubi-floor-glow"
    />
  );
}
