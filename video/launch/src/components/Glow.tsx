import React from 'react';
import {AbsoluteFill, useCurrentFrame} from 'remotion';
import {alpha, BAR, resolveColor, type Accent} from '../design/tokens';
import {kf, oscillate, type Keyframed} from '../design/motion';

export type GlowProps = {
	/** Centre X in composition px (keyframeable). Default: canvas centre (960). */
	x?: Keyframed;
	/** Centre Y in composition px (keyframeable). Default: canvas centre (540). */
	y?: Keyframed;
	/** Diameter in px (keyframeable). Default 900. */
	size?: Keyframed;
	/** Palette accent or any CSS colour. Default volt. */
	color?: Accent | string;
	/** Peak alpha of the centre, 0–1 (keyframeable). Default 0.35. */
	intensity?: Keyframed;
	/** Horizontal stretch (1 = circle, 2 = wide ellipse). Default 1. */
	aspect?: number;
	/** Pulse on the beat: amplitude 0–1 of intensity modulation. Default 0. */
	pulse?: number;
	/** Pulse period in frames. Default one bar (60). */
	pulsePeriod?: number;
	/** Blend mode against what's below. Default "screen". */
	blend?: React.CSSProperties['mixBlendMode'];
};

/**
 * Soft radial light blob — use behind a hero word, under a window, or as a
 * pulsing accent on the beat. Pure CSS gradient (no filter), so it's cheap.
 */
export const Glow: React.FC<GlowProps> = ({
	x = 960,
	y = 540,
	size = 900,
	color = 'volt',
	intensity = 0.35,
	aspect = 1,
	pulse = 0,
	pulsePeriod = BAR,
	blend = 'screen',
}) => {
	const frame = useCurrentFrame();
	const c = resolveColor(color);
	const d = kf(size, frame);
	const a = Math.max(0, kf(intensity, frame) * (1 + (pulse ? oscillate(frame, pulsePeriod, pulse) : 0)));
	const w = d * aspect;
	return (
		<AbsoluteFill style={{pointerEvents: 'none', mixBlendMode: blend}}>
			<div
				style={{
					position: 'absolute',
					left: kf(x, frame) - w / 2,
					top: kf(y, frame) - d / 2,
					width: w,
					height: d,
					borderRadius: '50%',
					background: `radial-gradient(closest-side, ${alpha(c, a)} 0%, ${alpha(c, a * 0.45)} 38%, ${alpha(c, a * 0.12)} 66%, ${alpha(c, 0)} 100%)`,
				}}
			/>
		</AbsoluteFill>
	);
};
