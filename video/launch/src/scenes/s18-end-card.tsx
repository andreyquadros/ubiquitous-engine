/**
 * s18-end-card — S18 · abs 1380–1559 (180 f) · cta
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - ubiqX AI
 * - Retome o controle do seu dia.
 * - Baixe em ubiqx.com.br
 * - macOS · Windows · Linux
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "riser_1bar_1.wav", atFrame: 0, gainDb: -6, note: "Placed by its END (tonal glide landing on A4): plays abs 1320–1379 and ends exactly on the final hit. Master-track cue."},
	{ref: "impact_deep_1.wav", atFrame: 0, gainDb: 2, note: "Final hit; bed duck −4 dB."},
	{ref: "shimmer_3.wav", atFrame: 10, gainDb: -14, note: "Glint."},
	{ref: "bloop_1.wav", atFrame: 15, gainDb: -12, note: "UBI lands on the lockup."},
];

const S18EndCard: React.FC = () => <SceneStub id="s18-end-card" />;

export default S18EndCard;
