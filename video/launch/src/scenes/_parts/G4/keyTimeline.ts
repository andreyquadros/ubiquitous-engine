/**
 * s10's keycap choreography as a pure function of the scene frame, so s11 can
 * start its match cut from EXACTLY the pose s10 ends on (s10 f29).
 *
 *   f0–8   mount: scale 0.94 → 1, y +30 → 0 (SNAPPY)
 *   f12–15 press: face +24 px into the well, skirt 34 → 10 (E.exit, 3 f)
 *   f15    CONTACT (abs 675, beat 2): legend latches volt, underglow decays over 8 f,
 *          shockwave rings expand in the key plane over 16 f, floor light flares
 *   f16–23 release (SNAPPY), legend stays volt
 */
import {E, springAt} from '../../../shared/motion';
import {clamp01, lerp, ramp} from './common';
import {KEY_REST, type KeyPose} from './KeyCap3D';

export const S10 = {
	mount: 0,
	pressStart: 12,
	contact: 15,
	travel: 24,
	underglowFrames: 8,
	shockFrames: 16,
} as const;

export const s10KeyPose = (f: number): KeyPose => {
	const m = f < S10.mount ? 0 : springAt(f, S10.mount, 'SNAPPY');
	let press = 0;
	if (f >= S10.pressStart && f <= S10.contact) press = S10.travel * E.exit(ramp(f, S10.pressStart, S10.contact));
	else if (f > S10.contact) press = S10.travel * (1 - springAt(f, S10.contact, 'SNAPPY'));
	const lit = f >= S10.contact ? 1 : 0;
	const glow = f >= S10.contact ? 1 - E.push(clamp01((f - S10.contact) / S10.underglowFrames)) : 0;
	const shock = f >= S10.contact ? clamp01((f - S10.contact + 1) / S10.shockFrames) : 0;
	return {
		...KEY_REST,
		scale: lerp(0.94, 1, m),
		dy: lerp(30, 0, m),
		press,
		skirt: KEY_REST.skirt - press,
		legendVolt: lit,
		underglow: glow,
		shock: shock >= 1 ? 0 : shock,
		// the floor light flares with the contact, then settles a touch brighter (the key is "on")
		floorGlow: KEY_REST.floorGlow + (f >= S10.contact ? 0.35 + 0.65 * glow : 0),
	};
};

/** The pose s10 hands over on its last frame (f29). */
export const S10_FINAL = s10KeyPose(29);
