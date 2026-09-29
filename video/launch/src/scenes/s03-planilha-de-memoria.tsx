/**
 * s03-planilha-de-memoria — abs 180–224 (45 f) · problem
 *
 * f0 (downbeat) hard cut INTO the slam contact: "Chutar as horas?" at full
 * opacity, scale 1.06→1 (SLAM) + 6 px shake. The s01 timesheet is back, flat
 * (rx 6°). Rose "?" guesses pop on 8ths (f8 TER/11h, f15 SEG/15h, f23 QUI/09h,
 * f30 TER/14h; SNAPPY, no bounce). f20–30 a rose strike-through crosses out
 * "Chutar". The camera converges on the volt caret in TER/10h (E.glide pan +
 * linear push 1.00→1.03 around the caret) so that on f44 the caret sits
 * exactly on (960, 520) at 4×56 px — the spot where s04 keeps it and s05's X
 * pops out of it. Caret blinks, then is solid f30–44. Hard cut at 45.
 */
import React from 'react';
import {AbsoluteFill, interpolate} from 'remotion';
import {color, font} from '../design/tokens';
import {E, SceneTransitions, springAt, useScene, type SfxCue} from '../shared';
import {
	Backdrop,
	boxTopForCap,
	Caret,
	CARET_SPOT,
	camTransform,
	cellCenter,
	CLAMP,
	lerp,
	project,
	ramp,
	shakeTransform,
	slamShake,
	SweepBar,
	TER,
	TimesheetPlane,
	useFrame,
	type Cam2D,
	type Mark,
	type PlanePose,
} from './_parts/G1/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'impact_soft_2.wav', atFrame: 0, gainDb: 0, note: 'Slam contact; bed duck −5 dB.'},
	{ref: 'ui_tick_1.wav', atFrame: 8, gainDb: -22, note: '“?” pops.'},
	{ref: 'ui_tick_1.wav', atFrame: 15, gainDb: -22, note: '“?” pops.'},
	{ref: 'ui_tick_1.wav', atFrame: 23, gainDb: -22, note: '“?” pops.'},
	{ref: 'ui_tick_1.wav', atFrame: 30, gainDb: -22, note: '“?” pops.'},
];

const POSE: PlanePose = {cx: 960, cy: 680, rx: 6, rz: 0, s: 1};
const LAST = 44;
/** The caret's cell, projected with the camera at rest. */
const CARET_F = (() => {
	const c = cellCenter(TER, 1);
	return project(POSE, c.x, c.y);
})();

const camAt = (f: number): Cam2D => {
	const e = interpolate(f, [0, LAST], [0, 1], {...CLAMP, easing: E.glide});
	const t = interpolate(f, [0, LAST], [0, 1], CLAMP);
	return {
		fx: CARET_F.x,
		fy: CARET_F.y,
		ax: lerp(CARET_F.x, CARET_SPOT.x, e),
		ay: lerp(CARET_F.y, CARET_SPOT.y, e),
		zoom: Math.exp(lerp(0, Math.log(1.03), t)),
	};
};

const GUESSES: {col: number; row: number; at: number}[] = [
	{col: TER, row: 2, at: 8}, // TER/11h
	{col: 0, row: 6, at: 15}, // SEG/15h
	{col: 3, row: 0, at: 23}, // QUI/09h
	{col: TER, row: 5, at: 30}, // TER/14h
];

const HEAD = {size: 120, capTop: 170};

/** Caret: 8 on / 8 off from f0, off f24–29, solid f30–44. */
const caretOn = (f: number) => (f >= 30 ? true : f >= 24 ? false : f % 16 < 8);

const S03PlanilhaDeMemoria: React.FC = () => {
	const scene = useScene();
	const {frame} = useFrame();
	const headline = scene.copy[0].text; // Chutar as horas?
	const emph = scene.copy[0].emphasis[0] ?? 'Chutar';
	const at = headline.indexOf(emph);

	const cam = camAt(frame);
	const shake = slamShake(frame, 0, 's03');
	const sp = springAt(frame, 0, 'SLAM');
	let slamScale = 1.06 - 0.06 * sp;
	if (frame > 8 && Math.abs(slamScale - 1) < 0.001) slamScale = 1;
	const strike = ramp(frame, 20, 30, E.enter);
	const struckColor = strike > 0 ? `rgba(232,237,249,${(1 - 0.32 * strike).toFixed(3)})` : color.ink;

	const marks: Mark[] = GUESSES.map((g) => ({
		col: g.col,
		row: g.row,
		p: springAt(frame, g.at - 1, 'SNAPPY'),
		o: interpolate(frame, [g.at - 2, g.at + 1], [0, 1], CLAMP),
	}));

	return (
		<SceneTransitions>
			<Backdrop seed="s03">
				<AbsoluteFill style={{transform: shakeTransform(shake)}}>
					<AbsoluteFill style={{transformOrigin: '0 0', transform: camTransform(cam)}}>
						<TimesheetPlane pose={POSE} marks={marks} />
					</AbsoluteFill>
					{/* the caret, screen space: it IS the camera's anchor, so it lands on (960, 520) at 4×56 exactly */}
					<Caret x={cam.ax} y={cam.ay} on={caretOn(frame)} />

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
							letterSpacing: '-0.04em',
							color: color.ink,
							whiteSpace: 'nowrap',
							transform: slamScale === 1 ? undefined : `scale(${slamScale})`,
							transformOrigin: '50% 50%',
						}}
					>
						{headline.slice(0, at)}
						<span style={{position: 'relative', display: 'inline-block', color: struckColor}}>
							{emph}
							<SweepBar progress={strike} color={color.rose} topEm={0.535} thicknessEm={0.07} overhangEm={0.04} />
						</span>
						{headline.slice(at + emph.length)}
					</div>
				</AbsoluteFill>
			</Backdrop>
		</SceneTransitions>
	);
};

export default S03PlanilhaDeMemoria;
