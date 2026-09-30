/**
 * ubiqX AI launch video — design tokens.
 *
 * Single source of truth for colour, type, spacing, depth and the musical
 * grid. Every scene and primitive imports from here; never hard-code hex
 * values or frame counts that belong to the beat grid.
 */
import {Easing} from 'remotion';
import type {SpringConfig} from 'remotion';

/* ------------------------------------------------------------------------ */
/* Colour                                                                    */
/* ------------------------------------------------------------------------ */

/** Brand palette (exact hex from the project contract). */
export const color = {
	/** Deepest background — the "stage". */
	canvas: '#0a0d16',
	/** Card / window surface. */
	panel: '#111726',
	/** Raised surface (chips, key caps, inputs). */
	panel2: '#172033',
	/** Hairlines and dividers. */
	line: '#1f2a40',
	/** Primary text. */
	ink: '#e8edf9',
	/** Secondary text. */
	ink2: '#a8b4cd',
	/** Tertiary text / captions. */
	ink3: '#6f7d99',
	/** Primary accent — the blue of the "X" in ubiqX. */
	volt: '#4d8dff',
	/** Secondary accent — UBI's sash, warnings, "attention". */
	ember: '#ff7a1f',
	/** Negative / distraction. */
	rose: '#f2555a',
	/** Positive / focus. */
	mint: '#2ecc8f',
	white: '#ffffff',
	black: '#000000',
} as const;

export type ColorName = keyof typeof color;

/** Accent names accepted by primitives that take an `accent` prop. */
export type Accent = 'volt' | 'ember' | 'rose' | 'mint' | 'ink';

/** Resolve an accent name *or* any CSS colour string to a CSS colour. */
export const resolveColor = (c: Accent | ColorName | string | undefined, fallback: string = color.volt): string => {
	if (!c) return fallback;
	return (color as Record<string, string>)[c] ?? c;
};

/**
 * `alpha('#4d8dff', 0.3)` → `rgba(77, 141, 255, 0.3)`. Accepts #rgb / #rrggbb
 * or a palette name.
 */
export const alpha = (c: string, a: number): string => {
	const hex = resolveColor(c).replace('#', '');
	const full = hex.length === 3 ? hex.split('').map((x) => x + x).join('') : hex;
	const n = parseInt(full.slice(0, 6), 16);
	if (Number.isNaN(n)) return c;
	return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${a})`;
};

/** Pre-mixed gradients used across scenes. */
export const gradient = {
	/** Volt → light-blue, for highlighted headline words. */
	voltText: 'linear-gradient(92deg, #4d8dff 0%, #8fb8ff 55%, #4d8dff 100%)',
	/** Ember → amber, for "attention" words. */
	emberText: 'linear-gradient(92deg, #ff7a1f 0%, #ffb067 60%, #ff7a1f 100%)',
	/** Ink with a soft fade to ink2 — premium headline fill. */
	inkText: 'linear-gradient(180deg, #ffffff 0%, #e8edf9 45%, #a8b4cd 100%)',
	/** Card surface. */
	panel: 'linear-gradient(180deg, rgba(23,32,51,0.92) 0%, rgba(17,23,38,0.92) 100%)',
	/** Window title bar. */
	titleBar: 'linear-gradient(180deg, #1a2236 0%, #141b2b 100%)',
} as const;

/* ------------------------------------------------------------------------ */
/* Typography                                                                */
/* ------------------------------------------------------------------------ */

/** Font stacks. Faces are registered in src/design/fonts.ts. */
export const font = {
	/** Sora — headlines, numbers, the wordmark. */
	display: '"Sora", "Inter", ui-sans-serif, system-ui, sans-serif',
	/** Inter — UI copy, captions, labels. */
	text: '"Inter", ui-sans-serif, system-ui, sans-serif',
	/** Monospace for key caps / code-ish labels. */
	mono: 'ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace',
} as const;

export type FontRole = keyof typeof font;

/**
 * Type scale in px for a 1920x1080 canvas. Values chosen so the largest
 * headline still leaves ≥120px side margins at ~14 characters.
 */
export const type = {
	/** One or two words, full-screen slam. */
	hero: 168,
	/** Main headline. */
	display: 120,
	h1: 92,
	h2: 68,
	h3: 48,
	/** Card titles, callout headings. */
	title: 36,
	body: 28,
	small: 22,
	micro: 18,
} as const;

/** Letter-spacing presets (em). Sora wants to be tightened at large sizes. */
export const tracking = {
	hero: '-0.045em',
	display: '-0.04em',
	heading: '-0.025em',
	body: '-0.005em',
	caps: '0.14em',
} as const;

/** Font weights for the variable faces. */
export const weight = {
	regular: 400,
	medium: 500,
	semibold: 600,
	bold: 700,
	black: 800,
} as const;

/* ------------------------------------------------------------------------ */
/* Space, radii, depth                                                       */
/* ------------------------------------------------------------------------ */

/** Spacing scale (px). */
export const space = {
	xxs: 4,
	xs: 8,
	sm: 16,
	md: 24,
	lg: 40,
	xl: 64,
	xxl: 96,
	/** Title-safe side margin at 1080p. */
	safeX: 120,
	/** Title-safe top/bottom margin at 1080p. */
	safeY: 90,
} as const;

/** Corner radii (px). */
export const radius = {
	sm: 8,
	md: 14,
	lg: 20,
	xl: 28,
	/** App window corners at a ~1600px wide window. */
	window: 18,
	pill: 999,
} as const;

/** Box-shadows. */
export const shadow = {
	/** Floating card. */
	card: '0 2px 6px rgba(0,0,0,0.35), 0 24px 60px -24px rgba(0,0,0,0.8)',
	/** Big app window floating in space. */
	window:
		'0 0 0 1px rgba(255,255,255,0.06), 0 30px 60px -20px rgba(0,0,0,0.85), 0 80px 160px -40px rgba(0,0,0,0.9)',
	/** Tight contact shadow for small elements (pills, keys). */
	contact: '0 1px 2px rgba(0,0,0,0.5), 0 8px 24px -8px rgba(0,0,0,0.7)',
	/** Inner top highlight for glassy surfaces. */
	innerHighlight: 'inset 0 1px 0 rgba(255,255,255,0.07)',
} as const;

/** Glow helpers (box-shadow / text-shadow strings). */
export const glow = {
	volt: `0 0 24px ${alpha(color.volt, 0.55)}, 0 0 80px ${alpha(color.volt, 0.25)}`,
	ember: `0 0 24px ${alpha(color.ember, 0.5)}, 0 0 80px ${alpha(color.ember, 0.22)}`,
	mint: `0 0 24px ${alpha(color.mint, 0.5)}, 0 0 80px ${alpha(color.mint, 0.22)}`,
	/** Build a glow for any colour at a given strength (0–1). */
	of: (c: string, strength = 1): string =>
		`0 0 ${Math.round(24 * strength)}px ${alpha(c, 0.55 * strength)}, 0 0 ${Math.round(80 * strength)}px ${alpha(c, 0.25 * strength)}`,
	/** Text glow (text-shadow). */
	text: (c: string, strength = 1): string =>
		`0 0 ${Math.round(18 * strength)}px ${alpha(c, 0.55 * strength)}, 0 0 ${Math.round(48 * strength)}px ${alpha(c, 0.3 * strength)}`,
} as const;

/* ------------------------------------------------------------------------ */
/* Canvas + musical grid                                                     */
/* ------------------------------------------------------------------------ */

export const WIDTH = 1920;
export const HEIGHT = 1080;
export const FPS = 30;
/** Tempo of the soundtrack. */
export const BPM = 120;
/** Frames per beat (quarter note) at 120 BPM / 30 fps. */
export const BEAT = (FPS * 60) / BPM; // 15
/** Frames per 4/4 bar. */
export const BAR = BEAT * 4; // 60

/** `beats(2)` → 30 frames. Fractions allowed (`beats(0.5)` → 8, rounded). */
export const beats = (n: number): number => Math.round(n * BEAT);
/** `bars(1.5)` → 90 frames. */
export const bars = (n: number): number => Math.round(n * BAR);
/** Seconds → frames. */
export const sec = (s: number): number => Math.round(s * FPS);
/** Snap an arbitrary frame to the nearest beat (or subdivision: 2 = 8th notes). */
export const snapToBeat = (frame: number, subdivision = 1): number => {
	const step = BEAT / subdivision;
	return Math.round(Math.round(frame / step) * step);
};

/* ------------------------------------------------------------------------ */
/* Motion presets                                                            */
/* ------------------------------------------------------------------------ */

/** Spring configs for `spring({config})`. */
export const springs = {
	/** Fast, decisive, no visible wobble. UI pops, cards, pills. */
	snappy: {damping: 22, mass: 0.6, stiffness: 240, overshootClamping: false} satisfies Partial<SpringConfig>,
	/** Critically damped glide. Camera moves, big elements. */
	smooth: {damping: 200, mass: 1, stiffness: 120, overshootClamping: false} satisfies Partial<SpringConfig>,
	/** One small overshoot then settle. Logos, key caps, badges. */
	subtleBounce: {damping: 11, mass: 0.7, stiffness: 150, overshootClamping: false} satisfies Partial<SpringConfig>,
} as const;

export type SpringPreset = keyof typeof springs;

/** Easing curves for `interpolate(..., {easing})`. */
export const ease = {
	/** Expo-out: big initial velocity, long gentle landing. The default "push-in". */
	push: Easing.bezier(0.16, 1, 0.3, 1),
	/** Fast in-out with a hard middle — whip pans, swaps. */
	whip: Easing.bezier(0.83, 0, 0.17, 1),
	/** Quint-out: calm arrival, for things coming to rest. */
	settle: Easing.bezier(0.22, 1, 0.36, 1),
	/** Symmetric, gentle in-out — camera drifts. */
	inOut: Easing.bezier(0.65, 0, 0.35, 1),
	/** Accelerating exit. */
	exit: Easing.bezier(0.7, 0, 0.84, 0),
	/** Slight anticipation at the start, then push (for "slam"). */
	slam: Easing.bezier(0.2, 0.9, 0.1, 1),
	linear: Easing.linear,
} as const;

export type EasePreset = keyof typeof ease;

/** z-index layers so overlays stack predictably across primitives. */
export const layer = {
	background: 0,
	content: 10,
	screen: 20,
	callout: 40,
	cursor: 50,
	overlay: 80,
	grain: 90,
} as const;
