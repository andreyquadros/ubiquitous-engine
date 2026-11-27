/**
 * s12-um-clique — S12 · abs 780–869 (90 f) · features · bar 14.1 → 15.3
 *
 * v2. f0 (abs 780, downbeat) hard cut on the review list (review-settled-
 * expanded, tilted rx 8° ry −13°, on screen ×1.1); the real "Confirmar os 19"
 * button LIFTS out of its bar as a hero card (LiftCard `lift`, SNAPPY, landed
 * ≈ f10) to 2.35× — label ≈ 56 px — floating in front of the window, which
 * steps back under a light navy veil. The bar keeps an empty socket
 * (LiftHole). f4–26 the cursor arcs in from off-frame bottom-right (E.cursor,
 * 12 % arc) onto the card; f24 hover lift. f30 CLICK (abs 810, beat 3): the
 * card presses 0.96 + turns mint, click ripple + a shockwave ring off the
 * card. f32 (C+2) hard swap to review-confirmed with the SAME camera; the card
 * DROPS (f32–44) into row 1's mint "você" badge as the camera pulls back
 * (E.push f32–50) to the column of "você"; a bright mint wave runs down the
 * rows (row wash + badge glow, 2-f stagger from f33). f30–40 "Um clique vira
 * memória." (Sora 700 116 px, "memória." volt) over a navy top band.
 * f50–89 hold: camera drift + tilt settle, the badge column breathes, a light
 * sweep crosses the window. Cut out.
 *
 * Claims: the keys legend is patched (widened, both captures); the confidence
 * percentages of the reviewed rows are patched in the BEFORE capture (a big
 * "100 %" beside "regra" would read as an accuracy claim, facts §6.13, same as
 * s08); the sidebar "Revisão" pending badge is covered; the toast ("21 blocos…")
 * stays out of frame; the settled header reads "(19)" = the button's number.
 */
import React from 'react';
import {AbsoluteFill, Easing} from 'remotion';
import {LiftCard, LiftHole, liftCardPose, type LiftCardProps} from '../components/LiftCard';
import {Screen, type ScreenConfig} from '../components/Screen';
import {navyDim} from '../components/Stage';
import {mapImageRect, mapWithGeometry, screenGeometry, type CameraKey, type Point, type Rect} from '../components/screen-geometry';
import {alpha, color} from '../design/tokens';
import {E, hotspot, Patches, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {
	arcPoint,
	ArrowCursor,
	Backdrop,
	ClickRipple,
	Crop,
	Finish,
	H,
	Headline,
	lerp,
	LightSweep,
	pressAt,
	ramp,
	Shockwave,
	unitsOf,
	W,
	widenLegend,
} from './_parts/G5/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{
		ref: 'whoosh_in_3.wav',
		atFrame: 2,
		gainDb: -14,
		note: 'v2: “Confirmar os 19” lifts out of its bar toward the camera. G5 fix: the whoosh PEAK (0.31 s into the file) sits on the lift’s fastest frame (SNAPPY from f0: Δ peaks f1–3, 94 % by f6), abs 782; the file starts in s11’s tail (abs 773). −22 → −14 dB: it was buried under the bed (peak ≈ −17.6 dBFS in the mix, was −25.6).',
	},
	{ref: 'click.wav', atFrame: 30, gainDb: -12, note: 'Confirmar os 19.'},
	{ref: 'shimmer_1.wav', atFrame: 33, gainDb: -14, note: 'All badges flip to “você” (C+3).'},
	{ref: 'ui_pop_1.wav', atFrame: 44, gainDb: -20, note: 'v2: the button card lands in row 1’s “você” badge.'},
];

const BEFORE = 'ui/review-settled-expanded.png';
const AFTER = 'ui/review-confirmed.png';
const COMP = {width: W, height: H};

const CLICK = 30;
const SWAP = 32;
const DROP_LEN = 12;
const LAND = SWAP + DROP_LEN;

const BUTTON = hotspot(BEFORE, 'confirm-all-button'); // 1785,446 226×64
const BADGE_NAV = hotspot(AFTER, 'nav-revisao-badge');
const ROW1 = hotspot(AFTER, 'reviewed-row-1'); // 514,432 1529×114
const ROW_H = 114;
/** The mint "você" origin badge of reviewed row i (measured on review-confirmed: x 1830–1939, y 468–507 in row 1). */
const voce = (i: number): Rect => ({x: 1830, y: 468 + ROW_H * i, w: 110, h: 40});
/**
 * G5 fix (proof ≥ 34 px): the badge's label is ≈ 23 image px, so even at the hold zoom (≈ 1.31 comp px per image px)
 * it reads ≈ 30 px. As the mint wave reaches a row its badge POPS (1 → 1.4) and settles SWOLLEN at 1.28× about its
 * centre (x 1815–1955: clear of the patched "100%" at x ≤ 1812 and of the chevron at x ≥ 1982): label ≈ 39 px.
 */
const CHIP_SWELL = 1.28;
const swellOf = (r: Rect, s: number): Rect => ({x: r.x + (r.w * (1 - s)) / 2, y: r.y + (r.h * (1 - s)) / 2, w: r.w * s, h: r.h * s});
/** Swell of row i's badge at frame f (the wave reaches row i at SWAP + 1 + 2i). */
const chipSwell = (i: number, f: number) => {
	const at = SWAP + 1 + 2 * i;
	const up = ramp(f, at, at + 3, E.push);
	const pop = ramp(f, at + 1, at + 3) * (1 - ramp(f, at + 3, at + 9, E.glide));
	return 1 + (CHIP_SWELL - 1) * up + 0.12 * pop;
};
/** Button crop: the face plus a little of the bar around it (#0e1524). */
const BTN_CROP: Rect = {x: BUTTON.x - 14, y: BUTTON.y - 12, w: BUTTON.w + 28, h: BUTTON.h + 24};
const BAR_BG = '#0e1524';
const ROW_BG = '#0c1220';
/** Card size: 2.35 comp px per image px → the 24-image-px label reads ≈ 61 px. */
const CARD_K = 2.55;
const CARD_SCALE = CARD_K * 2; // LiftCard scale is relative to a 1440-wide Screen (0.5 comp px per image px)
const CARD_AT = {x: 1130, y: 560};
/** Cursor tip on the label ("…os 19"), image px of the capture. */
const TIP: Point = {x: BUTTON.x + 150, y: BUTTON.y + 42};
const ENTER: Point = {x: 1990, y: 1140};
const REST: Point = {x: 1760, y: 960};

/** Confidence labels of the reviewed rows in the BEFORE capture (right edge per row; the origin badge differs). */
const PCT_RIGHT = [1800, 1831, 1762, 1762, 1831, 1762, 1800, 1762, 1831, 1831, 1831];
const pctPatches = PCT_RIGHT.map((r, i) => ({x: r - 72, y: 572 + ROW_H * i, w: 76, h: 28}));
/** After the confirm every row reads "100%" (x 1740–1804): patched too, so no column of "100 %" reads as an accuracy claim. */
const pctPatchesAfter = Array.from({length: 12}, (_, i) => ({x: 1730, y: 473 + ROW_H * i, w: 82, h: 32}));
/** v2 review: the picker's subtitle "Calendário, 5min" (x 2128–2317, y 183–206, both captures) contradicts s11's "Calendário → IFRO". */
const PICKER_SUB: Rect = {x: 2120, y: 176, w: 210, h: 38};
/** The two pending rows' "60%" / "68%" (IA suggestions; x 1783–1832, same place in both captures): patched too (v2 crit). */
const pctPatchesPending = [130, 242].map((y) => ({x: 1776, y, w: 62, h: 32}));

/** Scene grade: a touch brighter than the default window grade (the review list is the darkest UI in the film). */
const GRADE_S12 = {brightness: 1.3, lift: 0.07};

const HOLD_ZOOM = 2.6;
/** Row 1's top edge (image y 432) is pinned on canvas y 345, just under the headline band, for the whole hold. */
const ROW1_TOP_ON_CANVAS = 345;
const CAMERA: CameraKey[] = [
	{at: 0, zoom: 1.75, focus: {x: 1500, y: 580}, anchor: {x: 830, y: 610}, duration: 0},
	{at: SWAP, zoom: 1.86, focus: {x: 1500, y: 580}, anchor: {x: 830, y: 610}, duration: SWAP, easing: E.linear},
	// G5 fix (proof ≥ 34 px): push IN on the confirmed rows. Measured through the projection (image px → canvas px):
	// row titles (28 image px) 35.1 px at f50 → 35.6 px at f89, the badge labels (23 image px) 29.5 px × CHIP_SWELL ≈ 38 px.
	// The zoom pivots on row 1's top edge (focus y 432 → canvas y 345), so the rows never slide up under the band; the
	// row icons (image x 545) → swollen badges (x 1955) span canvas x ≈ 72–1866 at f50, 45–1863 at f89 (a slow pan
	// left + a 1 % push); the sidebar and the picker stay out of frame
	{at: 50, zoom: HOLD_ZOOM, focus: {x: 1265, y: 432}, anchor: {x: 975, y: ROW1_TOP_ON_CANVAS}, duration: 50 - SWAP, easing: E.push},
	{at: 89, zoom: HOLD_ZOOM * 1.01, focus: {x: 1275, y: 432}, anchor: {x: 975, y: ROW1_TOP_ON_CANVAS}, duration: 39, easing: E.linear},
];
/**
 * G5 fix (clean headline band): everything of the list card above row 1 (the two pending rows, the section header
 * "Classificados neste dia (19)" and its subline, x 514–2043, y ≤ 427; all #0c1220 in the capture) is flattened to
 * the card colour while it slides under the navy band (f38–50, during the push), so the band is clean navy — no ghost
 * rows, no half-dimmed subline at its lower edge.
 */
const ABOVE_ROW1: Rect = {x: 514, y: 0, w: 1530, h: 427};

const shotOf = (src: string): ScreenConfig => ({
	src,
	camera: CAMERA,
	// G5 fix: the tilt settles WITH the push (by f50), then only creeps, so the framing above holds through the hold
	rotateX: [
		[0, 9],
		[SWAP, 8],
		[50, 6, E.push],
		[89, 5.5, E.linear],
	],
	rotateY: [
		[0, -14],
		[SWAP, -12],
		[50, -6, E.push],
		[89, -5, E.linear], // v2 review: flatter at the hold (−8 → −5) so row 1's top edge stays level under the band
	],
	dots: 'neutral',
	radius: 16,
	sheenAt: 64,
	grade: GRADE_S12,
});
const SHOT_BEFORE = shotOf(BEFORE);
const SHOT_AFTER = shotOf(AFTER);

const DROP_EASE = Easing.bezier(0.55, 0, 0.25, 1);

/** The hero card while it is up (f0–31): lifts out of the bar, hovers, presses. */
const CARD_UP: LiftCardProps = {
	src: BEFORE,
	rect: BTN_CROP,
	at: 0,
	enter: 'lift',
	spring: 'snappy',
	x: [
		[0, CARD_AT.x],
		[CLICK, CARD_AT.x - 24],
	],
	y: CARD_AT.y,
	// hover lift f24–28, press on the click
	scale: [
		[0, CARD_SCALE],
		[24, CARD_SCALE * 1.01],
		[28, CARD_SCALE * 1.05, E.enter],
		[CLICK - 1, CARD_SCALE * 1.05],
		// v2 review: pressed ON the click frame (the transient is at f30.1)
		[CLICK, CARD_SCALE * 0.99, E.exit],
		[CLICK + 4, CARD_SCALE * 1.06, E.push],
	],
	rotateX: [
		[0, 12],
		[CLICK, 7, E.glide],
	],
	rotateY: [
		[0, -16],
		[CLICK, -9, E.glide],
	],
	radius: 22,
	float: 4,
	floatPeriod: 60,
	glow: 'volt',
	glowOpacity: 0.6,
	grade: GRADE_S12,
	from: (f) => mapImageRect(SHOT_BEFORE, f, COMP, BTN_CROP),
	style: {zIndex: 30},
};

/**
 * The card per frame: up (CARD_UP) until the swap, then it DROPS into row 1's "você" badge
 * (its in-window rect on the pulling-back camera), flattening to the window's plane, and fades as it lands.
 */
const cardAt = (f: number, fps: number): LiftCardProps => {
	if (f < SWAP) return CARD_UP;
	const p0 = liftCardPose(CARD_UP, SWAP - 1, fps);
	const t = DROP_EASE(ramp(f, SWAP, LAND));
	// row 1's badge is swollen (CHIP_SWELL) by the time the card lands in it
	const to = mapImageRect(SHOT_AFTER, f, COMP, swellOf(voce(0), chipSwell(0, f)));
	const w0 = p0.w * p0.s;
	return {
		...CARD_UP,
		enter: 'none',
		from: undefined,
		scale: undefined,
		width: Math.exp(lerp(Math.log(w0), Math.log(to.w * 1.06), t)),
		x: lerp(p0.cx, to.x + to.w / 2, t),
		y: lerp(p0.cy, to.y + to.h / 2, t),
		rotateX: lerp(p0.rx, 8, t),
		rotateY: lerp(p0.ry, -12, t),
		float: 0,
		glowOpacity: 0.6 * (1 - t),
		shadow: 1 - t,
		opacity: 1 - ramp(f, LAND - 3, LAND),
	};
};

/** Image-space overlays on the card's button face: hover rim, pressed volt, then mint "confirmed". */
const CardFace: React.FC<{f: number}> = ({f}) => {
	const hover = ramp(f, 24, 28, E.enter);
	const press = pressAt(f, CLICK);
	// v2 review: the colour flips on the click frame itself (0.5 at f30, full at f31)
	const mint = ramp(f, CLICK - 1, CLICK + 1, E.enter);
	// v2 crit: over the drop's last frames the face turns solid mint (the label goes), so it lands in the "você" badge as colour
	const solid = ramp(f, LAND - 8, LAND - 4, E.enter);
	return (
		<>
			<div
				style={{
					position: 'absolute',
					left: BUTTON.x,
					top: BUTTON.y,
					width: BUTTON.w,
					height: BUTTON.h,
					borderRadius: 12,
					background: mint > 0.001 ? alpha(color.mint, 0.2 * mint) : alpha(color.volt, 0.06 * hover + 0.16 * press),
					boxShadow: `inset 0 0 0 2px ${mint > 0.001 ? alpha(color.mint, 0.85 * mint) : alpha(color.volt, 0.7 * hover + 0.3 * press)}`,
				}}
			/>
			{solid > 0.001 ? (
				<div
					style={{
						position: 'absolute',
						left: BTN_CROP.x,
						top: BTN_CROP.y,
						width: BTN_CROP.w,
						height: BTN_CROP.h,
						borderRadius: 14,
						opacity: solid,
						background: '#12352f',
						boxShadow: `inset 0 0 0 3px ${alpha(color.mint, 0.9)}`,
					}}
				/>
			) : null}
		</>
	);
};

/** Mint wave: each reviewed row washes mint as the wave passes; its "você" badge pops, stays swollen and keeps a breathing glow. */
const MintWave: React.FC<{f: number}> = ({f}) => (
	<>
		{Array.from({length: 11}, (_, i) => {
			const at = SWAP + 1 + 2 * i;
			const hit = ramp(f, at, at + 2) * (1 - ramp(f, at + 2, at + 14, E.enter));
			const edge = ramp(f, at - 1, at + 1) * (1 - ramp(f, at + 1, at + 5));
			const settle = ramp(f, at + 2, at + 10);
			const breathe = 0.5 + 0.5 * Math.sin(((f - 50) / 36) * Math.PI * 2 - i * 0.55);
			const glow = f < at ? 0 : Math.max(hit, settle * (0.5 + 0.2 * (f >= 50 ? breathe : 0.5)));
			const wash = Math.max(hit, 0.35 * settle);
			const b = voce(i);
			const s = chipSwell(i, f);
			// the capture's own badge pixels (the @3x twin via Crop), scaled about the badge centre; drawn UNDER the wash
			// so the row tint covers badge and row alike (the crop's corner pixels are the row colour)
			const swell: React.CSSProperties = {position: 'absolute', left: b.x, top: b.y, width: b.w, height: b.h, transformOrigin: '50% 50%', transform: `scale(${s.toFixed(4)})`};
			return (
				<React.Fragment key={i}>
					{s > 1.0005 ? (
						<div style={swell}>
							<Crop src={AFTER} rect={b} at={{x: 0, y: 0}} />
						</div>
					) : null}
					{wash > 0.001 ? (
						<div style={{position: 'absolute', left: ROW1.x, top: ROW1.y + ROW_H * i, width: ROW1.w, height: ROW_H, background: `linear-gradient(90deg, ${alpha(color.mint, 0.04)} 0%, ${alpha(color.mint, 0.2)} 70%, ${alpha(color.mint, 0.3)} 100%)`, opacity: wash}} />
					) : null}
					{edge > 0.001 ? <div style={{position: 'absolute', left: ROW1.x, top: ROW1.y + ROW_H * i - 2, width: ROW1.w, height: 4, background: alpha(color.mint, 0.9), boxShadow: `0 0 18px ${alpha(color.mint, 0.8)}`, opacity: edge}} /> : null}
					{glow > 0.001 ? (
						<div style={swell}>
							<div style={{position: 'absolute', left: -2, top: -2, width: b.w + 4, height: b.h + 4, borderRadius: 12, boxShadow: `0 0 0 2px ${alpha(color.mint, 0.8 * glow)}, 0 0 26px 4px ${alpha(color.mint, 0.55 * glow)}`, background: alpha(color.mint, 0.1 * glow)}} />
						</div>
					) : null}
				</React.Fragment>
			);
		})}
	</>
);

const S12UmClique: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const fps = 30;
	const head = scene.copy.find((c) => c.role === 'headline')!;
	const units = unitsOf(head.text, [
		{text: 'Um clique', at: 30},
		{text: 'vira', at: 32},
		{text: 'memória.', at: 34, volt: head.emphasis.includes('memória.') ? 'memória.' : undefined},
	]);
	const before = f < SWAP;
	const shot = before ? SHOT_BEFORE : SHOT_AFTER;
	const g = screenGeometry(shot, f, COMP);

	// --- hero card + cursor ---------------------------------------------------
	const card = cardAt(f, fps);
	const pose = liftCardPose(card, f, fps);
	const tipOnCard = (fr: number): Point => {
		const p = liftCardPose(cardAt(fr, fps), fr, fps);
		const k = p.k * p.s;
		return {x: p.cx + (TIP.x - (BTN_CROP.x + BTN_CROP.w / 2)) * k, y: p.cy + (TIP.y - (BTN_CROP.y + BTN_CROP.h / 2)) * k};
	};
	let cur: Point;
	if (f <= 4) cur = ENTER;
	else if (f < 26) cur = arcPoint(ENTER, tipOnCard(26), E.cursor(ramp(f, 4, 26)), 0.12);
	else if (f <= 34) cur = tipOnCard(Math.min(f, CLICK + 2));
	else cur = arcPoint(tipOnCard(CLICK + 2), REST, E.cursor(ramp(f, 34, 44)), 0.12);
	const press = pressAt(f, CLICK);
	const cursorSize = 30 * 1.45 * (1 - 0.15 * press);
	const cursorOpacity = 1 - ramp(f, 38, 44, E.exit);
	const clickPt = tipOnCard(CLICK);

	// --- landing pulse on row 1's badge ------------------------------------------
	const landR = mapImageRect(SHOT_AFTER, f, COMP, voce(0));
	const landC = {x: landR.x + landR.w / 2, y: landR.y + landR.h / 2};

	// --- stage ----------------------------------------------------------------------
	const veil = 0.1 * ramp(f, 0, 8, E.enter) * (1 - ramp(f, SWAP, SWAP + 10, E.enter));
	const band = ramp(f, 24, 34, E.enter);
	const spill = ramp(f, 0, 10, E.enter) * (1 - ramp(f, SWAP, SWAP + 8, E.enter)) * (1 + 0.35 * ramp(f, CLICK, CLICK + 2) * (1 - ramp(f, CLICK + 2, SWAP + 6)));
	const pull = ramp(f, SWAP, 50, E.push);
	const headClean = ramp(f, 38, 50, E.enter);
	const badgeC = mapWithGeometry(g, {x: 1880, y: 700});
	const layerPatches = (file: string) => widenLegend(storyboardPatches(scene, file));

	return (
		<TransitionOut>
			<TransitionIn>
				<Backdrop
					seed="s12"
					volt={0.1}
					look={{
						// key pool behind the hero card, then behind the column of "você" as the camera pulls back
						keyPool: {x: lerp(CARD_AT.x / W, badgeC.x / W, pull), y: lerp(0.52, 0.6, pull), w: 0.8, h: 0.9, opacity: 0.44},
						keyLight: {x: lerp(CARD_AT.x / W, 0.55, pull), y: lerp(0.52, 0.6, pull), w: 0.55, h: 0.62, opacity: 0.16},
					}}
				>
					<Screen {...shot} style={{zIndex: 'auto'}}>
						<Patches patches={layerPatches(shot.src)} />
						{/* the sidebar's "Revisão" pending-count badge */}
						<div style={{position: 'absolute', left: BADGE_NAV.x - 3, top: BADGE_NAV.y - 3, width: BADGE_NAV.w + 6, height: BADGE_NAV.h + 6, background: '#14223c'}} />
						<div style={{position: 'absolute', left: PICKER_SUB.x, top: PICKER_SUB.y, width: PICKER_SUB.w, height: PICKER_SUB.h, background: ROW_BG}} />
						{pctPatchesPending.map((r, i) => (
							<div key={`p${i}`} style={{position: 'absolute', left: r.x, top: r.y, width: r.w, height: r.h, background: ROW_BG}} />
						))}
						{before ? (
							<>
								{pctPatches.map((r, i) => (
									<div key={i} style={{position: 'absolute', left: r.x, top: r.y, width: r.w, height: r.h, background: ROW_BG}} />
								))}
								<LiftHole rect={BTN_CROP} at={0} enter="lift" color={BAR_BG} pad={2} feather={10} radius={16} socket={0.6} />
							</>
						) : (
							<>
								{pctPatchesAfter.map((r, i) => (
									<div key={i} style={{position: 'absolute', left: r.x, top: r.y, width: r.w, height: r.h, background: ROW_BG}} />
								))}
								{headClean > 0.001 ? (
									<div style={{position: 'absolute', left: ABOVE_ROW1.x, top: ABOVE_ROW1.y, width: ABOVE_ROW1.w, height: ABOVE_ROW1.h, background: ROW_BG, opacity: headClean}} />
								) : null}
								<MintWave f={f} />
							</>
						)}
					</Screen>
					{/* the card is the light source: a soft volt/white spill on the window around it while it is up */}
					{spill > 0.001 ? (
						<div
							style={{
								position: 'absolute',
								left: pose.cx - 1050,
								top: pose.cy - 600,
								width: 2100,
								height: 1200,
								borderRadius: '50%',
								opacity: spill,
								mixBlendMode: 'screen',
								pointerEvents: 'none',
								background: `radial-gradient(closest-side, rgba(165,200,255,0.5) 0%, rgba(120,168,255,0.3) 38%, rgba(77,141,255,0) 100%)`,
							}}
						/>
					) : null}
					{/* the window steps back while the hero card is up */}
					{veil > 0.001 ? <AbsoluteFill style={{background: navyDim(veil), pointerEvents: 'none'}} /> : null}
					<LiftCard {...card}>
						<CardFace f={f} />
					</LiftCard>
					{/* navy band under the headline (the window's top rows step back) */}
					{band > 0.001 ? (
						<AbsoluteFill
							style={{
								opacity: band,
								pointerEvents: 'none',
								// v2 review: a clean band behind the headline (y 0–296 at ≈ 0.98; the section header sits under it), fading out above row 1 (y ≈ 340)
								background: `linear-gradient(180deg, ${navyDim(0.98)} 0px, ${navyDim(0.975)} 296px, ${navyDim(0.4)} 318px, ${navyDim(0)} 336px)`,
								zIndex: 31,
							}}
						/>
					) : null}
					<AbsoluteFill style={{zIndex: 32, pointerEvents: 'none'}}>
						<Headline units={units} size={124} left={104} capTop={100} />
						<Shockwave x={pose.cx} y={pose.cy} at={CLICK} radius={440} len={18} tint={color.mint} />
						<ClickRipple x={clickPt.x} y={clickPt.y} at={CLICK} scale={1.4} />
						{/* row 1's badge receives the card */}
						<Shockwave x={landC.x} y={landC.y} at={LAND - 1} radius={120} len={14} tint={color.mint} strength={0.9} />
						{f >= 4 ? <ArrowCursor x={cur.x} y={cur.y} size={cursorSize} opacity={cursorOpacity} /> : null}
						<LightSweep from={56} to={86} strength={0.08} />
					</AbsoluteFill>
					<Finish seed="s12" />
				</Backdrop>
			</TransitionIn>
		</TransitionOut>
	);
};

export default S12UmClique;
