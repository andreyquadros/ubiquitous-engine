/**
 * s08-regras-memoria-ia — S08 · abs 510–584 (75 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - Regras,
 * - memória,
 * - IA.
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "impact_soft_1.wav", atFrame: 0, gainDb: 0, note: "“Regras,” — duck −5 dB."},
	{ref: "impact_soft_2.wav", atFrame: 15, gainDb: 0, note: "“memória,” — duck −5 dB."},
	{ref: "impact_soft_3.wav", atFrame: 30, gainDb: 2, note: "“IA.” on the downbeat — duck −5 dB."},
];

const S08RegrasMemoriaIa: React.FC = () => <SceneStub id="s08-regras-memoria-ia" />;

export default S08RegrasMemoriaIa;
