/**
 * s05 drop burst (v2 "the drop has to explode"): everything radiates from the
 * X's spot (the caret, 960 × 520) on the drop frame and is gone by f28, so the
 * settled lockup keeps the GUARD RULE (the lasting light sits behind UBI).
 *
 *  - core bloom: a white-blue (#cfe0ff) radial, 0.55 → 0 over f0–12 (screen);
 *  - two shockwave rings (white then volt, 3 f apart), radius 40 → 1150 (E.push);
 *  - light rays: a repeating conic fan, radially masked, 0.32 → 0 over f0–16;
 *  - 44 sparks: streaks flung out with drag, fading f4–28.
 * Then `Motes`: slow volt dust rising through the hold (secondary motion).
 * Deterministic (remotion `random`), frame-driven.
 */
import React from 'react';
import {AbsoluteFill, random} from 'remotion';
import {E} from '../../../shared';
import {CARET, clamp01, lerp, ramp} from './common';

const C = {x: CARET.x, y: CARET.y};
/** Centre of the settled lockup "ubiqX [AI]" (s05 LOCK_BOX: x 328–1256, y 420–659). */
const LOCK_C = {x: 792, y: 540};

const SPARKS = Array.from({length: 44}, (_, i) => {
	const a = ((i + random(`s05-spark-a-${i}`) * 0.8) / 44) * Math.PI * 2;
	return {
		a,
		v: lerp(34, 78, random(`s05-spark-v-${i}`)), // initial speed px/f
		w: lerp(2, 4, random(`s05-spark-w-${i}`)),
		hot: random(`s05-spark-c-${i}`),
		life: lerp(16, 28, random(`s05-spark-l-${i}`)),
	};
});

/** distance travelled after t frames with exponential drag (time constant 5 f) */
const travel = (v: number, t: number) => v * 5 * (1 - Math.exp(-t / 5));

export const DropBurst: React.FC<{f: number}> = ({f}) => {
	if (f > 30) return null;
	const bloom = 0.55 * (1 - ramp(f, 3, 15, E.enter)); // held through the X pop so the origin stays the brightest point
	const rays = 0.32 * (1 - ramp(f, 0, 16, E.enter));
	const raysScale = lerp(0.7, 1.35, ramp(f, 0, 16, E.push));
	const ring = (delay: number, col: string, width: number) => {
		const p = ramp(f, delay, delay + 20, E.push);
		if (f <= delay || p >= 1) return null; // first visible 1 f after its start, already expanding (no "eye" around the caret on the drop frame)
		const r = lerp(40, 1150, p);
		const o = (1 - p) ** 1.4;
		return (
			<div
				style={{
					position: 'absolute',
					left: C.x - r,
					top: C.y - r * 0.62,
					width: r * 2,
					height: r * 1.24,
					borderRadius: '50%',
					border: `${width}px solid ${col.replace('A', (0.85 * o).toFixed(3))}`,
					boxShadow: `0 0 ${(24 + 30 * p).toFixed(1)}px ${col.replace('A', (0.45 * o).toFixed(3))}, inset 0 0 ${(24 + 30 * p).toFixed(1)}px ${col.replace('A', (0.3 * o).toFixed(3))}`,
				}}
			/>
		);
	};
	// v2 review (minor: the drop read as thin rays): a big volt bloom behind the WHOLE lockup on the downbeat
	// (≈ 900 px radius, 0.4 → 0 over 10 f) and a brief exposure lift of the stage (0.1 → 0 over 6 f), both screen
	const lockBloom = 0.4 * (1 - ramp(f, 0, 10, E.exit));
	const lift = 0.1 * (1 - ramp(f, 0, 6, E.exit));
	return (
		<AbsoluteFill style={{pointerEvents: 'none'}}>
			{lift > 0.003 ? <AbsoluteFill style={{mixBlendMode: 'screen', background: `rgba(120, 165, 255, ${lift.toFixed(3)})`}} /> : null}
			{lockBloom > 0.003 ? (
				<div
					style={{
						position: 'absolute',
						left: LOCK_C.x - 900,
						top: LOCK_C.y - 640,
						width: 1800,
						height: 1280,
						borderRadius: '50%',
						mixBlendMode: 'screen',
						background: `radial-gradient(closest-side, rgba(77,141,255,${lockBloom.toFixed(3)}) 0%, rgba(77,141,255,${(lockBloom * 0.6).toFixed(3)}) 35%, rgba(77,141,255,${(lockBloom * 0.18).toFixed(3)}) 70%, transparent 100%)`,
					}}
				/>
			) : null}
			{bloom > 0.003 ? (
				<div
					style={{
						position: 'absolute',
						left: C.x - 900,
						top: C.y - 700,
						width: 1800,
						height: 1400,
						borderRadius: '50%',
						mixBlendMode: 'screen',
						background: `radial-gradient(closest-side, rgba(207,224,255,${bloom.toFixed(3)}) 0%, rgba(120,170,255,${(bloom * 0.55).toFixed(3)}) 30%, rgba(77,141,255,${(bloom * 0.18).toFixed(3)}) 60%, transparent 100%)`,
					}}
				/>
			) : null}
			{rays > 0.003 ? (
				<div
					style={{
						position: 'absolute',
						left: C.x - 1100,
						top: C.y - 1100,
						width: 2200,
						height: 2200,
						mixBlendMode: 'screen',
						opacity: rays,
						transform: `scale(${raysScale.toFixed(4)}, ${(raysScale * 0.7).toFixed(4)}) rotate(${(f * 0.5).toFixed(2)}deg)`,
						background:
							'repeating-conic-gradient(from 4deg at 50% 50%, rgba(160,195,255,0.9) 0deg, rgba(160,195,255,0) 2.2deg, rgba(160,195,255,0) 13deg, rgba(77,141,255,0.7) 14.5deg, rgba(77,141,255,0) 17deg, rgba(77,141,255,0) 26deg)',
						WebkitMaskImage: 'radial-gradient(closest-side, rgba(0,0,0,0.55) 0%, black 12%, rgba(0,0,0,0.5) 55%, transparent 100%)',
						maskImage: 'radial-gradient(closest-side, rgba(0,0,0,0.55) 0%, black 12%, rgba(0,0,0,0.5) 55%, transparent 100%)',
					}}
				/>
			) : null}
			{ring(0, 'rgba(225,236,255,A)', 3)}
			{ring(2, 'rgba(77,141,255,A)', 2)}
			{SPARKS.map((s, i) => {
				const t = f;
				if (t >= s.life) return null;
				const d = travel(s.v, t) + 30;
				const speed = s.v * Math.exp(-t / 5);
				const len = Math.max(6, speed * 1.6);
				const o = (1 - clamp01((t - 4) / (s.life - 4))) * clamp01(t / 1.5 + 0.35);
				if (o <= 0.01) return null;
				const x = C.x + Math.cos(s.a) * d;
				const y = C.y + Math.sin(s.a) * d * 0.72;
				const col = s.hot > 0.82 ? '255,170,110' : s.hot > 0.4 ? '140,185,255' : '230,238,255';
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: x - len,
							top: y - s.w / 2,
							width: len,
							height: s.w,
							borderRadius: s.w,
							transformOrigin: '100% 50%',
							transform: `rotate(${((Math.atan2(Math.sin(s.a) * 0.72, Math.cos(s.a)) * 180) / Math.PI).toFixed(2)}deg)`,
							background: `linear-gradient(90deg, rgba(${col},0) 0%, rgba(${col},${o.toFixed(3)}) 100%)`,
							boxShadow: `0 0 8px rgba(${col},${(0.6 * o).toFixed(3)})`,
						}}
					/>
				);
			})}
		</AbsoluteFill>
	);
};

const MOTES = Array.from({length: 26}, (_, i) => ({
	x: random(`s05-mote-x-${i}`) * 1920,
	y: 180 + random(`s05-mote-y-${i}`) * 900,
	r: lerp(1.5, 3.6, random(`s05-mote-r-${i}`)),
	v: lerp(0.35, 1.1, random(`s05-mote-v-${i}`)),
	ph: random(`s05-mote-p-${i}`) * Math.PI * 2,
	o: lerp(0.25, 0.6, random(`s05-mote-o-${i}`)),
}));

/** Slow volt dust rising through the hold; fades in after the burst, out for the match cut. */
export const Motes: React.FC<{f: number; presence: number}> = ({f, presence}) => {
	if (presence <= 0.003) return null;
	return (
		<AbsoluteFill style={{pointerEvents: 'none'}}>
			{MOTES.map((m, i) => {
				const y = m.y - m.v * f;
				const x = m.x + Math.sin(f * 0.045 + m.ph) * 14;
				const tw = 0.6 + 0.4 * Math.sin(f * 0.12 + m.ph * 3);
				const o = m.o * tw * presence;
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: x - m.r,
							top: y - m.r,
							width: m.r * 2,
							height: m.r * 2,
							borderRadius: '50%',
							background: `rgba(160,195,255,${o.toFixed(3)})`,
							boxShadow: `0 0 ${(m.r * 4).toFixed(1)}px rgba(77,141,255,${(o * 0.8).toFixed(3)})`,
						}}
					/>
				);
			})}
		</AbsoluteFill>
	);
};
