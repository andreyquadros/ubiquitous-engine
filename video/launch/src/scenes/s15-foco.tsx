/**
 * s15-foco — S15 · abs 1035–1124 (90 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out whip-left 4 f.
 * Copy:
 * - Não! Foque na sua produtividade.
 * - Ok, foco!
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "impact_soft_2.wav", atFrame: 0, gainDb: -2, note: "Window slam; bed duck −5 dB."},
	{ref: "glitch_2.wav", atFrame: 2, gainDb: -18, note: "The blocked tab is gone."},
	{ref: "click.wav", atFrame: 45, gainDb: -12, note: "Ok, foco!"},
];

const S15Foco: React.FC = () => <SceneStub id="s15-foco" />;

export default S15Foco;
