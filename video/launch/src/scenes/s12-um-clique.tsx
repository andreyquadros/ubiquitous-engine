/**
 * s12-um-clique — S12 · abs 780–869 (90 f) · features · bar 14.1 → 15.3
 *
 * f0 (abs 780, downbeat) cut in tight on "Confirmar os 19" (review-settled-
 * expanded, zoom 2.8, rx 4° ry −6°) under a 0.62 dim; the button is a crisp
 * replica cut from the @3x twin, lifted 1.08× above the plane (so its label
 * reads ≥ 36 px from the first frame). f4–26 the cursor arcs in from off-frame
 * bottom-right (E.cursor, 12 % arc); f24 hover. f30 CLICK (abs 810, beat 3):
 * cursor 1 → 0.85 → 1, replica 0.96 + pressed colour, ripple 44 px + 64 px.
 * f32 (C+2) hard swap to review-confirmed with the SAME camera: row 1's mint
 * "você" lands where the button was; a mint wave runs down the rows (12 %,
 * 2-f stagger, f33–51) while the camera pulls back (E.push f32–50) to the
 * whole column of "você"; the dim lifts. f30–40 "Um clique vira memória."
 * ("memória." volt) over a top-left scrim. f50–89 hold + drift. Cut out.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import type {Point, Rect} from '../components/screen-geometry';
import {E, hotspot, Patches, springAt, storyboardCamera, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {
	arcPoint,
	ArrowCursor,
	Backdrop,
	clamp01,
	ClickRipple,
	Crop,
	G4Plane,
	GlideSpot,
	mapPt,
	planeGeometry,
	ramp,
	Scrim,
	StaggerHeadline,
	unitsOf,
	widenLegend,
} from './_parts/G4/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'click.wav', atFrame: 30, gainDb: -12, note: 'Confirmar os 19.'},
	{ref: 'shimmer_1.wav', atFrame: 33, gainDb: -14, note: 'All badges flip to “você” (C+3).'},
];

const BEFORE = 'ui/review-settled-expanded.png';
const AFTER = 'ui/review-confirmed.png';

const CLICK = 30;
const SWAP = 32;

const BUTTON = hotspot(BEFORE, 'confirm-all-button'); // 1785,446 226×64
const BAR = hotspot(BEFORE, 'confirm-bar');
const BADGE = hotspot(AFTER, 'nav-revisao-badge');
const ROW1 = hotspot(AFTER, 'reviewed-row-1'); // 514,432 1529×114
const BTN_CROP: Rect = {x: BUTTON.x - 2, y: BUTTON.y - 2, w: BUTTON.w + 4, h: BUTTON.h + 4};
const BTN_RADIUS = 12;
/** Cursor tip on the label ("…os 19"). */
const TIP: Point = {x: BUTTON.x + 132, y: BUTTON.y + 40};
const ENTER: Point = {x: 1990, y: 1140};
const REST: Point = {x: 1700, y: 900};

/** The replica button: the real pixels (from the @3x twin), lifted, hovered, pressed. */
const Replica: React.FC<{f: number}> = ({f}) => {
	if (f >= SWAP) return null;
	const hover = ramp(f, 24, 28, E.enter);
	const press = f >= CLICK - 1 ? springAt(f, CLICK - 1, 'SNAPPY') : 0;
	const scale = 1.08 * (1 + 0.02 * hover) * (1 - 0.04 * press);
	return (
		<div
			style={{
				position: 'absolute',
				left: BTN_CROP.x,
				top: BTN_CROP.y,
				width: BTN_CROP.w,
				height: BTN_CROP.h,
				transformOrigin: '50% 50%',
				transform: `scale(${scale.toFixed(4)})`,
				borderRadius: BTN_RADIUS + 2,
				boxShadow: `0 ${(10 + 4 * hover).toFixed(1)}px ${(26 + 8 * hover).toFixed(1)}px -6px rgba(0,0,0,0.65)`,
			}}
		>
			<Crop src={BEFORE} rect={BTN_CROP} style={{left: 0, top: 0, borderRadius: BTN_RADIUS + 2}} />
			<div
				style={{
					position: 'absolute',
					left: 2,
					top: 2,
					right: 2,
					bottom: 2,
					borderRadius: BTN_RADIUS,
					background: `rgba(77,141,255,${(0.05 * hover + 0.14 * press).toFixed(3)})`,
					boxShadow: `inset 0 0 0 1.5px rgba(77,141,255,${(0.55 * hover + 0.35 * press).toFixed(3)})`,
				}}
			/>
		</div>
	);
};

const S12UmClique: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const cam = storyboardCamera(scene);
	const g = planeGeometry(cam, f);
	const head = scene.copy.find((c) => c.role === 'headline')!;
	const lines = unitsOf(head.text, [
		[
			{text: 'Um clique', at: 30},
			{text: 'vira', at: 32},
			{text: 'memória.', at: 34, emphasis: true},
		],
	]);

	// --- cursor ---------------------------------------------------------------
	const tipAt = (fr: number) => mapPt(planeGeometry(cam, fr), TIP);
	let cur: Point;
	if (f <= 4) cur = ENTER;
	else if (f < 26) cur = arcPoint(ENTER, tipAt(26), E.cursor(ramp(f, 4, 26)), 0.12);
	else if (f <= 36) cur = tipAt(f);
	else cur = arcPoint(tipAt(36), REST, E.cursor(ramp(f, 36, 44)), 0.12);
	const camScale = Math.min(1.5, Math.max(1, 1 + (g.k - 1) * 0.35));
	let press = 0;
	if (f >= CLICK - 1 && f <= CLICK + 1) press = ramp(f, CLICK - 1, CLICK + 1);
	else if (f > CLICK + 1) press = 1 - springAt(f, CLICK + 1, 'SNAPPY', 5);
	const cursorSize = 30 * camScale * (1 - 0.15 * clamp01(press));
	const cursorOpacity = 1 - ramp(f, 38, 44, E.exit);
	const clickPt = tipAt(CLICK);

	// --- plane ------------------------------------------------------------------
	const dimOut = 1 - ramp(f, SWAP, SWAP + 10, E.enter);
	const beforeLayer = (
		<>
			<Patches patches={widenLegend(storyboardPatches(scene, BEFORE))} />
		</>
	);
	const afterLayer = (
		<>
			<Patches patches={widenLegend(storyboardPatches(scene, AFTER))} />
			{/* the sidebar's orange "Revisão" badge (a pending count) peeks in at the left edge after the pull-back: cover it in the nav colour */}
			<div style={{position: 'absolute', left: BADGE.x - 3, top: BADGE.y - 3, width: BADGE.w + 6, height: BADGE.h + 6, background: '#14223c'}} />
			{Array.from({length: 11}, (_, i) => {
				const at = SWAP + 1 + 2 * i;
				const v = ramp(f, at, at + 2) * (1 - ramp(f, at + 2, at + 10, E.enter));
				return v > 0.001 ? (
					<div
						key={i}
						style={{position: 'absolute', left: ROW1.x, top: ROW1.y + ROW1.h * i, width: ROW1.w, height: ROW1.h, background: 'rgba(46,204,143,0.12)', opacity: v}}
					/>
				) : null;
			})}
		</>
	);

	return (
		<TransitionOut>
			<TransitionIn>
				<Backdrop seed="s12">
					<G4Plane {...cam} layers={f < SWAP ? [{src: BEFORE, children: beforeLayer}] : [{src: AFTER, children: afterLayer}]}>
						<GlideSpot rect={BAR} dim={0.62 * dimOut} outline={0.55 * (f < SWAP ? 1 : 0)} pad={0} radius={4} />
						<Replica f={f} />
					</G4Plane>
					<Scrim
						background="linear-gradient(180deg, rgba(10,13,22,0.9) 0px, rgba(10,13,22,0.85) 215px, rgba(10,13,22,0) 300px)"
						mask="linear-gradient(90deg, #000 0px, #000 1300px, transparent 1700px)"
						opacity={ramp(f, 26, 32, E.enter)}
					/>
					<StaggerHeadline lines={lines} size={88} left={144} capTops={[110]} />
					<AbsoluteFill style={{pointerEvents: 'none'}}>
						<ClickRipple x={clickPt.x} y={clickPt.y} at={CLICK} />
						{f >= 4 ? <ArrowCursor x={cur.x} y={cur.y} size={cursorSize} opacity={cursorOpacity} /> : null}
					</AbsoluteFill>
				</Backdrop>
			</TransitionIn>
		</TransitionOut>
	);
};

export default S12UmClique;
