/**
 * s11-e-ele-aprende — S11 · abs 690–779 (90 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in match-cut 12 f, out cut 0 f.
 * Copy:
 * - Uma tecla.
E ele aprende.
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "ui_tick_1.wav", atFrame: 12, gainDb: -18, note: "Chip lights."},
	{ref: "success_chime_1.wav", atFrame: 18, gainDb: -14, note: "Group classified (C6→E6)."},
	{ref: "pop.wav", atFrame: 30, gainDb: -16, note: "Rule chips appear."},
];

const S11EEleAprende: React.FC = () => <SceneStub id="s11-e-ele-aprende" />;

export default S11EEleAprende;
