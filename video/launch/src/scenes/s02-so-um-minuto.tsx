/**
 * s02-so-um-minuto — abs 90–179 (90 f) · problem
 *
 * v2. Layer A (texture): a flurry of generic, unbranded window-title chips
 * (44 px text, lit glass pills, coloured generic app squares) in six depth
 * rows streaking right→left with horizontal motion blur, re-laid out on 8ths
 * (f0, 8, 30, 38, 45, 53), each set with its own linear push 1.00→1.06 and
 * ±2° tilt. The two `near` rows fill the middle of f0–14 and are blown apart
 * on the contact; the four outer rows (y 100 / 226 / 860 / 990) frame the type
 * at 50 %. 38 px/f until the slam, then 8 px/f, then 4 px/f from the landing.
 * Layer B: the slam "“Só um minutinho.”" 136 px (mount f11 at 1.45 / blur 12,
 * SLAM contact f15 + 6 px shake). Layer C: the counter 1→40 at 300 px, "min"
 * 135 px (mount f29, counts f30→60, lands f60: pulse, ink→rose, underline,
 * a rose floor light blooms under it and breathes). Hard cuts in and out.
 */
import React from 'react';
import {AbsoluteFill, Easing, interpolate, random} from 'remotion';
import {alpha, color, font} from '../design/tokens';
import {E, SceneTransitions, springAt, useScene, type SfxCue} from '../shared';
import {Backdrop, boxTopForCap, CLAMP, G1_ORBS, inkGradient, lerp, ramp, shakeTransform, slamShake, SweepBar, useFrame} from './_parts/G1/common';

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
	'Chamada perdida',
	'Lembrete: responder',
	'Atualização disponível',
	'Rascunho — e-mail',
];
/** Generic app squares (no brands): muted accents. */
const HUES = ['#7d8bab', '#e0a73a', '#3fbf8c', '#e2656a', '#5aa9e6', '#9aa6c2'];
const SHOTS = [0, 8, 30, 38, 45, 53];
/**
 * v2: six depth rows. The two `near` rows fill the middle of the frame before
 * the slam (f0–14: no empty frames) and are blown away on the contact; the
 * four outer rows frame the type afterwards.
 */
const ROWS = [
	{y: 100, s: 0.82, near: false},
	{y: 226, s: 1, near: false},
	{y: 420, s: 1.2, near: true},
	{y: 640, s: 1.3, near: true},
	{y: 860, s: 1, near: false},
	{y: 990, s: 0.84, near: false},
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

const CHIP = {h: 82, text: 44, square: 32}; // far rows (×0.82–0.84) still ≥ 36 px
const chipWidth = (t: string) => Math.round(t.length * CHIP.text * 0.52 + 22 + CHIP.square + 16 + 32);

type ChipPlacement = {title: string; hue: string; x: number; row: number};

const layout = (shot: number): ChipPlacement[] => {
	const out: ChipPlacement[] = [];
	ROWS.forEach((r, ri) => {
		let x = -320 + random(`s02-${shot}-${ri}-start`) * 300;
		for (let i = 0; i < 5; i++) {
			const title = TITLES[Math.floor(random(`s02-${shot}-${ri}-${i}-t`) * TITLES.length)];
			const hue = HUES[Math.floor(random(`s02-${shot}-${ri}-${i}-h`) * HUES.length)];
			out.push({title, hue, x, row: ri});
			x += (chipWidth(title) + 70 + random(`s02-${shot}-${ri}-${i}-g`) * 150) * r.s;
		}
	});
	return out;
};

const Chip: React.FC<{title: string; hue: string}> = ({title, hue}) => (
	<div
		style={{
			display: 'inline-flex',
			alignItems: 'center',
			gap: 16,
			height: CHIP.h,
			padding: '0 32px 0 22px',
			borderRadius: 999,
			background: 'linear-gradient(180deg, rgba(196,210,240,0.26) 0%, rgba(150,168,210,0.16) 100%)',
			border: '1.5px solid rgba(220,230,255,0.26)',
			boxShadow: '0 1.5px 0 rgba(255,255,255,0.18) inset, 0 14px 30px -10px rgba(2,5,14,0.7)',
			whiteSpace: 'nowrap',
		}}
	>
		<div style={{width: CHIP.square, height: CHIP.square, borderRadius: 8, background: hue, boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.3)'}} />
		<span style={{fontFamily: font.text, fontWeight: 600, fontSize: CHIP.text, color: color.ink}}>{title}</span>
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
	const settle = ramp(frame, CONTACT, CONTACT + 6, E.enter);
	// outer rows step back to 50 %; the near rows are blown out on the contact
	const outer = lerp(1, 0.5, settle);
	const blown = ramp(frame, CONTACT - 4, CONTACT, E.exit); // gone by the contact: the slam lands on a clean middle
	const v = speed(frame);
	const streak = 2 + v * 0.22; // horizontal smear grows with speed; 2 px base blur
	return (
		<AbsoluteFill>
			<svg width={0} height={0} style={{position: 'absolute'}}>
				<filter id="s02-streak" x="-10%" y="-20%" width="120%" height="140%" colorInterpolationFilters="sRGB">
					<feGaussianBlur stdDeviation={`${streak.toFixed(2)} 2`} />
				</filter>
			</svg>
			<AbsoluteFill
				style={{
					transform: `scale(${push}) rotate(${rot}deg)`,
					filter: 'saturate(0.8) url(#s02-streak)',
				}}
			>
				{chips.map((c, i) => {
					const r = ROWS[c.row];
					const o = r.near ? 1 - blown : outer;
					if (o <= 0.001) return null;
					// near rows: pushed apart (up / down) and scaled as they blow out
					const push = r.near ? (r.y < 540 ? -1 : 1) * 160 * blown : 0;
					const k = r.s * (r.near ? 1 + 0.25 * blown : 1);
					return (
						<div
							key={i}
							style={{
								position: 'absolute',
								left: 0,
								top: 0,
								opacity: o,
								transformOrigin: '0 50%',
								transform: `translate(${(c.x - travelled * r.s).toFixed(2)}px, ${(r.y - CHIP.h / 2 + push).toFixed(2)}px) scale(${k.toFixed(4)})`,
							}}
						>
							<Chip title={c.title} hue={c.hue} />
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

const HEAD = {size: 136, capTop: 300};
const NUM = {size: 300, capTop: 470};
const MIN_K = 0.45; // "min" = 135 px

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
	const rose = ramp(frame, LAND - 1, LAND + 2);
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
							<span style={inkGradient}>{headline.slice(0, at)}</span>
							<span style={{color: color.rose, textShadow: `0 0 34px ${alpha(color.rose, 0.45)}`}}>{emph}</span>
							<span style={inkGradient}>{headline.slice(at + emph.length)}</span>
						</div>
					) : null}
				</AbsoluteFill>

				{/* the truth lands: a rose floor light blooms under the counter on f60, then breathes */}
				{frame >= LAND - 1 ? (
					<div
						style={{
							position: 'absolute',
							left: 960 - 760,
							top: 700,
							width: 1520,
							height: 420,
							borderRadius: '50%',
							opacity: (0.75 + 0.25 * Math.cos((frame - LAND) / 6)) * ramp(frame, LAND - 1, LAND + 3),
							background: `radial-gradient(closest-side, ${alpha(color.rose, 0.3)} 0%, ${alpha(color.rose, 0.12)} 50%, ${alpha(color.rose, 0)} 100%)`,
						}}
					/>
				) : null}

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
						<span
							style={{
								display: 'inline-grid',
								fontVariantNumeric: 'tabular-nums',
								letterSpacing: '-0.04em',
								color: numColor,
								textShadow: `0 0 ${(40 * rose).toFixed(1)}px ${alpha(color.rose, 0.5 * rose)}`,
							}}
						>
							<span style={{gridArea: '1 / 1', visibility: 'hidden'}}>40</span>
							<span style={{gridArea: '1 / 1', justifySelf: 'end'}}>{value}</span>
						</span>
						<span style={{fontSize: NUM.size * MIN_K, letterSpacing: '-0.02em', color: color.ink2}}>
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
