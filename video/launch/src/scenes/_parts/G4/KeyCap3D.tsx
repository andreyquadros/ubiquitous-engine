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
	/** 0 = rest legend colour (`legendInk`), 1 = lit (`legendLit`). */
	legendVolt: number;
	/** Rest / lit legend colours (#rrggbb). */
	legendInk: string;
	legendLit: string;
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
	/** Skirt gradient (#rrggbb, top → bottom). */
	skirtTop: string;
	skirtBottom: string;
	/** Volt back-rim on the skirt edge + face outline glow, 0–1 (v2 key light). */
	rim: number;
	/** Contact shockwave progress 0–1 (0 = none): two rings expand in the key plane. */
	shock: number;
	/** Volt/white bounce light on the floor under the key, 0–1. */
	floorGlow: number;
	opacity: number;
};

/**
 * v2 hero keycap (brief v2-scenes G4 s10): a silver-white key, 440 px across, lit by a cool key light
 * from the top-left with a volt back-rim; the navy "1" latches volt on contact.
 */
export const KEY_REST: KeyPose = {
	cx: 960,
	cy: 520,
	size: 440,
	rx: 28,
	ry: 0,
	rz: -6,
	perspective: 1500,
	radius: 64,
	skirt: 34,
	press: 0,
	scale: 1,
	dy: 0,
	legendSize: 188,
	legendVolt: 0,
	legendInk: '#18233d',
	legendLit: '#2f72f0',
	faceTop: '#f4f7fd',
	faceBottom: '#cfd8e8',
	border: '#ffffff',
	borderAlpha: 0.75,
	borderWidth: 1.5,
	highlight: 0.95,
	underglow: 0,
	shadow: 1,
	ring: 0,
	dish: 1,
	skirtTop: '#aeb9cd',
	skirtBottom: '#7d89a1',
	rim: 1,
	shock: 0,
	floorGlow: 0.55,
	opacity: 1,
};

export const KeyCap3D: React.FC<{pose: KeyPose; label?: string}> = ({pose: p, label = '1'}) => {
	if (p.opacity <= 0.001) return null;
	const s = p.size;
	const u = s / 240; // v1 unit (the 240-px key)
	const legendColor = mixHex(p.legendInk, p.legendLit, p.legendVolt);
	const bottom = s + p.skirt + p.press; // skirt bottom (face-local), constant while pressing
	const lit = clamp01(p.legendVolt);
	// shockwave: two rings in the key plane (ring 2 trails by 20 %)
	const rings =
		p.shock > 0.001 && p.shock < 0.999
			? [0, 0.2].map((lag, i) => {
					const t = clamp01((p.shock - lag) / (1 - lag));
					if (t <= 0 || t >= 1) return null;
					const e = 1 - Math.pow(1 - t, 3);
					const grow = 1 + (i === 0 ? 1.35 : 0.95) * e;
					const a = (i === 0 ? 0.95 : 0.6) * Math.pow(1 - t, 1.4);
					const bw = (i === 0 ? 7 : 4) * u * (1 - 0.75 * t);
					return (
						<div
							key={i}
							style={{
								position: 'absolute',
								left: s / 2 - (s * grow) / 2,
								top: p.press + s / 2 - (s * grow) / 2,
								width: s * grow,
								height: s * grow,
								borderRadius: p.radius * grow * 1.2,
								border: `${bw.toFixed(2)}px solid ${alpha(i === 0 ? '#cfe0ff' : color.volt, a)}`,
								boxShadow: `0 0 ${(28 * u).toFixed(1)}px ${alpha(color.volt, 0.7 * a)}, inset 0 0 ${(22 * u).toFixed(1)}px ${alpha(color.volt, 0.45 * a)}`,
							}}
						/>
					);
				})
			: null;
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
				{/* floor bounce light: a soft cool pool the key sits in (brighter with the volt contact) */}
				{p.floorGlow > 0.001 ? (
					<div
						style={{
							position: 'absolute',
							left: -s * 0.55,
							width: s * 2.1,
							top: -s * 0.25,
							height: bottom + s * 0.75,
							borderRadius: '50%',
							background: `radial-gradient(closest-side, ${alpha(rgbToHex(mixHex('#cfe0ff', color.volt, 0.35 + 0.65 * lit)), 0.34 * p.floorGlow)} 0%, ${alpha(color.volt, 0.16 * p.floorGlow)} 45%, ${alpha(color.volt, 0)} 100%)`,
						}}
					/>
				) : null}
				{rings}
				{/* contact shadow */}
				{p.shadow > 0.001 ? (
					<div
						style={{
							position: 'absolute',
							left: -s * 0.04,
							width: s * 1.08,
							top: s * 0.22,
							height: bottom - s * 0.14,
							borderRadius: p.radius * 1.3,
							background: `rgba(2,5,14,${(0.7 * p.shadow).toFixed(3)})`,
							filter: `blur(${(s * 0.07).toFixed(1)}px)`,
							transform: `translateY(${(s * 0.07).toFixed(1)}px)`,
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
							boxShadow: `0 0 ${(70 * u).toFixed(1)}px ${(10 * u).toFixed(1)}px ${alpha(color.volt, 0.75 * clamp01(p.underglow))}, 0 0 ${(20 * u).toFixed(1)}px ${alpha('#cfe0ff', 0.6 * clamp01(p.underglow))}`,
						}}
					/>
				) : null}
				{/* skirt (+ volt back-rim on its lower edge) */}
				{p.skirt > 0.05 ? (
					<div
						style={{
							position: 'absolute',
							left: 0,
							width: s,
							top: p.press + s / 2,
							height: bottom - p.press - s / 2,
							borderRadius: `0 0 ${p.radius}px ${p.radius}px`,
							background: `linear-gradient(180deg, ${p.skirtTop} 0%, ${p.skirtTop} 55%, ${p.skirtBottom} 100%)`,
							boxShadow: [
								`inset 0 ${(-2 * u).toFixed(2)}px 0 ${alpha(color.volt, 0.85 * p.rim)}`,
								`0 ${(3 * u).toFixed(2)}px ${(14 * u).toFixed(1)}px ${alpha(color.volt, 0.45 * p.rim)}`,
								'inset 1px 0 0 rgba(255,255,255,0.18), inset -1px 0 0 rgba(255,255,255,0.10)',
								'0 0 0 1px rgba(0,0,0,0.25)',
							].join(', '),
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
						background: `linear-gradient(165deg, ${p.faceTop} 0%, ${p.faceBottom} 100%)`,
						boxShadow: [
							`inset 0 0 0 ${p.borderWidth}px ${alpha(p.border, clamp01(p.borderAlpha))}`,
							p.highlight > 0.001 ? `inset 0 ${Math.max(1, 2.2 * u).toFixed(2)}px 0 rgba(255,255,255,${p.highlight.toFixed(3)})` : '',
							p.ring > 0.001 ? `0 0 ${(s * 0.35).toFixed(1)}px ${alpha(color.volt, 0.35 * p.ring)}` : '',
							p.rim > 0.001 ? `0 0 ${(26 * u).toFixed(1)}px ${alpha('#cfe0ff', 0.28 * p.rim)}` : '',
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
								background: 'radial-gradient(ellipse 90% 80% at 42% 28%, rgba(255,255,255,0.55), rgba(255,255,255,0.0) 55%, rgba(24,36,70,0.14) 100%)',
								boxShadow: `inset 0 ${(3 * u).toFixed(1)}px ${(8 * u).toFixed(1)}px rgba(24,36,70,0.10), inset 0 ${(-2 * u).toFixed(1)}px ${(4 * u).toFixed(1)}px rgba(255,255,255,0.5)`,
								opacity: p.dish,
							}}
						/>
					) : null}
					{/* the face takes a faint volt tint when the key is lit */}
					{lit > 0.001 && p.dish > 0.001 ? (
						<div style={{position: 'absolute', inset: 0, borderRadius: p.radius, background: `radial-gradient(closest-side, ${alpha('#dbe7ff', 0.5 * lit * p.dish)}, ${alpha('#dbe7ff', 0)})`}} />
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
							textShadow: lit > 0.001 ? `0 0 ${(22 * u).toFixed(1)}px ${alpha(color.volt, 0.55 * lit * clamp01(p.dish + p.ring))}` : undefined,
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
		legendInk: rgbToHex(mixHex(a.legendInk, b.legendInk, t)),
		legendLit: rgbToHex(mixHex(a.legendLit, b.legendLit, t)),
		faceTop: rgbToHex(mixHex(a.faceTop, b.faceTop, t)),
		faceBottom: rgbToHex(mixHex(a.faceBottom, b.faceBottom, t)),
		border: rgbToHex(mixHex(a.border, b.border, t)),
		borderAlpha: n('borderAlpha'),
		borderWidth: n('borderWidth'),
		highlight: n('highlight'),
		underglow: n('underglow'),
		shadow: n('shadow'),
		ring: n('ring'),
		dish: n('dish'),
		skirtTop: rgbToHex(mixHex(a.skirtTop, b.skirtTop, t)),
		skirtBottom: rgbToHex(mixHex(a.skirtBottom, b.skirtBottom, t)),
		rim: n('rim'),
		shock: n('shock'),
		floorGlow: n('floorGlow'),
		opacity: n('opacity'),
	};
};
