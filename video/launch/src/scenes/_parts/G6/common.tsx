/**
 * G6 (s17–s18) shared parts. Scene-local — not shared code.
 *
 *  - ramp / lerp / clamp01: frame maths (same shape as G2/G5 helpers)
 *  - TYPE: font metrics measured from public/fonts with HarfBuzz (cap-top offsets
 *    for line-height 1 boxes, so copy sits on the storyboard's exact cap-top y)
 *  - <Stage>: canvas + drifting orbs + optional perspective grid floor
 *    (G2 Backdrop recipe, every level a per-frame number)
 *  - <Vignette>, <FilmGrain>: the top of the stack, outside any camera
 *  - <MaskLine>: style S04 masked line reveal with the PT-accent padding
 */
import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame} from 'remotion';
import {noise2D} from '@remotion/noise';
import {Grain} from '../../../components/Grain';
import {StageBase, StageFinish, StageGuards, StageLights, STAGE, type StageLook} from '../../../components/Stage';
import {alpha, color} from '../../../design/tokens';
import {E} from '../../../shared/motion';

export const W = 1920;
export const H = 1080;
export const CLAMP = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
/** 0→1 between frames a and b (clamped), optional easing. */
export const ramp = (frame: number, a: number, b: number, easing?: (t: number) => number) =>
	b <= a ? (frame >= a ? 1 : 0) : interpolate(frame, [a, b], [0, 1], {...CLAMP, easing});

/**
 * Font metrics (hhea, em units) of the shipped variable faces, and the offset
 * of the cap top below the top of a `line-height: 1` box:
 * box top = capTop − capOffset × size.
 *   Sora:  ascent 0.97, descent 0.29, cap 0.73  → baseline 0.84 em, cap top 0.11 em
 *   Inter: ascent 0.969, descent 0.241, cap 0.7275 → baseline 0.8637 em, cap top 0.1362 em
 */
export const TYPE = {
	sora: {baseline: 0.84, capTop: 0.11, cap: 0.73},
	inter: {baseline: 0.8637, capTop: 0.1362, cap: 0.7275},
} as const;

/**
 * Advance widths measured with HarfBuzz on public/fonts (kerning on), px.
 * Used where a layout must be known before the DOM lays it out (chip morph,
 * connector anchor on "IA").
 */
export const MEASURED = {
	/** Sora 700 72 px −0.03em */
	line1: 989.1,
	line2: 698.5,
	line2BeforeIA: 67.1, // "A "
	ia: 75.9,
	/** Inter 500 48 px −0.01em */
	address: 431.0,
	/** Inter 600 48 px −0.01em */
	token: 157.0,
} as const;

/* ------------------------------------------------------------------------ */
/* Stage                                                                     */
/* ------------------------------------------------------------------------ */

export type Orb = {c: string; x: number; y: number; /** diameter px */ d: number; opacity: number; /** extra scale (breathing) */ s?: number};

export const Stage: React.FC<{
	seed: string;
	orbs: Orb[];
	/** Grid floor presence 0–1. */
	floor?: number;
	lineAlpha?: number;
	horizon?: number;
	gridSpeed?: number;
	/** Flat base instead of the v2 navy stage gradient. Default: the gradient. */
	base?: string;
	/** v2 stage rig overrides (components/Stage.tsx): key pool, pools, key light, aurora, level. */
	look?: StageLook;
	children?: React.ReactNode;
}> = ({seed, orbs, floor = 0, lineAlpha = 0.5, horizon = 0.7, gridSpeed = 0.6, base, look, children}) => {
	const frame = useCurrentFrame();
	const t = frame * 0.004;
	const cell = 110;
	const scroll = (frame * gridSpeed) % cell;
	const gc = color.volt;
	return (
		<AbsoluteFill style={{backgroundColor: base ?? STAGE.bottom, overflow: 'hidden'}}>
			{base ? null : <StageBase />}
			<StageLights seed={seed} {...look} />
			{/* soft stage light from the top: depth on every frame */}
			<AbsoluteFill style={{background: `radial-gradient(ellipse 75% 55% at 50% -8%, ${alpha(color.volt, 0.08)} 0%, transparent 70%)`}} />
			{orbs.map((o, i) => {
				if (o.opacity <= 0.001) return null;
				const nx = noise2D(`${seed}-ox-${i}`, t, i * 3.1) * 0.02 * W;
				const ny = noise2D(`${seed}-oy-${i}`, t, i * 7.7) * 0.014 * W;
				const d = o.d * (o.s ?? 1);
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: o.x + nx - d / 2,
							top: o.y + ny - d / 2,
							width: d,
							height: d,
							borderRadius: '50%',
							mixBlendMode: 'screen',
							background: `radial-gradient(closest-side, ${alpha(o.c, o.opacity)} 0%, ${alpha(o.c, o.opacity * 0.5)} 35%, ${alpha(o.c, o.opacity * 0.14)} 65%, ${alpha(o.c, 0)} 100%)`,
						}}
					/>
				);
			})}
			{floor > 0.002 ? (
				<AbsoluteFill style={{perspective: 700, perspectiveOrigin: `50% ${horizon * 100}%`, opacity: floor}}>
					<div
						style={{
							position: 'absolute',
							left: W / 2 - 2400,
							width: 4800,
							height: 2600,
							top: H + 40 - 2600,
							transformOrigin: '50% 100%',
							transform: 'rotateX(82deg)',
							backgroundImage: `linear-gradient(${alpha(gc, lineAlpha)} 1.5px, transparent 1.5px), linear-gradient(90deg, ${alpha(gc, lineAlpha)} 1.5px, transparent 1.5px)`,
							backgroundSize: `${cell}px ${cell}px`,
							backgroundPosition: `0px ${scroll}px`,
							WebkitMaskImage: 'linear-gradient(to top, rgba(0,0,0,1) 0%, rgba(0,0,0,0.75) 30%, rgba(0,0,0,0.3) 65%, transparent 96%)',
							maskImage: 'linear-gradient(to top, rgba(0,0,0,1) 0%, rgba(0,0,0,0.75) 30%, rgba(0,0,0,0.3) 65%, transparent 96%)',
						}}
					/>
					<div
						style={{
							position: 'absolute',
							left: 0,
							right: 0,
							top: horizon * H - 220,
							height: 440,
							background: `radial-gradient(ellipse 55% 50% at 50% 50%, ${alpha(gc, 0.18)} 0%, ${alpha(gc, 0.05)} 45%, transparent 75%)`,
						}}
					/>
					<div
						style={{
							position: 'absolute',
							left: W * 0.15,
							right: W * 0.15,
							top: horizon * H - 1,
							height: 2,
							background: `linear-gradient(90deg, transparent, ${alpha(gc, 0.45)}, transparent)`,
						}}
					/>
				</AbsoluteFill>
			) : null}
			<StageGuards guard={look?.guard} level={look?.level} keyPool={look?.keyPool} keyLight={look?.keyLight} />
			{children}
		</AbsoluteFill>
	);
};

/** Vignette (v1 strength scale; v2 caps it at 0.35 alpha, components/Stage.tsx). */
export const Vignette: React.FC<{strength: number}> = ({strength}) => <StageFinish vignette={strength} grain={0} />;

export const FilmGrain: React.FC<{opacity: number; seed: string}> = ({opacity, seed}) => <Grain opacity={opacity} seed={`${seed}-grain`} />;

/* ------------------------------------------------------------------------ */
/* Masked line (style S04)                                                   */
/* ------------------------------------------------------------------------ */

/**
 * One masked line: an overflow-hidden window padded .16em top / .12em bottom
 * (PT accents, Ç), the inner line rising translateY 110% → 0 and rotate 3° → 0
 * (origin left bottom), E.enter over `duration` frames from `at`.
 * Positioned by its cap top (`capTop`, canvas px) and left edge (or centred on `cx`).
 */
export const MaskLine: React.FC<{
	frame: number;
	at: number;
	duration?: number;
	capTop: number;
	left?: number;
	cx?: number;
	family: 'sora' | 'inter';
	size: number;
	weight: number;
	tracking: string;
	color: string;
	children: React.ReactNode;
	ariaLabel?: string;
}> = ({frame, at, duration = 18, capTop, left, cx, family, size, weight, tracking, color: c, children, ariaLabel}) => {
	if (frame < at) return null;
	const p = ramp(frame, at, at + duration, E.enter);
	const padT = 0.16 * size;
	const padB = 0.12 * size;
	const padX = 0.1 * size;
	const top = capTop - TYPE[family].capTop * size - padT;
	const fontFamily = family === 'sora' ? '"Sora", "Inter", sans-serif' : '"Inter", sans-serif';
	const box: React.CSSProperties =
		cx !== undefined
			? {position: 'absolute', left: 0, width: W, top, display: 'flex', justifyContent: 'center'}
			: {position: 'absolute', left: (left ?? 0) - padX, top};
	return (
		<div style={box} aria-label={ariaLabel}>
			<div style={{overflow: 'hidden', paddingTop: padT, paddingBottom: padB, paddingLeft: padX, paddingRight: padX}}>
				<div
					style={{
						fontFamily,
						fontWeight: weight,
						fontSize: size,
						lineHeight: 1,
						letterSpacing: tracking,
						color: c,
						whiteSpace: 'nowrap',
						transformOrigin: '0% 100%',
						transform: p < 1 ? `translateY(${((1 - p) * 110).toFixed(3)}%) rotate(${((1 - p) * 3).toFixed(3)}deg)` : undefined,
					}}
				>
					{children}
				</div>
			</div>
		</div>
	);
};
