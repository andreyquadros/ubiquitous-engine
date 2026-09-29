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
	{ref: 'whoosh_in_3.wav', atFrame: 10, gainDb: -22, note: 'v2: “Confirmar os 19” lifts out of its bar toward the camera (lands ≈ f10).'},
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
/** The mint "você" origin badge of reviewed row i (measured on review-confirmed: x 1830–1939, 40 px tall). */
const voce = (i: number): Rect => ({x: 1830, y: 468 + ROW_H * i, w: 110, h: 40});
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

/** Scene grade: a touch brighter than the default window grade (the review list is the darkest UI in the film). */
const GRADE_S12 = {brightness: 1.3, lift: 0.07};

const CAMERA: CameraKey[] = [
	{at: 0, zoom: 1.75, focus: {x: 1500, y: 580}, anchor: {x: 830, y: 610}, duration: 0},
	{at: SWAP, zoom: 1.86, focus: {x: 1500, y: 580}, anchor: {x: 830, y: 610}, duration: SWAP, easing: E.linear},
	// reframe on the column of "você" (the window's top rows sit under the headline band)
	{at: 50, zoom: 1.84, focus: {x: 1300, y: 760}, anchor: {x: 960, y: 690}, duration: 50 - SWAP, easing: E.push},
	{at: 89, zoom: 1.92, focus: {x: 1285, y: 760}, anchor: {x: 960, y: 690}, duration: 39, easing: E.linear},
];

const shotOf = (src: string): ScreenConfig => ({
	src,
	camera: CAMERA,
	rotateX: [
		[0, 9],
		[SWAP, 8],
		[89, 5.5, E.glide],
	],
	rotateY: [
		[0, -14],
		[SWAP, -12],
		[89, -8, E.glide],
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
		[CLICK + 1, CARD_SCALE * 1.0, E.exit],
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
	const to = mapImageRect(SHOT_AFTER, f, COMP, voce(0));
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
	const mint = ramp(f, CLICK, CLICK + 3, E.enter);
	return (
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
	);
};

/** Mint wave: each reviewed row washes mint as the wave passes; its "você" badge glows and keeps a breathing glow. */
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
			return (
				<React.Fragment key={i}>
					{wash > 0.001 ? (
						<div style={{position: 'absolute', left: ROW1.x, top: ROW1.y + ROW_H * i, width: ROW1.w, height: ROW_H, background: `linear-gradient(90deg, ${alpha(color.mint, 0.04)} 0%, ${alpha(color.mint, 0.2)} 70%, ${alpha(color.mint, 0.3)} 100%)`, opacity: wash}} />
					) : null}
					{edge > 0.001 ? <div style={{position: 'absolute', left: ROW1.x, top: ROW1.y + ROW_H * i - 2, width: ROW1.w, height: 4, background: alpha(color.mint, 0.9), boxShadow: `0 0 18px ${alpha(color.mint, 0.8)}`, opacity: edge}} /> : null}
					{glow > 0.001 ? (
						<div style={{position: 'absolute', left: b.x - 2, top: b.y - 2, width: b.w + 4, height: b.h + 4, borderRadius: 12, boxShadow: `0 0 0 2px ${alpha(color.mint, 0.8 * glow)}, 0 0 26px 4px ${alpha(color.mint, 0.55 * glow)}`, background: alpha(color.mint, 0.1 * glow)}} />
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
								background: `linear-gradient(180deg, ${navyDim(0.92)} 0px, ${navyDim(0.86)} 230px, ${navyDim(0.5)} 300px, ${navyDim(0)} 380px)`,
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
