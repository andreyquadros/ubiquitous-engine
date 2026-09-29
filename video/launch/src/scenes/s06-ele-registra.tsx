/**
 * s06-ele-registra — S06 · abs 360–449 (90 f) · features · bar 7.1 → 8.3
 *
 * f0 (downbeat) match cut: the frame is INSIDE the Hoje hero card at bitmap
 * 1:1 — the in-app UBI sits exactly where s05's 3D UBI stopped; f0–4 the 3D
 * UBI (idle 96…) crossfades into the bitmap, riding the camera. f0–24 pull-back
 * to the whole window (E.push), tilt settles to rx 6° ry −6°, the window grade
 * brightens (default GRADE on the match frame → 1.34 / lift 0.07 by f16); the
 * app's speech bubble pops (SNAPPY). f5–20 "Ele registra. Você trabalha."
 * (Sora 800 116 px) word stagger ("registra." volt gradient + glow). f24 sheen.
 *
 * v2: f26 the day-track strip (image x 1270–2300 = 08h → past 18h) LIFTS out of
 * the window as a 1740-px LiftCard centred under the headline (y 500) while the
 * window recedes (zoom 1 → 0.9, down, rx 13°, navy dim 0.24 + 2.2 px depth
 * blur); a LiftHole empties its slot in the window. f36–60 inside the card the
 * cover retracts 08h → 18h behind a volt playhead (E.glide 24 f): Tuesday
 * records itself block by block. f60 the playhead lands on the 18h tick,
 * pulses, the "18h" label lights up, the card bumps and a light sweeps across
 * it; f60–89 slow push (card width +30, camera 0.9 → 0.93), float, glow breath
 * on f75. f86–89 whip-left out-half (card, window and headline together).
 */
import React from 'react';
import {AbsoluteFill, Img, staticFile} from 'remotion';
import {LiftCard, LiftHole} from '../components/LiftCard';
import {GRADE, mapImageRect, mapWithGeometry, screenGeometry, type CameraKey, type ScreenConfig} from '../components/screen-geometry';
import {navyDim, STAGE} from '../components/Stage';
import type {Keyframe} from '../design/motion';
import {color, font} from '../design/tokens';
import {
	E,
	springAt,
	storyboardCamera,
	TransitionIn,
	TransitionOut,
	UbiClip,
	useScene,
	useSceneFrame,
	type SfxCue,
} from '../shared';
import {Backdrop, clamp01, HERO_CARD, lerp, ramp, UBI_MATCH, UBI_MATCH_IMG, W, H} from './_parts/G2/common';
import {G2Screen} from './_parts/G2/G2Screen';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'swoosh_long_3.wav', atFrame: 12, gainDb: -10, note: 'Deep pull-back for the hero reveal; the file starts 6 f before the scene (master track).'},
	{ref: 'sweep_1.wav', atFrame: 36, gainDb: -16, note: 'One tick cluster for the whole track build.'},
	{ref: 'ding_2.wav', atFrame: 60, gainDb: -12, note: 'Playhead lands on 18h (C6).'},
	{ref: 'whoosh_in_3.wav', atFrame: 34, gainDb: -20, note: 'v2: the day-track strip lifts out of the window toward the camera (arrives ≈ f34).'},
];

const FILE = 'ui/dashboard.png';

/* ---- day track, measured on ui/dashboard.png (image px) --------------- */
const TRACK = {
	/** interior between the 2-px border lines */
	top: 1180,
	bottom: 1244,
	/** cover's right end (left of the rounded right border, no blocks beyond) */
	right: 2772,
	/** 08h (first block starts at 1300) and the 18h tick (2230–2231; the last block before 18h ends at 2231) */
	x08: 1299,
	x18: 2232,
	fill: '#121a2b',
	tickColor: '#212c43',
	/** 3-hour ticks (2 px) inside the track right of 08h */
	ticks: [1394, 1672, 1952, 2230, 2510],
};
const BUILD_START = 36;
const BUILD_END = 60;

/* ---- the app's speech bubble (pop-in): its box incl. tail + shadow ----- */
const BUBBLE = {x: 2196, y: 262, w: 630, h: 284, tail: {x: 2506, y: 452}}; // incl. its soft drop shadow; stops above UBI's head (552)
const BUBBLE_AT = 3;
const BUBBLE_UNTIL = 24;
/** In-app UBI ink box (hidden for the whole shot: the 3D UBI stands in for him, see UBI_HOLD). */
const APP_UBI = {x: 2370, y: 520, w: 280, h: 384};
/** v2 review: the APP_UBI patch is a copy of the empty hero card 260 px to his left (x 2110–2390; stat ink ends at 2101, his ink starts at 2400),
 *  so it carries the card's own gradient and grade (a flat div read as a faint box under the brightened grade). */
const APP_UBI_SRC_DX = -260;
/** His ember floor glow (x 2378–2634, y 890–958, measured on the capture; the card border sits at y 992). v2 review:
 *  the ink is already hidden by APP_UBI, so this only takes the glow, as a copy of the empty card 300 px to its left
 *  (x 2048–2364, y 850–978: below the stat ink, which ends at y 796). */
const APP_UBI_AWAY = {x: 2348, y: 850, w: 316, h: 128};
const APP_GLOW_SRC_DX = -300;
/** 16-px feathered edge on all four sides (no hard or shadowed edge to read as a patch). */
const AWAY_MASK =
	'linear-gradient(90deg, transparent 0px, #000 16px, #000 calc(100% - 16px), transparent 100%), linear-gradient(180deg, transparent 0px, #000 16px, #000 calc(100% - 16px), transparent 100%)';

/**
 * v2 review (major: ghosted double UBI at abs 361–362). The 3D idle pose and the app's static UBI art are
 * different drawings (≈ 30 px apart in head/arm placement even on the match frame), so ANY crossfade between
 * them shows two robots. The 3D UBI now stays the only UBI: it rides the window through the pull-back (feet
 * pinned on the in-app UBI's feet, size from the projected height, so it tracks tilt/perspective), keeps
 * idling, and fades out f25–30 (E.enter: most of it in the first 2 f), before the lifting day-track card reaches
 * him, so the card never slices him (critique r1).
 */
const UBI_OUT = {start: 25, end: 30};
/** In-app UBI ink height (image px, 552 → 890): the projected length of this span sizes the 3D UBI. */
const UBI_INK_H = 338;

/**
 * v2 review (major: demo numbers as the subject of the match frame). Depth-of-field veils over the Resumo card's
 * stat sentence, review pill, "24min sem categoria" and the four metric values (ink x 1011–2101, y 435–796) and over
 * the 88 ring's interior (ring ⌀ 343 at (761.5, 611.5), stroke inner edge r ≈ 148): a blurred copy of the same
 * capture (so the card gradient and grade stay continuous), feathered, slightly dimmed. The focus is UBI; the card
 * reads as soft UI, never as a sentence or a number. The ring stroke itself stays sharp.
 */
const STAT_VEIL = {x: 940, y: 340, w: 1240, h: 560, blur: 15, dim: 0.16, feather: 80};
const RING_VEIL = {cx: 761.5, cy: 611.5, r: 142, blur: 24, dim: 0.4};
/** Sidebar "Revisão" pending-count badge (same patch as G3 s07/s08): the film never shows a pending number. */
const REVIEW_COUNT_PATCH = {x: 370, y: 372, w: 50, h: 48, fill: '#0a101c'};

/** A blurred, dimmed copy of the capture inside `rect` (image px), masked by `mask`. */
const Veil: React.FC<{x: number; y: number; w: number; h: number; blur: number; dim: number; mask: string; radius?: number | string}> = ({
	x,
	y,
	w,
	h,
	blur,
	dim,
	mask,
	radius,
}) => {
	const pad = Math.ceil(blur * 2.5);
	return (
		<div style={{position: 'absolute', left: x, top: y, width: w, height: h, overflow: 'hidden', borderRadius: radius, WebkitMaskImage: mask, maskImage: mask, WebkitMaskComposite: 'source-in', maskComposite: 'intersect'}}>
			<div style={{position: 'absolute', left: -pad, top: -pad, width: w + 2 * pad, height: h + 2 * pad, overflow: 'hidden', filter: `blur(${blur}px)`}}>
				<Img src={staticFile(FILE)} style={{position: 'absolute', left: -(x - pad), top: -(y - pad), width: 2880, height: 1800, maxWidth: 'none'}} />
			</div>
			<div style={{position: 'absolute', inset: 0, background: HERO_CARD, opacity: dim}} />
		</div>
	);
};

const feather = (px: number) =>
	`linear-gradient(90deg, transparent 0px, #000 ${px}px, #000 calc(100% - ${px}px), transparent 100%), linear-gradient(180deg, transparent 0px, #000 ${px}px, #000 calc(100% - ${px}px), transparent 100%)`;

/** Demo-number veils + nav badge patch (children of the screen, image px), on for the whole shot. */
const StatVeils: React.FC = () => (
	<>
		<Veil {...STAT_VEIL} mask={feather(STAT_VEIL.feather)} />
		<Veil
			x={RING_VEIL.cx - RING_VEIL.r}
			y={RING_VEIL.cy - RING_VEIL.r}
			w={2 * RING_VEIL.r}
			h={2 * RING_VEIL.r}
			blur={RING_VEIL.blur}
			dim={RING_VEIL.dim}
			radius="50%"
			mask="radial-gradient(closest-side, #000 86%, transparent 100%)"
		/>
		<div style={{position: 'absolute', left: REVIEW_COUNT_PATCH.x, top: REVIEW_COUNT_PATCH.y, width: REVIEW_COUNT_PATCH.w, height: REVIEW_COUNT_PATCH.h, background: REVIEW_COUNT_PATCH.fill}} />
	</>
);

const playheadX = (f: number) => lerp(TRACK.x08, TRACK.x18, ramp(f, BUILD_START, BUILD_END, E.glide));

/** Hero-card overlays glued to the dashboard (children of the screen): the bubble pop and the 3D → bitmap UBI cover. */
const HeroOverlays: React.FC<{f: number}> = ({f}) => {
	const away = ramp(f, UBI_OUT.start, UBI_OUT.end, E.enter); // his ember floor glow leaves with the 3D UBI
	const bubbleP = f < BUBBLE_AT ? 0 : springAt(f, BUBBLE_AT, 'SNAPPY');
	const bubbleOn = f <= BUBBLE_UNTIL;
	return (
		<>
			{/* speech bubble: hidden on the match frame, pops from its tail as the camera pulls back */}
			{bubbleOn ? (
				<>
					<div style={{position: 'absolute', left: BUBBLE.x, top: BUBBLE.y, width: BUBBLE.w, height: BUBBLE.h, background: HERO_CARD}} />
					{bubbleP > 0.001 ? (
						<div
							style={{
								position: 'absolute',
								left: BUBBLE.x,
								top: BUBBLE.y,
								width: BUBBLE.w,
								height: BUBBLE.h,
								overflow: 'hidden',
								opacity: clamp01(bubbleP * 2.5),
								transformOrigin: `${BUBBLE.tail.x - BUBBLE.x}px ${BUBBLE.tail.y - BUBBLE.y}px`,
								transform: `scale(${lerp(0.6, 1, bubbleP).toFixed(4)})`,
							}}
						>
							<Img src={staticFile(FILE)} style={{position: 'absolute', left: -BUBBLE.x, top: -BUBBLE.y, width: 2880, height: 1800, maxWidth: 'none'}} />
						</div>
					) : null}
				</>
			) : null}
			{/* v2 review: the in-app UBI stays hidden for the whole shot under a copy of the empty card to his left
			    (the 3D UBI stands in for him; no crossfade between two different drawings) */}
			<div
				style={{
					position: 'absolute',
					left: APP_UBI.x,
					top: APP_UBI.y,
					width: APP_UBI.w,
					height: APP_UBI.h,
					overflow: 'hidden',
					WebkitMaskImage: AWAY_MASK,
					maskImage: AWAY_MASK,
					WebkitMaskComposite: 'source-in',
					maskComposite: 'intersect',
				}}
			>
				<Img
					src={staticFile(FILE)}
					style={{position: 'absolute', left: -(APP_UBI.x + APP_UBI_SRC_DX), top: -APP_UBI.y, width: 2880, height: 1800, maxWidth: 'none'}}
				/>
			</div>
			{/* critique r1 / v2 review: his ember floor glow leaves with the 3D UBI (f25–30); on through f89 */}
			{away > 0.001 ? (
				<div
					style={{
						position: 'absolute',
						left: APP_UBI_AWAY.x,
						top: APP_UBI_AWAY.y,
						width: APP_UBI_AWAY.w,
						height: APP_UBI_AWAY.h,
						overflow: 'hidden',
						opacity: away,
						WebkitMaskImage: AWAY_MASK,
						maskImage: AWAY_MASK,
						WebkitMaskComposite: 'source-in',
						maskComposite: 'intersect',
					}}
				>
					<Img
						src={staticFile(FILE)}
						style={{position: 'absolute', left: -(APP_UBI_AWAY.x + APP_GLOW_SRC_DX), top: -APP_UBI_AWAY.y, width: 2880, height: 1800, maxWidth: 'none'}}
					/>
				</div>
			) : null}
		</>
	);
};

/**
 * Day-track recording, image px of the FULL capture (drawn inside the lifted card; sized for the card's
 * 1.75 comp px per image px): the cover right of the playhead (for good right of 18h, incl. the app's
 * now-marker), the write-head wash, the volt playhead with its landing pulse/ring, the "18h" label glow
 * and a light sweep across the card on the landing.
 */
const TrackOverlays: React.FC<{f: number}> = ({f}) => {
	const e = playheadX(f);
	const building = f >= BUILD_START - 2 && f <= BUILD_END + 2;
	const ph = ramp(f, BUILD_START - 6, BUILD_START, E.enter); // playhead fades in at 08h just before the build
	// landing pulse on f60 (dot scale 1 → 1.7 → 1) and a breath on f75
	const pulse = f >= BUILD_END ? Math.sin(Math.PI * ramp(f, BUILD_END, BUILD_END + 10, E.linear)) : 0;
	const breath = f >= 75 ? Math.sin(Math.PI * ramp(f, 75, 89, E.linear)) : 0;
	const glowK = 1 + 0.9 * pulse + 0.55 * breath;
	const ring = f >= BUILD_END && f < BUILD_END + 16 ? ramp(f, BUILD_END, BUILD_END + 16, E.push) : -1;
	const labelIn = ramp(f, BUILD_END - 2, BUILD_END + 4, E.enter);
	const label = labelIn * (0.75 + 0.25 * breath);
	const sweep = ramp(f, BUILD_END, BUILD_END + 16, E.glide);
	const TH = TRACK.bottom - TRACK.top;
	return (
		<>
			<div style={{position: 'absolute', left: e, top: TRACK.top, width: TRACK.right - e, height: TH, background: TRACK.fill}} />
			{TRACK.ticks
				.filter((t) => t >= e)
				.map((t) => (
					<div key={t} style={{position: 'absolute', left: t, top: TRACK.top, width: 2, height: TH, background: TRACK.tickColor}} />
				))}
			{/* write-head: a volt wash just behind the playhead while it records */}
			{building && e > TRACK.x08 + 1 ? (
				<div
					style={{
						position: 'absolute',
						left: Math.max(TRACK.x08, e - 110),
						top: TRACK.top,
						width: Math.min(110, e - TRACK.x08),
						height: TH,
						background: 'linear-gradient(90deg, rgba(77,141,255,0) 0%, rgba(120,170,255,0.45) 100%)',
						mixBlendMode: 'screen',
						opacity: 1 - ramp(f, BUILD_END - 2, BUILD_END + 2, E.linear),
					}}
				/>
			) : null}
			{/* "18h" lights up as the playhead lands on it (critique r1: a live white label over a patched bitmap glyph,
			    with a dim volt halo UNDER it, so the payoff hour is the most legible one on the card, not a blue smudge) */}
			{label > 0.003 ? (
				<>
					<div style={{position: 'absolute', left: LABEL_18.x - 36, top: LABEL_18.y - 15, width: 72, height: 30, background: TRACK_LABEL_BG, opacity: clamp01(labelIn * 3)}} />
					<div
						style={{
							position: 'absolute',
							left: LABEL_18.x - 64,
							top: LABEL_18.y - 26,
							width: 128,
							height: 52,
							borderRadius: 26,
							opacity: label,
							background: 'radial-gradient(closest-side, rgba(77,141,255,0.24) 0%, rgba(77,141,255,0.12) 60%, rgba(77,141,255,0) 100%)',
						}}
					/>
					<div
						style={{
							position: 'absolute',
							left: LABEL_18.x - 60,
							width: 120,
							top: LABEL_18.baseline - LABEL_18.size * 0.864,
							height: LABEL_18.size,
							lineHeight: 1,
							textAlign: 'center',
							fontFamily: font.text,
							fontWeight: 600,
							fontSize: LABEL_18.size,
							color: '#ffffff',
							textShadow: `0 0 ${(10 + 4 * breath).toFixed(1)}px rgba(77,141,255,0.8)`,
							opacity: labelIn,
							whiteSpace: 'nowrap',
						}}
					>
						18h
					</div>
				</>
			) : null}
			{/* playhead: 3 image px (≈ 5 comp px on the card), glow, dot on top */}
			{ph > 0.001 ? (
				<div style={{position: 'absolute', left: e - 1.5, top: TRACK.top - 12, width: 3, height: TH + 24, opacity: ph}}>
					<div
						style={{
							position: 'absolute',
							inset: 0,
							borderRadius: 2,
							background: '#8db6ff',
							boxShadow: `0 0 ${(18 * glowK).toFixed(1)}px ${(2.5 * glowK).toFixed(1)}px rgba(77,141,255,${(0.6 * Math.min(1.6, glowK)).toFixed(3)})`,
						}}
					/>
					<div
						style={{
							position: 'absolute',
							left: 1.5 - 7,
							top: -7,
							width: 14,
							height: 14,
							borderRadius: 7,
							background: color.volt,
							boxShadow: `0 0 ${(12 * glowK).toFixed(1)}px rgba(77,141,255,0.85)`,
							transform: `scale(${(1 + 0.7 * pulse).toFixed(3)})`,
						}}
					/>
					{ring >= 0 && ring < 1 ? (
						<div
							style={{
								position: 'absolute',
								left: 1.5 - lerp(7, 40, ring),
								top: -lerp(7, 40, ring),
								width: lerp(14, 80, ring),
								height: lerp(14, 80, ring),
								borderRadius: '50%',
								border: `2px solid rgba(120,170,255,${(0.8 * (1 - ring)).toFixed(3)})`,
							}}
						/>
					) : null}
				</div>
			) : null}
			{/* landing sweep across the card */}
			{sweep > 0 && sweep < 1 ? (
				<div
					style={{
						position: 'absolute',
						left: lerp(CROP.x - 300, CROP.x + CROP.w + 100, sweep),
						top: CROP.y - 40,
						width: 150,
						height: CROP.h + 80,
						transform: 'skewX(-20deg)',
						background: 'linear-gradient(90deg, rgba(255,255,255,0) 0%, rgba(220,232,255,0.13) 50%, rgba(255,255,255,0) 100%)',
					}}
				/>
			) : null}
		</>
	);
};

/** v2 headline: Sora 800 116 px (≥ 112), −0.04em, white top→bottom gradient, "registra." a volt gradient with a soft glow. */
const HEAD_SIZE = 116;
const Headline: React.FC<{f: number; text: string; emphasis: string[]}> = ({f, text, emphasis}) => {
	const size = HEAD_SIZE;
	const words = text.split(' ');
	return (
		<div
			style={{
				position: 'absolute',
				left: 0,
				width: W,
				top: Math.round(HEAD_TOP - 0.1 * size),
				textAlign: 'center',
				fontFamily: font.display,
				fontWeight: 800,
				fontSize: size,
				lineHeight: 1.1,
				letterSpacing: '-0.04em',
				whiteSpace: 'nowrap',
			}}
		>
			{words.map((w, i) => {
				const at = 5 + 3 * i;
				const sp = f < at ? 0 : springAt(f, at, 'SNAPPY');
				const o = clamp01((f - at + 1) / 6);
				const blur = 8 * clamp01(1 - sp);
				const volt = emphasis.includes(w) ? clamp01((sp - 0.55) / 0.4) : 0;
				const top = volt > 0 ? mix('#ffffff', '#8fb8ff', volt) : '#ffffff';
				const bottom = volt > 0 ? mix('#d3dcef', color.volt, volt) : '#d3dcef';
				const glow = volt > 0 ? `drop-shadow(0 0 ${(22 * volt).toFixed(1)}px rgba(77,141,255,${(0.45 * volt).toFixed(3)}))` : '';
				const filter = [blur > 0.15 ? `blur(${blur.toFixed(2)}px)` : '', glow].filter(Boolean).join(' ');
				return (
					<React.Fragment key={i}>
						<span
							style={{
								display: 'inline-block',
								opacity: o,
								transform: `translateY(${(32 * (1 - sp)).toFixed(2)}px)`,
								filter: filter || undefined,
								color: 'transparent',
								backgroundImage: `linear-gradient(180deg, ${top} 18%, ${bottom} 92%)`,
								WebkitBackgroundClip: 'text',
								backgroundClip: 'text',
								paddingBottom: '0.08em',
							}}
						>
							{w}
						</span>
						{i < words.length - 1 ? ' ' : null}
					</React.Fragment>
				);
			})}
		</div>
	);
};

const mix = (a: string, b: string, t: number) => {
	const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
	const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
	return `rgb(${pa.map((v, i) => Math.round(v + (pb[i] - v) * t)).join(', ')})`;
};

/** Headline cap-top (comp px). */
const HEAD_TOP = 58;

/** v2 top scrim: navy (not canvas black), only as deep as the headline band; the stage shows once the window recedes. */
const Scrim: React.FC<{f: number}> = ({f}) => {
	const on = ramp(f, 2, 14, E.enter);
	if (on <= 0.001) return null;
	return (
		<AbsoluteFill
			style={{
				opacity: on,
				background: `linear-gradient(180deg, ${navyDim(0.9)} 0px, ${navyDim(0.78)} 170px, ${navyDim(0.3)} 250px, ${navyDim(0)} 320px)`,
			}}
		/>
	);
};

const COMP = {width: W, height: H};

/** The lifted day-track strip: image x 1270–2300 (just before 08h → past the 18h tick), y 1140–1300 (playhead dot → hour labels). */
const CROP = {x: 1270, y: 1140, w: 1030, h: 160};
/** "18h" label (image px): centre, glyph baseline (digits 1265–1280) and the Inter size that matches its 16-px digits. */
const LABEL_18 = {x: 2222, y: 1273, baseline: 1281, size: 22};
/** Flat panel colour under the hour labels (sampled). */
const TRACK_LABEL_BG = '#0c1220';
const LIFT_AT = 26;
const CARD_W = 1740;
const TRACK_LIFT = {rect: CROP, at: LIFT_AT, enter: 'lift' as const, spring: 'smooth' as const};

/** v2 window grade: the default GRADE on the match frame (the s05 tint matches it), brighter by f16. */
const gradeAt = (f: number) => {
	const t = ramp(f, 3, 16, E.enter);
	return {brightness: lerp(GRADE.brightness, 1.34, t), contrast: GRADE.contrast, saturate: lerp(GRADE.saturate, 1.22, t), lift: lerp(GRADE.lift, 0.07, t)};
};
const CARD_GRADE = gradeAt(99);

const S06EleRegistra: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const headline = scene.copy[0];
	const sb = storyboardCamera(scene, {file: FILE});
	// v2 camera: the storyboard's match frame and pull-back (f0–24) are kept; then the window RECEDES
	// (zoom 1 → 0.9, down, more tilt) while the day-track strip lifts out of it toward the camera.
	const camera: CameraKey[] = [
		...sb.camera.filter((k) => k.at <= 24),
		{at: 60, zoom: 0.9, focus: {x: 1440, y: 980}, anchor: {x: 960, y: 760}, duration: 34, easing: E.glide},
		{at: 89, zoom: 0.93, focus: {x: 1440, y: 980}, anchor: {x: 960, y: 760}, duration: 29, easing: E.linear},
	];
	const rotateX: Keyframe[] = [...sb.rotateX.filter((k) => k[0] <= 24), [26, 6], [60, 13, E.glide], [89, 14]];
	const rotateY: Keyframe[] = [...sb.rotateY.filter((k) => k[0] <= 24), [26, -6], [60, -3, E.glide], [89, -2]];
	const shot: ScreenConfig = {
		src: FILE,
		width: 1440,
		radius: 18,
		float: 4,
		sheenAt: 24,
		glow: 'volt',
		camera,
		rotateX,
		rotateY,
		grade: gradeAt(f),
	};

	// v2 review: the 3D UBI is the only UBI; it rides the window (feet pinned on the in-app UBI's feet, size from the
	// projected ink height so tilt/perspective are tracked; identical to the old formula on the match frame) and
	// leaves as the day-track card lifts over him
	const g = screenGeometry(shot, f, COMP);
	const g0 = screenGeometry(shot, 0, COMP);
	const feet = mapWithGeometry(g, UBI_MATCH_IMG);
	const projH = (gg: typeof g) => {
		const a = mapWithGeometry(gg, UBI_MATCH_IMG);
		const b = mapWithGeometry(gg, {x: UBI_MATCH_IMG.x, y: UBI_MATCH_IMG.y - UBI_INK_H});
		return Math.hypot(a.x - b.x, a.y - b.y);
	};
	const ubiSize = UBI_MATCH.size * g0.k * g0.scale * g0.s0 * (projH(g) / projH(g0));
	const ubiO = 1 - ramp(f, UBI_OUT.start, UBI_OUT.end, E.enter);

	// the window steps back once the strip lifts: navy dim + depth blur
	const back = ramp(f, LIFT_AT + 6, LIFT_AT + 32, E.glide); // trails the lift so the lit window carries f26–40
	const cardGlow = 0.55 + 0.3 * (f >= BUILD_END ? Math.sin(Math.PI * ramp(f, BUILD_END, BUILD_END + 14, E.linear)) : 0);

	return (
		<AbsoluteFill style={{backgroundColor: STAGE.bottom}}>
			<Backdrop
				seed="s06"
				look={{
					keyPool: {x: 0.5, y: 0.5, w: 0.9, h: 0.9, opacity: 0.44},
					keyLight: {x: 0.5, y: 0.47, w: 0.62, h: 0.5, opacity: 0.17},
				}}
				orbs={[
					{c: color.volt, x: 420, y: 260, d: 1200, opacity: 0.2},
					{c: color.ember, x: 1650, y: 910, d: 960, opacity: 0.1},
				]}
				floor={0.3}
				topLight={1}
			/>
			<TransitionIn>
				{() => (
					<>
						<TransitionOut>
							<AbsoluteFill style={{filter: back > 0.01 ? `blur(${(2.2 * back).toFixed(2)}px)` : undefined}}>
								<G2Screen {...shot} style={{zIndex: 'auto'}}>
									<StatVeils />
									<HeroOverlays f={f} />
									<TrackOverlays f={f} />
									<LiftHole {...TRACK_LIFT} color={HERO_CARD} radius={20} pad={4} socket={0.6} />
								</G2Screen>
								{back > 0.001 ? <AbsoluteFill style={{background: navyDim(0.2 * back)}} /> : null}
							</AbsoluteFill>
							{ubiO > 0.001 ? (
								<UbiClip clip="idle" index={(96 + f) % 120} x={feet.x} y={feet.y} size={ubiSize} anchor="feet" opacity={ubiO} style={{zIndex: 1}} />
							) : null}
						</TransitionOut>
						<Scrim f={f} />
						<TransitionOut>
							<LiftCard
								src={FILE}
								{...TRACK_LIFT}
								from={(fr) => mapImageRect(shot, fr, COMP, CROP)}
								x={[
									[LIFT_AT, 960],
									[89, 948],
								]}
								y={[
									[LIFT_AT, 500],
									[89, 492],
								]}
								width={[
									[LIFT_AT, CARD_W],
									[BUILD_END, CARD_W],
									[BUILD_END + 3, CARD_W + 26],
									[BUILD_END + 12, CARD_W + 8],
									[89, CARD_W + 30],
								]}
								rotateX={[
									[LIFT_AT, 16],
									[BUILD_END, 9],
									[89, 7],
								]}
								rotateY={[
									[LIFT_AT, -9],
									[BUILD_END, -4],
									[89, -2.5],
								]}
								float={4}
								floatPeriod={90}
								radius={18}
								glowOpacity={cardGlow}
								grade={CARD_GRADE}
								style={{zIndex: 30}}
							>
								<TrackOverlays f={f} />
							</LiftCard>
						</TransitionOut>
						<TransitionOut>
							<Headline f={f} text={headline.text} emphasis={headline.emphasis} />
						</TransitionOut>
					</>
				)}
			</TransitionIn>
		</AbsoluteFill>
	);
};

export default S06EleRegistra;
