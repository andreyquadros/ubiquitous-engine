/**
 * s10's keycap choreography as a pure function of the scene frame, so s11 can
 * start its match cut from EXACTLY the pose s10 ends on (s10 f29).
 *
 *   f0–8   mount: scale 0.94 → 1, y +20 → 0 (SNAPPY)
 *   f12–15 press: face +14 px into the well, skirt 18 → 4 (E.exit, 3 f)
 *   f15    CONTACT (abs 675, beat 2): legend volt (latched), underglow decays over 8 f
 *   f16–23 release (SNAPPY), legend stays volt
 */
import {E, springAt} from '../../../shared/motion';
import {clamp01, lerp, ramp} from './common';
import {KEY_REST, type KeyPose} from './KeyCap3D';

export const S10 = {
	mount: 0,
	pressStart: 12,
	contact: 15,
	travel: 14,
	underglowFrames: 8,
} as const;

export const s10KeyPose = (f: number): KeyPose => {
	const m = f < S10.mount ? 0 : springAt(f, S10.mount, 'SNAPPY');
	let press = 0;
	if (f >= S10.pressStart && f <= S10.contact) press = S10.travel * E.exit(ramp(f, S10.pressStart, S10.contact));
	else if (f > S10.contact) press = S10.travel * (1 - springAt(f, S10.contact, 'SNAPPY'));
	const lit = f >= S10.contact ? 1 : 0;
	const glow = f >= S10.contact ? 1 - E.push(clamp01((f - S10.contact) / S10.underglowFrames)) : 0;
	return {
		...KEY_REST,
		scale: lerp(0.94, 1, m),
		dy: lerp(20, 0, m),
		press,
		skirt: KEY_REST.skirt - press,
		legendVolt: lit,
		underglow: glow,
	};
};

/** The pose s10 hands over on its last frame (f29). */
export const S10_FINAL = s10KeyPose(29);
