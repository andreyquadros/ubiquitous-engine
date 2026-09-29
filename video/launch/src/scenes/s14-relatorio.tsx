/**
 * s14-relatorio — S14 · abs 930–1034 (105 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - 18:00
 * - O relatório sai pronto.
 * - Markdown copiado
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "ding_1.wav", atFrame: 15, gainDb: -12, note: "18:00 lands (E6); rhymes with s06’s ding."},
	{ref: "whoosh-soft.wav", atFrame: 36, gainDb: -14, note: "Report window rises; loudest point ≈ fastest frame of the rise."},
	{ref: "click.wav", atFrame: 60, gainDb: -12, note: "Copiar Markdown."},
	{ref: "success_chime_2.wav", atFrame: 63, gainDb: -14, note: "Mint check."},
];

const S14Relatorio: React.FC = () => <SceneStub id="s14-relatorio" />;

export default S14Relatorio;
