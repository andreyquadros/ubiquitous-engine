/**
 * s10-tecla-1 — S10 · abs 660–689 (30 f) · features · bar 12.1 → 12.3
 *
 * The band stop. f0 (abs 660, downbeat) cut: the review-queue-selected plane
 * exactly where s09 left it (assign-option-1, zoom 2.3, rx 3° ry −5°), rack-
 * blurred 14 px and dimmed 0.55 with its claim patches, pushing 1.00 → 1.04
 * (linear). A big 3D keycap "1" mounts at (960, 560) (SNAPPY f0–8), goes down
 * f12–15 (E.exit), CONTACT f15 (abs 675, beat 2 — the key-down heard alone):
 * legend latches volt, volt underglow decays over 8 f, 2-px micro-shake;
 * release f16–23 (SNAPPY). f30 match cut: s11 starts from this exact pose.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {noise2D} from '@remotion/noise';
import {Patches, storyboardCamera, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {Backdrop, G4Plane, KeyPool, ramp} from './_parts/G4/common';
import {KeyCap3D} from './_parts/G4/KeyCap3D';
import {S10, s10KeyPose} from './_parts/G4/keyTimeline';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'key_down_1.wav', atFrame: 15, gainDb: -8, note: 'Contact, heard alone in the band stop.'},
	{ref: 'key_up_1.wav', atFrame: 21, gainDb: -18, note: 'Release.'},
];

const FILE = 'ui/review-queue-selected.png';

/** 2-px micro-shake on contact (noise, exp decay, 6 f). */
const microShake = (f: number) => {
	const t = f - S10.contact;
	if (t < 0 || t >= 6) return {x: 0, y: 0};
	const d = Math.exp(-t / 2);
	return {x: noise2D('s10-sx', t * 0.9, 0) * 2 * d, y: noise2D('s10-sy', t * 0.9, 4.2) * 2 * d};
};

const S10Tecla1: React.FC = () => {
	const scene = useScene();
	const {frame: f, duration} = useSceneFrame();
	const cam = storyboardCamera(scene, {file: FILE});
	const shake = microShake(f);
	const push = 1 + 0.04 * ramp(f, 0, duration - 1);
	const pose = s10KeyPose(f);
	return (
		<TransitionOut>
			<TransitionIn>
				<Backdrop seed="s10">
					<G4Plane
						{...cam}
						layers={[{src: FILE, children: <Patches patches={storyboardPatches(scene, FILE)} />}]}
						blur={14}
						dim={0.55}
						push={push}
						pushOrigin={{x: 960, y: 540}}
						shift={shake}
					/>
					<KeyPool />
					<AbsoluteFill style={{transform: shake.x || shake.y ? `translate(${shake.x.toFixed(2)}px, ${shake.y.toFixed(2)}px)` : undefined}}>
						<KeyCap3D pose={pose} label={scene.copy[0].text} />
					</AbsoluteFill>
				</Backdrop>
			</TransitionIn>
		</TransitionOut>
	);
};

export default S10Tecla1;
