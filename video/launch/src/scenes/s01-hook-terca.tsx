/**
 * s01-hook-terca — abs 0–89 (90 f) · hook
 *
 * f0 is the poster: rose kicker + the question fully set, the empty ANTES
 * timesheet leaning back like a desk (rx 32°, rz −4°, s 0.90, centre y 800),
 * volt caret on in TER/10h. f0–24 the sheet slams flat (E.push, 90 % by f6,
 * vertical motion blur only while it moves > 40 px/f). f2–14 volt underline
 * sweeps under "terça?". f30 (beat 3) the TER column is spotlit: the other
 * four go to 40 % over 8 f. Caret blinks 8/8. f24–89 camera push 1.00→1.04
 * into TER with the camera sliding 24 px toward it (content +24 px). The
 * type is pinned in screen space and never moves or blurs. Hard cut at 90.
 */
import React from 'react';
import {AbsoluteFill, interpolate} from 'remotion';
import {DirectionalBlur} from '../components/Transitions';
import {color, font} from '../design/tokens';
import {E, SceneTransitions, useScene, type SfxCue} from '../shared';
import {
	Backdrop,
	boxTopForCap,
	blinkOn,
	camTransform,
	cellCenter,
	CLAMP,
	lerp,
	project,
	ramp,
	SweepBar,
	TER,
	TimesheetPlane,
	TS,
	useFrame,
	type Cam2D,
	type PlanePose,
} from './_parts/G1/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [];

const POSE_0: PlanePose = {cx: 960, cy: 800, rx: 32, rz: -4, s: 0.9};
const POSE_1: PlanePose = {cx: 960, cy: 700, rx: 8, rz: 0, s: 1};
const SLAM_END = 24;
const PUSH_FROM = 24;
const PUSH_TO = 89;

const poseAt = (f: number): PlanePose => {
	const e = interpolate(f, [0, SLAM_END], [0, 1], {...CLAMP, easing: E.push});
	return {
		cx: lerp(POSE_0.cx, POSE_1.cx, e),
		cy: lerp(POSE_0.cy, POSE_1.cy, e),
		rx: lerp(POSE_0.rx, POSE_1.rx, e),
		rz: lerp(POSE_0.rz, POSE_1.rz, e),
		s: lerp(POSE_0.s, POSE_1.s, e),
	};
};

/** Focus of the push: the TER column's centre on the flat sheet. */
const TER_FOCUS = (() => {
	const c = cellCenter(TER, 0);
	return project(POSE_1, c.x, TS.h / 2);
})();

const camAt = (f: number): Cam2D => {
	const t = interpolate(f, [PUSH_FROM, PUSH_TO], [0, 1], CLAMP); // linear drift
	const zoom = Math.exp(lerp(Math.log(1), Math.log(1.04), t));
	return {fx: TER_FOCUS.x, fy: TER_FOCUS.y, ax: TER_FOCUS.x + 24 * t, ay: TER_FOCUS.y, zoom};
};

/** Max screen velocity of the sheet's edges (px/f) between f−1 and f. */
const planeSpeed = (f: number) => {
	if (f <= 0) return 0;
	const a = poseAt(f - 1);
	const b = poseAt(f);
	let v = 0;
	for (const [px, py] of [
		[TS.w / 2, 0],
		[TS.w / 2, TS.h],
		[0, 0],
		[TS.w, 0],
	]) {
		const p = project(a, px, py);
		const q = project(b, px, py);
		v = Math.max(v, Math.hypot(q.x - p.x, q.y - p.y));
	}
	return v;
};

const KICKER = {size: 26, capTop: 168};
const HEAD = {size: 112, capTop: 214};

const S01HookTerca: React.FC = () => {
	const scene = useScene();
	const {frame} = useFrame();
	const kicker = scene.copy[0].text; // SEXTA-FEIRA · 17:00
	const headline = scene.copy[1].text; // O que você fez na terça?
	const emph = scene.copy[1].emphasis[0] ?? 'terça?';
	const at = headline.lastIndexOf(emph);
	const before = headline.slice(0, at);
	const after = headline.slice(at + emph.length);

	const pose = poseAt(frame);
	const cam = camAt(frame);
	const v = planeSpeed(frame);
	// light, 1–2 frames only: the slam reads as speed, the labels never smear into streaks
	const blur = v > 40 ? Math.min(10, (v - 30) * 0.25) : 0;
	const dim = ramp(frame, 30, 38, E.enter);
	const underline = ramp(frame, 2, 14, E.glide);

	return (
		<SceneTransitions>
			<Backdrop seed="s01">
				{/* canvas layer: the timesheet (camera applies here only) */}
				<AbsoluteFill style={{transformOrigin: '0 0', transform: camTransform(cam)}}>
					<DirectionalBlur amount={blur} angle={90}>
						<TimesheetPlane pose={pose} dim={dim} caretOn={blinkOn(frame)} />
					</DirectionalBlur>
				</AbsoluteFill>

				{/* type layer: pinned, crisp */}
				<AbsoluteFill>
					<div
						style={{
							position: 'absolute',
							left: 0,
							width: 1920,
							top: Math.round(boxTopForCap('text', KICKER.size, KICKER.capTop)),
							textAlign: 'center',
							fontFamily: font.text,
							fontWeight: 600,
							fontSize: KICKER.size,
							lineHeight: 1,
							letterSpacing: '0.14em',
							paddingLeft: '0.14em',
							color: color.rose,
							whiteSpace: 'nowrap',
						}}
					>
						{kicker}
					</div>
					<div
						style={{
							position: 'absolute',
							left: 0,
							width: 1920,
							top: Math.round(boxTopForCap('display', HEAD.size, HEAD.capTop)),
							textAlign: 'center',
							fontFamily: font.display,
							fontWeight: 700,
							fontSize: HEAD.size,
							lineHeight: 1,
							letterSpacing: '-0.035em',
							color: color.ink,
							whiteSpace: 'nowrap',
							fontKerning: 'normal',
						}}
					>
						{before}
						<span style={{position: 'relative', display: 'inline-block'}}>
							{emph}
							<SweepBar progress={underline} color={color.volt} topEm={1.07} thicknessEm={0.08} overhangEm={0.01} />
						</span>
						{after}
					</div>
				</AbsoluteFill>
			</Backdrop>
		</SceneTransitions>
	);
};

export default S01HookTerca;
