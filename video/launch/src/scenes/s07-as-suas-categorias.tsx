/**
 * s07-as-suas-categorias — S07 · abs 450–509 (60 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in whip-left 4 f, out cut 0 f.
 * Copy:
 * - Suas categorias.
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "whip_2.wav", atFrame: 0, gainDb: -8, note: "Whip pass-by on the cut abs 450; file starts 2 f earlier (master track)."},
	{ref: "type-1.wav", atFrame: 8, gainDb: -22},
	{ref: "type-2.wav", atFrame: 12, gainDb: -22},
	{ref: "type-3.wav", atFrame: 16, gainDb: -22},
	{ref: "type-4.wav", atFrame: 20, gainDb: -22},
	{ref: "type-1.wav", atFrame: 24, gainDb: -22},
	{ref: "type-2.wav", atFrame: 28, gainDb: -22},
	{ref: "type-3.wav", atFrame: 32, gainDb: -22},
	{ref: "type-4.wav", atFrame: 36, gainDb: -22},
	{ref: "type-1.wav", atFrame: 40, gainDb: -22},
	{ref: "type-2.wav", atFrame: 44, gainDb: -22},
	{ref: "type-3.wav", atFrame: 48, gainDb: -22},
];

const S07AsSuasCategorias: React.FC = () => <SceneStub id="s07-as-suas-categorias" />;

export default S07AsSuasCategorias;
