/**
 * s06-ele-registra — S06 · abs 360–449 (90 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in match-cut 12 f, out whip-left 4 f.
 * Copy:
 * - Ele registra. Você trabalha.
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "swoosh_long_3.wav", atFrame: 12, gainDb: -10, note: "Deep pull-back for the hero reveal; the file starts 6 f before the scene (master track)."},
	{ref: "sweep_1.wav", atFrame: 36, gainDb: -16, note: "One tick cluster for the whole track build."},
	{ref: "ding_2.wav", atFrame: 60, gainDb: -12, note: "Playhead lands on 18h (C6)."},
];

const S06EleRegistra: React.FC = () => <SceneStub id="s06-ele-registra" />;

export default S06EleRegistra;
