/**
 * s15-foco — S15 · abs 1035–1124 (90 f) · features · bar 18.2 → 19.4
 *
 * f0 (abs 1035, beat 4) hard cut: the real "Aviso do UBI" window (intervention.png,
 * DPR 4, shown at width 1520 → bitmap scale 0.826, the sharpest UI in the film)
 * is ALREADY on screen at scale 1.04 and slams to 1 (SLAM, landed f4) with a
 * 4-px decaying shake, over the Foco page framed on the Bloqueios card (whole
 * plane blur 6 px, brightness 0.45, drifting −12 px). f2: the blocked tab is
 * gone — a 3-frame glitch jolt on the backdrop only. f20 the cursor fades in at
 * a rest point, arcs to "Ok, foco!" (arrives f41, hover lift), CLICK f45 (abs
 * 1080 downbeat): the button face presses to 0.96 (real pixels), volt ripple.
 * f45–89 the window pushes 1.00 → 1.04 (E.glide) and the in-app UBI's glow
 * breathes. f86–89 whip-left out-half (T2).
 */
import React from 'react';
import {AbsoluteFill, Img, staticFile} from 'remotion';
import {noise2D} from '@remotion/noise';
import {Screen, type ScreenConfig} from '../components/Screen';
import {alpha, color} from '../design/tokens';
import {E, hotspot, springAt, storyboardCamera, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {arcPoint, ArrowCursor, Backdrop, ClickRipple, Crop, cursorScale, Finish, pressAt, ramp} from './_parts/G5/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'impact_soft_2.wav', atFrame: 0, gainDb: -2, note: 'Window slam; bed duck −5 dB.'},
	{ref: 'glitch_2.wav', atFrame: 2, gainDb: -18, note: 'The blocked tab is gone.'},
	{ref: 'click.wav', atFrame: 45, gainDb: -12, note: 'Ok, foco!'},
];

const WIN = 'ui/intervention.png';
const BACK = 'ui/focus.png';
const IMG = {w: 1840, h: 752};
const S0 = 1520 / IMG.w; // 0.826
const WIN_W = 1520;
const WIN_H = IMG.h * S0;
const CENTER = {x: 960, y: 560};
const WIN_L = CENTER.x - WIN_W / 2;
const WIN_T = CENTER.y - WIN_H / 2;
const RADIUS = 56 * S0; // the capture's own corner (56 image px)

const CLICK = 45;
const OK = hotspot(WIN, 'ok-button'); // 1400, 408, 372 × 128 = the button face
const FACE_R = 30;
const UBI = hotspot(WIN, 'ubi');
const BADGE = hotspot(BACK, 'nav-revisao-badge');
/** Tip under the label's right half (never on the words). */
const TIP = {x: OK.x + 262, y: OK.y + 104};
const REST_IN = {x: 1560, y: 930};
const REST_OUT = {x: 1600, y: 920};

/** Window scale: SLAM 1.04 → 1 (landed f4), then the E.glide push 1 → 1.04 f45–89. */
const winScale = (f: number) => {
	const slam = 1.04 - 0.04 * springAt(f, 0, 'SLAM');
	const push = 1 + 0.04 * E.glide(ramp(f, CLICK, 89));
	return slam * push;
};
/** 4-px decaying shake over 6 f from the slam. */
const shake = (f: number) => {
	if (f < 0 || f >= 6) return {x: 0, y: 0};
	const d = Math.exp(-f / 2.2);
	return {x: noise2D('s15-sx', f * 0.9, 0) * 4 * d, y: noise2D('s15-sy', f * 0.9, 3.3) * 4 * d};
};
/** Image px of the window → composition px. */
const winToComp = (f: number, p: {x: number; y: number}) => {
	const s = winScale(f);
	const sh = shake(f);
	return {x: CENTER.x + s * (WIN_L + p.x * S0 - CENTER.x) + sh.x, y: CENTER.y + s * (WIN_T + p.y * S0 - CENTER.y) + sh.y};
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

const S15Foco: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	for (const c of scene.copy) {
		// both lines are the capture's own pixels — nothing to re-set; guard against a copy change the bitmap can't follow
		if (c.text !== 'Não! Foque na sua produtividade.' && c.text !== 'Ok, foco!') throw new Error(`s15 copy drift: ${c.text}`);
	}

	// backdrop: Foco page on the Bloqueios card (storyboard backdrop key), −12 px drift, glitch jolt f2–4
	const back: ScreenConfig = {src: BACK, ...storyboardCamera(scene, {file: BACK, layer: 'backdrop'}), glow: false, radius: 18};
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
	const cSize = 30 * cursorScale(s) * (1 - 0.15 * press);
	const cOpacity = ramp(f, 14, 20, E.enter) * (1 - ramp(f, 66, 72, E.exit));
	const clickPt = tip(CLICK);

	return (
		<Backdrop seed="s15" ember={0.07} volt={0.12}>
			<TransitionOut>
				<TransitionIn>
					{/* context, never a read: whole-plane blur 6 px, brightness 0.45 */}
					<AbsoluteFill
						style={{
							transform: `translateX(${(drift + g).toFixed(2)}px)`,
							filter: `blur(6px) brightness(${(0.45 + flicker).toFixed(3)})`,
						}}
					>
						<Screen {...back} style={{zIndex: 'auto'}}>
							{/* continuity: the review queue is empty since s13 — no pending-count badge in the sidebar */}
							<div style={{position: 'absolute', left: BADGE.x - 3, top: BADGE.y - 3, width: BADGE.w + 6, height: BADGE.h + 6, background: '#0a101c'}} />
						</Screen>
					</AbsoluteFill>
					{/* the Aviso do UBI window */}
					<div
						style={{
							position: 'absolute',
							left: WIN_L,
							top: WIN_T,
							width: WIN_W,
							height: WIN_H,
							transformOrigin: `${CENTER.x - WIN_L}px ${CENTER.y - WIN_T}px`,
							transform: `translate(${sh.x.toFixed(2)}px, ${sh.y.toFixed(2)}px) scale(${s.toFixed(5)})`,
						}}
					>
						<div
							style={{
								position: 'absolute',
								inset: 0,
								borderRadius: RADIUS,
								boxShadow: [
									'0 24px 48px -12px rgba(0,0,0,0.6)',
									'0 60px 120px -20px rgba(0,0,0,0.55)',
									`0 0 120px ${alpha(color.volt, 0.25)}`,
								].join(', '),
							}}
						/>
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
										background: `radial-gradient(closest-side, ${alpha(color.volt, 0.16 * breathe)} 0%, ${alpha(color.volt, 0.06 * breathe)} 55%, transparent 100%)`,
									}}
								/>
							) : null}
							<OkFace f={f} />
						</div>
					</div>
					<ClickRipple x={clickPt.x} y={clickPt.y} at={CLICK} />
					{f >= 14 ? <ArrowCursor x={cur.x} y={cur.y} size={cSize} opacity={cOpacity} /> : null}
				</TransitionIn>
			</TransitionOut>
			<Finish seed="s15" />
		</Backdrop>
	);
};

export default S15Foco;
