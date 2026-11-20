/**
 * The storyboard's motion vocabulary (brief/style.md §3.1–3.2), by the exact
 * names the storyboard uses: `E.push`, `E.glide`, `E.exit`…, and the spring
 * presets `SNAPPY`, `SMOOTH`, `SMOOTH_SLOW`, `BOUNCY_SUBTLE`, `SLAM`.
 *
 * NOTE: src/design/tokens.ts has older `springs`/`ease` presets with slightly
 * different numbers (used by the primitives). When a storyboard line names a
 * preset, use the one from HERE.
 */
import {Easing, spring, type SpringConfig} from 'remotion';
import {FPS} from '../design/tokens';

/** Bézier easings (style.md §3.2). */
export const E = {
	/** Expo-out: push-ins, reveals, incoming halves of transitions. 97 % done at half time. */
	push: Easing.bezier(0.16, 1, 0.3, 1),
	/** Masked lines, fades of UI elements, logo row. */
	enter: Easing.bezier(0.22, 1, 0.36, 1),
	/** Camera travel between two holds (span type), wipes, split divider. */
	glide: Easing.bezier(0.65, 0, 0.35, 1),
	/** Single-curve whip pan; cut at 0.5. */
	whip: Easing.bezier(0.85, 0, 0.15, 1),
	/** Expo-in: exits, outgoing halves of zoom-through and whip, key press down. */
	exit: Easing.bezier(0.7, 0, 0.84, 0),
	/** Cursor travel (quick departure, soft arrival). */
	cursor: Easing.bezier(0.45, 0, 0.15, 1),
	/** Only for drifts, grain, orb rotation, typewriter. */
	linear: Easing.linear,
} as const;

export type EName = keyof typeof E;

/** Spring presets (style.md §3.1), verified at 30 fps against remotion 4.0.529. */
export const SPRING = {
	/** 90 % at 6 f, 1.4 % overshoot, settles 13 f. Text, cards, toasts, UI state. */
	SNAPPY: {damping: 20, mass: 0.7, stiffness: 220},
	/** 90 % at 12 f, no overshoot. Big panels, windows, end-card lockup. */
	SMOOTH: {damping: 200, mass: 1, stiffness: 100},
	/** 90 % at 16 f, no overshoot. Hero drift, glow orbs, slow floats. */
	SMOOTH_SLOW: {damping: 200, mass: 1, stiffness: 60},
	/** 90 % at 6 f, 7 % overshoot. Icons, badges, mascot, CTA pill, bubbles (one per shot). */
	BOUNCY_SUBTLE: {damping: 15, mass: 0.8, stiffness: 170},
	/** 90 % at 4 f, 4.2 % overshoot. Word slams, logo contact. */
	SLAM: {damping: 22, mass: 0.6, stiffness: 400},
} as const satisfies Record<string, Partial<SpringConfig>>;

export type SpringName = keyof typeof SPRING;

/**
 * Spring value 0→1 (may overshoot) for a named preset, starting at `start`.
 * `durationInFrames` time-stretches it to settle exactly on a beat.
 */
export const springAt = (frame: number, start: number, preset: SpringName = 'SNAPPY', durationInFrames?: number): number =>
	spring({frame: frame - start, fps: FPS, config: SPRING[preset], durationInFrames});

/**
 * A spring preset as an easing function t∈[0,1] → value (for interpolate /
 * camera keys). `frames` is the span the easing is applied over.
 */
export const springEasing = (preset: SpringName, frames: number) => (t: number): number =>
	spring({frame: t * frames, fps: FPS, config: SPRING[preset], durationInFrames: frames});

/**
 * Resolve an easing name as written in the storyboard ("E.push", "E.glide",
 * "linear", "SMOOTH", "SLAM"…) to an easing function over `frames`.
 * Unknown names fall back to E.glide.
 */
export const easeByName = (name: string | undefined, frames = 24): ((t: number) => number) => {
	const n = (name ?? '').trim();
	const m = n.match(/^E\.(\w+)/);
	if (m && m[1] in E) return E[m[1] as EName];
	if (/^linear/i.test(n)) return E.linear;
	const s = n.toUpperCase().replace(/[^A-Z_]/g, '');
	if (s in SPRING) return springEasing(s as SpringName, frames);
	return E.glide;
};
