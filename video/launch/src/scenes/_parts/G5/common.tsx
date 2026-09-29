/**
 * G5 (s14–s16) shared parts. Scene-local — not shared code.
 *
 *  - helpers: ramp / lerp / clamp01 / mixHex / arcPoint (same maths as G4's, so
 *    cursor arcs bend the same way across the film)
 *  - APP: the desktop app's own dark-theme tokens (apps/desktop/src/index.css),
 *    used by every vector re-set so it matches the captures pixel for pixel
 *  - <Backdrop>: canvas + two slow orbs + vignette + grain (G2/G4 recipe)
 *  - <Headline>: the S03 word stagger (SNAPPY, y 28 → 0, blur 8 → 0, opacity 6 f)
 *    at an exact cap-top, emphasis as a volt sub-span of a unit
 *  - <ArrowCursor> / <ClickRipple> / cursorScale(): style S13 pointer (G4 recipe)
 *  - <Crop>: an image-space bitmap crop (from the @3x twin when shipped)
 *  - lucide icon paths (KeyRound, Copy, Check) as inline SVG, stroke in image px
 */
import React from 'react';
import {AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame} from 'remotion';
import {noise2D} from '@remotion/noise';
import {StageBase, StageFinish, StageLights, STAGE, type StageLook} from '../../../components/Stage';
import type {Point, Rect} from '../../../components/screen-geometry';
import {alpha, color, font} from '../../../design/tokens';
import {E, springAt} from '../../../shared/motion';
import {useHires} from '../../../shared/ui';

export const W = 1920;
export const H = 1080;
export const CLAMP = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
/** 0→1 between frames a and b (clamped), optional easing. */
export const ramp = (frame: number, a: number, b: number, easing?: (t: number) => number) =>
	b <= a ? (frame >= a ? 1 : 0) : interpolate(frame, [a, b], [0, 1], {...CLAMP, easing});

/** Mix two #rrggbb colours → #rrggbb (chainable). */
export const mixHex = (a: string, b: string, t: number) => {
	const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
	const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
	const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * clamp01(t)));
	return `#${c.map((v) => v.toString(16).padStart(2, '0')).join('')}`;
};

/** Point on a quadratic arc from a to b (control point `bend` × distance off the chord, left-hand normal — G4's bend). */
export const arcPoint = (a: Point, b: Point, t: number, bend = 0.12): Point => {
	const dx = b.x - a.x;
	const dy = b.y - a.y;
	const mx = (a.x + b.x) / 2 + dy * bend;
	const my = (a.y + b.y) / 2 - dx * bend;
	const u = 1 - t;
	return {x: u * u * a.x + 2 * u * t * mx + t * t * b.x, y: u * u * a.y + 2 * u * t * my + t * t * b.y};
};

export const rectCenter = (r: Rect): Point => ({x: r.x + r.w / 2, y: r.y + r.h / 2});

/** Style S13 travel time: clamp(10 + dist/60, 12, 26) f. */
export const travelFrames = (a: Point, b: Point) => Math.round(Math.max(12, Math.min(26, 10 + Math.hypot(b.x - a.x, b.y - a.y) / 60)));

/* ------------------------------------------------------------------------ */
/* The app's own tokens (apps/desktop/src/index.css, dark theme)             */
/* ------------------------------------------------------------------------ */

export const APP = {
	panel: '#0c1220',
	panel2: '#121a2b',
	/** line-2 = rgb(126 158 214 / 0.28) over panel (sampled #2c3953). */
	line2: '#2c3953',
	/** line = rgb(126 158 214 / 0.14) over panel-2 (sampled #212c43). */
	line: '#212c43',
	ink: '#eaf0fa',
	ink2: '#9daec7',
	signal: '#2ee6a6',
} as const;

/* ------------------------------------------------------------------------ */
/* Backdrop                                                                  */
/* ------------------------------------------------------------------------ */

export const Backdrop: React.FC<{
	seed: string;
	children?: React.ReactNode;
	grain?: number;
	vignette?: number;
	ember?: number;
	volt?: number;
	/** v2 stage rig overrides (components/Stage.tsx). */
	look?: StageLook;
}> = ({seed, children, ember = 0.06, volt = 0.16, look}) => {
	const frame = useCurrentFrame();
	const t = frame * 0.004;
	const orbs = [
		{c: color.volt, x: 0.2, y: 0.12, d: 1100, o: volt},
		{c: color.ember, x: 0.86, y: 0.9, d: 760, o: ember},
	];
	return (
		<AbsoluteFill style={{backgroundColor: STAGE.bottom, overflow: 'hidden'}}>
			<StageBase />
			<StageLights seed={seed} {...look} />
			{orbs.map((o, i) => {
				if (o.o <= 0) return null;
				const nx = noise2D(`${seed}-ox-${i}`, t, i * 3.1) * 0.025 * W;
				const ny = noise2D(`${seed}-oy-${i}`, t, i * 7.7) * 0.018 * W;
				const d = o.d * (1 + noise2D(`${seed}-ob-${i}`, t * 0.7, i) * 0.03);
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: o.x * W + nx - d / 2,
							top: o.y * H + ny - d / 2,
							width: d,
							height: d,
							borderRadius: '50%',
							background: `radial-gradient(closest-side, ${alpha(o.c, o.o)} 0%, ${alpha(o.c, o.o * 0.45)} 38%, ${alpha(o.c, 0)} 100%)`,
						}}
					/>
				);
			})}
			{children}
		</AbsoluteFill>
	);
};

/** Finishing layer (vignette, v2: ≤ 0.35 alpha + grain) — put it last. */
export const Finish: React.FC<{seed: string; vignette?: number; grain?: number}> = ({seed, vignette = 0.55, grain = 0.045}) => (
	<StageFinish vignette={vignette} grain={grain} seed={seed} />
);

/* ------------------------------------------------------------------------ */
/* Headline: S03 word stagger                                                */
/* ------------------------------------------------------------------------ */

/** Sora cap top sits 0.11 × size below the line box top when line-height = 1 (ascent 0.97, descent 0.29, cap 0.73). */
export const SORA_CAP = 0.11;

export type HeadUnit = {
	/** Unit text; spaces inside a unit are glued with U+00A0. */
	text: string;
	at: number;
	/** Trailing part of the unit that turns volt as it lands (e.g. "pronto." or "Ubi."). */
	volt?: string;
};

/** Throws if the units don't rebuild the storyboard string byte-exact. */
export const unitsOf = (text: string, units: HeadUnit[]): HeadUnit[] => {
	const rebuilt = units.map((u) => u.text).join(' ');
	if (rebuilt !== text) throw new Error(`G5 headline copy drift: "${rebuilt}" ≠ storyboard "${text}"`);
	return units;
};

export const Headline: React.FC<{units: HeadUnit[]; size: number; left: number; capTop: number; tracking?: string}> = ({
	units,
	size,
	left,
	capTop,
	tracking = '-0.03em',
}) => {
	const frame = useCurrentFrame();
	return (
		<div
			style={{
				position: 'absolute',
				left,
				top: Math.round(capTop - SORA_CAP * size),
				fontFamily: font.display,
				fontWeight: 700,
				fontSize: size,
				lineHeight: 1,
				letterSpacing: tracking,
				color: color.ink,
				whiteSpace: 'nowrap',
				pointerEvents: 'none',
			}}
		>
			{units.map((u, i) => {
				if (frame < u.at) {
					return (
						<React.Fragment key={i}>
							<span style={{display: 'inline-block', opacity: 0}}>{u.text.replace(/ /g, ' ')}</span>
							{i < units.length - 1 ? ' ' : null}
						</React.Fragment>
					);
				}
				const s = springAt(frame, u.at, 'SNAPPY');
				const o = clamp01((frame - u.at + 1) / 6);
				const settled = frame >= u.at + 14;
				const ty = settled ? 0 : 28 * (1 - s);
				const blur = settled ? 0 : 8 * clamp01(1 - s);
				const v = u.volt ? clamp01((s - 0.55) / 0.4) : 0;
				const lead = u.volt ? u.text.slice(0, u.text.length - u.volt.length) : u.text;
				return (
					<React.Fragment key={i}>
						<span
							style={{
								display: 'inline-block',
								opacity: o,
								transform: Math.abs(ty) > 0.05 ? `translateY(${ty.toFixed(2)}px)` : undefined,
								filter: blur > 0.15 ? `blur(${blur.toFixed(2)}px)` : undefined,
							}}
						>
							{lead.replace(/ /g, ' ')}
							{u.volt ? <span style={{color: mixHex(color.ink, color.volt, v)}}>{u.volt}</span> : null}
						</span>
						{i < units.length - 1 ? ' ' : null}
					</React.Fragment>
				);
			})}
		</div>
	);
};

/* ------------------------------------------------------------------------ */
/* Cursor (style S13): white arrow, 1.5 px dark stroke, press + ripple        */
/* ------------------------------------------------------------------------ */

/** Cursor size for a camera k: 30 × clamp(1 + (k − 1)·0.35, 1, 1.5) (same rule as G4). */
export const cursorScale = (k: number) => Math.min(1.5, Math.max(1, 1 + (k - 1) * 0.35));

/** Press factor 0→1→0 around a click at C: down C−1..C+1, SNAPPY release. */
export const pressAt = (f: number, click: number) => {
	if (f < click - 1) return 0;
	if (f <= click + 1) return ramp(f, click - 1, click + 1);
	return clamp01(1 - springAt(f, click + 1, 'SNAPPY', 5));
};

export const ArrowCursor: React.FC<{x: number; y: number; size: number; opacity?: number}> = ({x, y, size, opacity = 1}) => {
	if (opacity <= 0.001) return null;
	const k = size / 30;
	return (
		<svg
			width={20 * k + 8}
			height={30 * k + 8}
			viewBox={`0 0 ${20 + 8 / k} ${30 + 8 / k}`}
			style={{position: 'absolute', left: x - 2 * k, top: y - 1.6 * k, opacity, overflow: 'visible', filter: `drop-shadow(0 ${2 * k}px ${3 * k}px rgba(0,0,0,0.5))`}}
		>
			<path
				d="M2 1.6 L2 24.2 L7.3 19.3 L10.9 27.6 L14.9 25.9 L11.4 17.8 L18.6 17.8 Z"
				fill="#ffffff"
				stroke="#0a0d16"
				strokeWidth={1.5 / k}
				strokeLinejoin="round"
			/>
		</svg>
	);
};

/** Screen-space click ripple centred on (x, y): 0→44 px, 14 f E.push, + ring 3 f later → 64 px. */
export const ClickRipple: React.FC<{x: number; y: number; at: number; scale?: number; tint?: string}> = ({x, y, at, scale = 1, tint = color.volt}) => {
	const frame = useCurrentFrame();
	const rings = [
		{start: at, r: 44, o: 0.6, len: 14},
		{start: at + 3, r: 64, o: 0.3, len: 14},
	];
	return (
		<>
			{rings.map((g, i) => {
				const d = frame - g.start;
				if (d < 0 || d > g.len) return null;
				const t = clamp01(d / g.len);
				const r = g.r * E.push(t) * scale;
				const sw = lerp(2, 0.5, t) * scale;
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: x - r,
							top: y - r,
							width: r * 2,
							height: r * 2,
							borderRadius: '50%',
							border: `${sw.toFixed(2)}px solid ${alpha(tint, g.o * (1 - t))}`,
							boxSizing: 'border-box',
						}}
					/>
				);
			})}
		</>
	);
};

/* ------------------------------------------------------------------------ */
/* Bitmap crop (image space)                                                 */
/* ------------------------------------------------------------------------ */

/** A piece of a capture drawn at `at` (default: its own place). Draws the @3x twin when shipped. */
export const Crop: React.FC<{src: string; rect: Rect; at?: Point; imgW?: number; imgH?: number; style?: React.CSSProperties}> = ({
	src,
	rect,
	at,
	imgW = 2880,
	imgH = 1800,
	style,
}) => {
	const hires = useHires(src);
	const file = hires ? `ui/${hires.hires}` : src;
	const p = at ?? {x: rect.x, y: rect.y};
	return (
		<div style={{position: 'absolute', left: p.x, top: p.y, width: rect.w, height: rect.h, overflow: 'hidden', ...style}}>
			<Img src={staticFile(file)} style={{position: 'absolute', left: -rect.x, top: -rect.y, width: imgW, height: imgH, maxWidth: 'none'}} />
		</div>
	);
};

/* ------------------------------------------------------------------------ */
/* lucide icons (v1.4x paths) — the app's own icons                          */
/* ------------------------------------------------------------------------ */

export const Icon: React.FC<{
	name: 'key-round' | 'copy' | 'check';
	size: number;
	stroke: string;
	strokeWidth?: number;
	/** 0–1: evolvePath-style draw-on (check). */
	draw?: number;
	style?: React.CSSProperties;
}> = ({name, size, stroke, strokeWidth = 1.75, draw = 1, style}) => {
	const common = {fill: 'none', stroke, strokeWidth, strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const};
	return (
		<svg width={size} height={size} viewBox="0 0 24 24" style={{display: 'block', overflow: 'visible', ...style}}>
			{name === 'key-round' ? (
				<>
					<path
						{...common}
						d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"
					/>
					<circle {...common} cx="16.5" cy="7.5" r=".5" fill={stroke} />
				</>
			) : name === 'copy' ? (
				<>
					<rect {...common} width="14" height="14" x="8" y="8" rx="2" ry="2" />
					<path {...common} d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2" />
				</>
			) : (
				<path {...common} d="M20 6 9 17l-5-5" pathLength={1} strokeDasharray={1} strokeDashoffset={1 - clamp01(draw)} />
			)}
		</svg>
	);
};
