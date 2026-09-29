/**
 * s16-sua-ia — S16 · abs 1125–1229 (105 f) · proof
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in whip-left 4 f, out blur-dissolve 6 f.
 * Copy:
 * - Claude, OpenAI ou Grok.
 * - Ou deixe com o Ubi.
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "whip_3.wav", atFrame: 0, gainDb: -8, note: "Whip on the cut abs 1125; file starts 3 f earlier (master track)."},
	{ref: "ui_tick_2.wav", atFrame: 30, gainDb: -22, note: "Hover."},
	{ref: "ui_tick_3.wav", atFrame: 45, gainDb: -22, note: "Hover."},
	{ref: "click.wav", atFrame: 75, gainDb: -12, note: "IA do Ubi."},
	{ref: "shimmer_2.wav", atFrame: 78, gainDb: -14, note: "State change."},
];

const S16SuaIa: React.FC = () => <SceneStub id="s16-sua-ia" />;

export default S16SuaIa;
