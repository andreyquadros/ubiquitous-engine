import React from 'react';
import {AbsoluteFill, useCurrentFrame, useVideoConfig} from 'remotion';
import {noise2D} from '@remotion/noise';
import {alpha, layer, resolveColor, type Accent} from '../design/tokens';
import {StageBase, StageFinish, StageGuards, StageLights, STAGE, type StageLook} from './Stage';

export type OrbSpec = {
	/** Palette accent or CSS colour. */
	color: Accent | string;
	/** Rest position, fraction of canvas width (0–1). */
	x: number;
	/** Rest position, fraction of canvas height (0–1). */
	y: number;
	/** Diameter as a fraction of canvas width. */
	size: number;
	/** Peak alpha at the orb centre (0–1). */
	opacity: number;
};

export type BackgroundProps = {
	/**
	 * - `orbs`  — deep canvas + slowly drifting volt/ember light orbs (default)
	 * - `grid`  — orbs + a perspective grid floor receding to a glowing horizon
	 * - `plain` — flat canvas with a faint top light, for maximum focus
	 */
	variant?: 'grid' | 'orbs' | 'plain';
	/**
	 * Extra drifting orbs on top of the v2 stage rig (see <Stage>). Default: none
	 * (the rig's volt key pool + indigo/ember pools light the stage).
	 */
	orbs?: OrbSpec[];
	/** Global multiplier on orb opacity AND the stage lights. Default 1. */
	orbIntensity?: number;
	/** How far orbs drift, fraction of canvas width. Default 0.06. */
	drift?: number;
	/** Drift speed multiplier (1 ≈ one slow wander per ~10 s). Default 1. */
	speed?: number;
	/** Grid line colour. Default volt. */
	gridColor?: Accent | string;
	/** Grid line alpha (0–1). Default 0.3. */
	gridOpacity?: number;
	/** Grid scroll speed in px/frame toward the viewer. 0 = static. Default 2. */
	gridSpeed?: number;
	/** Horizon height as a fraction of canvas height (where the floor fades out). Default 0.68. */
	horizon?: number;
	/** Vignette strength 0–1 (v1 scale; 0.6+ maps to the v2 cap of 0.35 alpha). Default 0.6. */
	vignette?: number;
	/** Grain opacity (0 disables). Default 0.042. */
	grain?: number;
	/** Noise seed — vary it so consecutive scenes don't drift identically. Default "bg". */
	seed?: string;
	/** Flat base colour instead of the navy stage gradient. Default: the gradient. */
	base?: string;
	/** Stage rig overrides (key pool position, pools, key light, aurora, level). */
	look?: StageLook;
	/** Content rendered above the background but under the vignette + grain. */
	children?: React.ReactNode;
};

const NO_ORBS: OrbSpec[] = [];

/**
 * Full-frame stage background. Deterministic (noise seeded by `seed`, grain
 * seeded by frame). Wrap scene content in it, or place it first in a scene.
 *
 * @example
 * <Background variant="grid" seed="hero">
 *   <Center><KineticText text="Retome o controle." mode="slam" /></Center>
 * </Background>
 */
export const Background: React.FC<BackgroundProps> = ({
	variant = 'orbs',
	orbs = NO_ORBS,
	orbIntensity = 1,
	drift = 0.06,
	speed = 1,
	gridColor = 'volt',
	gridOpacity = 0.3,
	gridSpeed = 2,
	horizon = 0.68,
	vignette = 0.6,
	grain = STAGE.grain,
	seed = 'bg',
	base,
	look,
	children,
}) => {
	const frame = useCurrentFrame();
	const {width, height} = useVideoConfig();
	const t = frame * 0.004 * speed;
	const gc = resolveColor(gridColor);
	const showOrbs = variant !== 'plain';
	// plain = maximum focus: the rig at 70 %, no aurora
	const lookLevel = (look?.level ?? 1) * orbIntensity * (variant === 'plain' ? 0.7 : 1);
	const cell = 110;
	const scroll = (frame * gridSpeed) % cell;

	return (
		<AbsoluteFill style={{backgroundColor: base ?? STAGE.bottom, overflow: 'hidden'}}>
			{base ? null : <StageBase />}
			<StageLights seed={seed} speed={speed} {...look} aurora={variant === 'plain' ? 0 : look?.aurora} level={lookLevel} />

			{showOrbs
				? orbs.map((o, i) => {
						const d = o.size * width;
						const nx = noise2D(`${seed}-ox-${i}`, t, i * 3.1) * drift * width;
						const ny = noise2D(`${seed}-oy-${i}`, t, i * 7.7) * drift * width * 0.7;
						const breathe = 1 + noise2D(`${seed}-os-${i}`, t * 0.7, i) * 0.08;
						const c = resolveColor(o.color);
						const a = o.opacity * orbIntensity;
						return (
							<div
								key={i}
								style={{
									position: 'absolute',
									left: o.x * width + nx - (d * breathe) / 2,
									top: o.y * height + ny - (d * breathe) / 2,
									width: d * breathe,
									height: d * breathe,
									borderRadius: '50%',
									mixBlendMode: 'screen',
									background: `radial-gradient(closest-side, ${alpha(c, a)} 0%, ${alpha(c, a * 0.5)} 35%, ${alpha(c, a * 0.14)} 65%, ${alpha(c, 0)} 100%)`,
								}}
							/>
						);
					})
				: null}

			{variant === 'grid' ? (
				<AbsoluteFill style={{perspective: 700, perspectiveOrigin: `50% ${horizon * 100}%`}}>
					<div
						style={{
							position: 'absolute',
							left: width / 2 - 2400,
							width: 4800,
							height: 2600,
							top: height + 40 - 2600,
							transformOrigin: '50% 100%',
							transform: 'rotateX(82deg)',
							backgroundImage: `linear-gradient(${alpha(gc, gridOpacity)} 1.5px, transparent 1.5px), linear-gradient(90deg, ${alpha(gc, gridOpacity)} 1.5px, transparent 1.5px)`,
							backgroundSize: `${cell}px ${cell}px`,
							backgroundPosition: `0px ${scroll}px`,
							WebkitMaskImage:
								'linear-gradient(to top, rgba(0,0,0,1) 0%, rgba(0,0,0,0.75) 30%, rgba(0,0,0,0.3) 65%, transparent 96%)',
							maskImage:
								'linear-gradient(to top, rgba(0,0,0,1) 0%, rgba(0,0,0,0.75) 30%, rgba(0,0,0,0.3) 65%, transparent 96%)',
						}}
					/>
					{/* horizon glow */}
					<div
						style={{
							position: 'absolute',
							left: 0,
							right: 0,
							top: horizon * height - 220,
							height: 440,
							background: `radial-gradient(ellipse 55% 50% at 50% 50%, ${alpha(gc, 0.22)} 0%, ${alpha(gc, 0.06)} 45%, transparent 75%)`,
						}}
					/>
					<div
						style={{
							position: 'absolute',
							left: width * 0.15,
							right: width * 0.15,
							top: horizon * height - 1,
							height: 2,
							background: `linear-gradient(90deg, transparent, ${alpha(gc, 0.55)}, transparent)`,
						}}
					/>
				</AbsoluteFill>
			) : null}

			<StageGuards guard={look?.guard} level={look?.level} keyPool={look?.keyPool} keyLight={look?.keyLight} />

			{children ? <AbsoluteFill style={{zIndex: layer.content}}>{children}</AbsoluteFill> : null}

			<StageFinish vignette={vignette} grain={grain} seed={seed} zIndex />
		</AbsoluteFill>
	);
};
