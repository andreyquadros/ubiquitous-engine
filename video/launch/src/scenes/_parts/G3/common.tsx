/**
 * G3 (s07–s09) shared parts — scene-local, not shared code.
 *
 *  - Stage / StageTop: canvas base and the top finishing layer (soft vignette +
 *    grain, same grain level as G1/G2's backdrops).
 *  - Words: the S03 word-stagger headline (SNAPPY, translateY 28 → 0, blur 8 → 0,
 *    opacity over 6 f) with volt-word / marker emphasis, positioned by CAP-TOP or
 *    BASELINE from the real Sora metrics.
 *  - DimMask: an image-space dim with any number of rounded holes (SVG even-odd),
 *    so two lit regions can coexist (the shared Spotlight stacks one 6000-px
 *    box-shadow per spotlight and cannot union holes).
 *  - Ring: a volt focus ring (2 px on screen + soft glow) in image space.
 *  - slamShake, ramp, lerp, mixHex helpers.
 */
import React from 'react';
import {AbsoluteFill, interpolate} from 'remotion';
import {noise2D} from '@remotion/noise';
import {StageBase, StageFinish, StageLights, type StageLook} from '../../../components/Stage';
import {color, font} from '../../../design/tokens';
import {springAt, type SpringName} from '../../../shared';

export const W = 1920;
export const H = 1080;
export const CLAMP = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
/** 0→1 between frames a and b (clamped), optional easing. */
export const ramp = (frame: number, a: number, b: number, easing?: (t: number) => number) =>
	b <= a ? (frame >= b ? 1 : 0) : interpolate(frame, [a, b], [0, 1], {...CLAMP, easing});

/** Mix two #rrggbb colours → rgb(). */
export const mixHex = (a: string, b: string, t: number) => {
	const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
	const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
	const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * clamp01(t)));
	return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
};

/* ------------------------------------------------------------------------ */
/* Font metrics (read from public/fonts with fontTools)                      */
/* ------------------------------------------------------------------------ */

/** Sora: ascent 970, descent 290, cap 730 (upm 1000). Inter: 1984 / 494 / 1490 (upm 2048). */
export const METRICS = {
	sora: {ascent: 0.97, descent: 0.29, cap: 0.73},
	inter: {ascent: 1984 / 2048, descent: 494 / 2048, cap: 1490 / 2048},
} as const;

/** Baseline offset from the top of a line box with line-height `lh` (em). */
export const baselineIn = (m: {ascent: number; descent: number}, size: number, lh = 1) =>
	((lh - (m.ascent + m.descent)) / 2 + m.ascent) * size;

/** Top of a line box (line-height `lh`) whose cap-top lands on `capTop`. */
export const boxTopForCapTop = (m: {ascent: number; descent: number; cap: number}, size: number, capTop: number, lh = 1) =>
	capTop - (baselineIn(m, size, lh) - m.cap * size);

/** Top of a line box (line-height `lh`) whose baseline lands on `baseline`. */
export const boxTopForBaseline = (m: {ascent: number; descent: number}, size: number, baseline: number, lh = 1) =>
	baseline - baselineIn(m, size, lh);

/* ------------------------------------------------------------------------ */
/* Stage                                                                     */
/* ------------------------------------------------------------------------ */

/**
 * v2 stage under the G3 planes (navy gradient + light rig, components/Stage.tsx).
 * The UI planes fill most of the frame; the stage shows around/through them
 * and through the scrims. `look` moves the key pool etc. per scene.
 */
export const Stage: React.FC<{children?: React.ReactNode; seed?: string; look?: StageLook}> = ({children, seed = 'g3', look}) => (
	<AbsoluteFill style={{backgroundColor: '#0a0f20', overflow: 'hidden'}}>
		<StageBase />
		<StageLights seed={seed} {...look} />
		{children}
	</AbsoluteFill>
);

/** Finishing layer over everything: a soft vignette (v2: ≤ 0.35 alpha) + grain (dithers the gradients). */
export const StageTop: React.FC<{seed: string; vignette?: number; grain?: number}> = ({seed, vignette = 0.42, grain = 0.04}) => (
	<StageFinish vignette={vignette} grain={grain} seed={seed} />
);

/* ------------------------------------------------------------------------ */
/* Slam shake (style S02): noise2D × amp × exp(−t/3) for 8 f, ±0.4°           */
/* ------------------------------------------------------------------------ */

export const slamShake = (frame: number, contact: number, seed: string, amp = 6, rotAmp = 0.4, len = 8) => {
	const t = frame - contact;
	if (t < 0 || t >= len) return {x: 0, y: 0, r: 0};
	const d = Math.exp(-t / 3);
	return {
		x: noise2D(`${seed}-sx`, t * 0.9, 0) * amp * d,
		y: noise2D(`${seed}-sy`, t * 0.9, 3.3) * amp * d,
		r: noise2D(`${seed}-sr`, t * 0.9, 7.1) * rotAmp * d,
	};
};

export const shakeTransform = (s: {x: number; y: number; r: number}) =>
	s.x === 0 && s.y === 0 && s.r === 0 ? undefined : `translate(${s.x.toFixed(2)}px, ${s.y.toFixed(2)}px) rotate(${s.r.toFixed(3)}deg)`;

/* ------------------------------------------------------------------------ */
/* Words: S03 word stagger                                                   */
/* ------------------------------------------------------------------------ */

export type WordUnit = {
	/** The unit's text (NBSP-glued groups count as one unit). */
	text: string;
	/** Frame the unit starts moving. */
	at: number;
	/** Turns volt as it lands. */
	volt?: boolean;
};

export type MarkerSpec = {
	/** Indices of the units the marker sweeps behind (contiguous). */
	units: [number, number];
	/** Sweep start and end frames. */
	from: number;
	to: number;
	easing?: (t: number) => number;
};

/**
 * One line of display type, units staggered in (S03). Position with `left` and
 * `capTop` or `baseline` (canvas px, exact from Sora's metrics).
 */
export const Words: React.FC<{
	f: number;
	units: WordUnit[];
	size: number;
	weight?: number;
	letterSpacing?: string;
	left: number;
	capTop?: number;
	baseline?: number;
	color?: string;
	marker?: MarkerSpec;
	preset?: SpringName;
	rise?: number;
}> = ({f, units, size, weight = 700, letterSpacing = '-0.035em', left, capTop, baseline, color: ink = color.ink, marker, preset = 'SNAPPY', rise = 28}) => {
	const top =
		capTop !== undefined
			? boxTopForCapTop(METRICS.sora, size, capTop)
			: boxTopForBaseline(METRICS.sora, size, baseline ?? 0);
	const renderUnit = (u: WordUnit, i: number) => {
		const s = f < u.at ? 0 : springAt(f, u.at, preset);
		const o = clamp01((f - u.at + 1) / 6);
		const blur = 8 * clamp01(1 - s);
		const volt = u.volt ? clamp01((s - 0.55) / 0.4) : 0;
		const y = rise * (1 - s);
		return (
			<span
				key={i}
				style={{
					display: 'inline-block',
					opacity: o,
					transform: Math.abs(y) > 0.01 ? `translateY(${y.toFixed(2)}px)` : undefined,
					filter: blur > 0.15 ? `blur(${blur.toFixed(2)}px)` : undefined,
					color: volt > 0 ? mixHex(color.ink, color.volt, volt) : undefined,
					position: 'relative',
					zIndex: 1,
				}}
			>
				{u.text}
			</span>
		);
	};
	const children: React.ReactNode[] = [];
	let i = 0;
	while (i < units.length) {
		if (marker && i === marker.units[0]) {
			const [a, b] = marker.units;
			const p = ramp(f, marker.from, marker.to, marker.easing);
			const group: React.ReactNode[] = [];
			for (let j = a; j <= b; j++) {
				group.push(renderUnit(units[j], j));
				if (j < b) group.push(' ');
			}
			children.push(
				<span key={`m${i}`} style={{position: 'relative', display: 'inline-block'}}>
					{p > 0 ? (
						<span
							style={{
								position: 'absolute',
								left: '-0.07em',
								top: '0.1em',
								height: '0.86em',
								width: `calc((100% + 0.14em) * ${p.toFixed(4)})`,
								borderRadius: '0.08em',
								background: 'rgba(77,141,255,0.16)',
								zIndex: 0,
							}}
						/>
					) : null}
					{group}
				</span>,
			);
			i = b + 1;
		} else {
			children.push(renderUnit(units[i], i));
			i += 1;
		}
		if (i < units.length) children.push(' ');
	}
	return (
		<div
			style={{
				position: 'absolute',
				left,
				top,
				fontFamily: font.display,
				fontWeight: weight,
				fontSize: size,
				lineHeight: 1,
				letterSpacing,
				color: ink,
				whiteSpace: 'pre',
			}}
		>
			{children}
		</div>
	);
};

/* ------------------------------------------------------------------------ */
/* Image-space dim with holes, focus rings                                   */
/* ------------------------------------------------------------------------ */

export type Hole = {x: number; y: number; w: number; h: number; r?: number; /** 0–1: how open the hole is (1 = fully lit) */ open?: number};

const rrPath = (x: number, y: number, w: number, h: number, r: number) => {
	const rr = Math.min(r, w / 2, h / 2);
	return `M${x + rr},${y} H${x + w - rr} A${rr},${rr} 0 0 1 ${x + w},${y + rr} V${y + h - rr} A${rr},${rr} 0 0 1 ${x + w - rr},${y + h} H${x + rr} A${rr},${rr} 0 0 1 ${x},${y + h - rr} V${y + rr} A${rr},${rr} 0 0 1 ${x + rr},${y} Z`;
};

/**
 * Dims the image outside the holes (a Screen child: image px). `dim` is the
 * darkness (0–1). Holes with open < 1 are drawn as partially dimmed patches.
 */
export const DimMask: React.FC<{dim: number; holes: Hole[]; size?: {w: number; h: number}; tint?: string}> = ({
	dim,
	holes,
	size = {w: 2880, h: 1800},
	tint = '10,16,36', // v2: navy tint (v1 '6,9,16')
}) => {
	if (dim <= 0.001) return null;
	const full = holes.filter((h) => (h.open ?? 1) >= 0.999);
	const partial = holes.filter((h) => (h.open ?? 1) < 0.999);
	const cut = [...full, ...partial];
	const d = `M0,0 H${size.w} V${size.h} H0 Z ` + cut.map((h) => rrPath(h.x, h.y, h.w, h.h, h.r ?? 14)).join(' ');
	return (
		<svg width={size.w} height={size.h} style={{position: 'absolute', left: 0, top: 0, overflow: 'visible'}}>
			<path d={d} fillRule="evenodd" fill={`rgba(${tint},${dim.toFixed(4)})`} />
			{partial.map((h, i) => (
				<path key={i} d={rrPath(h.x, h.y, h.w, h.h, h.r ?? 14)} fill={`rgba(${tint},${(dim * (1 - (h.open ?? 1))).toFixed(4)})`} />
			))}
		</svg>
	);
};

/**
 * A volt ring around an image-space rect: `line` and `glow` are ON-SCREEN px
 * (divided by the current on-screen scale `onScreen` = image px → canvas px).
 */
export const Ring: React.FC<{
	rect: {x: number; y: number; w: number; h: number};
	onScreen: number;
	opacity: number;
	radius?: number;
	line?: number;
	glow?: number;
	glowAlpha?: number;
	scale?: number;
	rgb?: string;
}> = ({rect, onScreen, opacity, radius = 14, line = 2, glow = 40, glowAlpha = 0.3, scale = 1, rgb = '77,141,255'}) => {
	if (opacity <= 0.001) return null;
	const k = Math.max(0.05, onScreen);
	const lw = line / k;
	const gw = glow / k;
	return (
		<div
			style={{
				position: 'absolute',
				left: rect.x,
				top: rect.y,
				width: rect.w,
				height: rect.h,
				borderRadius: radius,
				transform: scale !== 1 ? `scale(${scale.toFixed(4)})` : undefined,
				boxShadow: `0 0 0 ${lw.toFixed(3)}px rgba(${rgb},${(0.95 * opacity).toFixed(3)}), 0 0 ${gw.toFixed(2)}px ${(gw * 0.1).toFixed(2)}px rgba(${rgb},${(glowAlpha * opacity).toFixed(3)})`,
			}}
		/>
	);
};

/** Solid image-space rect (claim-safety patch or fill). */
export const Patch: React.FC<{x: number; y: number; w: number; h: number; fill: string; opacity?: number; radius?: number}> = ({x, y, w, h, fill, opacity = 1, radius = 0}) => (
	<div style={{position: 'absolute', left: x, top: y, width: w, height: h, background: fill, opacity, borderRadius: radius}} />
);
