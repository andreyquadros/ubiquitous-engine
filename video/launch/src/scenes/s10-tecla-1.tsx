/**
 * s10-tecla-1 — S10 · abs 660–689 (30 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out match-cut 0 f.
 * Copy:
 * - 1
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "key_down_1.wav", atFrame: 15, gainDb: -8, note: "Contact, heard alone in the band stop."},
	{ref: "key_up_1.wav", atFrame: 21, gainDb: -18, note: "Release."},
];

const S10Tecla1: React.FC = () => <SceneStub id="s10-tecla-1" />;

export default S10Tecla1;
