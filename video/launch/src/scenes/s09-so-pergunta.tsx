/**
 * s09-so-pergunta — S09 · abs 585–659 (75 f) · features
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out cut 0 f.
 * Copy:
 * - Só pergunta o que não sabe.
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [];

const S09SoPergunta: React.FC = () => <SceneStub id="s09-so-pergunta" />;

export default S09SoPergunta;
