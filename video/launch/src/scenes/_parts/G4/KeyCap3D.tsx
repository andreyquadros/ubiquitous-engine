/**
 * The s10 keycap "1" (style S14) as a fully parametric pose, so the same
 * element can be pressed in s10 and fly/flatten into the assign-key-1 chip in
 * s11 (T5 match cut): every visual property is a number or a colour that the
 * scene interpolates.
 *
 * Geometry: a 2.5D key — top face (size × size, radius) and a skirt below it
 * in the same plane, the whole key tilted with its own perspective around the
 * face centre (cx, cy). Pressing moves the face down by `press` inside the
 * plane while the skirt shortens by the same amount (its bottom stays put).
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {alpha, color, font} from '../../../design/tokens';
import {clamp01, mixHex} from './common';

export type KeyPose = {
	/** Screen centre of the top face AT REST (press = 0). */
	cx: number;
	cy: number;
	/** Face side, px (before perspective). */
	size: number;
	rx: number;
	ry: number;
	rz: number;
	perspective: number;
	radius: number;
	/** Visible skirt below the face, px. */
	skirt: number;
	/** Face travel into the well, px. */
	press: number;
	/** Extra uniform scale (mount). */
	scale: number;
	/** Extra screen-space y offset (mount). */
	dy: number;
	legendSize: number;
	/** 0 = ink legend, 1 = volt. */
	legendVolt: number;
	faceTop: string;
	faceBottom: string;
	/** Face border colour (#rrggbb) and alpha. */
	border: string;
	borderAlpha: number;
	borderWidth: number;
	/** Top inner highlight alpha. */
	highlight: number;
	/** Volt underglow 0–1 (0 0 60px rgba(77,141,255,0.55)). */
	underglow: number;
	/** Floor shadow 0–1. */
	shadow: number;
	/** Soft volt ring around the face (lit chip), 0–1. */
	ring: number;
	/** Concave dish shading on the face, 0–1. */
	dish: number;
	opacity: number;
};

export const KEY_REST: KeyPose = {
	cx: 960,
	cy: 560,
	size: 240,
	rx: 28,
	ry: 0,
	rz: -6,
	perspective: 1200,
	radius: 34,
	skirt: 18,
	press: 0,
	scale: 1,
	dy: 0,
	legendSize: 92,
	legendVolt: 0,
	faceTop: '#1c2740',
	faceBottom: '#141c2e',
	border: '#ffffff',
	borderAlpha: 0.1,
	borderWidth: 1,
	highlight: 0.16,
	underglow: 0,
	shadow: 1,
	ring: 0,
	dish: 1,
	opacity: 1,
};

export const KeyCap3D: React.FC<{pose: KeyPose; label?: string}> = ({pose: p, label = '1'}) => {
	if (p.opacity <= 0.001) return null;
	const s = p.size;
	const legendColor = mixHex(color.ink, color.volt, p.legendVolt);
	const bottom = s + p.skirt + p.press; // skirt bottom (face-local), constant while pressing
	return (
		<AbsoluteFill style={{perspective: p.perspective, perspectiveOrigin: `${p.cx}px ${p.cy + p.dy}px`, pointerEvents: 'none', opacity: p.opacity}}>
			<div
				style={{
					position: 'absolute',
					left: p.cx - s / 2,
					top: p.cy + p.dy - s / 2,
					width: s,
					height: s,
					transformOrigin: '50% 50%',
					transform: `rotateX(${p.rx.toFixed(3)}deg) rotateY(${p.ry.toFixed(3)}deg) rotateZ(${p.rz.toFixed(3)}deg) scale(${p.scale.toFixed(4)})`,
				}}
			>
				{/* floor shadow */}
				{p.shadow > 0.001 ? (
					<div
						style={{
							position: 'absolute',
							left: -s * 0.06,
							width: s * 1.12,
							top: s * 0.2,
							height: bottom - s * 0.1,
							borderRadius: p.radius * 1.4,
							background: `rgba(0,0,0,${(0.62 * p.shadow).toFixed(3)})`,
							filter: `blur(${(s * 0.11).toFixed(1)}px)`,
							transform: `translateY(${(s * 0.08).toFixed(1)}px)`,
						}}
					/>
				) : null}
				{/* underglow (contact) */}
				{p.underglow > 0.001 ? (
					<div
						style={{
							position: 'absolute',
							left: 0,
							width: s,
							top: p.press,
							height: bottom - p.press,
							borderRadius: p.radius,
							boxShadow: `0 0 ${(60 * (s / 240)).toFixed(1)}px ${alpha(color.volt, 0.55 * clamp01(p.underglow))}, 0 0 ${(18 * (s / 240)).toFixed(1)}px ${alpha(color.volt, 0.35 * clamp01(p.underglow))}`,
						}}
					/>
				) : null}
				{/* skirt */}
				{p.skirt > 0.05 ? (
					<div
						style={{
							position: 'absolute',
							left: 0,
							width: s,
							top: p.press + s / 2,
							height: bottom - p.press - s / 2,
							borderRadius: `0 0 ${p.radius}px ${p.radius}px`,
							background: 'linear-gradient(180deg, #0c111d 0%, #0c111d 55%, #111829 100%)',
							boxShadow: 'inset 0 -1.5px 0 rgba(255,255,255,0.10), inset 1px 0 0 rgba(255,255,255,0.05), inset -1px 0 0 rgba(255,255,255,0.05), 0 0 0 1px rgba(0,0,0,0.35)',
						}}
					/>
				) : null}
				{/* top face */}
				<div
					style={{
						position: 'absolute',
						left: 0,
						top: p.press,
						width: s,
						height: s,
						borderRadius: p.radius,
						background: `linear-gradient(180deg, ${p.faceTop} 0%, ${p.faceBottom} 100%)`,
						boxShadow: [
							`inset 0 0 0 ${p.borderWidth}px ${alpha(p.border, clamp01(p.borderAlpha))}`,
							p.highlight > 0.001 ? `inset 0 ${Math.max(1, s / 160).toFixed(2)}px 0 rgba(255,255,255,${p.highlight.toFixed(3)})` : '',
							p.ring > 0.001 ? `0 0 ${(s * 0.35).toFixed(1)}px ${alpha(color.volt, 0.35 * p.ring)}` : '',
						]
							.filter(Boolean)
							.join(', '),
						display: 'flex',
						alignItems: 'center',
						justifyContent: 'center',
					}}
				>
					{p.dish > 0.001 ? (
						<div
							style={{
								position: 'absolute',
								inset: s * 0.07,
								borderRadius: p.radius * 0.75,
								background: 'radial-gradient(ellipse 85% 75% at 50% 30%, rgba(255,255,255,0.055), rgba(0,0,0,0.10))',
								opacity: p.dish,
							}}
						/>
					) : null}
					<div
						style={{
							fontFamily: font.text,
							fontWeight: 600,
							fontSize: p.legendSize,
							lineHeight: 1,
							color: legendColor,
							letterSpacing: '-0.01em',
							fontVariantNumeric: 'tabular-nums',
							position: 'relative',
						}}
					>
						{label}
					</div>
				</div>
			</div>
		</AbsoluteFill>
	);
};

const rgbToHex = (rgb: string) => {
	const m = rgb.match(/\d+/g) ?? ['0', '0', '0'];
	return `#${m
		.slice(0, 3)
		.map((v) => Number(v).toString(16).padStart(2, '0'))
		.join('')}`;
};

/** Interpolate two poses (numbers linearly, colours in RGB). */
export const mixPose = (a: KeyPose, b: KeyPose, t: number): KeyPose => {
	const n = (k: keyof KeyPose) => (a[k] as number) + ((b[k] as number) - (a[k] as number)) * t;
	return {
		cx: n('cx'),
		cy: n('cy'),
		size: n('size'),
		rx: n('rx'),
		ry: n('ry'),
		rz: n('rz'),
		perspective: n('perspective'),
		radius: n('radius'),
		skirt: n('skirt'),
		press: n('press'),
		scale: n('scale'),
		dy: n('dy'),
		legendSize: n('legendSize'),
		legendVolt: n('legendVolt'),
		faceTop: mixHex(a.faceTop, b.faceTop, t),
		faceBottom: mixHex(a.faceBottom, b.faceBottom, t),
		border: rgbToHex(mixHex(a.border, b.border, t)),
		borderAlpha: n('borderAlpha'),
		borderWidth: n('borderWidth'),
		highlight: n('highlight'),
		underglow: n('underglow'),
		shadow: n('shadow'),
		ring: n('ring'),
		dish: n('dish'),
		opacity: n('opacity'),
	};
};
