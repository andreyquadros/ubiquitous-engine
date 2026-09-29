/**
 * s05-drop-ubiqx — S05 · abs 240–359 (120 f) · reveal
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in flash 4 f, out match-cut 15 f.
 * Copy:
 * - ubiqX AI
 * - Controle de tempo automático com IA.
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "impact_deep_2.wav", atFrame: 0, gainDb: 2, note: "The drop (impact + 45 Hz sub in one file); bed duck −6 dB."},
	{ref: "shimmer_1.wav", atFrame: 10, gainDb: -14, note: "Glint."},
	{ref: "bloop_1.wav", atFrame: 15, gainDb: -12, note: "UBI lands."},
	{ref: "whoosh_out_3.wav", atFrame: 108, gainDb: -14, note: "UBI shrinks away into the app."},
];

const S05DropUbiqx: React.FC = () => <SceneStub id="s05-drop-ubiqx" />;

export default S05DropUbiqx;
