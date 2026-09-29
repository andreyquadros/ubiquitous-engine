/**
 * s11-e-ele-aprende — S11 · abs 690–779 (90 f) · features · bar 12.3 → 14.1
 *
 * f0 (abs 690, beat 3) T5 match cut: s10's keycap, drawn at its exact s10
 * pose, flies on an arc and flattens into the assign-key-1 chip of the now-
 * sharp Revisão plane (12 f SNAPPY: rect, tilt → plane tilt, radius 34 → 8,
 * skirt → 0, fill → chip, legend 92 → chip size; latched volt = lit chip).
 * Backdrop blur 12 → 0 and dim 0.4 → 0 (SMOOTH 12 f).
 * f12 the IFRO row takes the pressed tint (volt 16 %) under the spotlight.
 * f15 (abs 705, beat) state change → review-after-assign: the Calendário row
 * slides out left (−140 px) and fades, the list closes up (−112 px), the
 * assign card switches in place (its subtitle would double up in a dissolve); the new active row flashes mint 12 %.
 * f12–27 "Uma tecla. / E ele aprende." (units at f12/15/18/21, "aprende." volt)
 * over a left canvas scrim. f30 (abs 720, downbeat) the rule chips
 * "Sempre: Calendário → IFRO" morph in (y +8 → 0, SNAPPY, 2-f stagger) while
 * the camera glides to them (E.glide f30–60, scale 1.3 → 1.15) and the
 * spotlight glides from the IFRO row to the chips. f60–89 drift. Cut out.
 */
import React from 'react';
import type {Rect} from '../components/screen-geometry';
import {sceneById} from '../storyboard';
import {color, font} from '../design/tokens';
import {E, hotspot, Patches, springAt, storyboardCamera, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {
	arcPoint,
	Backdrop,
	CaptureImg,
	clamp01,
	G4Plane,
	GlideSpot,
	insetOf,
	KeyPool,
	lerp,
	mapPt,
	planeGeometry,
	ramp,
	Scrim,
	StaggerHeadline,
	UI,
	unitsOf,
	widenLegend,
} from './_parts/G4/common';
import {KeyCap3D, mixPose, type KeyPose} from './_parts/G4/KeyCap3D';
import {S10_FINAL} from './_parts/G4/keyTimeline';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'ui_tick_1.wav', atFrame: 12, gainDb: -18, note: 'Chip lights.'},
	{ref: 'success_chime_1.wav', atFrame: 18, gainDb: -14, note: 'Group classified (C6→E6).'},
	{ref: 'pop.wav', atFrame: 30, gainDb: -16, note: 'Rule chips appear.'},
];

const BEFORE = 'ui/review-queue-selected.png';
const AFTER = 'ui/review-after-assign.png';

/** The keycap legend is s10's copy line ("1"). */
const KEY_LABEL = sceneById('s10-tecla-1').copy[0].text;

const MORPH = 12; // match-cut half, frames (storyboard transitionIn)
const SWAP = 15;
const SWAP_LEN = 6;
const RULES = 30;

const KEY1 = hotspot(BEFORE, 'assign-key-1'); // 2746,402 40×40
const IFRO_ROW = hotspot(AFTER, 'assign-option-1');
const SUGGEST = hotspot(AFTER, 'last-suggestions');
const ROW1 = hotspot(BEFORE, 'queue-row-1'); // Calendário (selected)
const LIST_BELOW: Rect = {x: 514, y: 462, w: 1529, h: 1092}; // rows 2… + settled header + card bottom
const NEW_ACTIVE = hotspot(AFTER, 'queue-row-1');

/** Rule-chip reveal: the storyboard's cover (2127,800,663×150) leaves 3 px of the second chip's border (y 953–955) → 160 tall. */
const RULE_COVER: Rect = {x: 2127, y: 800, w: 663, h: 160};
/** The three pieces of the suggestion block (measured on review-after-assign.png). */
const RULE_PIECES: Rect[] = [
	{x: 2120, y: 802, w: 400, h: 36}, // "Da última decisão"
	{x: 2120, y: 842, w: 500, h: 58}, // "Criar regra:" + chip 1
	{x: 2120, y: 902, w: 370, h: 58}, // chip 2
];

/** The key chip as a flat, lit pose over its hotspot at frame f (screen space). */
const chipPose = (cam: ReturnType<typeof storyboardCamera>, f: number): KeyPose => {
	const g = planeGeometry(cam, f);
	const c = mapPt(g, {x: KEY1.x + KEY1.w / 2, y: KEY1.y + KEY1.h / 2});
	const l = mapPt(g, {x: KEY1.x, y: KEY1.y + KEY1.h / 2});
	const r = mapPt(g, {x: KEY1.x + KEY1.w, y: KEY1.y + KEY1.h / 2});
	const size = Math.hypot(r.x - l.x, r.y - l.y);
	const k = size / KEY1.w;
	return {
		...S10_FINAL,
		cx: c.x,
		cy: c.y,
		size,
		rx: 3,
		ry: -6,
		rz: 0,
		perspective: 2400,
		radius: 8 * k,
		skirt: 0,
		press: 0,
		scale: 1,
		dy: 0,
		legendSize: 23 * k,
		legendVolt: 1,
		faceTop: UI.chipFill,
		faceBottom: UI.chipFill,
		border: color.volt,
		borderAlpha: 0.9,
		borderWidth: 1.5,
		highlight: 0,
		underglow: 0,
		shadow: 0,
		ring: 1,
		dish: 0,
		opacity: 1,
	};
};

/** The lit chip, glued to the plane (image space) once the key has landed. */
const LitChip: React.FC<{opacity: number; label: string}> = ({opacity, label}) =>
	opacity <= 0.001 ? null : (
		<div
			style={{
				position: 'absolute',
				left: KEY1.x,
				top: KEY1.y,
				width: KEY1.w,
				height: KEY1.h,
				borderRadius: 8,
				background: UI.chipFill,
				boxShadow: `inset 0 0 0 1.15px rgba(77,141,255,0.9), 0 0 14px rgba(77,141,255,0.35)`,
				display: 'flex',
				alignItems: 'center',
				justifyContent: 'center',
				opacity,
			}}
		>
			<div style={{fontFamily: font.text, fontWeight: 600, fontSize: 23, lineHeight: 1, color: color.volt, fontVariantNumeric: 'tabular-nums'}}>{label}</div>
		</div>
	);

/** A clipped piece of the BEFORE capture (+ its patches) that can move and fade: the list re-layout. */
const BeforeSlice: React.FC<{rect: Rect; tx?: number; ty?: number; opacity: number; patches: React.ReactNode}> = ({rect, tx = 0, ty = 0, opacity, patches}) =>
	opacity <= 0.001 ? null : (
		<div
			style={{
				position: 'absolute',
				left: 0,
				top: 0,
				width: 2880,
				height: 1800,
				clipPath: insetOf(rect),
				transform: tx || ty ? `translate(${tx.toFixed(2)}px, ${ty.toFixed(2)}px)` : undefined,
				opacity,
			}}
		>
			<CaptureImg src={BEFORE} />
			{patches}
		</div>
	);

const S11EEleAprende: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const cam = storyboardCamera(scene);
	const copy = scene.copy[0];
	const lines = unitsOf(copy.text, [
		[
			{text: 'Uma', at: 12},
			{text: 'tecla.', at: 15},
		],
		[
			{text: 'E\u00a0ele', at: 18},
			{text: 'aprende.', at: 21, emphasis: true},
		],
	]);

	// --- match cut: keycap → chip ------------------------------------------
	// travel: a slow lift-off then a soft landing on the chip over the 12-f half (E.cursor), so the eye can follow the key
	const m = E.cursor(ramp(f, 0, MORPH));
	const target = chipPose(cam, f);
	const p = arcPoint({x: S10_FINAL.cx, y: S10_FINAL.cy}, {x: target.cx, y: target.cy}, clamp01(m), 0.1);
	const pose: KeyPose = {...mixPose(S10_FINAL, target, clamp01(m)), cx: p.x, cy: p.y, size: lerp(S10_FINAL.size, target.size, m)};
	// shape/colour settle a touch earlier than the travel so the chip is flat when it arrives
	const flat = ramp(f, 3, 11, E.glide);
	pose.skirt = lerp(S10_FINAL.skirt, 0, flat);
	pose.shadow = lerp(1, 0, flat);
	pose.dish = lerp(1, 0, flat);
	const rack = f < MORPH ? springAt(f, 0, 'SMOOTH', MORPH) : 1;

	// --- plane state ---------------------------------------------------------
	const before = widenLegend(storyboardPatches(scene, BEFORE));
	const afterPatches = widenLegend(storyboardPatches(scene, AFTER)).filter((q) => !/rule chips/.test(q.covers ?? ''));
	const swapped = f >= SWAP;
	const e = swapped ? springAt(f, SWAP, 'SNAPPY') : 0;
	const inSwap = swapped && f < SWAP + SWAP_LEN;
	const beforePatchEls = <Patches patches={before} />;

	const tint = ramp(f, 12, 18, E.enter) * (1 - ramp(f, RULES, RULES + 10, E.exit));
	const spotDim = 0.45 * ramp(f, 12, 22, E.enter);
	// the spot stretches down to the chips (bottom edge leads), then its top follows: it never frames an unrelated row alone
	const lead = ramp(f, RULES, 42, E.glide);
	const follow = ramp(f, 38, 60, E.glide);
	const spotTop = lerp(IFRO_ROW.y, SUGGEST.y, follow);
	const spotBottom = lerp(IFRO_ROW.y + IFRO_ROW.h, SUGGEST.y + SUGGEST.h, lead);
	const spotX = lerp(IFRO_ROW.x, SUGGEST.x, (lead + follow) / 2);
	const spotW = lerp(IFRO_ROW.w, SUGGEST.w, (lead + follow) / 2);
	const spotRect: Rect = {x: spotX, y: spotTop, w: spotW, h: spotBottom - spotTop};
	const mint = ramp(f, SWAP, SWAP + 2) * (1 - ramp(f, SWAP + 2, SWAP + 10, E.enter));
	const chipLit = f >= MORPH ? 1 - ramp(f, RULES, RULES + 10, E.enter) : 0;
	const coverOn = swapped && f < RULES + 14;

	const afterLayer = (
		<>
			<Patches patches={afterPatches} />
			{coverOn ? <div style={{position: 'absolute', left: RULE_COVER.x - 2, top: RULE_COVER.y - 2, width: RULE_COVER.w + 4, height: RULE_COVER.h + 4, background: UI.card}} /> : null}
			{coverOn && f >= RULES
				? RULE_PIECES.map((r, i) => {
						const s = springAt(f, RULES + 2 * i, 'SNAPPY');
						return (
							<div
								key={i}
								style={{
									position: 'absolute',
									left: 0,
									top: 0,
									width: 2880,
									height: 1800,
									clipPath: insetOf(r),
									opacity: clamp01(s * 1.4),
									transform: `translateY(${(8 * (1 - s)).toFixed(2)}px)`,
								}}
							>
								<CaptureImg src={AFTER} />
							</div>
						);
					})
				: null}
			{inSwap ? (
				<>
					<BeforeSlice rect={LIST_BELOW} ty={-112 * e} opacity={1 - ramp(f, SWAP + 1, SWAP + SWAP_LEN)} patches={beforePatchEls} />
					<BeforeSlice rect={ROW1} tx={-140 * e} opacity={1 - ramp(f, SWAP, SWAP + SWAP_LEN, E.enter)} patches={beforePatchEls} />
				</>
			) : null}
			{mint > 0.001 ? (
				<div style={{position: 'absolute', left: NEW_ACTIVE.x, top: NEW_ACTIVE.y, width: NEW_ACTIVE.w, height: NEW_ACTIVE.h, background: 'rgba(46,204,143,0.12)', opacity: mint}} />
			) : null}
		</>
	);

	return (
		<TransitionOut>
			<TransitionIn>
				{() => (
					<Backdrop seed="s11">
						<G4Plane
							{...cam}
							layers={swapped ? [{src: AFTER, children: afterLayer}] : [{src: BEFORE, children: beforePatchEls}]}
							blur={12 * (1 - rack)}
							dim={0.4 * (1 - rack)}
						>
							{tint > 0.001 ? (
								<div
									style={{
										position: 'absolute',
										left: IFRO_ROW.x,
										top: IFRO_ROW.y,
										width: IFRO_ROW.w,
										height: IFRO_ROW.h,
										borderRadius: 12,
										background: 'rgba(77,141,255,0.16)',
										opacity: tint,
									}}
								/>
							) : null}
							<LitChip opacity={chipLit} label={KEY_LABEL} />
							<GlideSpot rect={spotRect} dim={spotDim} outline={spotDim / 0.45} pad={10} />
						</G4Plane>
						<KeyPool opacity={1 - ramp(f, 0, 8, E.enter)} />
						{f < MORPH ? <KeyCap3D pose={pose} label={KEY_LABEL} /> : null}
						<Scrim
							background="linear-gradient(90deg, rgba(10,13,22,0.85) 0px, rgba(10,13,22,0.8) 560px, rgba(10,13,22,0.35) 760px, rgba(10,13,22,0) 900px)"
							opacity={ramp(f, 6, 14, E.enter)}
						/>
						<StaggerHeadline lines={lines} size={88} left={144} capTops={[300, 400]} />
					</Backdrop>
				)}
			</TransitionIn>
		</TransitionOut>
	);
};

export default S11EEleAprende;
