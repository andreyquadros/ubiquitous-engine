/**
 * s13-nada-em-duvida — S13 · abs 870–929 (60 f) · features · bar 15.3 → 16.3
 *
 * f0 (abs 870) cut on the empty queue (review-done, zoom 1.6, near-flat
 * rx 2° ry −2°); continuity/claim patches over the resolved-count paragraph,
 * the settled header count and the keys legend. f0–15 push onto the in-app
 * UBI (E.push). f15 (abs 885, beat 2) T5 match: the 3D UBI takes the in-app
 * UBI's exact place (frame 352 px, 3-f crossfade bitmap → 3D, the bitmap UBI
 * and its bubble covered in the card colour), the app racks out (blur 0 → 8,
 * brightness 1 → 0.45, SMOOTH 12 f) and pushes 1.00 → 1.10 (E.push f15–35);
 * UBI grows to a 760-px frame and leaps left on a short arc (jump-from-idle
 * from index 6: apex f24, contact f30 = abs 900 downbeat, squash f33), 3-px
 * shake on contact, ember floor glow + volt rim. The in-app bubble grows into
 * the big "Nada em dúvida!" bubble (scale 0.36 → 1 + translate, SNAPPY 12 f,
 * lands f27, "dúvida!" volt, tail to his head). f35–59 idle + slow float.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {noise2D} from '@remotion/noise';
import {alpha, color, font} from '../design/tokens';
import {E, Patches, springAt, storyboardCamera, storyboardPatches, TransitionIn, TransitionOut, UbiTrack, UBI_ANCHORS, useScene, useSceneFrame, type SfxCue} from '../shared';
import {Backdrop, clamp01, G4Plane, lerp, ramp, UI} from './_parts/G4/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'shimmer_2.wav', atFrame: 15, gainDb: -12, note: 'UBI steps out of the app.'},
	{ref: 'pop+2.wav', atFrame: 27, gainDb: -16, note: 'Bubble lands.'},
	{ref: 'bloop_1.wav', atFrame: 30, gainDb: -12, note: 'UBI lands on the downbeat.'},
];

const FILE = 'ui/review-done.png';
const MATCH = 15;
const CONTACT = 30;

/** Match rect: the in-app UBI body at f15 = a 352-px clip frame with its top-left at (788, 460) → feet anchor. */
const START = {size: 352, x: 788 + UBI_ANCHORS.feet.x * (352 / 900), y: 460 + UBI_ANCHORS.feet.y * (352 / 900)};
/** Landing: 760-px frame, body centre x 640 (body anchor x 443 → feet x ≈ 646), feet on y 900. */
const END = {size: 760, x: 640 + (UBI_ANCHORS.feet.x - UBI_ANCHORS.body.x) * (760 / 900), y: 900};
const ARC = 70; // extra leap height (px) on top of the clip's own jump, 0 at f15 and at contact

/** The in-app bubble on screen at f15 and the big bubble's final box. */
const SMALL = {cx: 961, cy: 433, w: 238};
const BUBBLE = {x: 822, y: 292, w: 668, h: 136, r: 36};
const TAIL = {base0: 78, base1: 150, tip: {x: 768, y: 474}};

const Bubble: React.FC<{f: number; text: string}> = ({f, text}) => {
	if (f < MATCH) return null;
	const s = springAt(f, MATCH, 'SNAPPY', 12);
	const k = lerp(SMALL.w / BUBBLE.w, 1, s);
	const cx = lerp(SMALL.cx, BUBBLE.x + BUBBLE.w / 2, s);
	const cy = lerp(SMALL.cy, BUBBLE.y + BUBBLE.h / 2, s);
	const o = ramp(f, MATCH, MATCH + 3);
	// text: "Nada em dúvida!" with the last word volt (split on the last space / NBSP)
	const cut = Math.max(text.lastIndexOf(' '), text.lastIndexOf(' '));
	const lead = text.slice(0, cut + 1);
	const emph = text.slice(cut + 1);
	const {x: x0, y: y0, w, h, r} = BUBBLE;
	const x1 = x0 + w;
	const y1 = y0 + h;
	const path = [
		`M ${x0 + r} ${y0}`,
		`H ${x1 - r}`,
		`Q ${x1} ${y0} ${x1} ${y0 + r}`,
		`V ${y1 - r}`,
		`Q ${x1} ${y1} ${x1 - r} ${y1}`,
		`H ${x0 + TAIL.base1}`,
		`Q ${x0 + 70} ${y1 + 6} ${TAIL.tip.x} ${TAIL.tip.y}`,
		`Q ${x0 + 40} ${y1 - 2} ${x0 + TAIL.base0 - 40} ${y1}`,
		`H ${x0 + r}`,
		`Q ${x0} ${y1} ${x0} ${y1 - r}`,
		`V ${y0 + r}`,
		`Q ${x0} ${y0} ${x0 + r} ${y0}`,
		'Z',
	].join(' ');
	return (
		<AbsoluteFill
			style={{
				opacity: o,
				transformOrigin: `${BUBBLE.x + BUBBLE.w / 2}px ${BUBBLE.y + BUBBLE.h / 2}px`,
				transform: s > 0.999 ? undefined : `translate(${(cx - (BUBBLE.x + BUBBLE.w / 2)).toFixed(2)}px, ${(cy - (BUBBLE.y + BUBBLE.h / 2)).toFixed(2)}px) scale(${k.toFixed(4)})`,
			}}
		>
			<svg width={1920} height={1080} style={{position: 'absolute', left: 0, top: 0, overflow: 'visible', filter: 'drop-shadow(0 18px 30px rgba(0,0,0,0.45))'}}>
				<path d={path} fill="rgba(23,32,51,0.94)" stroke="rgba(255,255,255,0.08)" strokeWidth={1} />
			</svg>
			<div
				style={{
					position: 'absolute',
					left: x0,
					top: y0,
					width: w,
					height: h,
					display: 'flex',
					alignItems: 'center',
					justifyContent: 'center',
					fontFamily: font.display,
					fontWeight: 700,
					fontSize: 72,
					lineHeight: 1,
					letterSpacing: '-0.03em',
					color: color.ink,
					whiteSpace: 'nowrap',
					paddingBottom: 4,
				}}
			>
				<span>
					{lead}
					<span style={{color: color.volt}}>{emph}</span>
				</span>
			</div>
		</AbsoluteFill>
	);
};

const S13NadaEmDuvida: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const cam = storyboardCamera(scene, {file: FILE});
	const patches = storyboardPatches(scene, FILE);
	const staticPatches = patches.filter((p) => !/UBI/.test(p.covers ?? ''));
	const ubiPatch = patches.find((p) => /UBI/.test(p.covers ?? ''));

	// app racks out + push
	const rack = f < MATCH ? 0 : springAt(f, MATCH, 'SMOOTH', 12);
	const push = 1 + 0.1 * E.push(ramp(f, MATCH, 35));
	// 3-px shake on contact
	const t = f - CONTACT;
	const sh = t >= 0 && t < 7 ? {x: noise2D('s13-x', t * 0.9, 0) * 3 * Math.exp(-t / 2.5), y: noise2D('s13-y', t * 0.9, 5) * 3 * Math.exp(-t / 2.5)} : {x: 0, y: 0};

	// UBI placement
	const g = f < MATCH ? 0 : springAt(f, MATCH, 'SMOOTH', 20);
	const leap = f >= MATCH && f <= CONTACT ? Math.sin(Math.PI * ramp(f, MATCH, CONTACT)) : 0;
	const float = 6 * E.glide(ramp(f, 36, 59));
	const size = lerp(START.size, END.size, g);
	const x = lerp(START.x, END.x, g);
	const floorY = lerp(START.y, END.y, g);
	const y = floorY - ARC * leap * (0.35 + 0.65 * g) - float;
	const ubiOpacity = ramp(f, MATCH, MATCH + 3);
	// floor light arrives with him; flares on contact
	const land = ramp(f, 22, CONTACT, E.enter);
	const flare = f >= CONTACT ? 1 - ramp(f, CONTACT, CONTACT + 12, E.push) : 0;
	const glowW = size * 0.78;

	return (
		<TransitionOut>
			<TransitionIn>
				<Backdrop seed="s13" ember={0.07}>
					<AbsoluteFill style={{transform: sh.x || sh.y ? `translate(${sh.x.toFixed(2)}px, ${sh.y.toFixed(2)}px)` : undefined}}>
						<G4Plane
							{...cam}
							layers={[
								{
									src: FILE,
									children: (
										<>
											<Patches patches={staticPatches} />
											{ubiPatch && f >= MATCH ? (
												<div
													style={{
														position: 'absolute',
														left: ubiPatch.rect.x - 2,
														top: ubiPatch.rect.y - 2,
														width: ubiPatch.rect.w + 4,
														height: ubiPatch.rect.h + 4,
														background: UI.card,
														opacity: ramp(f, MATCH, MATCH + 3),
													}}
												/>
											) : null}
										</>
									),
								},
							]}
							blur={8 * rack}
							brightness={1 - 0.55 * rack}
							push={push}
							pushOrigin={{x: END.x, y: 640}}
						/>
						{/* ember floor glow + contact shadow on the stage floor (not tied to the jump) */}
						{land > 0.001 ? (
							<>
								<div
									style={{
										position: 'absolute',
										left: x - glowW / 2,
										top: floorY - size * 0.07,
										width: glowW,
										height: size * 0.14,
										borderRadius: '50%',
										opacity: land,
										background: `radial-gradient(closest-side, ${alpha(color.ember, 0.3 + 0.18 * flare)} 0%, ${alpha(color.ember, 0.1)} 50%, transparent 100%)`,
									}}
								/>
								<div
									style={{
										position: 'absolute',
										left: x - size * 0.19,
										top: floorY - size * 0.022,
										width: size * 0.38,
										height: size * 0.044,
										borderRadius: '50%',
										opacity: land * (1 - 0.5 * clamp01((floorY - y) / 80)),
										background: 'radial-gradient(closest-side, rgba(0,0,0,0.5) 0%, rgba(0,0,0,0.22) 55%, transparent 100%)',
									}}
								/>
							</>
						) : null}
						<UbiTrack segments={scene.ubiTrack} size={size} x={x} y={y} opacity={ubiOpacity} rim={alpha(color.volt, 0.35)} />
						<Bubble f={f} text={scene.copy[0].text} />
					</AbsoluteFill>
				</Backdrop>
			</TransitionIn>
		</TransitionOut>
	);
};

export default S13NadaEmDuvida;
