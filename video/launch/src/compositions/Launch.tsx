/**
 * "Launch" — the master composition: the whole film (1560 frames) straight
 * from brief/storyboard.json. Each scene is an absolute
 * <Sequence from={startFrame} durationInFrames name={id}> (no overlaps; the
 * transitions are split halves inside the scenes) and the master audio layer
 * (music + every scene's `sfx` cues) sits outside them. See ./Film.tsx.
 *
 * Scene components live in src/scenes/<scene-id>.tsx (registry: src/scenes/index.ts).
 */
import React from 'react';
import {TOTAL} from '../timeline';
import {SCENE_MODULES} from '../scenes';
import {Film} from './Film';

/** Map scene ids → scene components (kept for backward compatibility). */
export const SCENES: Record<string, React.FC> = Object.fromEntries(Object.entries(SCENE_MODULES).map(([id, m]) => [id, m.component]));

export const Launch: React.FC = () => <Film from={0} to={TOTAL} />;
