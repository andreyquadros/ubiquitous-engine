/**
 * s17-privacidade — S17 · abs 1230–1379 (150 f) · proof
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in blur-dissolve 6 f, out cut 0 f.
 * Copy:
 * - Seus dados ficam com você.
 * - A IA vê só o mínimo.
 * - ana@example.com
 * - [email]
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "ui_tick_1.wav", atFrame: 76, gainDb: -22, note: "Marker sweep starts."},
	{ref: "pop.wav", atFrame: 90, gainDb: -14, note: "Mask contact."},
];

const S17Privacidade: React.FC = () => <SceneStub id="s17-privacidade" />;

export default S17Privacidade;
