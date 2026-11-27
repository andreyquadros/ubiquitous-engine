/**
 * s04-silencio — abs 225–239 (15 f) · problem · T8 drop-out
 *
 * Canvas + vignette + grain only (no orbs), and the 4×56 volt caret on
 * (960, 520) — pixel-identical to s03's last frame. f0–7 on, f8–14 off.
 * Nothing else moves; grain keeps the frame alive. Total silence: the only
 * cue is the riser, placed by its END (last sample abs 224), so nothing is
 * audible here. Out: flash (0 f) — the flash lives in s05 f0–3.
 */
import React from 'react';
import {SceneTransitions, type SfxCue} from '../shared';
import {Backdrop, Caret, CARET_SPOT, useFrame} from './_parts/G1/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{
		ref: 'riser-2bar.wav',
		atFrame: 0,
		gainDb: -6,
		note: 'Placed by its END: the riser’s last sample is abs 224, so it plays abs 105–224 (starts at s02 f15) and ends exactly as the silence begins. Master-track cue; nothing is audible at abs 225–239.',
	},
];

const S04Silencio: React.FC = () => {
	const {frame} = useFrame();
	return (
		<SceneTransitions>
			<Backdrop seed="s04" orbs={[]} look={{level: 0 /* the designed silence: lights out, navy base + vignette + grain */}}>
				<Caret x={CARET_SPOT.x} y={CARET_SPOT.y} on={frame < 8} />
			</Backdrop>
		</SceneTransitions>
	);
};

export default S04Silencio;
