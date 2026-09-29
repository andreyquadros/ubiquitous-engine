/**
 * s03-planilha-de-memoria — S03 · abs 180–224 (45 f) · problem
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - Chutar as horas?
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "impact_soft_2.wav", atFrame: 0, gainDb: 0, note: "Slam contact; bed duck −5 dB."},
	{ref: "ui_tick_1.wav", atFrame: 8, gainDb: -22, note: "“?” pops."},
	{ref: "ui_tick_1.wav", atFrame: 15, gainDb: -22, note: "“?” pops."},
	{ref: "ui_tick_1.wav", atFrame: 23, gainDb: -22, note: "“?” pops."},
	{ref: "ui_tick_1.wav", atFrame: 30, gainDb: -22, note: "“?” pops."},
];

const S03PlanilhaDeMemoria: React.FC = () => <SceneStub id="s03-planilha-de-memoria" />;

export default S03PlanilhaDeMemoria;
