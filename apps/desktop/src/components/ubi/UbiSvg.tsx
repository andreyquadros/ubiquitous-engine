import { motion, useReducedMotion } from 'framer-motion';
import { useId } from 'react';
import type { Mood } from '../../lib/types';
import { MOOD_GLOW } from './moods';
import { useT } from '../../i18n';

const VOLT = '#4d8dff';
const EYE = '#7fb4ff';
const SASH = '#1e3a8a';
const SASH_EDGE = '#3b63d6';
const EMBER = '#ff7a1f';
const EMBER_DEEP = '#d95f0e';
const LINE = '#b9c6dc';
const VISOR_EDGE = '#0b1224';

function Eyes({ mood, glow }: { mood: Mood; glow: string }) {
  const common = { fill: 'none', stroke: EYE, strokeLinecap: 'round' as const, strokeWidth: 6, filter: `url(#${glow})` };
  switch (mood) {
    case 'sleeping':
      return (
        <g>
          <path d="M64 94 Q78 101 92 94" {...common} strokeWidth={4.5} />
          <path d="M108 94 Q122 101 136 94" {...common} strokeWidth={4.5} />
        </g>
      );
    case 'focused':
      return (
        <g>
          <path d="M64 92 Q78 82 92 90" {...common} />
          <path d="M108 90 Q122 82 136 92" {...common} />
        </g>
      );
    case 'excited':
      return (
        <g filter={`url(#${glow})`}>
          <path d="M64 96 Q78 72 92 96" {...common} strokeWidth={7} filter={undefined} />
          <path d="M108 96 Q122 72 136 96" {...common} strokeWidth={7} filter={undefined} />
          <circle cx={78} cy={84} r={3} fill="#eaf4ff" />
          <circle cx={122} cy={84} r={3} fill="#eaf4ff" />
        </g>
      );
    case 'worried':
      return (
        <g>
          <path d="M64 98 Q78 82 92 92" {...common} />
          <path d="M108 92 Q122 82 136 98" {...common} />
        </g>
      );
    default:
      return (
        <g>
          <path d="M64 96 Q78 76 92 96" {...common} />
          <path d="M108 96 Q122 76 136 96" {...common} />
        </g>
      );
  }
}

/** Blue ring joint (ears, elbows, knees). */
function Joint({ cx, cy, r, shell, glow }: { cx: number; cy: number; r: number; shell: string; glow: string }) {
  return (
    <g>
      <circle cx={cx} cy={cy} r={r} fill={`url(#${shell})`} stroke={LINE} strokeWidth={1} />
      <circle cx={cx} cy={cy} r={r * 0.55} fill="none" stroke={VOLT} strokeWidth={r * 0.28} filter={`url(#${glow})`} />
    </g>
  );
}

/** Inline SVG UBI — the fallback when neither the PNG nor the 3D model is installed. 200×240 viewBox. */
export function UbiSvg({ mood, size = 160, className }: { mood: Mood; size?: number; className?: string }) {
  const id = useId().replace(/:/g, '');
  const t = useT();
  const glow = `glow-${id}`;
  const soft = `soft-${id}`;
  const shell = `shell-${id}`;
  const visor = `visor-${id}`;
  const ribbon = `ribbon-${id}`;
  const holo = `holo-${id}`;
  const color = MOOD_GLOW[mood];
  const reduce = useReducedMotion();
  const speed = mood === 'sleeping' ? 5.6 : mood === 'excited' ? 3.2 : 4.2;
  const loop = reduce ? { duration: 0 } : { duration: speed, repeat: Infinity, ease: 'easeInOut' as const };

  return (
    <svg
      data-testid="ubi-svg"
      data-mood={mood}
      role="img"
      aria-label={t('ubi.mascot', { mood: t(`common.mood.${mood}`) })}
      viewBox="0 0 200 240"
      width={size}
      height={size * 1.2}
      className={className}
      style={{ overflow: 'visible' }}
    >
      <defs>
        <filter id={glow} x="-40%" y="-40%" width="180%" height="180%">
          <feGaussianBlur stdDeviation="2.4" result="b" />
          <feMerge>
            <feMergeNode in="b" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
        <filter id={soft} x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="7" />
        </filter>
        <linearGradient id={shell} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ffffff" />
          <stop offset="0.55" stopColor="#f3f6fb" />
          <stop offset="1" stopColor="#d9e1ee" />
        </linearGradient>
        <linearGradient id={visor} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#233252" />
          <stop offset="0.5" stopColor="#0d1526" />
          <stop offset="1" stopColor="#04060d" />
        </linearGradient>
        <linearGradient id={ribbon} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor={EMBER} />
          <stop offset="1" stopColor="#ffa25c" />
        </linearGradient>
        <linearGradient id={holo} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor={VOLT} stopOpacity="0.35" />
          <stop offset="1" stopColor={VOLT} stopOpacity="0.08" />
        </linearGradient>
      </defs>

      {/* ground glow */}
      {/* Floor shadow: scaled through a group (animating the `rx` attribute makes browsers log "Expected length"). */}
      <motion.g style={{ transformOrigin: '100px 230px' }} animate={reduce ? undefined : { scaleX: [1, 0.85, 1], opacity: [0.4, 0.26, 0.4] }} transition={loop} opacity={0.4}>
        <ellipse cx={100} cy={230} rx={54} ry={8} fill={color} filter={`url(#${soft})`} />
      </motion.g>

      <motion.g animate={reduce ? undefined : { y: [0, -6, 0] }} transition={loop}>
        {/* flowing ribbon (behind body) */}
        <motion.path
          d="M134 196 C150 194 164 186 180 176 C174 188 176 198 186 210 C170 204 154 206 134 206 Z"
          fill={`url(#${ribbon})`}
          style={{ transformBox: 'fill-box', transformOrigin: '0% 50%' }}
          animate={reduce ? undefined : { rotate: [0, 6, -3, 0], scaleY: [1, 0.9, 1.05, 1] }}
          transition={{ ...loop, duration: speed * 0.65 }}
        />

        {/* legs + feet */}
        <rect x={74} y={196} width={20} height={30} rx={9} fill={`url(#${shell})`} stroke={LINE} />
        <rect x={106} y={196} width={20} height={30} rx={9} fill={`url(#${shell})`} stroke={LINE} />
        <Joint cx={84} cy={210} r={6} shell={shell} glow={glow} />
        <Joint cx={116} cy={210} r={6} shell={shell} glow={glow} />
        <rect x={66} y={222} width={32} height={11} rx={5.5} fill={`url(#${shell})`} stroke={LINE} />
        <rect x={102} y={222} width={32} height={11} rx={5.5} fill={`url(#${shell})`} stroke={LINE} />

        {/* left arm */}
        <rect x={34} y={146} width={18} height={50} rx={9} fill={`url(#${shell})`} stroke={LINE} />
        <Joint cx={43} cy={170} r={6.5} shell={shell} glow={glow} />

        {/* torso */}
        <motion.g style={{ transformBox: 'fill-box', transformOrigin: '50% 100%' }} animate={reduce ? undefined : { scaleY: [1, 1.015, 1] }} transition={loop}>
          <rect x={90} y={130} width={20} height={14} rx={4} fill="#0d1526" />
          <rect x={54} y={138} width={92} height={64} rx={26} fill={`url(#${shell})`} stroke={LINE} />
          {/* chest plate line */}
          <path d="M70 150 Q100 142 130 150" fill="none" stroke={LINE} strokeWidth={1} opacity={0.7} />
          {/* sash */}
          <path d="M58 150 C80 158 106 176 126 202 L108 202 C94 184 76 170 56 164 Z" fill={SASH} />
          <path d="M60 152 C80 160 104 176 118 194" fill="none" stroke={SASH_EDGE} strokeWidth={2} opacity={0.8} strokeLinecap="round" />
          {/* belt */}
          <rect x={54} y={186} width={92} height={13} rx={4} fill={EMBER} />
          <rect x={54} y={196} width={92} height={3} fill={EMBER_DEEP} opacity={0.7} />
          <rect x={91} y={183} width={18} height={19} rx={5} fill="#ffb476" stroke={EMBER_DEEP} strokeWidth={1.5} />
          <rect x={97} y={189} width={6} height={7} rx={1.5} fill={EMBER_DEEP} />
        </motion.g>

        {/* right arm + holographic panel */}
        <rect x={148} y={146} width={18} height={44} rx={9} fill={`url(#${shell})`} stroke={LINE} />
        <Joint cx={157} cy={166} r={6.5} shell={shell} glow={glow} />
        <circle cx={157} cy={190} r={8} fill={`url(#${shell})`} stroke={LINE} />
        <motion.g animate={reduce ? undefined : { y: [0, -2, 0], opacity: [0.9, 1, 0.9] }} transition={{ ...loop, duration: speed * 0.8 }}>
          <g transform="rotate(-8 176 166)" filter={`url(#${glow})`}>
            <rect x={154} y={148} width={46} height={34} rx={5} fill={`url(#${holo})`} stroke={VOLT} strokeWidth={1.2} opacity={0.95} />
            <path d="M160 158 H184 M160 164 H178 M160 170 H188" stroke={EYE} strokeWidth={1.6} strokeLinecap="round" opacity={0.9} />
            <rect x={186} y={154} width={4} height={12} rx={1} fill={EYE} opacity={0.8} />
            <rect x={192} y={158} width={4} height={8} rx={1} fill={EYE} opacity={0.6} />
          </g>
        </motion.g>

        {/* head */}
        <g>
          {/* crest: three spikes with blue light lines */}
          <g filter={`url(#${glow})`}>
            <path d="M70 42 L78 12 L88 40 Z" fill={`url(#${shell})`} stroke={LINE} strokeLinejoin="round" />
            <path d="M112 40 L122 12 L130 42 Z" fill={`url(#${shell})`} stroke={LINE} strokeLinejoin="round" />
            <path d="M90 40 L100 2 L110 40 Z" fill={`url(#${shell})`} stroke={LINE} strokeLinejoin="round" />
            <path d="M79 36 L79 20" stroke={EYE} strokeWidth={2.5} strokeLinecap="round" />
            <path d="M100 34 L100 12" stroke={EYE} strokeWidth={2.5} strokeLinecap="round" />
            <path d="M121 36 L121 20" stroke={EYE} strokeWidth={2.5} strokeLinecap="round" />
          </g>
          {/* ear rings */}
          <Joint cx={36} cy={88} r={11} shell={shell} glow={glow} />
          <Joint cx={164} cy={88} r={11} shell={shell} glow={glow} />
          {/* shell */}
          <rect x={38} y={34} width={124} height={100} rx={40} fill={`url(#${shell})`} stroke={LINE} />
          {/* rim light */}
          <path d="M48 110 C40 90 42 60 60 44" fill="none" stroke="#dbeaff" strokeWidth={2} strokeLinecap="round" opacity={0.9} />
          {/* visor */}
          <rect x={50} y={52} width={100} height={66} rx={30} fill={`url(#${visor})`} stroke={VISOR_EDGE} strokeWidth={1.5} />
          <path d="M60 112 Q100 122 140 112" fill="none" stroke={VOLT} strokeWidth={1.5} opacity={0.35} strokeLinecap="round" />
          <ellipse cx={82} cy={64} rx={24} ry={6} fill="#ffffff" opacity={0.09} />
          <Eyes mood={mood} glow={glow} />
        </g>

        {/* mood extras */}
        {mood === 'sleeping' && (
          <motion.text x={150} y={36} fill="#9daec7" fontSize={17} fontWeight={700} fontFamily="Sora Variable, Inter Variable, system-ui" animate={reduce ? undefined : { y: [36, 18], opacity: [0, 1, 0], x: [150, 164] }} transition={{ duration: 2.6, repeat: Infinity, ease: 'easeOut' }}>
            zzz
          </motion.text>
        )}
        {mood === 'excited' && (
          <g fill="#ffc24d">
            <motion.path d="M24 36 l3 7 7 3 -7 3 -3 7 -3 -7 -7 -3 7 -3z" animate={reduce ? undefined : { scale: [0.8, 1.2, 0.8], opacity: [0.6, 1, 0.6] }} style={{ transformBox: 'fill-box', transformOrigin: 'center' }} transition={{ duration: 1.4, repeat: Infinity }} />
            <motion.path d="M176 22 l2.5 6 6 2.5 -6 2.5 -2.5 6 -2.5 -6 -6 -2.5 6 -2.5z" animate={reduce ? undefined : { scale: [1.2, 0.8, 1.2], opacity: [1, 0.6, 1] }} style={{ transformBox: 'fill-box', transformOrigin: 'center' }} transition={{ duration: 1.4, repeat: Infinity }} />
            <motion.path d="M22 118 l2 5 5 2 -5 2 -2 5 -2 -5 -5 -2 5 -2z" animate={reduce ? undefined : { scale: [0.9, 1.3, 0.9] }} style={{ transformBox: 'fill-box', transformOrigin: 'center' }} transition={{ duration: 1.8, repeat: Infinity }} />
          </g>
        )}
        {mood === 'worried' && (
          <motion.path d="M170 48 C170 48 162 60 162 66 A8 8 0 0 0 178 66 C178 60 170 48 170 48Z" fill="#7dd3fc" stroke="#38bdf8" animate={reduce ? undefined : { y: [0, 6, 0], opacity: [0.9, 1, 0.9] }} transition={{ duration: 1.6, repeat: Infinity }} />
        )}
      </motion.g>
    </svg>
  );
}
