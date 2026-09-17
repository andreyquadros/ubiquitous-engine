import { motion } from 'framer-motion';
import { focusColor, MOOD_LABEL } from '../../lib/format';
import type { Mood } from '../../lib/types';

export function FocusRing({ score, mood, size = 132 }: { score: number; mood: Mood; size?: number }) {
  const stroke = 10;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const pct = Math.max(0, Math.min(100, score)) / 100;
  const color = focusColor(score);
  return (
    <div className="relative" style={{ width: size, height: size }} role="img" aria-label={`Score de foco ${score} de 100, ${MOOD_LABEL[mood]}`}>
      <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="-rotate-90">
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="var(--surface-3)" strokeWidth={stroke} />
        <motion.circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={color}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          initial={{ strokeDashoffset: c }}
          animate={{ strokeDashoffset: c * (1 - pct) }}
          transition={{ duration: 0.9, ease: 'easeOut' }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <span className="text-3xl font-semibold tracking-tight tabular-nums" style={{ color }}>
          {score}
        </span>
        <span className="text-[11px] font-medium text-ink-3">{MOOD_LABEL[mood]}</span>
      </div>
    </div>
  );
}
