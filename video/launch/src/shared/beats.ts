/**
 * Beat-grid helpers (120 BPM at 30 fps: BEAT = 15 f, BAR = 60 f), re-exported
 * from src/design/tokens.ts plus a few bar/beat conveniences.
 */
import {BAR, BEAT, BPM, FPS, bars, beats, sec, snapToBeat} from '../design/tokens';

export {BAR, BEAT, BPM, FPS, bars, beats, sec, snapToBeat};

/** An 8th note is 7.5 f: the i-th eighth from `start`, rounded like the storyboard (0, 8, 15, 23, 30…). */
export const eighth = (i: number, start = 0): number => start + Math.round(i * (BEAT / 2));
/** A 16th note is 3.75 f: the i-th sixteenth from `start`, rounded. */
export const sixteenth = (i: number, start = 0): number => start + Math.round(i * (BEAT / 4));

/** Is an ABSOLUTE film frame on a beat (subdivision 2 = eighths, rounded like `eighth`)? */
export const isOnBeat = (abs: number, subdivision = 1): boolean => {
	if (subdivision === 1) return abs % BEAT === 0;
	const step = BEAT / subdivision;
	return Math.abs(Math.round(Math.round(abs / step) * step) - abs) < 1e-9;
};

/** Is an ABSOLUTE film frame on a downbeat (bar start)? */
export const isDownbeat = (abs: number): boolean => abs % BAR === 0;

/**
 * Musical position of an ABSOLUTE film frame as "bar.beat" (1-based, like the
 * storyboard): 0 → "1.1", 255 → "5.2", 1380 → "24.1". Off-beat frames get a
 * "+n" frame suffix: 98 → "2.3+8".
 */
export const barBeat = (abs: number): string => {
	const bar = Math.floor(abs / BAR) + 1;
	const inBar = abs - (bar - 1) * BAR;
	const beat = Math.floor(inBar / BEAT) + 1;
	const rest = inBar - (beat - 1) * BEAT;
	return rest === 0 ? `${bar}.${beat}` : `${bar}.${beat}+${rest}`;
};

/** Seconds timecode of an absolute frame: 255 → "00:08.50". */
export const timecode = (abs: number): string => {
	const s = abs / FPS;
	const m = Math.floor(s / 60);
	return `${String(m).padStart(2, '0')}:${(s - m * 60).toFixed(2).padStart(5, '0')}`;
};
