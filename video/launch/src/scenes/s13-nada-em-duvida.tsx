/**
 * s13-nada-em-duvida — S13 · abs 870–929 (60 f) · features · bar 15.3 → 16.3
 *
 * v2. f0 (abs 870) cut on the empty queue (review-done, graded a touch
 * brighter, near-flat rx 2° ry −2°), framed from the first frame BELOW the
 * title bar (zoom 1.9, no canvas strip); continuity/claim patches over the
 * resolved-count paragraph, the settled header count and the keys legend.
 * f0–15 push onto the in-app UBI (E.push, zoom 2.3). f15 (abs 885, beat 2) T5
 * match: the 3D UBI takes the in-app UBI's exact place in ONE frame (the
 * bitmap UBI and its bubble are covered on the same frame: no double image),
 * the app racks out (blur 0 → 10 px, brightness 1 → 0.78, SMOOTH 12 f) and
 * pushes 1.00 → 1.10, while a white-blue key light blooms behind UBI and the
 * bubble. UBI grows to a 940-px frame (body ≈ 670 px) and leaps left on a
 * short arc (jump-from-idle from index 6: apex f24, contact f30 = abs 900
 * downbeat, squash f33), 3-px shake on contact, ember floor glow + volt rim,
 * and a burst of volt/mint sparks off the floor. The in-app bubble grows into
 * the big "Nada em dúvida!" bubble (Sora 700 100 px, "dúvida!" volt gradient,
 * SNAPPY 12 f, lands f27, tail to his head). f35–59 idle + slow float, the
 * bubble floats against him, a light sweep crosses the bubble.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {noise2D} from '@remotion/noise';
import {Screen, type ScreenConfig} from '../components/Screen';
import {mapImageRect, type CameraKey} from '../components/screen-geometry';
import {alpha, color, font} from '../design/tokens';
import {E, Patches, springAt, storyboardPatches, TransitionIn, TransitionOut, UbiTrack, UBI_ANCHORS, useScene, useSceneFrame, type SfxCue} from '../shared';
import {APP, Backdrop, clamp01, Finish, gradientFill, H, lerp, LightSweep, ramp, voltGlow, W, widenLegend} from './_parts/G5/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'shimmer_2.wav', atFrame: 15, gainDb: -12, note: 'UBI steps out of the app.'},
	{ref: 'pop+2.wav', atFrame: 27, gainDb: -16, note: 'Bubble lands.'},
	{ref: 'bloop_1.wav', atFrame: 30, gainDb: -12, note: 'UBI lands on the downbeat.'},
];

const FILE = 'ui/review-done.png';
const COMP = {width: W, height: H};
const MATCH = 15;
const CONTACT = 30;
const CARD_BG = APP.panel; // #0c1220, the empty-state card

/** The in-app UBI's 900-px clip frame as drawn in the capture (image px: top-left + size) and its small bubble. */
const INAPP = {x: 1107, y: 400, size: 352};
const INAPP_BUBBLE = {cx: 1280, cy: 373, w: 238};
/** Landing: 1000-px frame (body ≈ 710 px), feet on y 985, body centre x ≈ 610. */
const END_SIZE = 1000;
const END = {size: END_SIZE, x: 610 + (UBI_ANCHORS.feet.x - UBI_ANCHORS.body.x) * (END_SIZE / 900), y: 985};
/** "Nada esperando por você" and the "Ver a Timeline" button of the empty-state card (image px, measured). */
const CARD_TEXT = [
	{x: 1030, y: 764, w: 500, h: 68},
	{x: 1140, y: 942, w: 280, h: 90},
];
const ARC = 70; // extra leap height (px) on top of the clip's own jump, 0 at f15 and at contact

/** The big bubble's final box and its tail (tip at his helmet). */
const BUBBLE = {x: 872, y: 286, w: 968, h: 188, r: 48};
const TEXT_SIZE = 100;
const TAIL = {base0: 96, base1: 190, tip: {x: 812, y: 520}};

const CAMERA: CameraKey[] = [
	// framed below the title bar from the first frame (v1 showed the canvas + title-bar strip f0–6)
	{at: 0, zoom: 1.9, focus: {x: 1279, y: 640}, anchor: {x: 960, y: 560}, duration: 0},
	{at: MATCH, zoom: 2.3, focus: {x: 1280, y: 560}, anchor: {x: 960, y: 590}, duration: MATCH, easing: E.push},
];
const SHOT: ScreenConfig = {
	src: FILE,
	camera: CAMERA,
	rotateX: 2,
	rotateY: -2,
	dots: 'neutral',
	radius: 16,
	grade: {brightness: 1.3, lift: 0.07},
};

const Bubble: React.FC<{f: number; text: string; from: {cx: number; cy: number; w: number}; bob: number}> = ({f, text, from, bob}) => {
	if (f < MATCH) return null;
	const s = springAt(f, MATCH, 'SNAPPY', 12);
	const k = lerp(from.w / BUBBLE.w, 1, s);
	const bx = BUBBLE.x + BUBBLE.w / 2;
	const by = BUBBLE.y + BUBBLE.h / 2;
	const cx = lerp(from.cx, bx, s);
	const cy = lerp(from.cy, by, s) + bob;
	const o = ramp(f, MATCH, MATCH + 2);
	const v = ramp(f, MATCH + 8, MATCH + 12);
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
		`Q ${x0 + 90} ${y1 + 8} ${TAIL.tip.x} ${TAIL.tip.y}`,
		`Q ${x0 + 50} ${y1 - 2} ${x0 + TAIL.base0 - 50} ${y1}`,
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
				transformOrigin: `${bx}px ${by}px`,
				transform: `translate(${(cx - bx).toFixed(2)}px, ${(cy - by).toFixed(2)}px) scale(${k.toFixed(4)})`,
			}}
		>
			<svg width={W} height={H} style={{position: 'absolute', left: 0, top: 0, overflow: 'visible', filter: 'drop-shadow(0 24px 40px rgba(0,0,0,0.5))'}}>
				<defs>
					<linearGradient id="s13-bubble" x1="0" y1="0" x2="0" y2="1">
						<stop offset="0%" stopColor="#1d2942" />
						<stop offset="100%" stopColor="#141c2f" />
					</linearGradient>
					<linearGradient id="s13-rim" x1="0" y1="0" x2="1" y2="1">
						<stop offset="0%" stopColor={alpha(color.volt, 0.75)} />
						<stop offset="45%" stopColor="rgba(255,255,255,0.14)" />
						<stop offset="100%" stopColor="rgba(255,255,255,0.06)" />
					</linearGradient>
				</defs>
				<path d={path} fill="url(#s13-bubble)" fillOpacity={0.97} stroke="url(#s13-rim)" strokeWidth={2} />
			</svg>
			<div style={{position: 'absolute', left: x0, top: y0, width: w, height: h, borderRadius: r, overflow: 'hidden'}}>
				<LightSweep from={38} to={58} strength={0.12} />
			</div>
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
					fontSize: TEXT_SIZE,
					lineHeight: 1,
					letterSpacing: '-0.035em',
					whiteSpace: 'nowrap',
					paddingBottom: 6,
				}}
			>
				<span>
					<span style={gradientFill(0)}>{lead}</span>
					<span style={{...gradientFill(v), filter: v > 0.01 ? voltGlow(v) : undefined}}>{emph}</span>
				</span>
			</div>
		</AbsoluteFill>
	);
};

/** Contact sparks: small volt / mint / ember dots bursting off the floor around his feet (deterministic). */
const Sparks: React.FC<{f: number; x: number; y: number}> = ({f, x, y}) => {
	const d = f - CONTACT;
	if (d < 0 || d > 22) return null;
	const n = 22;
	return (
		<>
			{Array.from({length: n}, (_, i) => {
				const a = Math.PI * (1.05 + (0.9 * i) / (n - 1)) + noise2D('s13-sa', i, 0) * 0.12; // upper half-fan
				const sp = 380 + 220 * (0.5 + 0.5 * noise2D('s13-ss', i, 1));
				const t = clamp01(d / 22);
				const e = E.push(t);
				const px = x + Math.cos(a) * sp * e * 1.25;
				const py = y + Math.sin(a) * sp * e * 0.7 + 90 * t * t; // a little gravity
				const r = (i % 3 === 0 ? 10 : 7) * (1 - 0.5 * t);
				const c = i % 3 === 0 ? color.mint : i % 3 === 1 ? color.volt : color.ember;
				const o = (1 - t) * ramp(d, 0, 2);
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: px - r,
							top: py - r,
							width: r * 2,
							height: r * 2,
							borderRadius: '50%',
							background: c,
							opacity: o,
							boxShadow: `0 0 ${(14 * (1 - t)).toFixed(1)}px ${alpha(c, 0.9)}`,
						}}
					/>
				);
			})}
		</>
	);
};

const S13NadaEmDuvida: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const patches = widenLegend(storyboardPatches(scene, FILE));
	const staticPatches = patches.filter((p) => !/UBI/.test(p.covers ?? ''));
	const ubiPatch = patches.find((p) => /UBI/.test(p.covers ?? ''));

	// the in-app UBI + bubble at the match frame (comp px on the settled camera)
	const inApp = mapImageRect(SHOT, MATCH, COMP, {x: INAPP.x, y: INAPP.y, w: INAPP.size, h: INAPP.size});
	const startSize = inApp.w;
	const START = {size: startSize, x: inApp.x + UBI_ANCHORS.feet.x * (startSize / 900), y: inApp.y + UBI_ANCHORS.feet.y * (startSize / 900)};
	const sb = mapImageRect(SHOT, MATCH, COMP, {x: INAPP_BUBBLE.cx - INAPP_BUBBLE.w / 2, y: INAPP_BUBBLE.cy - 36, w: INAPP_BUBBLE.w, h: 72});
	const smallBubble = {cx: sb.x + sb.w / 2, cy: sb.y + sb.h / 2, w: sb.w};

	// app racks out + push
	const rack = f < MATCH ? 0 : springAt(f, MATCH, 'SMOOTH', 12);
	const push = 1 + 0.1 * E.push(ramp(f, MATCH, 35));
	// 3-px shake on contact
	const t = f - CONTACT;
	const sh = t >= 0 && t < 7 ? {x: noise2D('s13-x', t * 0.9, 0) * 3 * Math.exp(-t / 2.5), y: noise2D('s13-y', t * 0.9, 5) * 3 * Math.exp(-t / 2.5)} : {x: 0, y: 0};

	// UBI placement
	const g = f < MATCH ? 0 : springAt(f, MATCH, 'SMOOTH', 20);
	const leap = f >= MATCH && f <= CONTACT ? Math.sin(Math.PI * ramp(f, MATCH, CONTACT)) : 0;
	const float = 8 * Math.sin(((f - 36) / 48) * Math.PI) * ramp(f, 34, 40);
	const size = lerp(START.size, END.size, g);
	const x = lerp(START.x, END.x, g);
	const floorY = lerp(START.y, END.y, g);
	const y = floorY - ARC * leap * (0.35 + 0.65 * g) - float;
	// floor light arrives with him; flares on contact
	const land = ramp(f, 22, CONTACT, E.enter);
	const flare = f >= CONTACT ? 1 - ramp(f, CONTACT, CONTACT + 12, E.push) : 0;
	const glowW = size * 0.8;
	// key light behind UBI + bubble blooms on the match
	const bloom = ramp(f, MATCH, MATCH + 10, E.enter);
	const breathe = 1 + 0.08 * Math.sin(((f - 30) / 40) * Math.PI * 2) * ramp(f, 30, 40);

	return (
		<TransitionOut>
			<TransitionIn>
				<Backdrop seed="s13" ember={0.08} volt={0.1} look={{keyPool: {x: 0.42, y: 0.5, w: 0.8, h: 0.9, opacity: 0.44}, keyLight: {x: 0.36, y: 0.55, w: 0.5, h: 0.62, opacity: 0.16}}}>
					<AbsoluteFill style={{transform: sh.x || sh.y ? `translate(${sh.x.toFixed(2)}px, ${sh.y.toFixed(2)}px)` : undefined}}>
						{/* the app: racks out behind him (blur + a gentle dim, never a blackout) and pushes toward his landing */}
						<AbsoluteFill
							style={{
								filter: rack > 0.002 ? `blur(${(10 * rack).toFixed(2)}px) brightness(${(1 - 0.22 * rack).toFixed(3)})` : undefined,
								transformOrigin: `${END.x}px 640px`,
								transform: Math.abs(push - 1) > 1e-4 ? `scale(${push.toFixed(5)})` : undefined,
							}}
						>
							<Screen {...SHOT} style={{zIndex: 'auto'}}>
								<Patches patches={staticPatches} />
								{/* he stepped out of the card: its heading and button melt away with the rack (no blurred text smudge at his feet) */}
								{f >= MATCH
									? CARD_TEXT.map((r, i) => (
											<div key={i} style={{position: 'absolute', left: r.x, top: r.y, width: r.w, height: r.h, background: CARD_BG, borderRadius: 12, opacity: ramp(f, MATCH, MATCH + 9, E.enter)}} />
										))
									: null}
								{/* only one UBI: the bitmap UBI and its bubble go on the match frame itself */}
								{ubiPatch && f >= MATCH ? (
									<div style={{position: 'absolute', left: ubiPatch.rect.x - 2, top: ubiPatch.rect.y - 2, width: ubiPatch.rect.w + 4, height: ubiPatch.rect.h + 4, background: CARD_BG}} />
								) : null}
							</Screen>
						</AbsoluteFill>
						{/* anticipation: the in-app UBI starts to glow as the camera pushes in (he is about to step out) */}
						{f < MATCH + 6 ? (
							<div
								style={{
									position: 'absolute',
									left: inApp.x + inApp.w / 2 - inApp.w * 1.6,
									top: inApp.y + inApp.h * 0.55 - inApp.w * 1.3,
									width: inApp.w * 3.2,
									height: inApp.w * 2.6,
									borderRadius: '50%',
									opacity: ramp(f, 2, MATCH, E.enter) * (1 - ramp(f, MATCH, MATCH + 6)),
									mixBlendMode: 'screen',
									background: 'radial-gradient(closest-side, rgba(170,200,255,0.34) 0%, rgba(77,141,255,0.14) 50%, rgba(77,141,255,0) 100%)',
								}}
							/>
						) : null}
						{/* key light: white-blue behind UBI and the bubble (screen), so the stage reads lit, not blacked out */}
						{bloom > 0.001 ? (
							<>
								<div
									style={{
										position: 'absolute',
										left: END.x - 760,
										top: 620 - 560,
										width: 1520,
										height: 1120,
										borderRadius: '50%',
										opacity: bloom * breathe,
										mixBlendMode: 'screen',
										background: 'radial-gradient(closest-side, rgba(170,200,255,0.34) 0%, rgba(77,141,255,0.16) 45%, rgba(77,141,255,0) 100%)',
									}}
								/>
								<div
									style={{
										position: 'absolute',
										left: BUBBLE.x - 220,
										top: BUBBLE.y - 200,
										width: BUBBLE.w + 440,
										height: BUBBLE.h + 400,
										borderRadius: '50%',
										opacity: bloom * ramp(f, MATCH + 4, MATCH + 14),
										mixBlendMode: 'screen',
										background: 'radial-gradient(closest-side, rgba(207,224,255,0.16) 0%, rgba(207,224,255,0.05) 55%, rgba(207,224,255,0) 100%)',
									}}
								/>
							</>
						) : null}
						{/* ember floor glow + contact shadow on the stage floor (not tied to the jump) */}
						{land > 0.001 ? (
							<>
								<div
									style={{
										position: 'absolute',
										left: x - glowW / 2,
										top: floorY - size * 0.08,
										width: glowW,
										height: size * 0.16,
										borderRadius: '50%',
										opacity: land,
										background: `radial-gradient(closest-side, ${alpha(color.ember, 0.42 + 0.25 * flare)} 0%, ${alpha(color.ember, 0.14)} 50%, transparent 100%)`,
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
						<Sparks f={f} x={END.x} y={END.y - 20} />
						<UbiTrack segments={scene.ubiTrack} size={size} x={x} y={y} opacity={f >= MATCH ? 1 : 0} rim={alpha(color.volt, 0.45)} />
						<Bubble f={f} text={scene.copy[0].text} from={smallBubble} bob={-0.5 * float} />
					</AbsoluteFill>
					<Finish seed="s13" />
				</Backdrop>
			</TransitionIn>
		</TransitionOut>
	);
};

export default S13NadaEmDuvida;
