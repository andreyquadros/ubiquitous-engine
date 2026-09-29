/**
 * s15-foco — S15 · abs 1035–1124 (90 f) · features · bar 18.2 → 19.4
 *
 * v2. f0 (abs 1035, beat 4) hard cut: the real "Aviso do UBI" window
 * (intervention.png, DPR 4) is ALREADY on screen, now 1740 px wide (bitmap
 * scale 0.946: the bubble line ≈ 49 px, "Ok, foco!" ≈ 44 px), graded + rim-lit
 * like every v2 window, slightly turned in 3D (ry −7° → −2°, rx 5° → 1.5°), and
 * slams 1.05 → 1 (SLAM, landed f4) with a 4-px decaying shake over the Foco
 * page (framed on the Bloqueios card, blur 6 px, brightness 0.72, drifting
 * −12 px). f2: the blocked tab is gone — a 3-frame glitch jolt on the backdrop
 * only. f8–30 a light sweep crosses the window. f20–30 the headline
 * "Foco que se defende." (facts §3.11 manchete, Sora 700 124 px, "defende."
 * volt) staggers in above it and lands on the beat f30 (abs 1065). f20 the
 * cursor fades in, arcs to "Ok, foco!" (arrives f41); f30–44 the button pulses
 * twice (volt ring breath); CLICK f45 (abs 1080 downbeat): the face presses
 * 0.96 (real pixels), volt ripple + a shockwave off the button and a 1.5 %
 * punch-in of the window. f45–89 the window keeps pushing (1.00 → 1.035 over
 * the scene) and the in-app UBI's glow breathes. f86–89 whip-left out-half (T2).
 */
import React from 'react';
import {AbsoluteFill, Img, staticFile} from 'remotion';
import {noise2D} from '@remotion/noise';
import {GLASS_INSET, RIM_PX, rimBackground, Screen, WINDOW_SHADOW, type ScreenConfig} from '../components/Screen';
import {gradeFilter, resolveGrade} from '../components/screen-geometry';
import {alpha, color} from '../design/tokens';
import {E, hotspot, springAt, storyboardCamera, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {arcPoint, ArrowCursor, Backdrop, ClickRipple, Crop, Finish, Headline, lerp, LightSweep, pressAt, ramp, Shockwave, unitsOf} from './_parts/G5/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'impact_soft_2.wav', atFrame: 0, gainDb: -2, note: 'Window slam; bed duck −5 dB.'},
	{ref: 'glitch_2.wav', atFrame: 2, gainDb: -18, note: 'The blocked tab is gone.'},
	{ref: 'ui_tick_2.wav', atFrame: 30, gainDb: -22, note: 'v2: the headline “Foco que se defende.” lands (beat, abs 1065).'},
	{ref: 'click.wav', atFrame: 45, gainDb: -12, note: 'Ok, foco!'},
];

/** The one copy exception of the v2 pass (brief/v2-scenes.md G5, facts §3.11 manchete). */
const HEADLINE = 'Foco que se defende.';

const WIN = 'ui/intervention.png';
const BACK = 'ui/focus.png';
const IMG = {w: 1840, h: 752};
const WIN_W = 1740;
const S0 = WIN_W / IMG.w; // 0.946
const WIN_H = IMG.h * S0;
const CENTER = {x: 950, y: 640};
const WIN_L = CENTER.x - WIN_W / 2;
const WIN_T = CENTER.y - WIN_H / 2;
const RADIUS = 56 * S0; // the capture's own corner (56 image px)
const PERSPECTIVE = 2600;

const CLICK = 45;
const OK = hotspot(WIN, 'ok-button'); // 1400, 408, 372 × 128 = the button face
const FACE_R = 30;
const UBI = hotspot(WIN, 'ubi');
const BADGE = hotspot(BACK, 'nav-revisao-badge');
/** Tip under the label's right half (never on the words). */
const TIP = {x: OK.x + 262, y: OK.y + 104};
const REST_IN = {x: 1600, y: 1000};
const REST_OUT = {x: 1640, y: 1000};
const GRADE_FILTER = gradeFilter(resolveGrade({brightness: 1.25, lift: 0.06}));

/** Window scale: SLAM 1.05 → 1 (landed f4), a slow push 1 → 1.035 over the scene, + a 1.5 % punch on the click. */
const winScale = (f: number) => {
	const slam = 1.05 - 0.05 * springAt(f, 0, 'SLAM');
	const push = 1 + 0.035 * E.glide(ramp(f, 4, 89));
	const punch = f < CLICK ? 0 : 0.015 * Math.sin(Math.PI * Math.min(1, (f - CLICK) / 10)) * (f - CLICK < 10 ? 1 : 0);
	return slam * (push + punch);
};
const rotY = (f: number) => lerp(-7, -2, E.glide(ramp(f, 0, 89)));
const rotX = (f: number) => lerp(5, 1.5, E.glide(ramp(f, 0, 89)));
/** 4-px decaying shake over 6 f from the slam. */
const shake = (f: number) => {
	if (f < 0 || f >= 6) return {x: 0, y: 0};
	const d = Math.exp(-f / 2.2);
	return {x: noise2D('s15-sx', f * 0.9, 0) * 4 * d, y: noise2D('s15-sy', f * 0.9, 3.3) * 4 * d};
};
/** Image px of the window → composition px (scale + shake; the small 3D turn is approximated by its projection). */
const winToComp = (f: number, p: {x: number; y: number}) => {
	const s = winScale(f);
	const sh = shake(f);
	// local coords about the window centre, rotateY/rotateX projected with the same perspective
	const lx = (WIN_L + p.x * S0 - CENTER.x) * s;
	const ly = (WIN_T + p.y * S0 - CENTER.y) * s;
	const ry = (rotY(f) * Math.PI) / 180;
	const rx = (rotX(f) * Math.PI) / 180;
	const x1 = lx * Math.cos(ry);
	const z1 = -lx * Math.sin(ry);
	const y2 = ly * Math.cos(rx) - z1 * Math.sin(rx);
	const z2 = ly * Math.sin(rx) + z1 * Math.cos(rx);
	const k = PERSPECTIVE / (PERSPECTIVE - z2);
	return {x: CENTER.x + x1 * k + sh.x, y: CENTER.y + y2 * k + sh.y};
};

/** The "Ok, foco!" face: the real pixels, hovered then pressed to 0.96. */
const OkFace: React.FC<{f: number}> = ({f}) => {
	const hover = ramp(f, CLICK - 6, CLICK - 2, E.enter) * (1 - ramp(f, CLICK + 10, CLICK + 18, E.enter));
	const press = pressAt(f, CLICK);
	if (hover <= 0.001 && press <= 0.001) return null;
	const scale = (1 + 0.012 * hover) * (1 - 0.04 * press);
	return (
		<>
			{/* the gap colour behind the face, so the pressed face reads as sunk inside its focus ring */}
			<div style={{position: 'absolute', left: OK.x, top: OK.y, width: OK.w, height: OK.h, borderRadius: FACE_R, background: '#16284a'}} />
			<div
				style={{
					position: 'absolute',
					left: OK.x,
					top: OK.y,
					width: OK.w,
					height: OK.h,
					borderRadius: FACE_R,
					overflow: 'hidden',
					transform: `scale(${scale.toFixed(4)})`,
					transformOrigin: '50% 50%',
					filter: `brightness(${(1 + 0.08 * hover - 0.14 * press).toFixed(3)})`,
				}}
			>
				<Crop src={WIN} rect={OK} at={{x: 0, y: 0}} imgW={IMG.w} imgH={IMG.h} />
			</div>
		</>
	);
};

/** Two volt breaths around the button before the click ("press me"), image space. */
const OkPulse: React.FC<{f: number}> = ({f}) => {
	if (f < 28 || f > CLICK + 1) return null;
	const u = (f - 28) / 8; // one breath per 8 f (≈ half a beat), two of them, the second peaks into the click
	const b = Math.max(0, Math.sin(Math.PI * Math.min(2, u) - 0.001)) * (u < 2 ? 1 : 0);
	const env = ramp(f, 28, 31) * (1 - ramp(f, CLICK - 1, CLICK + 1));
	const g = Math.abs(b) * env;
	if (g <= 0.01) return null;
	const pad = 10 + 16 * g;
	return (
		<div
			style={{
				position: 'absolute',
				left: OK.x - pad,
				top: OK.y - pad,
				width: OK.w + pad * 2,
				height: OK.h + pad * 2,
				borderRadius: FACE_R + pad,
				boxShadow: `0 0 0 ${(3 + 3 * g).toFixed(1)}px ${alpha(color.volt, 0.55 * g)}, 0 0 ${(60 * g).toFixed(1)}px ${(10 * g).toFixed(1)}px ${alpha(color.volt, 0.5 * g)}`,
			}}
		/>
	);
};

const S15Foco: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	for (const c of scene.copy) {
		// both lines are the capture's own pixels — nothing to re-set; guard against a copy change the bitmap can't follow
		if (c.text !== 'Não! Foque na sua produtividade.' && c.text !== 'Ok, foco!') throw new Error(`s15 copy drift: ${c.text}`);
	}
	const units = unitsOf(HEADLINE, [
		{text: 'Foco', at: 20},
		{text: 'que se', at: 22},
		{text: 'defende.', at: 24, volt: 'defende.'},
	]);

	// backdrop: Foco page on the Bloqueios card (storyboard backdrop key), −12 px drift, glitch jolt f2–4
	const back: ScreenConfig = {src: BACK, ...storyboardCamera(scene, {file: BACK, layer: 'backdrop'}), glow: false, radius: 18, dots: 'neutral'};
	const drift = -12 * (f / 89);
	const g = f >= 2 && f <= 4 ? [9, -6, 3][f - 2] : 0;
	const flicker = f >= 2 && f <= 4 ? [0.2, -0.08, 0.06][f - 2] : 0;

	const s = winScale(f);
	const sh = shake(f);
	const breathe = f < CLICK ? 0 : ramp(f, CLICK, CLICK + 10, E.enter) * (0.5 + 0.5 * Math.sin(((f - CLICK) / 30) * Math.PI * 2 - Math.PI / 2));

	// cursor
	const tip = (fr: number) => winToComp(fr, TIP);
	let cur = REST_IN;
	if (f > 26 && f < 41) cur = arcPoint(REST_IN, tip(f), E.cursor(ramp(f, 26, 41)), 0.12);
	else if (f >= 41 && f <= 58) cur = tip(f);
	else if (f > 58) cur = arcPoint(tip(58), REST_OUT, E.cursor(ramp(f, 58, 70)), 0.12);
	const press = pressAt(f, CLICK);
	const cSize = 42 * (1 - 0.15 * press);
	const cOpacity = ramp(f, 14, 20, E.enter) * (1 - ramp(f, 66, 72, E.exit));
	const clickPt = tip(CLICK);
	const okC = winToComp(f, {x: OK.x + OK.w / 2, y: OK.y + OK.h / 2});

	return (
		<Backdrop
			seed="s15"
			ember={0.08}
			volt={0.08}
			look={{keyPool: {x: 0.55, y: 0.62, w: 0.84, h: 0.9, opacity: 0.44}, keyLight: {x: 0.55, y: 0.6, w: 0.6, h: 0.62, opacity: 0.16}}}
		>
			<TransitionOut>
				<TransitionIn>
					{/* context, never a read: whole-plane blur 6 px, brightness 0.72 */}
					<AbsoluteFill
						style={{
							transform: `translateX(${(drift + g).toFixed(2)}px)`,
							filter: `blur(6px) brightness(${(0.72 + flicker).toFixed(3)})`,
						}}
					>
						<Screen {...back} style={{zIndex: 'auto'}}>
							{/* continuity: the review queue is empty since s13 — no pending-count badge in the sidebar */}
							<div style={{position: 'absolute', left: BADGE.x - 3, top: BADGE.y - 3, width: BADGE.w + 6, height: BADGE.h + 6, background: '#0a101c'}} />
						</Screen>
					</AbsoluteFill>
					{/* the headline band steps the backdrop back so the type reads */}
					<AbsoluteFill
						style={{
							background: 'linear-gradient(180deg, rgba(10,16,36,0.78) 0px, rgba(10,16,36,0.6) 200px, rgba(10,16,36,0) 330px)',
							opacity: 0.55 + 0.45 * ramp(f, 14, 22, E.enter),
						}}
					/>
					{/* the Aviso do UBI window */}
					<AbsoluteFill style={{perspective: PERSPECTIVE, perspectiveOrigin: `${CENTER.x}px ${CENTER.y}px`}}>
						<div
							style={{
								position: 'absolute',
								left: WIN_L,
								top: WIN_T,
								width: WIN_W,
								height: WIN_H,
								transformOrigin: `${CENTER.x - WIN_L}px ${CENTER.y - WIN_T}px`,
								transform: `translate(${sh.x.toFixed(2)}px, ${sh.y.toFixed(2)}px) rotateX(${rotX(f).toFixed(3)}deg) rotateY(${rotY(f).toFixed(3)}deg) scale(${s.toFixed(5)})`,
							}}
						>
							{/* v2 window: rim light + deep shadow + volt ambient glow */}
							<div
								style={{
									position: 'absolute',
									inset: -RIM_PX,
									borderRadius: RADIUS + RIM_PX,
									background: rimBackground(),
									boxShadow: `${WINDOW_SHADOW}, 0 0 140px ${alpha(color.volt, 0.3)}`,
								}}
							/>
							<div style={{position: 'absolute', inset: 0, borderRadius: RADIUS, overflow: 'hidden'}}>
								<div style={{position: 'absolute', inset: 0, filter: GRADE_FILTER}}>
									<Img src={staticFile(WIN)} style={{position: 'absolute', left: 0, top: 0, width: WIN_W, height: WIN_H, display: 'block'}} />
									<div style={{position: 'absolute', left: 0, top: 0, width: IMG.w, height: IMG.h, transformOrigin: '0 0', transform: `scale(${S0})`}}>
										{/* the in-app UBI's glow breathes after the click */}
										{breathe > 0.001 ? (
											<div
												style={{
													position: 'absolute',
													left: UBI.x - 60,
													top: UBI.y - 20,
													width: UBI.w + 120,
													height: UBI.h + 40,
													borderRadius: '50%',
													mixBlendMode: 'screen',
													background: `radial-gradient(closest-side, ${alpha(color.volt, 0.2 * breathe)} 0%, ${alpha(color.volt, 0.07 * breathe)} 55%, transparent 100%)`,
												}}
											/>
										) : null}
										<OkPulse f={f} />
										<OkFace f={f} />
									</div>
								</div>
								<LightSweep from={8} to={32} strength={0.16} />
								<div style={{position: 'absolute', inset: 0, borderRadius: RADIUS, boxShadow: GLASS_INSET}} />
							</div>
						</div>
					</AbsoluteFill>
					<Headline units={units} size={124} left={100} capTop={92} />
					<Shockwave x={okC.x} y={okC.y} at={CLICK} radius={380} len={18} tint={color.volt} />
					<ClickRipple x={clickPt.x} y={clickPt.y} at={CLICK} scale={1.4} />
					{f >= 14 ? <ArrowCursor x={cur.x} y={cur.y} size={cSize} opacity={cOpacity} /> : null}
				</TransitionIn>
			</TransitionOut>
			<Finish seed="s15" />
		</Backdrop>
	);
};

export default S15Foco;
