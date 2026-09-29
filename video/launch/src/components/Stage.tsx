/**
 * The v2 stage (brief/v2-look.md §3 "Stage" + "Key light"): every backdrop of
 * the film is built from these layers so the whole cut shares one lighting rig.
 *
 *   <StageBase>    navy vertical gradient #0f1730 → #0a0f20 (not black)
 *   <StageLights>  volt key pool behind the subject (0.30–0.45), indigo + ember
 *                  secondary pools (0.10–0.20), a slow aurora sweep, and a soft
 *                  white-blue key light (#cfe0ff, 0.10–0.18) right behind the subject
 *   <StageFinish>  vignette (≤ 0.35 alpha) + film grain (0.035–0.05)
 *   <Stage>        all three with children between lights and finish
 *
 * Pure CSS gradients composited with normal alpha (no blend modes, no
 * filters), so the rig costs a few full-frame fills per frame. Deterministic:
 * motion comes from `frame` + `noise2D(seed)` only.
 */
import React from 'react';
import {AbsoluteFill, useCurrentFrame, useVideoConfig} from 'remotion';
import {noise2D} from '@remotion/noise';
import {alpha, color, layer, resolveColor, type Accent} from '../design/tokens';
import {Grain} from './Grain';

/** Stage palette. Indigo is a LIGHT colour only: never use it for UI or type. */
export const STAGE = {
	top: '#0f1730',
	bottom: '#0a0f20',
	indigo: '#6c5cff',
	keyLight: '#cfe0ff',
	/** Navy tint used for dims/scrims instead of black (spotlights, plane dims). */
	dim: '#0a1024',
	/** Vignette alpha at `vignette` strength 0.6 (the v1 default) and above. */
	vignetteMax: 0.35,
	grain: 0.042,
} as const;

/** v2 default spotlight dim (v1: 0.62 black): "the rest steps back", navy tinted. */
export const SPOTLIGHT_DIM = 0.38;

/** Map a v1 dim level (tuned against 0.62 black) to v2 (0.62 → 0.38, proportional). */
export const v2Dim = (v1: number) => (v1 * SPOTLIGHT_DIM) / 0.62;

/** rgba() of the navy dim tint (spotlights, scrims). */
export const navyDim = (a: number) => `rgba(10, 16, 36, ${Math.max(0, Math.min(1, a)).toFixed(3)})`;

/**
 * v1 vignette "strength" (alpha = 0.75 × s) → v2 alpha. Strength 0.6 (the v1
 * default) and above maps to the 0.35 cap; lower strengths scale linearly, so
 * per-frame animated vignettes keep their shape.
 */
export const vignetteAlpha = (strength: number) => Math.max(0, Math.min(STAGE.vignetteMax, (STAGE.vignetteMax * strength) / 0.6));

export type StagePool = {
	/** Palette accent or CSS colour. */
	color: Accent | string;
	/** Centre, fraction of canvas width / height. */
	x: number;
	y: number;
	/** Width as a fraction of canvas width. */
	w: number;
	/** Height as a fraction of canvas height. Default: same px as the width (circle). */
	h?: number;
	/** Peak alpha at the centre (before `level`). */
	opacity: number;
	/** Wander distance, fraction of width. Default 0.03. */
	drift?: number;
};

export type StageKeyLight = {
	/** Centre, fraction of canvas. */
	x: number;
	y: number;
	/** Width / height as fractions of the canvas. */
	w: number;
	h: number;
	/** Peak alpha (0.10–0.18). */
	opacity: number;
};

export type StageLook = {
	/** Global multiplier on every light (0 = lights out, base only). Default 1. */
	level?: number;
	/** The volt key pool behind the subject, or false. Default: centre, 0.38. (Not `key`: React reserves it.) */
	keyPool?: Partial<StagePool> | false;
	/** Secondary pools (indigo, ember). Default: indigo top-right 0.16, ember bottom-left 0.12. */
	pools?: StagePool[];
	/** The white-blue key light right behind the subject, or false. Default centre, 0.14. */
	keyLight?: Partial<StageKeyLight> | false;
	/** Aurora sweep strength 0–1. Default 1. */
	aurora?: number;
	/** Drift speed multiplier. Default 1. */
	speed?: number;
};

export const DEFAULT_KEY: StagePool = {color: 'volt', x: 0.5, y: 0.48, w: 1.15, h: 1.3, opacity: 0.38, drift: 0.025};
export const DEFAULT_POOLS: StagePool[] = [
	{color: STAGE.indigo, x: 0.84, y: 0.16, w: 0.8, opacity: 0.16},
	{color: 'ember', x: 0.1, y: 0.92, w: 0.66, opacity: 0.12},
];
export const DEFAULT_KEY_LIGHT: StageKeyLight = {x: 0.5, y: 0.48, w: 0.62, h: 0.78, opacity: 0.14};

/** Navy vertical gradient. `opacity` < 1 lets whatever is below show (s05's tint to the hero card). */
export const StageBase: React.FC<{opacity?: number; top?: string; bottom?: string}> = ({opacity = 1, top = STAGE.top, bottom = STAGE.bottom}) =>
	opacity <= 0.001 ? null : (
		<AbsoluteFill style={{background: `linear-gradient(180deg, ${top} 0%, ${bottom} 100%)`, opacity: opacity < 0.999 ? opacity : undefined}} />
	);

const poolGradient = (c: string, a: number) =>
	`radial-gradient(closest-side, ${alpha(c, a)} 0%, ${alpha(c, a * 0.62)} 30%, ${alpha(c, a * 0.26)} 62%, ${alpha(c, 0)} 100%)`;

/** The light rig (no base, no finish). Place it right above the base, under the content. */
export const StageLights: React.FC<StageLook & {seed?: string}> = ({
	level = 1,
	keyPool,
	pools = DEFAULT_POOLS,
	keyLight,
	aurora = 1,
	speed = 1,
	seed = 'stage',
}) => {
	const frame = useCurrentFrame();
	const {width: W, height: H} = useVideoConfig();
	if (level <= 0.001) return null;
	const t = frame * 0.004 * speed;
	const k = keyPool === false ? null : {...DEFAULT_KEY, ...keyPool};
	const kl = keyLight === false ? null : {...DEFAULT_KEY_LIGHT, ...keyLight};
	const all = k ? [k, ...pools] : pools;

	// aurora: two soft diagonal bands crossing the stage very slowly (one sweep ≈ 12 s)
	const a = aurora * level;
	const sweep = (frame * speed) / 360;
	const bandX1 = ((sweep % 1) * 1.8 - 0.4) * W;
	const bandX2 = ((((sweep + 0.55) % 1) * 1.8 - 0.4) * W);
	const wob = noise2D(`${seed}-aur`, t, 0.5) * 0.06 * H;

	return (
		<AbsoluteFill style={{pointerEvents: 'none', overflow: 'hidden'}}>
			{all.map((p, i) => {
				const op = p.opacity * level;
				if (op <= 0.002) return null;
				const c = resolveColor(p.color);
				const dr = p.drift ?? 0.03;
				const w = p.w * W;
				const h = p.h !== undefined ? p.h * H : w;
				const nx = noise2D(`${seed}-px-${i}`, t, i * 3.1) * dr * W;
				const ny = noise2D(`${seed}-py-${i}`, t, i * 7.7) * dr * W * 0.6;
				const b = 1 + noise2D(`${seed}-pb-${i}`, t * 0.7, i) * 0.04;
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: p.x * W + nx - (w * b) / 2,
							top: p.y * H + ny - (h * b) / 2,
							width: w * b,
							height: h * b,
							borderRadius: '50%',
							background: poolGradient(c, op),
						}}
					/>
				);
			})}
			{a > 0.01 ? (
				<>
					<div
						style={{
							position: 'absolute',
							left: bandX1 - 0.28 * W,
							top: -0.35 * H + wob,
							width: 0.56 * W,
							height: 1.7 * H,
							transform: 'rotate(24deg)',
							borderRadius: '50%',
							background: `radial-gradient(closest-side, ${alpha(color.volt, 0.085 * a)} 0%, ${alpha(STAGE.indigo, 0.05 * a)} 55%, transparent 100%)`,
						}}
					/>
					<div
						style={{
							position: 'absolute',
							left: bandX2 - 0.22 * W,
							top: -0.3 * H - wob,
							width: 0.44 * W,
							height: 1.6 * H,
							transform: 'rotate(24deg)',
							borderRadius: '50%',
							background: `radial-gradient(closest-side, ${alpha(STAGE.indigo, 0.08 * a)} 0%, ${alpha(color.volt, 0.035 * a)} 55%, transparent 100%)`,
						}}
					/>
				</>
			) : null}
			{kl && kl.opacity * level > 0.002 ? (
				<div
					style={{
						position: 'absolute',
						left: (kl.x - kl.w / 2) * W,
						top: (kl.y - kl.h / 2) * H,
						width: kl.w * W,
						height: kl.h * H,
						borderRadius: '50%',
						background: `radial-gradient(closest-side, ${alpha(STAGE.keyLight, kl.opacity * level)} 0%, ${alpha(STAGE.keyLight, kl.opacity * level * 0.45)} 40%, ${alpha(STAGE.keyLight, 0)} 100%)`,
					}}
				/>
			) : null}
		</AbsoluteFill>
	);
};

/** Vignette (strength as in v1; mapped to ≤ 0.35 alpha) + grain. Put it last. */
export const StageFinish: React.FC<{vignette?: number; grain?: number; seed?: string; zIndex?: boolean}> = ({
	vignette = 0.6,
	grain = STAGE.grain,
	seed = 'stage',
	zIndex = false,
}) => {
	const va = vignetteAlpha(vignette);
	return (
		<>
			{va > 0.002 ? (
				<AbsoluteFill
					style={{
						zIndex: zIndex ? layer.overlay : undefined,
						pointerEvents: 'none',
						background: `radial-gradient(ellipse 88% 84% at 50% 50%, transparent 52%, rgba(4, 7, 16, ${va.toFixed(3)}) 100%)`,
					}}
				/>
			) : null}
			{grain > 0 ? <Grain opacity={grain} seed={`${seed}-grain`} /> : null}
		</>
	);
};

export type StageProps = StageLook & {
	seed?: string;
	/** Flat colour under the stage (shows only where the base opacity is < 1). Default STAGE.bottom. */
	under?: string;
	/** Opacity of the navy gradient base. Default 1. */
	baseOpacity?: number;
	vignette?: number;
	grain?: number;
	children?: React.ReactNode;
};

/**
 * Full stage: navy base + light rig + children + vignette + grain.
 *
 * @example
 * <Stage seed="s09" keyPool={{x: 0.62, y: 0.55}} keyLight={{x: 0.62, y: 0.52}}>
 *   <Screen {...shot} style={{zIndex: 'auto'}} />
 * </Stage>
 */
export const Stage: React.FC<StageProps> = ({seed = 'stage', under = STAGE.bottom, baseOpacity = 1, vignette = 0.6, grain = STAGE.grain, children, ...look}) => (
	<AbsoluteFill style={{backgroundColor: under, overflow: 'hidden'}}>
		<StageBase opacity={baseOpacity} />
		<StageLights seed={seed} {...look} />
		{children}
		<StageFinish vignette={vignette} grain={grain} seed={seed} />
	</AbsoluteFill>
);
