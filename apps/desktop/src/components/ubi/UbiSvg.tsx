import { motion, useReducedMotion } from 'framer-motion';
import { useId } from 'react';
import type { Mood } from '../../lib/types';
import { MOOD_GLOW } from './moods';

const BLUE = '#2563eb';
const EYE = '#60a5fa';
const ORANGE = '#f97316';
const SHELL = '#ffffff';
const SHELL_LINE = '#cbd5e1';

function Eyes({ mood, glow }: { mood: Mood; glow: string }) {
  const common = { fill: 'none', stroke: EYE, strokeLinecap: 'round' as const, strokeWidth: 5, filter: `url(#${glow})` };
  switch (mood) {
    case 'sleeping':
      return (
        <g>
          <path d="M70 92 Q82 98 94 92" {...common} strokeWidth={4} />
          <path d="M106 92 Q118 98 130 92" {...common} strokeWidth={4} />
        </g>
      );
    case 'focused':
      return (
        <g>
          <path d="M70 91 Q82 83 94 89" {...common} />
          <path d="M106 89 Q118 83 130 91" {...common} />
        </g>
      );
    case 'excited':
      return (
        <g filter={`url(#${glow})`}>
          <circle cx={82} cy={90} r={8.5} fill={EYE} />
          <circle cx={118} cy={90} r={8.5} fill={EYE} />
          <circle cx={85} cy={87} r={2.6} fill="#e0f2fe" />
          <circle cx={121} cy={87} r={2.6} fill="#e0f2fe" />
        </g>
      );
    case 'worried':
      return (
        <g>
          <path d="M70 96 Q82 82 94 90" {...common} />
          <path d="M106 90 Q118 82 130 96" {...common} />
        </g>
      );
    default:
      return (
        <g>
          <path d="M70 93 Q82 77 94 93" {...common} />
          <path d="M106 93 Q118 77 130 93" {...common} />
        </g>
      );
  }
}

function Ring({ x, y, w }: { x: number; y: number; w: number }) {
  return <rect x={x} y={y} width={w} height={4} rx={2} fill={BLUE} opacity={0.95} />;
}

/** Inline SVG version of UBI — used when the 3D model is missing or WebGL is unavailable. */
export function UbiSvg({ mood, size = 160, className }: { mood: Mood; size?: number; className?: string }) {
  const id = useId().replace(/:/g, '');
  const glow = `glow-${id}`;
  const visor = `visor-${id}`;
  const soft = `soft-${id}`;
  const color = MOOD_GLOW[mood];
  const reduce = useReducedMotion();
  const loop = reduce ? { duration: 0 } : { duration: 3.2, repeat: Infinity, ease: 'easeInOut' as const };

  return (
    <svg
      data-testid="ubi-svg"
      data-mood={mood}
      role="img"
      aria-label={`UBI, o mascote (${mood})`}
      viewBox="0 0 200 240"
      width={size}
      height={size * 1.2}
      className={className}
      style={{ overflow: 'visible' }}
    >
      <defs>
        <filter id={glow} x="-40%" y="-40%" width="180%" height="180%">
          <feGaussianBlur stdDeviation="2.2" result="b" />
          <feMerge>
            <feMergeNode in="b" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <filter id={soft} x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="6" />
        </filter>
        <linearGradient id={visor} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#1e293b" />
          <stop offset="1" stopColor="#020617" />
        </linearGradient>
      </defs>

      {/* ground glow */}
      <motion.ellipse cx={100} cy={228} rx={50} ry={8} fill={color} opacity={0.35} filter={`url(#${soft})`} animate={reduce ? undefined : { rx: [50, 44, 50], opacity: [0.35, 0.25, 0.35] }} transition={loop} />

      <motion.g animate={reduce ? undefined : { y: [0, -4, 0] }} transition={loop}>
        {/* flowing ribbon (behind body) */}
        <motion.path
          d="M132 184 C150 178 162 166 176 150 C170 162 170 172 178 186 C164 182 150 186 132 190 Z"
          fill={ORANGE}
          style={{ transformBox: 'fill-box', transformOrigin: '0% 50%' }}
          animate={reduce ? undefined : { rotate: [0, 5, -2, 0], scaleY: [1, 0.92, 1.04, 1] }}
          transition={{ ...loop, duration: 2.6 }}
        />

        {/* legs */}
        <rect x={76} y={188} width={18} height={36} rx={8} fill={SHELL} stroke={SHELL_LINE} />
        <rect x={106} y={188} width={18} height={36} rx={8} fill={SHELL} stroke={SHELL_LINE} />
        <Ring x={77} y={203} w={16} />
        <Ring x={107} y={203} w={16} />
        <rect x={72} y={216} width={26} height={10} rx={5} fill={SHELL} stroke={SHELL_LINE} />
        <rect x={102} y={216} width={26} height={10} rx={5} fill={SHELL} stroke={SHELL_LINE} />

        {/* arms */}
        <rect x={42} y={138} width={16} height={46} rx={8} fill={SHELL} stroke={SHELL_LINE} />
        <rect x={142} y={138} width={16} height={46} rx={8} fill={SHELL} stroke={SHELL_LINE} />
        <Ring x={43} y={147} w={14} />
        <Ring x={143} y={147} w={14} />
        <Ring x={43} y={166} w={14} />
        <Ring x={143} y={166} w={14} />

        {/* body */}
        <motion.g style={{ transformBox: 'fill-box', transformOrigin: '50% 100%' }} animate={reduce ? undefined : { scaleY: [1, 1.02, 1] }} transition={loop}>
          <rect x={60} y={126} width={80} height={70} rx={26} fill={SHELL} stroke={SHELL_LINE} />
          {/* sash */}
          <path d="M64 142 C84 150 108 168 128 194 L110 194 C96 176 78 162 62 156 Z" fill={BLUE} />
          <path d="M66 144 C84 152 104 168 118 186" fill="none" stroke="#93c5fd" strokeWidth={2} opacity={0.6} strokeLinecap="round" />
          {/* belt */}
          <rect x={60} y={178} width={80} height={12} rx={3} fill={ORANGE} />
          <rect x={92} y={176} width={16} height={16} rx={4} fill="#fdba74" stroke="#c2410c" />
        </motion.g>

        {/* head */}
        <g>
          {/* crest spikes with blue-lit cores */}
          <g filter={`url(#${glow})`}>
            <polygon points="76,44 87,16 98,44" fill={SHELL} stroke={SHELL_LINE} />
            <polygon points="93,42 104,6 115,42" fill={SHELL} stroke={SHELL_LINE} />
            <polygon points="110,44 121,16 132,44" fill={SHELL} stroke={SHELL_LINE} />
            <path d="M87 40 L87 22" stroke={EYE} strokeWidth={3} strokeLinecap="round" />
            <path d="M104 38 L104 14" stroke={EYE} strokeWidth={3} strokeLinecap="round" />
            <path d="M121 40 L121 22" stroke={EYE} strokeWidth={3} strokeLinecap="round" />
          </g>
          <rect x={40} y={38} width={120} height={98} rx={36} fill={SHELL} stroke={SHELL_LINE} />
          {/* side pods */}
          <rect x={30} y={72} width={12} height={30} rx={6} fill={SHELL} stroke={SHELL_LINE} />
          <rect x={158} y={72} width={12} height={30} rx={6} fill={SHELL} stroke={SHELL_LINE} />
          <Ring x={31} y={85} w={10} />
          <Ring x={159} y={85} w={10} />
          {/* visor */}
          <rect x={54} y={56} width={92} height={64} rx={28} fill={`url(#${visor})`} stroke="#0f172a" />
          <ellipse cx={82} cy={68} rx={20} ry={6} fill="#ffffff" opacity={0.10} />
          <Eyes mood={mood} glow={glow} />
        </g>

        {/* mood extras */}
        {mood === 'sleeping' && (
          <motion.text x={150} y={40} fill="#94a3b8" fontSize={16} fontWeight={700} fontFamily="Inter, system-ui" animate={reduce ? undefined : { y: [40, 24], opacity: [0, 1, 0], x: [150, 162] }} transition={{ duration: 2.4, repeat: Infinity, ease: 'easeOut' }}>
            zzz
          </motion.text>
        )}
        {mood === 'excited' && (
          <g fill="#fbbf24">
            <motion.path d="M28 40 l3 7 7 3 -7 3 -3 7 -3 -7 -7 -3 7 -3z" animate={reduce ? undefined : { scale: [0.8, 1.2, 0.8], opacity: [0.6, 1, 0.6] }} style={{ transformBox: 'fill-box', transformOrigin: 'center' }} transition={{ duration: 1.4, repeat: Infinity }} />
            <motion.path d="M172 26 l2.5 6 6 2.5 -6 2.5 -2.5 6 -2.5 -6 -6 -2.5 6 -2.5z" animate={reduce ? undefined : { scale: [1.2, 0.8, 1.2], opacity: [1, 0.6, 1] }} style={{ transformBox: 'fill-box', transformOrigin: 'center' }} transition={{ duration: 1.4, repeat: Infinity }} />
            <motion.path d="M166 120 l2 5 5 2 -5 2 -2 5 -2 -5 -5 -2 5 -2z" animate={reduce ? undefined : { scale: [0.9, 1.3, 0.9] }} style={{ transformBox: 'fill-box', transformOrigin: 'center' }} transition={{ duration: 1.8, repeat: Infinity }} />
          </g>
        )}
        {mood === 'worried' && (
          <motion.path d="M160 52 C160 52 152 64 152 70 A8 8 0 0 0 168 70 C168 64 160 52 160 52Z" fill="#7dd3fc" stroke="#38bdf8" animate={reduce ? undefined : { y: [0, 6, 0], opacity: [0.9, 1, 0.9] }} transition={{ duration: 1.6, repeat: Infinity }} />
        )}
      </motion.g>
    </svg>
  );
}
