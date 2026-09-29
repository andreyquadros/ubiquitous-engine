/**
 * s12-um-clique — S12 · abs 780–869 (90 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - Confirmar os 19
 * - Um clique vira memória.
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "click.wav", atFrame: 30, gainDb: -12, note: "Confirmar os 19."},
	{ref: "shimmer_1.wav", atFrame: 33, gainDb: -14, note: "All badges flip to “você” (C+3)."},
];

const S12UmClique: React.FC = () => <SceneStub id="s12-um-clique" />;

export default S12UmClique;
