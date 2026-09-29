/**
 * G2 (s05–s06) shared parts: the reveal backdrop (canvas + orbs + a grid
 * floor whose presence is a single 0–1 knob + vignette + grain), the slam
 * shake, easing/ramp helpers and the measured constants of the match cut.
 * Scene-local — not shared code.
 */
import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame} from 'remotion';
import {noise2D} from '@remotion/noise';
import {Grain} from '../../../components/Grain';
import {alpha, color} from '../../../design/tokens';

export const W = 1920;
export const H = 1080;
export const CLAMP = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
/** 0→1 between frames a and b (clamped), optional easing. */
export const ramp = (frame: number, a: number, b: number, easing?: (t: number) => number) =>
	interpolate(frame, [a, b], [0, 1], {...CLAMP, easing});

/** Mix two #rrggbb colours. */
export const mixHex = (a: string, b: string, t: number) => {
	const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
	const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
	const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * clamp01(t)));
	return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
};

/* ------------------------------------------------------------------------ */
/* Measured constants                                                        */
/* ------------------------------------------------------------------------ */

/**
 * Hoje hero-card colour around the in-app UBI, sampled from ui/dashboard.png
 * (image px 2230–2780 × 280–960 all read #0c1220; the storyboard's estimate was #0d1424).
 */
export const HERO_CARD = '#0c1220';

/** The volt caret of s03/s04 (G1: 4 × 56 px on (960, 520)). */
export const CARET = {x: 960, y: 520, w: 4, h: 56} as const;

/**
 * Match cut s05 f119 ↔ s06 f0. At s06 f0 the dashboard is shown at bitmap 1:1
 * with image point (2507, 721) on canvas (1560, 560): comp = image − (947, 161).
 * The in-app UBI ink box is image x 2402–2614, y 552–≈890 (measured) →
 * comp x 1455–1667, y 391–729. The 3D idle clip at a 475-px frame puts its ink
 * (idle 95: x 256–620, y 159–799 of 900) on the same box when its FEET anchor
 * (450, 797) sits on comp (UBI_MATCH.x, UBI_MATCH.y).
 */
export const DASH_OFFSET = {x: 947, y: 161} as const;
export const UBI_MATCH = {x: 1567, y: 729.5, size: 475} as const;
/** Same point in dashboard image px (for s06, where the camera moves). */
export const UBI_MATCH_IMG = {x: UBI_MATCH.x + DASH_OFFSET.x, y: UBI_MATCH.y + DASH_OFFSET.y} as const;
/** The app's own ember floor glow under its UBI (image px centre ≈ (2490, 905), ≈ 230 × 56). */
export const APP_GLOW = {x: 2490 - DASH_OFFSET.x, y: 906 - DASH_OFFSET.y, w: 250, h: 60} as const;

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
/* Backdrop                                                                  */
/* ------------------------------------------------------------------------ */

export type Orb = {c: string; x: number; y: number; /** diameter px */ d: number; opacity: number};

export type BackdropProps = {
	base?: string;
	orbs?: Orb[];
	/** Grid floor presence 0–1 (layer opacity of lines + horizon). */
	floor?: number;
	/** Line alpha of the grid at floor = 1. */
	lineAlpha?: number;
	horizon?: number;
	gridSpeed?: number;
	vignette?: number;
	grain?: number;
	seed: string;
	/** Soft top light (the product act's "stage" light). 0 = off. */
	topLight?: number;
	children?: React.ReactNode;
};

/**
 * Canvas + drifting orbs + optional perspective grid floor + vignette + grain.
 * Same recipe as <Background> (and G1's Backdrop), but every level is a
 * number the scene can animate per frame (the drop's orb bloom, the floor
 * fade-in on the downbeat, the tint to the hero-card colour for the match cut).
 */
export const Backdrop: React.FC<BackdropProps> = ({
	base = color.canvas,
	orbs = [],
	floor = 0,
	lineAlpha = 0.5,
	horizon = 0.66,
	gridSpeed = 1.2,
	vignette = 0.6,
	grain = 0.045,
	seed,
	topLight = 0,
	children,
}) => {
	const frame = useCurrentFrame();
	const t = frame * 0.004;
	const cell = 110;
	const scroll = (frame * gridSpeed) % cell;
	const gc = color.volt;
	return (
		<AbsoluteFill style={{backgroundColor: base, overflow: 'hidden'}}>
			{topLight > 0 ? (
				<AbsoluteFill
					style={{background: `radial-gradient(ellipse 75% 55% at 50% -8%, ${alpha(color.volt, 0.12 * topLight)} 0%, transparent 70%)`}}
				/>
			) : null}
			{orbs.map((o, i) => {
				if (o.opacity <= 0.001) return null;
				const nx = noise2D(`${seed}-ox-${i}`, t, i * 3.1) * 0.025 * W;
				const ny = noise2D(`${seed}-oy-${i}`, t, i * 7.7) * 0.018 * W;
				const b = 1 + noise2D(`${seed}-ob-${i}`, t * 0.7, i) * 0.03;
				const d = o.d * b;
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
							background: `radial-gradient(ellipse 55% 50% at 50% 50%, ${alpha(gc, 0.22)} 0%, ${alpha(gc, 0.06)} 45%, transparent 75%)`,
						}}
					/>
					<div
						style={{
							position: 'absolute',
							left: W * 0.15,
							right: W * 0.15,
							top: horizon * H - 1,
							height: 2,
							background: `linear-gradient(90deg, transparent, ${alpha(gc, 0.55)}, transparent)`,
						}}
					/>
				</AbsoluteFill>
			) : null}
			{children}
			{vignette > 0 ? (
				<AbsoluteFill
					style={{
						pointerEvents: 'none',
						background: `radial-gradient(ellipse 85% 80% at 50% 50%, transparent 50%, rgba(3,5,10,${0.75 * vignette}) 100%)`,
					}}
				/>
			) : null}
			{grain > 0 ? <Grain opacity={grain} seed={`${seed}-grain`} /> : null}
		</AbsoluteFill>
	);
};
