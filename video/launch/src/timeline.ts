/**
 * Master timeline of the "Launch" composition, in frames on the 120 BPM grid
 * (BEAT = 15, BAR = 60 at 30 fps). Scene agents: add entries here; Launch.tsx
 * derives its duration from TIMELINE.durationInFrames.
 *
 * PLACEHOLDER: 50 s (25 bars) until the edit is locked.
 */
import {bars, FPS} from './design/tokens';

export type SceneSlot = {
	/** Stable id used by Launch.tsx to pick the scene component. */
	id: string;
	/** Start frame in the master timeline. */
	from: number;
	/** Length in frames. */
	durationInFrames: number;
	/** What happens (for humans). */
	note?: string;
};

const scenes: SceneSlot[] = [{id: 'placeholder', from: 0, durationInFrames: bars(25), note: 'Placeholder until scenes land.'}];

export const TIMELINE = {
	fps: FPS,
	scenes,
	durationInFrames: Math.max(...scenes.map((s) => s.from + s.durationInFrames)),
} as const;
