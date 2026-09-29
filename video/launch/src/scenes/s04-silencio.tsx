/**
 * s04-silencio — S04 · abs 225–239 (15 f) · problem
 * STUB: replace the component with the real scene (frames are scene-relative).
 * Transitions: in cut 0 f, out flash 0 f.
 * Copy:
 * (no copy)
 */
import React from 'react';
import {SceneStub, type SfxCue} from '../shared';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: "riser-2bar.wav", atFrame: 0, gainDb: -6, note: "Placed by its END: the riser’s last sample is abs 224, so it plays abs 105–224 (starts at s02 f15) and ends exactly as the silence begins. Master-track cue; nothing is audible at abs 225–239."},
];

const S04Silencio: React.FC = () => <SceneStub id="s04-silencio" />;

export default S04Silencio;
