/**
 * Frame-math helpers shared by every primitive. Pure functions only —
 * everything is a function of the frame number so renders are deterministic.
 */
import {interpolate, spring} from 'remotion';
import {ease, springs, type SpringPreset} from './tokens';

export type EaseFn = (t: number) => number;

/**
 * A keyframe: `[frame, value]` or `[frame, value, easing]`. The easing applies
 * to the segment that *arrives* at this key (from the previous key).
 */
export type Keyframe = readonly [frame: number, value: number] | readonly [frame: number, value: number, easing: EaseFn];

/**
 * A value that is either static (`12`) or keyframed
 * (`[[0, -20], [30, 0, ease.push]]`). Frames are local to the enclosing
 * `<Sequence>`. Before the first key the first value holds; after the last
 * key the last value holds.
 */
export type Keyframed = number | readonly Keyframe[];

/** Evaluate a {@link Keyframed} value at `frame`. */
export const kf = (value: Keyframed | undefined, frame: number, fallback = 0, defaultEase: EaseFn = ease.inOut): number => {
	if (value === undefined) return fallback;
	if (typeof value === 'number') return value;
	if (value.length === 0) return fallback;
	if (value.length === 1 || frame <= value[0][0]) return value[0][1];
	const last = value[value.length - 1];
	if (frame >= last[0]) return last[1];
	for (let i = 1; i < value.length; i++) {
		const b = value[i];
		if (frame <= b[0]) {
			const a = value[i - 1];
			const e = b[2] ?? defaultEase;
			if (b[0] === a[0]) return b[1];
			return interpolate(frame, [a[0], b[0]], [a[1], b[1]], {
				easing: e,
				extrapolateLeft: 'clamp',
				extrapolateRight: 'clamp',
			});
		}
	}
	return last[1];
};

/** Clamp `v` into [lo, hi]. */
export const clamp = (v: number, lo = 0, hi = 1): number => Math.min(hi, Math.max(lo, v));

/** Linear mix between a and b. */
export const lerp = (a: number, b: number, t: number): number => a + (b - a) * t;

/**
 * 0→1 progress of a tween that starts at `start` and lasts `duration` frames,
 * eased and clamped. `duration <= 0` returns a hard cut (0 before, 1 after).
 */
export const progress = (frame: number, start: number, duration: number, easing: EaseFn = ease.push): number => {
	if (duration <= 0) return frame >= start ? 1 : 0;
	return interpolate(frame, [start, start + duration], [0, 1], {
		easing,
		extrapolateLeft: 'clamp',
		extrapolateRight: 'clamp',
	});
};

/** Eased, clamped tween from `from` to `to`. */
export const tween = (
	frame: number,
	start: number,
	duration: number,
	from: number,
	to: number,
	easing: EaseFn = ease.push,
): number => lerp(from, to, progress(frame, start, duration, easing));

/**
 * Spring from 0 → 1 starting at `delay`. Uses a preset from tokens.springs.
 * `durationInFrames` stretches the spring to an exact length when given.
 */
export const springIn = (
	frame: number,
	fps: number,
	delay = 0,
	preset: SpringPreset = 'snappy',
	durationInFrames?: number,
): number =>
	spring({
		frame: frame - delay,
		fps,
		config: springs[preset],
		durationInFrames,
	});

/** Sine oscillation — `period` in frames. Deterministic. */
export const oscillate = (frame: number, period: number, amplitude = 1, phase = 0): number =>
	Math.sin(((frame / period) * 2 + phase) * Math.PI) * amplitude;

/** Stagger orders for groups of items. */
export type StaggerOrder = 'index' | 'reverse' | 'center-out' | 'random';

/**
 * Delay (frames) for item `i` of `count` given `each` frames between items.
 * `random` is seeded by `i` so it is stable across renders.
 */
export const staggerDelay = (i: number, count: number, each: number, order: StaggerOrder = 'index'): number => {
	switch (order) {
		case 'reverse':
			return (count - 1 - i) * each;
		case 'center-out': {
			const mid = (count - 1) / 2;
			return Math.round(Math.abs(i - mid) * each);
		}
		case 'random': {
			const r = Math.abs(Math.sin((i + 1) * 12.9898) * 43758.5453) % 1;
			return Math.round(r * (count - 1) * each);
		}
		default:
			return i * each;
	}
};

/** Convert degrees to radians. */
export const rad = (deg: number): number => (deg * Math.PI) / 180;
