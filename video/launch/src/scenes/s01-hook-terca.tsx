/**
 * s01-hook-terca — S01 · abs 0–89 (90 f) · hook
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - SEXTA-FEIRA · 17:00
 * - O que você fez na terça?
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [];

const S01HookTerca: React.FC = () => <SceneStub id="s01-hook-terca" />;

export default S01HookTerca;
