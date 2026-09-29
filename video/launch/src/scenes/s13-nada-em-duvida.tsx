/**
 * s13-nada-em-duvida — S13 · abs 870–929 (60 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - Nada em dúvida!
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "shimmer_2.wav", atFrame: 15, gainDb: -12, note: "UBI steps out of the app."},
	{ref: "pop+2.wav", atFrame: 27, gainDb: -16, note: "Bubble lands."},
	{ref: "bloop_1.wav", atFrame: 30, gainDb: -12, note: "UBI lands on the downbeat."},
];

const S13NadaEmDuvida: React.FC = () => <SceneStub id="s13-nada-em-duvida" />;

export default S13NadaEmDuvida;
