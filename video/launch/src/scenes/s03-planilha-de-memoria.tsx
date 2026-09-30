/**
 * s03-planilha-de-memoria — abs 180–224 (45 f) · problem
 *
 * v2. f0 (downbeat) hard cut INTO the slam contact: "Chutar as horas?" 136 px
 * at full opacity, scale 1.06→1 (SLAM) + 6 px shake. The s01 sheet is back,
 * big (1.2×, flat rx 6°, top edge y ≈ 300). Rose "?" guesses (≈ 82 px →
 * 120 px on the canvas, in rose-lit cells) pop on 8ths with a small spring
 * overshoot (f8 TER/11h, f15 SEG/15h, f23 QUI/09h, f30 TER/14h). f20–30 a
 * thick rose strike-through crosses out "Chutar", which steps back to 60 %.
 * The camera converges on the volt caret in TER/10h (E.glide pan + an
 * accelerating push 1.00→1.52 around the caret) so that on f44 the caret sits
 * exactly on (960, 520) at 4×56 px — the spot where s04 keeps it and s05's X
 * pops out of it — with the sheet running off both sides (v1's last frame was
 * lopsided). Caret blinks, then is solid f30–44. Hard cut at 45.
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
	inkGradient,
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

// v2: the sheet is big again (1.2×, top edge y ≈ 300) and the camera pushes in hard on the caret
const POSE: PlanePose = {cx: 960, cy: 682, rx: 6, rz: 0, s: 1.2};
/** End zoom: big enough that at f44 the sheet runs off both sides (fixes v1's lopsided last frame). */
const Z_END = 1.52;
const LAST = 44;
/** The caret's cell, projected with the camera at rest. */
const CARET_F = (() => {
	const c = cellCenter(TER, 1);
	return project(POSE, c.x, c.y);
})();

const camAt = (f: number): Cam2D => {
	const e = interpolate(f, [0, LAST], [0, 1], {...CLAMP, easing: E.glide});
	// accelerating push (ease-in): the frame closes in on the caret right into the drop-out
	const t = Math.pow(interpolate(f, [0, LAST], [0, 1], CLAMP), 1.6);
	return {
		fx: CARET_F.x,
		fy: CARET_F.y,
		ax: lerp(CARET_F.x, CARET_SPOT.x, e),
		ay: lerp(CARET_F.y, CARET_SPOT.y, e),
		zoom: Math.exp(lerp(0, Math.log(Z_END), t)),
	};
};

const GUESSES: {col: number; row: number; at: number}[] = [
	{col: TER, row: 2, at: 8}, // TER/11h
	{col: 0, row: 6, at: 15}, // SEG/15h
	{col: 3, row: 0, at: 23}, // QUI/09h
	{col: TER, row: 5, at: 30}, // TER/14h
];

const HEAD = {size: 136, capTop: 100};

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

	// "?" guesses pop with a spring (small overshoot) from 25 %
	const marks: Mark[] = GUESSES.map((g) => ({
		col: g.col,
		row: g.row,
		p: frame < g.at - 1 ? 0 : 0.25 + 0.75 * springAt(frame, g.at - 1, 'BOUNCY_SUBTLE'),
		o: interpolate(frame, [g.at - 1, g.at + 1], [0, 1], CLAMP),
	}));

	return (
		<SceneTransitions>
			<Backdrop seed="s03" look={{keyPool: {x: 0.46, y: 0.62, w: 0.9, h: 0.9, opacity: 0.42}, keyLight: {x: 0.46, y: 0.6, w: 0.6, h: 0.6, opacity: 0.16}}}>
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
							filter: 'drop-shadow(0 6px 30px rgba(5, 9, 22, 0.6))',
						}}
					>
						{headline.slice(0, at) ? <span style={inkGradient}>{headline.slice(0, at)}</span> : null}
						<span style={{position: 'relative', display: 'inline-block'}}>
							{/* the struck word steps back (60 %) under a thick rose bar */}
							<span style={{...inkGradient, opacity: 1 - 0.4 * strike}}>{emph}</span>
							<SweepBar progress={strike} color={color.rose} topEm={0.5} thicknessEm={0.09} overhangEm={0.05} glow />
						</span>
						<span style={inkGradient}>{headline.slice(at + emph.length)}</span>
					</div>
				</AbsoluteFill>
			</Backdrop>
		</SceneTransitions>
	);
};

export default S03PlanilhaDeMemoria;
