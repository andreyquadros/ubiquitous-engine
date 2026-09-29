/**
 * s05-drop-ubiqx — S05 · abs 240–359 (120 f) · reveal · bar 5.1 → 7.1
 *
 * THE DROP. f0 (abs 240): flash in-half (T7), the wordmark is on screen at
 * full opacity and slams 1.10 → 1 (SLAM) around the X's spot, tracking
 * +0.02em → −0.03em (E.push, 20 f). f0–2 the volt caret of s04 is still on
 * (960, 520); f3 the X pops out of it (−90° → 0, 0 → 1, BOUNCY_SUBTLE — the
 * shot's one bouncy element). f8 "AI" pill (SNAPPY). f3–15 descriptor masks
 * up (E.enter, 12 f). f0–15 UBI launches in and lands on beat 2 (contact f15).
 * f10–28 glint. f60 (downbeat) grid floor + slow canvas push 1 → 1.04 (E.glide,
 * 45 f). f105–119 match-cut prep: canvas tints to the hero-card colour, the
 * app's ember floor glow fades in, UBI shrinks onto the in-app UBI's rect of
 * s06 f0 (E.exit, 15 f).
 *
 * Layers: canvas camera (backdrop + floor glow; scaled 1 → 1.04) · type (screen
 * space, whole px; only the ≤ 8-f slam shake moves it) · UBI (screen space,
 * position mapped through the canvas camera, then to the match rect).
 */
import React from 'react';
import {AbsoluteFill, Easing} from 'remotion';
import {GUARD_OPACITY, guardFor} from '../components/Stage';
import {color, font} from '../design/tokens';
import {E, springAt, TransitionIn, TransitionOut, UbiTrack, useScene, useSceneFrame, type SfxCue} from '../shared';
import {APP_GLOW, Backdrop, CARET, HERO_CARD_GRADED, lerp, mixHex, ramp, shakeTransform, slamShake, UBI_MATCH, W, H} from './_parts/G2/common';
import {AiPill, Wordmark} from './_parts/G2/Lockup';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'impact_deep_2.wav', atFrame: 0, gainDb: 2, note: 'The drop (impact + 45 Hz sub in one file); bed duck −6 dB.'},
	{ref: 'shimmer_1.wav', atFrame: 10, gainDb: -14, note: 'Glint.'},
	{ref: 'bloop_1.wav', atFrame: 15, gainDb: -12, note: 'UBI lands.'},
	{ref: 'whoosh_out_3.wav', atFrame: 108, gainDb: -14, note: 'UBI shrinks away into the app.'},
];

/* UBI in the lockup: 600-px frame, box (1075, 129) → feet anchor (450, 797) on (1375, 660.3). */
const UBI_HOME = {x: 1075 + 450 * (600 / 900), y: 129 + 797 * (600 / 900), size: 600};
const MATCH_START = 105;
const MATCH_END = 119;

/** Canvas camera zoom (around the canvas centre). */
const canvasZoom = (f: number) => lerp(1, 1.04, ramp(f, 60, 104, E.glide));
const cam = (f: number, x: number, y: number) => {
	const z = canvasZoom(f);
	return {x: W / 2 + (x - W / 2) * z, y: H / 2 + (y - H / 2) * z, z};
};

/** Launch offset (px, + = below the floor): +260 → −30 f0–9 (E.push), −30 → 0 f9–15 (E.exit = gravity). */
const launchOffset = (f: number) => {
	if (f <= 9) return lerp(260, -30, ramp(f, 0, 9, E.push));
	return lerp(-30, 0, ramp(f, 9, 15, E.exit));
};

const Descriptor: React.FC<{frame: number; text: string}> = ({frame, text}) => {
	// Inter 500 44 px, cap-top y 752 (cap offset 0.1358 em) → line box top 746.0; centred on x 960.
	const size = 44;
	const p = ramp(frame, 3, 15, E.enter);
	if (frame < 3) return null;
	const padT = 0.16 * size;
	const padB = 0.12 * size;
	return (
		<div
			style={{
				position: 'absolute',
				left: 0,
				width: W,
				top: Math.round(752 - 0.1358 * size) - padT,
				display: 'flex',
				justifyContent: 'center',
			}}
		>
			<div style={{overflow: 'hidden', paddingTop: padT, paddingBottom: padB, paddingLeft: 12, paddingRight: 12}}>
				<div
					style={{
						fontFamily: font.text,
						fontWeight: 500,
						fontSize: size,
						lineHeight: 1,
						letterSpacing: '-0.01em',
						color: color.ink2,
						whiteSpace: 'nowrap',
						transformOrigin: '0% 100%',
						transform: p < 1 ? `translateY(${((1 - p) * 110).toFixed(2)}%) rotate(${((1 - p) * 3).toFixed(3)}deg)` : undefined,
					}}
				>
					{text}
				</div>
			</div>
		</div>
	);
};

/**
 * v2 type guard: the settled lockup "ubiqX [AI]" glyph box (x 446–1170, y 439–632, from a still).
 * Round 2: the light (key pool, key light, volt orb) sits behind UBI, so the lockup is on the
 * light's falloff and the guard only trims it (GUARD RULE, components/Stage.tsx). Short
 * horizontal feather (0.3 × w) so the guard core stays clear of the pool behind UBI. Peak
 * `lockupGuardPeak`: the default 0.68 while the wordmark's own glow halo holds (f0–45), 0.55 once the
 * floor's horizon glow sits right under it (f60+; a denser guard there reads as a dip).
 */
const LOCKUP_GUARD = guardFor({x: 808, y: 532, w: 724, h: 194}, {padX: 0.3 * 724});
const lockupGuardPeak = (f: number) => lerp(GUARD_OPACITY, 0.55, ramp(f, 45, 75, E.glide));
/** The light behind UBI (his settled body centre ≈ (1375, 470)). */
const UBI_LIGHT = {x: 1375, y: 470};

const S05DropUbiqx: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const brand = scene.copy[0].text; // "ubiqX AI" (drawn as the wordmark + pill)
	const descriptor = scene.copy[1].text;

	/* ---- canvas ---------------------------------------------------------- */
	const tint = ramp(f, MATCH_START, MATCH_END, Easing.inOut(Easing.quad));
	// volt orb Ø1100 behind the lockup: 0 → 0.30 over 6 f, settles 0.18 by f30
	const orbA = f < 6 ? lerp(0, 0.3, ramp(f, 0, 6, E.push)) : lerp(0.3, 0.18, ramp(f, 6, 30, E.glide));
	const fade = 1 - 0.85 * tint;
	const floor = 0.35 * ramp(f, 59, 75, E.enter) * fade;
	const z = canvasZoom(f);
	const shake = slamShake(f, 0, 's05');

	/* ---- type ------------------------------------------------------------ */
	const slam = springAt(f, 0, 'SLAM');
	const lockScale = lerp(1.1, 1, slam);
	const spread = 10 * (1 - ramp(f, 0, 20, E.push)); // +0.02em → −0.03em = 0.05em × 200 px per gap
	const xPop = f < 3 ? 0 : springAt(f, 3, 'BOUNCY_SUBTLE');
	const caretOn = f <= 2 || (f <= 5 && xPop < 0.6);
	const caretScale = f <= 2 ? 1 : Math.max(0, 1 - ramp(f, 3, 6, E.exit));
	const pill = f < 8 ? 0 : springAt(f, 8, 'SNAPPY');
	const glint = f < 10 ? 0 : ramp(f, 10, 28, Easing.inOut(Easing.cubic));
	const glow = f < 45 ? 1 : 1 - ramp(f, 45, 60, E.glide);

	/* ---- UBI ------------------------------------------------------------- */
	const yOff = launchOffset(f);
	const home = cam(f, UBI_HOME.x, UBI_HOME.y + yOff);
	const m = ramp(f, MATCH_START, MATCH_END, E.exit);
	const ubiX = lerp(home.x, UBI_MATCH.x, m);
	const ubiY = lerp(home.y, UBI_MATCH.y, m);
	const ubiSize = lerp(UBI_HOME.size * z, UBI_MATCH.size, m);
	// volt floor glow on the lockup floor (y 660): appears as he comes down, flares on contact, fades for the match
	const land = ramp(f, 8, 15, E.enter) * (1 - ramp(f, MATCH_START, MATCH_START + 8, E.exit));
	const ring = f >= 15 && f < 33 ? ramp(f, 15, 33, E.push) : 0;
	const floorPt = cam(f, UBI_HOME.x, UBI_HOME.y);
	const appGlow = ramp(f, MATCH_START + 3, MATCH_END, E.enter);

	return (
		<TransitionOut>
			{() => (
				<TransitionIn>
					<AbsoluteFill style={{backgroundColor: color.canvas}}>
						{/* canvas camera: backdrop */}
						<AbsoluteFill style={{transform: `${shakeTransform(shake) ?? ''} scale(${z.toFixed(5)})`, transformOrigin: '50% 50%'}}>
							<Backdrop
								seed="s05"
								base={mixHex(color.canvas, HERO_CARD_GRADED, tint)}
								look={{
									level: 1 - tint /* v2 stage fades out onto the hero-card colour for the match cut */,
									guard: [{...LOCKUP_GUARD, opacity: lockupGuardPeak(f)}] /* trims the light's falloff under the volt X + AI pill (≥ 4.5:1) */,
									// the key pool and the white-blue key light sit behind UBI (the lit subject); the lockup is on their falloff
									keyPool: {x: UBI_LIGHT.x / W, y: UBI_LIGHT.y / H, w: 0.6, h: 0.92},
									keyLight: {x: 0.71, y: 0.42, w: 0.36, h: 0.6, opacity: 0.17},
								}}
								orbs={[
									// the drop's volt bloom, behind UBI (round 2: was centred on the lockup, under the guard → a navy hole in a bright ring)
									{c: color.volt, x: UBI_LIGHT.x, y: UBI_LIGHT.y, d: 1100, opacity: orbA * fade},
									{c: color.ember, x: 1640, y: 900, d: 900, opacity: 0.07 * fade},
								]}
								floor={floor}
								vignette={0.6 * (1 - 0.6 * tint)}
								grain={0.045}
							/>
						</AbsoluteFill>
						{/* floor glow + landing ring (canvas camera) */}
						<AbsoluteFill style={{transform: shakeTransform(shake)}}>
							{land > 0.002 ? (
								<div
									style={{
										position: 'absolute',
										left: floorPt.x - 230 * z,
										top: floorPt.y - 44 * z,
										width: 460 * z,
										height: 88 * z,
										borderRadius: '50%',
										opacity: land,
										background: `radial-gradient(closest-side, rgba(77,141,255,0.42) 0%, rgba(77,141,255,0.16) 45%, transparent 100%)`,
									}}
								/>
							) : null}
							{ring > 0 && ring < 1 ? (
								<div
									style={{
										position: 'absolute',
										left: floorPt.x - lerp(90, 300, ring),
										top: floorPt.y - lerp(14, 46, ring),
										width: lerp(180, 600, ring),
										height: lerp(28, 92, ring),
										borderRadius: '50%',
										border: `2px solid rgba(77,141,255,${(0.55 * (1 - ring)).toFixed(3)})`,
										boxShadow: `0 0 18px rgba(77,141,255,${(0.35 * (1 - ring)).toFixed(3)})`,
									}}
								/>
							) : null}
							{/* the app's own ember floor glow, fading in under the target rect for the match */}
							{appGlow > 0.002 ? (
								<div
									style={{
										position: 'absolute',
										left: APP_GLOW.x - APP_GLOW.w / 2,
										top: APP_GLOW.y - APP_GLOW.h / 2,
										width: APP_GLOW.w,
										height: APP_GLOW.h,
										borderRadius: '50%',
										opacity: appGlow,
										background: 'radial-gradient(closest-side, rgba(255,122,31,0.42) 0%, rgba(255,122,31,0.16) 50%, transparent 100%)',
									}}
								/>
							) : null}
						</AbsoluteFill>

						{/* type: screen space, slam shake only */}
						<AbsoluteFill style={{transform: shakeTransform(shake)}} aria-label={brand}>
							<AbsoluteFill style={{transform: `scale(${lockScale.toFixed(5)})`, transformOrigin: `${CARET.x}px ${CARET.y}px`}}>
								<Wordmark spread={spread} xPop={xPop} glint={glint} glow={glow} />
								<AiPill p={pill} />
							</AbsoluteFill>
							{caretOn && caretScale > 0.01 ? (
								<div
									style={{
										position: 'absolute',
										left: CARET.x - CARET.w / 2,
										top: CARET.y - CARET.h / 2,
										width: CARET.w,
										height: CARET.h,
										borderRadius: 2,
										background: color.volt,
										boxShadow: '0 0 14px rgba(77, 141, 255, 0.55), 0 0 2px rgba(77, 141, 255, 0.9)',
										transform: caretScale < 1 ? `scaleY(${caretScale.toFixed(3)})` : undefined,
									}}
								/>
							) : null}
							<Descriptor frame={f} text={descriptor} />
						</AbsoluteFill>

						{/* UBI: jump-from-idle (index 6 at f0, contact index 21 on f15) → idle 11…95 */}
						<UbiTrack
							segments={scene.ubiTrack}
							x={ubiX + shake.x}
							y={ubiY + shake.y}
							size={ubiSize}
							anchor="feet"
							rim={m < 0.999 ? `rgba(77, 141, 255, ${(0.35 * (1 - m)).toFixed(3)})` : undefined}
						/>
					</AbsoluteFill>
				</TransitionIn>
			)}
		</TransitionOut>
	);
};

export default S05DropUbiqx;
