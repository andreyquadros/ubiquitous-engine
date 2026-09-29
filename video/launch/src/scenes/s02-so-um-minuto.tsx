/**
 * s02-so-um-minuto — S02 · abs 90–179 (90 f) · problem
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - “Só um minutinho.”
 * - 40 min
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "glitch_1.wav", atFrame: 0, gainDb: -16, note: "Flash re-layout 1."},
	{ref: "glitch_3.wav", atFrame: 8, gainDb: -16, note: "Flash re-layout 2 (8th)."},
	{ref: "impact.wav", atFrame: 15, gainDb: 0, note: "Slam contact; bed duck −5 dB."},
	{ref: "ui_tick_2.wav", atFrame: 33, gainDb: -22},
	{ref: "ui_tick_2.wav", atFrame: 36, gainDb: -22},
	{ref: "ui_tick_2.wav", atFrame: 39, gainDb: -22},
	{ref: "ui_tick_2.wav", atFrame: 42, gainDb: -22},
	{ref: "ui_tick_2.wav", atFrame: 45, gainDb: -22},
	{ref: "ui_tick_2.wav", atFrame: 48, gainDb: -22},
	{ref: "ui_tick_2.wav", atFrame: 51, gainDb: -22},
	{ref: "ui_tick_2.wav", atFrame: 54, gainDb: -22},
	{ref: "ui_tick_2.wav", atFrame: 57, gainDb: -22},
	{ref: "impact_soft_3.wav", atFrame: 60, gainDb: -2, note: "Counter lands on 40 min."},
];

const S02SoUmMinuto: React.FC = () => <SceneStub id="s02-so-um-minuto" />;

export default S02SoUmMinuto;
