/**
 * s06-ele-registra — S06 · abs 360–449 (90 f) · features · bar 7.1 → 8.3
 *
 * f0 (downbeat) match cut: the frame is INSIDE the Hoje hero card at bitmap
 * 1:1 — the in-app UBI sits exactly where s05's 3D UBI stopped; f0–4 the 3D
 * UBI (idle 96…) crossfades into the bitmap, riding the camera. f0–24 pull-back
 * to the whole window (E.push), tilt settles to rx 6° ry −6°; the app's speech
 * bubble pops (SNAPPY). f5–20 "Ele registra. Você trabalha." word stagger
 * ("registra." volt). f24 sheen. f30–60 glide to the day-track card (E.glide),
 * the top scrim thickens, spotlight from f45. f36–60 the cover over the day
 * track retracts 08h → 18h behind a volt playhead (E.glide 24 f): Tuesday
 * records itself block by block. f60 the playhead lands on the 18h tick and
 * pulses; f75 its glow breathes. f86–89 whip-left out-half.
 */
import React from 'react';
import {AbsoluteFill, Img, staticFile} from 'remotion';
import {mapWithGeometry, screenGeometry, type ScreenConfig} from '../components/screen-geometry';
import {color, font} from '../design/tokens';
import {
	E,
	springAt,
	storyboardCamera,
	storyboardSpotlights,
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
const BUBBLE = {x: 2236, y: 276, w: 540, h: 216, tail: {x: 2506, y: 452}};
const BUBBLE_AT = 3;
const BUBBLE_UNTIL = 24;
/** In-app UBI ink box (patched during the 3D → bitmap crossfade). */
const APP_UBI = {x: 2392, y: 540, w: 236, h: 356};
const XFADE = 4;

const playheadX = (f: number) => lerp(TRACK.x08, TRACK.x18, ramp(f, BUILD_START, BUILD_END, E.glide));

/** Image-space layers glued to the dashboard (children of the screen). */
const DashOverlays: React.FC<{f: number}> = ({f}) => {
	const e = playheadX(f);
	const building = f >= BUILD_START - 2 && f <= BUILD_END + 2;
	const ph = ramp(f, 30, 36, E.enter); // playhead fades in at 08h just before the build
	// landing pulse on f60 (dot scale 1 → 1.7 → 1) and a breath on f75
	const pulse = f >= BUILD_END ? Math.sin(Math.PI * ramp(f, BUILD_END, BUILD_END + 10, E.linear)) : 0;
	const breath = f >= 75 ? Math.sin(Math.PI * ramp(f, 75, 89, E.linear)) : 0;
	const glowK = 1 + 0.9 * pulse + 0.55 * breath;
	const ring = f >= BUILD_END && f < BUILD_END + 16 ? ramp(f, BUILD_END, BUILD_END + 16, E.push) : -1;
	const xf = clamp01(1 - f / XFADE);
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
				<div style={{position: 'absolute', left: APP_UBI.x, top: APP_UBI.y, width: APP_UBI.w, height: APP_UBI.h, background: HERO_CARD, opacity: xf}} />
			) : null}

			{/* day-track cover: hides everything right of the playhead (and, for good, right of 18h incl. the now-marker) */}
			<div
				style={{
					position: 'absolute',
					left: e,
					top: TRACK.top,
					width: TRACK.right - e,
					height: TRACK.bottom - TRACK.top,
					background: TRACK.fill,
				}}
			/>
			{TRACK.ticks
				.filter((t) => t >= e)
				.map((t) => (
					<div key={t} style={{position: 'absolute', left: t, top: TRACK.top, width: 2, height: TRACK.bottom - TRACK.top, background: TRACK.tickColor}} />
				))}
			{/* write-head: a short volt wash just behind the playhead while it records */}
			{building && e > TRACK.x08 + 1 ? (
				<div
					style={{
						position: 'absolute',
						left: Math.max(TRACK.x08, e - 90),
						top: TRACK.top,
						width: Math.min(90, e - TRACK.x08),
						height: TRACK.bottom - TRACK.top,
						background: 'linear-gradient(90deg, rgba(77,141,255,0) 0%, rgba(77,141,255,0.28) 100%)',
						mixBlendMode: 'screen',
						opacity: 1 - ramp(f, BUILD_END - 2, BUILD_END + 2, E.linear),
					}}
				/>
			) : null}
			{/* playhead: 3 px on screen at the card framing (4 image px), 24 px glow, dot on top */}
			{ph > 0.001 ? (
				<div style={{position: 'absolute', left: e - 2, top: TRACK.top - 16, width: 4, height: TRACK.bottom - TRACK.top + 32, opacity: ph}}>
					<div
						style={{
							position: 'absolute',
							inset: 0,
							borderRadius: 2,
							background: color.volt,
							boxShadow: `0 0 ${(32 * glowK).toFixed(1)}px ${(4 * glowK).toFixed(1)}px rgba(77,141,255,${(0.55 * Math.min(1.6, glowK)).toFixed(3)})`,
						}}
					/>
					<div
						style={{
							position: 'absolute',
							left: 2 - 9,
							top: -9,
							width: 18,
							height: 18,
							borderRadius: 9,
							background: color.volt,
							boxShadow: `0 0 ${(18 * glowK).toFixed(1)}px rgba(77,141,255,0.8)`,
							transform: `scale(${(1 + 0.7 * pulse).toFixed(3)})`,
						}}
					/>
					{ring >= 0 && ring < 1 ? (
						<div
							style={{
								position: 'absolute',
								left: 2 - lerp(9, 46, ring),
								top: -lerp(9, 46, ring),
								width: lerp(18, 92, ring),
								height: lerp(18, 92, ring),
								borderRadius: '50%',
								border: `3px solid rgba(77,141,255,${(0.7 * (1 - ring)).toFixed(3)})`,
							}}
						/>
					) : null}
				</div>
			) : null}
		</>
	);
};

const Headline: React.FC<{f: number; text: string; emphasis: string[]}> = ({f, text, emphasis}) => {
	const size = 80;
	const words = text.split(' ');
	return (
		<div
			style={{
				position: 'absolute',
				left: 0,
				width: W,
				top: Math.round(88 - 0.11 * size),
				textAlign: 'center',
				fontFamily: font.display,
				fontWeight: 700,
				fontSize: size,
				lineHeight: 1,
				letterSpacing: '-0.03em',
				color: color.ink,
				whiteSpace: 'nowrap',
			}}
		>
			{words.map((w, i) => {
				const at = 5 + 3 * i;
				const s = f < at ? 0 : springAt(f, at, 'SNAPPY');
				const o = clamp01((f - at + 1) / 6);
				const blur = 8 * clamp01(1 - s);
				const volt = emphasis.includes(w) ? clamp01((s - 0.55) / 0.4) : 0;
				return (
					<React.Fragment key={i}>
						<span
							style={{
								display: 'inline-block',
								opacity: o,
								transform: `translateY(${(28 * (1 - s)).toFixed(2)}px)`,
								filter: blur > 0.15 ? `blur(${blur.toFixed(2)}px)` : undefined,
								color: volt > 0 ? mix(color.ink, color.volt, volt) : undefined,
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

/** Top canvas scrim: solid 0–200 → 0 at 260, thickening f30–60 to solid 0–420 → 0 at 480. */
const Scrim: React.FC<{f: number}> = ({f}) => {
	const on = ramp(f, 2, 14, E.enter);
	const t = ramp(f, 30, 60, E.glide);
	const solid = lerp(200, 420, t);
	const end = lerp(260, 480, t);
	if (on <= 0.001) return null;
	return (
		<AbsoluteFill
			style={{
				opacity: on,
				background: `linear-gradient(180deg, ${color.canvas} 0px, ${color.canvas} ${solid.toFixed(1)}px, rgba(10,13,22,0.55) ${((solid + end) / 2).toFixed(1)}px, rgba(10,13,22,0) ${end.toFixed(1)}px)`,
			}}
		/>
	);
};

const S06EleRegistra: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const headline = scene.copy[0];
	const cam = storyboardCamera(scene, {file: FILE});
	const shot: ScreenConfig = {
		src: FILE,
		width: 1440,
		radius: 18,
		float: 4,
		sheenAt: 24,
		glow: 'volt',
		camera: cam.camera,
		rotateX: cam.rotateX,
		rotateY: cam.rotateY,
		spotlights: storyboardSpotlights(scene, FILE),
	};

	// the 3D UBI rides the camera over the crossfade (feet anchored on the in-app UBI's feet)
	const g = screenGeometry(shot, f, {width: W, height: H});
	const feet = mapWithGeometry(g, UBI_MATCH_IMG);
	const ubiSize = UBI_MATCH.size * g.k * g.scale * g.s0;
	const xf = clamp01(1 - f / XFADE);

	return (
		<AbsoluteFill style={{backgroundColor: color.canvas}}>
			<Backdrop
				seed="s06"
				orbs={[
					{c: color.volt, x: 420, y: 260, d: 1200, opacity: 0.22},
					{c: color.ember, x: 1650, y: 910, d: 960, opacity: 0.1},
					{c: color.volt, x: 1180, y: 560, d: 1500, opacity: 0.07},
				]}
				floor={0.3}
				topLight={1}
			/>
			<TransitionIn>
				{() => (
					<>
						<TransitionOut>
							<G2Screen {...shot}>
								<DashOverlays f={f} />
							</G2Screen>
							{xf > 0.001 ? (
								<UbiClip clip="idle" index={96 + f} x={feet.x} y={feet.y} size={ubiSize} anchor="feet" opacity={xf} />
							) : null}
						</TransitionOut>
						<Scrim f={f} />
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
