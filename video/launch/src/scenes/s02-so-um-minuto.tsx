/**
 * s02-so-um-minuto — abs 90–179 (90 f) · problem
 *
 * Layer A (texture, never read): 12 generic window-title chips in three depth
 * rows (y 170 / 300 / 900, scale 0.8 / 1 / 0.9) streaking right→left, re-laid
 * out on 8ths (f0, 8, 30, 38, 45, 53) — each set with its own linear push
 * 1.00→1.06 and ±2° tilt. 38 px/f until the slam, then 8 px/f and 40 %, then
 * 4 px/f from the landing. Layer B: the slam "“Só um minutinho.”" (mount f11 at
 * 1.45 / blur 12, SLAM contact f15 + 6 px shake). Layer C: the counter 1→40
 * (mount f29, counts f30→60, lands f60: pulse, ink→rose, underline under
 * "min"). Everything on the dark canvas. Hard cuts in and out.
 */
import React from 'react';
import {AbsoluteFill, Easing, interpolate, random} from 'remotion';
import {color, font} from '../design/tokens';
import {E, SceneTransitions, springAt, useScene, type SfxCue} from '../shared';
import {Backdrop, boxTopForCap, CLAMP, G1_ORBS, lerp, ramp, shakeTransform, slamShake, SweepBar, useFrame} from './_parts/G1/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'glitch_1.wav', atFrame: 0, gainDb: -16, note: 'Flash re-layout 1.'},
	{ref: 'glitch_3.wav', atFrame: 8, gainDb: -16, note: 'Flash re-layout 2 (8th).'},
	{ref: 'impact.wav', atFrame: 15, gainDb: 0, note: 'Slam contact; bed duck −5 dB.'},
	{ref: 'ui_tick_2.wav', atFrame: 33, gainDb: -22},
	{ref: 'ui_tick_2.wav', atFrame: 36, gainDb: -22},
	{ref: 'ui_tick_2.wav', atFrame: 39, gainDb: -22},
	{ref: 'ui_tick_2.wav', atFrame: 42, gainDb: -22},
	{ref: 'ui_tick_2.wav', atFrame: 45, gainDb: -22},
	{ref: 'ui_tick_2.wav', atFrame: 48, gainDb: -22},
	{ref: 'ui_tick_2.wav', atFrame: 51, gainDb: -22},
	{ref: 'ui_tick_2.wav', atFrame: 54, gainDb: -22},
	{ref: 'ui_tick_2.wav', atFrame: 57, gainDb: -22},
	{ref: 'impact_soft_3.wav', atFrame: 60, gainDb: -2, note: 'Counter lands on 40 min.'},
];

/* ------------------------------------------------------------------------ */
/* Chips                                                                     */
/* ------------------------------------------------------------------------ */

const TITLES = [
	'Nova aba',
	'Re: Re: Fwd: reunião',
	'Planilha de horas — set',
	'(12) Caixa de entrada',
	'Proposta_v3_final.pdf',
	'Sem título — Documento',
];
const SHOTS = [0, 8, 30, 38, 45, 53];
const ROWS = [
	{y: 170, s: 0.8},
	{y: 300, s: 1},
	{y: 900, s: 0.9},
];
const SLAM_IN = 11;
const CONTACT = 15;
const COUNT_FROM = 30;
const LAND = 60;

/** Chip speed (px/f at row scale 1). */
const speed = (f: number) => {
	if (f < CONTACT) return 38;
	if (f < LAND) return lerp(38, 8, interpolate(f, [CONTACT, CONTACT + 6], [0, 1], {...CLAMP, easing: E.enter}));
	return lerp(8, 4, interpolate(f, [LAND, LAND + 6], [0, 1], {...CLAMP, easing: E.enter}));
};
/** Distance travelled from f0 to f (sum of per-frame speeds). */
const dist = (f: number) => {
	let d = 0;
	for (let i = 0; i < f; i++) d += speed(i);
	return d;
};

const chipWidth = (t: string) => Math.round(t.length * 30 * 0.53 + 26 * 2 + 18 + 14);

type ChipPlacement = {title: string; x: number; row: number};

const layout = (shot: number): ChipPlacement[] => {
	const out: ChipPlacement[] = [];
	ROWS.forEach((r, ri) => {
		let x = -260 + random(`s02-${shot}-${ri}-start`) * 260;
		for (let i = 0; i < 4; i++) {
			const title = TITLES[Math.floor(random(`s02-${shot}-${ri}-${i}-t`) * TITLES.length)];
			out.push({title, x, row: ri});
			x += (chipWidth(title) + 90 + random(`s02-${shot}-${ri}-${i}-g`) * 170) * r.s;
		}
	});
	return out;
};

const Chip: React.FC<{title: string}> = ({title}) => (
	<div
		style={{
			display: 'inline-flex',
			alignItems: 'center',
			gap: 14,
			height: 60,
			padding: '0 26px',
			borderRadius: 999,
			background: color.panel2,
			border: '1px solid rgba(255,255,255,0.08)',
			boxShadow: '0 1px 0 rgba(255,255,255,0.05) inset, 0 8px 20px -6px rgba(0,0,0,0.55)',
			whiteSpace: 'nowrap',
		}}
	>
		<div style={{width: 18, height: 18, borderRadius: 5, background: '#3a4560'}} />
		<span style={{fontFamily: font.text, fontWeight: 500, fontSize: 30, color: color.ink2, opacity: 0.7}}>{title}</span>
	</div>
);

const ChipLayer: React.FC<{frame: number}> = ({frame}) => {
	const shotIdx = SHOTS.filter((s) => s <= frame).length - 1;
	const start = SHOTS[shotIdx];
	const end = SHOTS[shotIdx + 1] ?? 90;
	const chips = layout(shotIdx);
	const travelled = dist(frame) - dist(start);
	const push = lerp(1, 1.06, interpolate(frame, [start, end], [0, 1], CLAMP));
	const rot = (random(`s02-rot-${shotIdx}`) * 2 - 1) * 2;
	const dim = lerp(1, 0.4, ramp(frame, CONTACT, CONTACT + 6, E.enter));
	const v = speed(frame);
	const streak = 2 + v * 0.22; // horizontal smear grows with speed; 2 px base blur
	return (
		<AbsoluteFill style={{opacity: dim}}>
			<svg width={0} height={0} style={{position: 'absolute'}}>
				<filter id="s02-streak" x="-10%" y="-20%" width="120%" height="140%" colorInterpolationFilters="sRGB">
					<feGaussianBlur stdDeviation={`${streak.toFixed(2)} 2`} />
				</filter>
			</svg>
			<AbsoluteFill
				style={{
					transform: `scale(${push}) rotate(${rot}deg)`,
					filter: 'saturate(0.7) url(#s02-streak)',
				}}
			>
				{chips.map((c, i) => {
					const r = ROWS[c.row];
					return (
						<div
							key={i}
							style={{
								position: 'absolute',
								left: 0,
								top: 0,
								transformOrigin: '0 50%',
								transform: `translate(${(c.x - travelled * r.s).toFixed(2)}px, ${r.y - 30}px) scale(${r.s})`,
							}}
						>
							<Chip title={c.title} />
						</div>
					);
				})}
			</AbsoluteFill>
		</AbsoluteFill>
	);
};

/* ------------------------------------------------------------------------ */
/* Scene                                                                     */
/* ------------------------------------------------------------------------ */

const HEAD = {size: 128, capTop: 330};
const NUM = {size: 240, capTop: 560};

const S02SoUmMinuto: React.FC = () => {
	const scene = useScene();
	const {frame} = useFrame();
	const headline = scene.copy[0].text; // “Só um minutinho.”
	const emph = scene.copy[0].emphasis[0] ?? 'minutinho';
	const at = headline.indexOf(emph);

	// canvas camera 1.00 → 1.02 (linear) on bg + chips; type is pinned
	const camZoom = lerp(1, 1.02, frame / 89);
	const shake = slamShake(frame, CONTACT, 's02');

	// slam
	const sp = springAt(frame, SLAM_IN, 'SLAM');
	let slamScale = 1.45 - 0.45 * sp;
	if (frame > CONTACT + 8 && Math.abs(slamScale - 1) < 0.002) slamScale = 1;
	const slamOpacity = interpolate(frame, [SLAM_IN - 1, SLAM_IN + 1], [0, 1], CLAMP);
	const slamBlur = interpolate(frame, [SLAM_IN, SLAM_IN + 4], [12, 0], CLAMP);
	const tracking = lerp(-0.01, -0.04, Math.min(1, sp));

	// counter
	const shown = frame >= COUNT_FROM - 1;
	const value = Math.round(
		interpolate(frame, [COUNT_FROM, LAND], [1, 40], {...CLAMP, easing: Easing.bezier(0.25, 0.1, 0.25, 1)}),
	);
	const cIn = springAt(frame, COUNT_FROM - 1, 'SNAPPY');
	const cY = frame >= COUNT_FROM + 14 ? 0 : 24 * (1 - cIn);
	const cOpacity = interpolate(frame, [COUNT_FROM - 1, COUNT_FROM + 2], [0, 1], CLAMP);
	let pulse = 1;
	if (frame >= LAND && frame < LAND + 3) pulse = 1 + 0.05 * E.push((frame - LAND) / 3);
	else if (frame >= LAND + 3) {
		pulse = 1 + 0.05 * (1 - springAt(frame, LAND + 3, 'BOUNCY_SUBTLE'));
		if (Math.abs(pulse - 1) < 0.0015 && frame > LAND + 12) pulse = 1;
	}
	const rose = ramp(frame, LAND, LAND + 3);
	const numColor = mix(color.ink, color.rose, rose);
	const underline = ramp(frame, LAND + 2, LAND + 14, E.glide);

	return (
		<SceneTransitions>
			<Backdrop seed="s02" orbs={[{...G1_ORBS[0], opacity: 0.08}, G1_ORBS[1]]}>
				<AbsoluteFill style={{transform: shakeTransform(shake)}}>
					<AbsoluteFill style={{transform: `scale(${camZoom})`}}>
						<ChipLayer frame={frame} />
					</AbsoluteFill>

					{/* B: the slam */}
					{frame >= SLAM_IN ? (
						<div
							style={{
								position: 'absolute',
								left: 0,
								width: 1920,
								top: Math.round(boxTopForCap('display', HEAD.size, HEAD.capTop)),
								textAlign: 'center',
								fontFamily: font.display,
								fontWeight: 800,
								fontSize: HEAD.size,
								lineHeight: 1,
								letterSpacing: `${tracking}em`,
								color: color.ink,
								whiteSpace: 'nowrap',
								opacity: slamOpacity,
								filter: slamBlur > 0.1 ? `blur(${slamBlur}px)` : undefined,
								transform: slamScale === 1 ? undefined : `scale(${slamScale})`,
								transformOrigin: '50% 50%',
							}}
						>
							{headline.slice(0, at)}
							<span style={{color: color.rose}}>{emph}</span>
							{headline.slice(at + emph.length)}
						</div>
					) : null}
				</AbsoluteFill>

				{/* C: the counter */}
				{shown ? (
					<div
						style={{
							position: 'absolute',
							left: 0,
							width: 1920,
							top: Math.round(boxTopForCap('display', NUM.size, NUM.capTop)),
							textAlign: 'center',
							fontFamily: font.display,
							fontWeight: 700,
							fontSize: NUM.size,
							lineHeight: 1,
							whiteSpace: 'nowrap',
							opacity: cOpacity,
							transform: `translateY(${cY.toFixed(2)}px)${pulse === 1 ? '' : ` scale(${pulse})`}`,
							transformOrigin: '50% 60%',
						}}
					>
						<span style={{display: 'inline-grid', fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.04em', color: numColor}}>
							<span style={{gridArea: '1 / 1', visibility: 'hidden'}}>40</span>
							<span style={{gridArea: '1 / 1', justifySelf: 'end'}}>{value}</span>
						</span>
						<span style={{fontSize: NUM.size * 0.5, letterSpacing: '-0.02em', color: color.ink2}}>
							{' '}
							<span style={{position: 'relative', display: 'inline-block', lineHeight: 1}}>
								min
								<SweepBar progress={underline} color={color.rose} topEm={0.98} thicknessEm={0.08} />
							</span>
						</span>
					</div>
				) : null}
			</Backdrop>
		</SceneTransitions>
	);
};

const mix = (a: string, b: string, t: number) => {
	const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
	const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
	const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * t));
	return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
};

export default S02SoUmMinuto;
