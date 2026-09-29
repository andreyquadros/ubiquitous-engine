/**
 * G6 kit (s16–s18): the pieces s16 used to import from G5's common, copied here
 * so G6 does not depend on another group's scene-local file (same maths, same
 * look), plus v2 additions:
 *
 *  - APP: the desktop app's own dark-theme tokens (apps/desktop/src/index.css)
 *  - mixHex / arcPoint: colour mix, cursor arc (G4/G5 bend)
 *  - <Headline>: S03 word stagger (SNAPPY, y 28 → 0, blur 8 → 0) at an exact cap
 *    top; v2 fill (top-lit white gradient, volt gradient + glow for the emphasis);
 *    optional per-unit `lit` (0–1) that brightens a word (a soft white glow)
 *  - <ArrowCursor> / <ClickRipple> / cursorScale() / pressAt(): style S13 pointer
 *  - <Icon name="key-round">: lucide KeyRound (the app's own icon)
 *  - <Shockwave>: click impact ring
 *  - projectCardPoint(): where a point of a <LiftCard> lands on the canvas (the
 *    card's CSS rotateX · rotateY · rotateZ · scale, perspective around its centre)
 */
import React from 'react';
import {useCurrentFrame} from 'remotion';
import type {Point} from '../../../components/screen-geometry';
import {alpha, color, font} from '../../../design/tokens';
import {E, springAt} from '../../../shared/motion';
import {clamp01, lerp, ramp} from './common';

export const APP = {
	panel: '#0c1220',
	panel2: '#121a2b',
	/** line-2 = rgb(126 158 214 / 0.28) over panel (sampled #2c3953). */
	line2: '#2c3953',
	line: '#212c43',
	ink: '#eaf0fa',
	ink2: '#9daec7',
	signal: '#2ee6a6',
} as const;

/** Mix two #rrggbb colours → #rrggbb. */
export const mixHex = (a: string, b: string, t: number) => {
	const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
	const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
	const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * clamp01(t)));
	return `#${c.map((v) => v.toString(16).padStart(2, '0')).join('')}`;
};

/** Point on a quadratic arc from a to b (control point `bend` × distance off the chord, left-hand normal). */
export const arcPoint = (a: Point, b: Point, t: number, bend = 0.12): Point => {
	const dx = b.x - a.x;
	const dy = b.y - a.y;
	const mx = (a.x + b.x) / 2 + dy * bend;
	const my = (a.y + b.y) / 2 - dx * bend;
	const u = 1 - t;
	return {x: u * u * a.x + 2 * u * t * mx + t * t * b.x, y: u * u * a.y + 2 * u * t * my + t * t * b.y};
};

/* ------------------------------------------------------------------------ */
/* Headline                                                                  */
/* ------------------------------------------------------------------------ */

export const SORA_CAP = 0.11;

export type HeadUnit = {
	/** Unit text; spaces inside a unit are glued with U+00A0. */
	text: string;
	at: number;
	/** Trailing part of the unit that turns volt as it lands. */
	volt?: string;
	/** 0–1 per frame: a soft white lift on this word (hover sync). */
	lit?: (frame: number) => number;
};

/** Throws if the units don't rebuild the storyboard string byte-exact. */
export const unitsOf = (text: string, units: HeadUnit[]): HeadUnit[] => {
	// the storyboard glues short words with U+00A0; unit texts may use either space
	const norm = (t: string) => t.replace(/\u00a0/g, ' ');
	const rebuilt = units.map((u) => u.text).join(' ');
	if (norm(rebuilt) !== norm(text)) throw new Error(`G6 headline copy drift: "${rebuilt}" ≠ storyboard "${text}"`);
	return units;
};

const INK_TOP = '#ffffff';
const INK_BOTTOM = '#c3cde0';
const VOLT_TOP = '#8ab4ff';
export const gradientFill = (volt = 0, lit = 0): React.CSSProperties => ({
	backgroundImage: `linear-gradient(180deg, ${mixHex(INK_TOP, VOLT_TOP, volt)} 18%, ${mixHex(mixHex(INK_BOTTOM, '#ffffff', lit * 0.8), color.volt, volt)} 92%)`,
	WebkitBackgroundClip: 'text',
	backgroundClip: 'text',
	color: 'transparent',
	WebkitTextFillColor: 'transparent',
});
export const voltGlow = (a: number) => `drop-shadow(0 0 18px rgba(77,141,255,${(0.45 * a).toFixed(3)}))`;
const TYPE_SHADOW = 'drop-shadow(0 4px 18px rgba(6,10,24,0.55))';

export const Headline: React.FC<{units: HeadUnit[]; size: number; left: number; capTop: number; tracking?: string; weight?: number; style?: React.CSSProperties}> = ({
	units,
	size,
	left,
	capTop,
	tracking = '-0.04em',
	weight = 700,
	style,
}) => {
	const frame = useCurrentFrame();
	return (
		<div
			style={{
				position: 'absolute',
				left,
				top: Math.round(capTop - SORA_CAP * size),
				fontFamily: font.display,
				fontWeight: weight,
				fontSize: size,
				lineHeight: 1,
				letterSpacing: tracking,
				color: color.ink,
				whiteSpace: 'nowrap',
				pointerEvents: 'none',
				filter: TYPE_SHADOW,
				...style,
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
				const lit = u.lit ? clamp01(u.lit(frame)) : 0;
				const lead = u.volt ? u.text.slice(0, u.text.length - u.volt.length) : u.text;
				const filters = [blur > 0.15 ? `blur(${blur.toFixed(2)}px)` : '', lit > 0.01 ? `drop-shadow(0 0 22px rgba(207,224,255,${(0.42 * lit).toFixed(3)}))` : '']
					.filter(Boolean)
					.join(' ');
				return (
					<React.Fragment key={i}>
						<span
							style={{
								display: 'inline-block',
								opacity: o,
								transform: Math.abs(ty) > 0.05 || lit > 0.01 ? `translateY(${(ty - 4 * lit).toFixed(2)}px)` : undefined,
								filter: filters || undefined,
							}}
						>
							{lead ? <span style={gradientFill(0, lit)}>{lead.replace(/ /g, ' ')}</span> : null}
							{u.volt ? <span style={{...gradientFill(v), filter: v > 0.01 ? voltGlow(v) : undefined}}>{u.volt}</span> : null}
						</span>
						{i < units.length - 1 ? ' ' : null}
					</React.Fragment>
				);
			})}
		</div>
	);
};

/* ------------------------------------------------------------------------ */
/* Cursor (style S13)                                                        */
/* ------------------------------------------------------------------------ */

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
			<path d="M2 1.6 L2 24.2 L7.3 19.3 L10.9 27.6 L14.9 25.9 L11.4 17.8 L18.6 17.8 Z" fill="#ffffff" stroke="#0a0d16" strokeWidth={1.5 / k} strokeLinejoin="round" />
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

/** lucide KeyRound (v1.4x path), stroke in the parent's px. */
export const KeyIcon: React.FC<{size: number; stroke: string; strokeWidth?: number}> = ({size, stroke, strokeWidth = 1.75}) => {
	const common = {fill: 'none', stroke, strokeWidth, strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const};
	return (
		<svg width={size} height={size} viewBox="0 0 24 24" style={{display: 'block', overflow: 'visible'}}>
			<path
				{...common}
				d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"
			/>
			<circle {...common} cx="16.5" cy="7.5" r=".5" fill={stroke} />
		</svg>
	);
};

/** Click shockwave: a thin ring + soft disc expanding from (x, y) over `len` frames (screen space). */
export const Shockwave: React.FC<{x: number; y: number; at: number; radius?: number; len?: number; tint?: string; strength?: number; squash?: number}> = ({
	x,
	y,
	at,
	radius = 360,
	len = 16,
	tint = color.volt,
	strength = 1,
	squash = 1,
}) => {
	const frame = useCurrentFrame();
	const d = frame - at;
	if (d < 0 || d > len) return null;
	const t = clamp01(d / len);
	const r = radius * E.push(t);
	const fade = (1 - t) * strength;
	const box = {position: 'absolute' as const, left: x - r, top: y - r * squash, width: r * 2, height: r * 2 * squash, borderRadius: '50%'};
	return (
		<>
			<div style={{...box, background: `radial-gradient(closest-side, ${alpha(tint, 0)} 55%, ${alpha(tint, 0.16 * fade)} 88%, ${alpha(tint, 0)} 100%)`}} />
			<div
				style={{
					...box,
					boxSizing: 'border-box',
					border: `${lerp(4, 1, t).toFixed(2)}px solid ${alpha(tint, 0.75 * fade)}`,
					boxShadow: `0 0 ${(24 * fade).toFixed(1)}px ${alpha(tint, 0.5 * fade)}`,
				}}
			/>
		</>
	);
};

/* ------------------------------------------------------------------------ */
/* LiftCard projection                                                       */
/* ------------------------------------------------------------------------ */

/**
 * Canvas position of a point on a LiftCard. `u`, `v` are offsets from the card
 * centre in the card's own (unrotated) px; `pose` from liftCardPose().
 * CSS: transform = rotateX(rx) rotateY(ry) rotateZ(rz) scale(s) about the
 * centre, parent perspective P with its origin at the card centre.
 */
export const projectCardPoint = (pose: {cx: number; cy: number; rx: number; ry: number; rz: number; s: number}, u: number, v: number, perspective = 1600): Point => {
	const rad = Math.PI / 180;
	let x = u * pose.s;
	let y = v * pose.s;
	let z = 0;
	// rotateZ
	const cz = Math.cos(pose.rz * rad);
	const sz = Math.sin(pose.rz * rad);
	[x, y] = [x * cz - y * sz, x * sz + y * cz];
	// rotateY
	const cy = Math.cos(pose.ry * rad);
	const sy = Math.sin(pose.ry * rad);
	[x, z] = [x * cy + z * sy, -x * sy + z * cy];
	// rotateX
	const cx = Math.cos(pose.rx * rad);
	const sx = Math.sin(pose.rx * rad);
	[y, z] = [y * cx - z * sx, y * sx + z * cx];
	const k = perspective / (perspective - z);
	return {x: pose.cx + x * k, y: pose.cy + y * k};
};
