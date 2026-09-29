/**
 * s10-tecla-1 — S10 · abs 660–689 (30 f) · features · bar 12.1 → 12.3
 *
 * The band stop, v2: the key is the HERO.
 * f0 (abs 660, downbeat) cut: the review plane exactly where s09 left it
 * (REVIEW_END, the picker card still lifting off the assign card), rack-blurred
 * 16 px and stepped back under a navy veil, pushing 1.00 → 1.04 (linear).
 * A 440-px silver keycap "1" mounts at (960, 520) (SNAPPY f0–8) in a cool key
 * light with a volt back-rim and a floor pool; goes down f12–15 (E.exit),
 * CONTACT f15 (abs 675, beat 2 — the key-down heard alone): the legend latches
 * volt, underglow burst, two shockwave rings expand in the key plane, the floor
 * light flares, a volt flash lifts the backdrop, 3-px micro-shake + a 1.5 %
 * punch on the backdrop; release f16–23 (SNAPPY). f30 match cut: s11 starts
 * from this exact pose (S10_FINAL).
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {noise2D} from '@remotion/noise';
import {LiftCard, LiftHole} from '../components/LiftCard';
import {navyDim} from '../components/Stage';
import type {ScreenConfig} from '../components/screen-geometry';
import {alpha, color} from '../design/tokens';
import {Patches, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {Backdrop, G4Plane, ramp, widenLegend} from './_parts/G4/common';
import {KeyCap3D} from './_parts/G4/KeyCap3D';
import {S10, s10KeyPose} from './_parts/G4/keyTimeline';
import {CARD_BG, PICKER, PICKER_AT, pickerLift, REVIEW_END, REVIEW_FILE, S09_LEN, S09_SUBTITLE_PATCH} from './_parts/G4/review';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'key_down_1.wav', atFrame: 15, gainDb: -8, note: 'Contact, heard alone in the band stop.'},
	{ref: 'impact_soft_3.wav', atFrame: 15, gainDb: -13, note: 'v2: the shockwave rings + floor flare on contact (65 Hz round thump under the key-down).'},
	{ref: 'key_up_1.wav', atFrame: 21, gainDb: -18, note: 'Release.'},
];

/** 3-px micro-shake on contact (noise, exp decay, 7 f). */
const microShake = (f: number) => {
	const t = f - S10.contact;
	if (t < 0 || t >= 7) return {x: 0, y: 0};
	const d = Math.exp(-t / 2.2);
	return {x: noise2D('s10-sx', t * 0.9, 0) * 3 * d, y: noise2D('s10-sy', t * 0.9, 4.2) * 3 * d};
};

/** The backdrop plane: s09's last framing, held. */
export const S10_PLANE: Omit<ScreenConfig, 'src'> = {
	width: 1440,
	radius: 18,
	glow: false,
	camera: [{at: 0, zoom: REVIEW_END.zoom, focus: REVIEW_END.focus, anchor: REVIEW_END.anchor, duration: 0}],
	rotateX: REVIEW_END.tilt.rx,
	rotateY: REVIEW_END.tilt.ry,
};

const S10Tecla1: React.FC = () => {
	const scene = useScene();
	const {frame: f, duration} = useSceneFrame();
	const shake = microShake(f);
	const hit = f >= S10.contact ? Math.exp(-(f - S10.contact) / 3) : 0;
	const push = 1 + 0.04 * ramp(f, 0, duration - 1) + 0.015 * hit;
	const pose = s10KeyPose(f);
	const lift = pickerLift(PICKER_AT - S09_LEN, PICKER_LIFT_FROM);
	const patches = widenLegend(storyboardPatches(scene, REVIEW_FILE));
	return (
		<TransitionOut>
			<TransitionIn>
				<Backdrop
					seed="s10"
					vignette={0.5}
					look={{
						keyPool: {x: 0.5, y: 0.52, w: 0.62, h: 0.82, opacity: 0.44},
						keyLight: {x: 0.5, y: 0.46, w: 0.42, h: 0.6, opacity: 0.18},
					}}
				>
					{/* blurred, veiled backdrop: plane + the lifting picker (s09 f75 → f104 of the same motion) */}
					<AbsoluteFill
						style={{
							filter: 'blur(16px)',
							transform: `translate(${(960 + shake.x).toFixed(2)}px, ${(540 + shake.y).toFixed(2)}px) scale(${push.toFixed(5)}) translate(-960px, -540px)`,
							transformOrigin: '0 0',
						}}
					>
						<G4Plane
							{...S10_PLANE}
							layers={[
								{
									src: REVIEW_FILE,
									children: (
										<>
											<Patches patches={patches} />
											<div style={{position: 'absolute', ...rectStyle(S09_SUBTITLE_PATCH)}} />
											<LiftHole rect={PICKER} at={lift.at} enter="lift" color={CARD_BG} pad={4} feather={10} radius={14} />
										</>
									),
								},
							]}
						/>
						<LiftCard src={REVIEW_FILE} {...lift} patches={patches} style={{zIndex: 5}} />
						<AbsoluteFill style={{background: navyDim(0.34 - 0.1 * hit)}} />
					</AbsoluteFill>
					{/* key light above the veiled backdrop: a cool white-blue pool with a volt surround behind the key */}
					<AbsoluteFill
						style={{
							background: `radial-gradient(ellipse 34% 44% at 50% 50%, ${alpha('#cfe0ff', 0.2)} 0%, ${alpha(color.volt, 0.16)} 45%, ${alpha(color.volt, 0)} 100%)`,
						}}
					/>
					{/* contact flash: a volt lift of the whole backdrop, decays over ~6 f */}
					{hit > 0.01 ? (
						<AbsoluteFill style={{background: `radial-gradient(ellipse 70% 70% at 50% 55%, ${alpha(color.volt, 0.22 * hit)}, ${alpha(color.volt, 0.05 * hit)} 60%, ${alpha(color.volt, 0)} 100%)`}} />
					) : null}
					<AbsoluteFill style={{transform: shake.x || shake.y ? `translate(${shake.x.toFixed(2)}px, ${shake.y.toFixed(2)}px)` : undefined}}>
						<KeyCap3D pose={pose} label={scene.copy[0].text} />
					</AbsoluteFill>
				</Backdrop>
			</TransitionIn>
		</TransitionOut>
	);
};

const rectStyle = (p: {x: number; y: number; w: number; h: number; fill: string}): React.CSSProperties => ({left: p.x, top: p.y, width: p.w, height: p.h, background: p.fill});

/** The picker lift was already mid-flight at s09's end: its origin is only used for the first frames of the spring. */
const PICKER_LIFT_FROM = {x: 1300 - 440, y: 470 - 232, w: 880, h: 465};

export default S10Tecla1;
