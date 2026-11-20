/**
 * s10's keycap choreography as a pure function of the scene frame, so s11 can
 * start its match cut from EXACTLY the pose s10 ends on (s10 f29).
 *
 *   f0–8   mount: scale 0.94 → 1, y +30 → 0 (SNAPPY)
 *   f12–15 press: face +24 px into the well, skirt 34 → 10 (E.exit, 3 f)
 *   f15    CONTACT (abs 675, beat 2): legend latches volt, underglow decays over 8 f,
 *          shockwave rings expand in the key plane over 11 f, floor light flares
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
	shockFrames: 11, // critique fix: rings fully gone by f26 (were 16 f, a ghost popped off at the cut)
} as const;

/**
 * v2 review fix (no dead hold into the match cut): after the release the key keeps floating up and turning a touch, so
 * the motion runs through s10 f24–29 and on into s11 (s11 evaluates s10KeyPose(30 + f) as the flight's source pose).
 * Velocity eases in over f18–26 to 1.1 px/f of rise and 0.06°/f of roll; integrated per frame (pure, deterministic).
 */
export const S10_FLOAT = {from: 18, to: 26, vy: -1.1, vrz: 0.06} as const;
const floatInt = (f: number): number => {
	let s = 0;
	for (let i = S10_FLOAT.from + 1; i <= f; i++) s += E.glide(ramp(i, S10_FLOAT.from, S10_FLOAT.to));
	// fractional tail keeps it continuous for non-integer frames
	const fi = Math.floor(f);
	if (f > fi && fi >= S10_FLOAT.from) s += (f - fi) * E.glide(ramp(fi + 1, S10_FLOAT.from, S10_FLOAT.to));
	return s;
};

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
		dy: lerp(30, 0, m) + S10_FLOAT.vy * floatInt(f),
		rz: KEY_REST.rz + S10_FLOAT.vrz * floatInt(f),
		press,
		skirt: KEY_REST.skirt - press,
		legendVolt: lit,
		underglow: glow,
		shock: shock >= 1 ? 0 : shock,
		// the floor light flares with the contact, then settles a touch brighter (the key is "on")
		floorGlow: KEY_REST.floorGlow + (f >= S10.contact ? 0.35 + 0.65 * glow : 0),
	};
};

/** The pose s10 hands over on its last frame (f29); s11 f0 continues it one frame on (s10KeyPose(30 + f): the float keeps running across the cut). */
export const S10_FINAL = s10KeyPose(29);
