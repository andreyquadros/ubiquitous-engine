/**
 * s18-end-card — S18 · abs 1380–1559 (180 f) · cta · bar 24.1 → 27.1
 *
 * f0 (abs 1380, FINAL HIT, hard cut): the wordmark letters rise out of their
 * mask 1 f apart (the wave is already under way on the hit frame), the lockup
 * settles 1.06 → 1 (SMOOTH), the volt orb blooms and a volt bloom decays off the
 * wordmark within the bar. f10–28 glint. f6 tagline masks up (landed f20).
 * f12 CTA pill pops (BOUNCY_SUBTLE, the one bouncy element; landed f18).
 * f18 platform row rises, 3 f apart (landed f30). UBI falls in from above the
 * frame and lands on the lockup's floor line on f15 (abs 1395), squash f18,
 * rest f24, waves f25–85 (arm up across abs 1440), idles to the end.
 * f120 (abs 1500, downbeat) a light sheen crosses the CTA. The background drifts
 * 1 → 1.02; the type rests at scale 1 on whole pixels. The last frame is the
 * end card (no fade).
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {DirectionalBlur} from '../components/Transitions';
import {color} from '../design/tokens';
import {E, springAt, TransitionIn, TransitionOut, UbiTrack, useSceneFrame, type SfxCue} from '../shared';
import {FilmGrain, lerp, MaskLine, ramp, Stage, Vignette} from './_parts/G6/common';
import {AiPill, CtaPill, PlatformRow, ROW_Y, TAGLINE, Wordmark} from './_parts/G6/EndLockup';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'riser_1bar_1.wav', atFrame: 0, gainDb: -6, note: 'Placed by its END (tonal glide landing on A4): plays abs 1320–1379 and ends exactly on the final hit. Master-track cue.'},
	{ref: 'impact_deep_1.wav', atFrame: 0, gainDb: 2, note: 'Final hit; bed duck −4 dB.'},
	{ref: 'shimmer_3.wav', atFrame: 10, gainDb: -14, note: 'Glint.'},
	{ref: 'bloop_1.wav', atFrame: 15, gainDb: -12, note: 'UBI lands on the lockup.'},
];

/** UBI: 560-px frame, box left 1031 → feet anchor (450, 797) on (1311, 540), the lockup's floor line. */
const UBI = {size: 560, x: 1031 + 450 * (560 / 900), y: 540} as const;

/**
 * Fall from fully above the frame (offset −620: the soles are 80 px above the
 * top edge) to the floor, gravity-shaped (t²) f7 → f15, so contact (jump index
 * 21) lands on f15 = abs 1395, the beat after the hit.
 */
const FALL_FROM = 7;
const CONTACT = 15;
const DROP = 620;
const fallOffset = (f: number) => {
	if (f >= CONTACT) return 0;
	const t = Math.max(0, (f - FALL_FROM) / (CONTACT - FALL_FROM));
	return -DROP * (1 - t * t);
};
/** Vertical speed px/f (for the motion blur). */
const fallSpeed = (f: number) => Math.abs(fallOffset(f) - fallOffset(f - 1));

const S18EndCard: React.FC = () => {
	const {frame: f, scene} = useSceneFrame();
	const [hero, tagline, cta, platforms] = scene.copy.map((c) => c.text);

	/* ---- background: bloom on the hit, breathing orb, drift 1 → 1.02 ------ */
	// onset accent: the bloom's first moving frame is the hit's frame − 1
	const orbA = f < 6 ? lerp(0, 0.3, ramp(f + 1, 0, 6, E.push)) : lerp(0.3, 0.22, ramp(f, 6, 36, E.glide));
	const breathe = 1.025 - 0.025 * Math.cos((2 * Math.PI * f) / 120);
	const drift = lerp(1, 1.02, f / 179);

	/* ---- lockup settle (SMOOTH 1.06 → 1 around the row centre) ------------ */
	const settle = springAt(f + 1, 0, 'SMOOTH');
	const lockScale = lerp(1.06, 1, settle);
	const lockT = Math.abs(lockScale - 1) > 0.0004 ? `scale(${lockScale.toFixed(5)})` : undefined;

	/* ---- UBI ------------------------------------------------------------- */
	const off = fallOffset(f);
	const vblur = f >= FALL_FROM && f < CONTACT ? Math.min(12, fallSpeed(f) * 0.09) : 0;
	const near = ramp(f, 10, CONTACT, E.exit); // floor glow/shadow grow as he approaches
	const ring = f >= CONTACT && f < CONTACT + 18 ? ramp(f, CONTACT, CONTACT + 18, E.push) : 0;
	const flare = f >= CONTACT ? 1 + 0.6 * (1 - ramp(f, CONTACT, CONTACT + 14, E.enter)) : 1;

	return (
		<TransitionOut>
			<TransitionIn>
				<AbsoluteFill style={{backgroundColor: color.canvas}}>
					{/* background drift (the only camera move; type stays at scale 1) */}
					<AbsoluteFill style={{transform: `scale(${drift.toFixed(5)})`, transformOrigin: '50% 45%'}}>
						<Stage
							seed="s18"
							orbs={[
								{c: color.volt, x: 960, y: 430, d: 1150, opacity: orbA, s: breathe},
								{c: color.ember, x: 250, y: 960, d: 800, opacity: 0.06},
							]}
							floor={0.2}
							lineAlpha={0.5}
							horizon={0.7}
							gridSpeed={0.5}
						/>
					</AbsoluteFill>

					{/* floor glow, landing ring and contact shadow on the lockup floor line */}
					{f >= FALL_FROM ? (
						<AbsoluteFill>
							<div
								style={{
									position: 'absolute',
									left: UBI.x - 200 * flare,
									top: UBI.y - 34 * flare,
									width: 400 * flare,
									height: 68 * flare,
									borderRadius: '50%',
									opacity: near,
									background: 'radial-gradient(closest-side, rgba(77,141,255,0.38) 0%, rgba(77,141,255,0.14) 45%, transparent 100%)',
								}}
							/>
							{ring > 0 && ring < 1 ? (
								<div
									style={{
										position: 'absolute',
										left: UBI.x - lerp(80, 270, ring),
										top: UBI.y - lerp(12, 40, ring),
										width: lerp(160, 540, ring),
										height: lerp(24, 80, ring),
										borderRadius: '50%',
										border: `2px solid rgba(77,141,255,${(0.55 * (1 - ring)).toFixed(3)})`,
										boxShadow: `0 0 18px rgba(77,141,255,${(0.35 * (1 - ring)).toFixed(3)})`,
									}}
								/>
							) : null}
							<div
								style={{
									position: 'absolute',
									left: UBI.x - 110 * lerp(0.5, 1, near),
									top: UBI.y - 12,
									width: 220 * lerp(0.5, 1, near),
									height: 24,
									borderRadius: '50%',
									opacity: near,
									background: 'radial-gradient(closest-side, rgba(0,0,0,0.5) 0%, rgba(0,0,0,0.22) 55%, transparent 100%)',
								}}
							/>
						</AbsoluteFill>
					) : null}

					{/* the lockup: wordmark + AI pill (screen space) */}
					<AbsoluteFill style={{transform: lockT, transformOrigin: `960px ${ROW_Y}px`}} aria-label={hero}>
						<Wordmark frame={f} />
						<AiPill frame={f} />
					</AbsoluteFill>

					{/* UBI: hidden → jump from its apex (15) → wave → idle 0…93 (storyboard track verbatim) */}
					{vblur > 0.3 ? (
						<DirectionalBlur amount={vblur} angle={90}>
							<UbiTrack segments={scene.ubiTrack} x={UBI.x} y={UBI.y + off} size={UBI.size} />
						</DirectionalBlur>
					) : (
						<UbiTrack segments={scene.ubiTrack} x={UBI.x} y={UBI.y + off} size={UBI.size} />
					)}

					{/* tagline, CTA, platforms */}
					<MaskLine
						frame={f}
						at={6}
						capTop={TAGLINE.capTop}
						cx={960}
						family="inter"
						size={TAGLINE.size}
						weight={500}
						tracking="-0.011em"
						color={color.ink2}
						ariaLabel={tagline}
					>
						{tagline}
					</MaskLine>
					<CtaPill frame={f} at={12} sheenAt={120} text={cta} />
					<PlatformRow frame={f} at={18} text={platforms} />

					<Vignette strength={0.55} />
					<FilmGrain opacity={0.045} seed="s18" />
				</AbsoluteFill>
			</TransitionIn>
		</TransitionOut>
	);
};

export default S18EndCard;
