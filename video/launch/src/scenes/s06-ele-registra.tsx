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
/** In-app UBI ink box (patched during the 3D → bitmap crossfade). */
const APP_UBI = {x: 2392, y: 540, w: 236, h: 348};
const XFADE = 4;

/** 3D → bitmap crossfade over f0–4 (1, .56, .25, .06, 0): quadratic so the double image barely shows while the camera races. */
const xfade = (f: number) => clamp01(1 - f / XFADE) ** 2;

const playheadX = (f: number) => lerp(TRACK.x08, TRACK.x18, ramp(f, BUILD_START, BUILD_END, E.glide));

/** Hero-card overlays glued to the dashboard (children of the screen): the bubble pop and the 3D → bitmap UBI cover. */
const HeroOverlays: React.FC<{f: number}> = ({f}) => {
	const xf = xfade(f);
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
			{/* in-app UBI hidden under the 3D UBI on the cut, revealed over the 4-f crossfade */}
			{xf > 0.001 ? (
				<div style={{position: 'absolute', left: APP_UBI.x, top: APP_UBI.y, width: APP_UBI.w, height: APP_UBI.h, background: HERO_CARD, opacity: xf, borderRadius: 40, boxShadow: `0 0 16px 8px ${HERO_CARD}`}} />
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
	const label = ramp(f, BUILD_END - 2, BUILD_END + 4, E.enter) * (0.75 + 0.25 * breath);
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
			{/* "18h" lights up as the playhead lands on it */}
			{label > 0.003 ? (
				<div
					style={{
						position: 'absolute',
						left: LABEL_18.x - 60,
						top: LABEL_18.y - 34,
						width: 120,
						height: 68,
						borderRadius: '50%',
						mixBlendMode: 'screen',
						opacity: label,
						background: 'radial-gradient(closest-side, rgba(120,170,255,0.75) 0%, rgba(77,141,255,0.28) 55%, transparent 100%)',
					}}
				/>
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
/** "18h" label centre (image px). */
const LABEL_18 = {x: 2222, y: 1273};
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

	// the 3D UBI rides the camera over the crossfade (feet anchored on the in-app UBI's feet)
	const g = screenGeometry(shot, f, COMP);
	const feet = mapWithGeometry(g, UBI_MATCH_IMG);
	const ubiSize = UBI_MATCH.size * g.k * g.scale * g.s0;
	const xf = xfade(f);

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
									<HeroOverlays f={f} />
									<TrackOverlays f={f} />
									<LiftHole {...TRACK_LIFT} color={HERO_CARD} radius={20} pad={4} socket={0.6} />
								</G2Screen>
								{back > 0.001 ? <AbsoluteFill style={{background: navyDim(0.2 * back)}} /> : null}
							</AbsoluteFill>
							{xf > 0.001 ? (
								<UbiClip clip="idle" index={96 + f} x={feet.x} y={feet.y} size={ubiSize} anchor="feet" opacity={xf} style={{zIndex: 1}} />
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
