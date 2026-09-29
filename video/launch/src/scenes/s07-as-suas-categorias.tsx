/**
 * s07-as-suas-categorias — S07 · abs 450–509 (60 f) · features · bar 8.3 → 9.3
 *
 * Close on the real Categorias editor (IFRO). f0–3 whip-left in-half (the plane
 * arrives from +960 px, blur 40 → 0). f3/f6 "Suas categorias." word stagger
 * ("Suas" volt), landed by f15, over a top canvas scrim. f6 the field block is
 * lit (dim 0 → 0.55 over 12 f) and the textarea takes the app's focus ring. f8 a
 * volt caret appears; f8–52 the capture's own description value is re-typed live
 * at 2 chars/f as a vector layer glued to the plane (Inter 400, the capture's
 * size/colour), then the caret holds. The camera drifts 20 px left and pushes
 * 1.00 → 1.02 (the @3x twin keeps the bitmap ≤ 0.95×). f60 hard cut.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import type {ScreenConfig, CameraKey} from '../components/screen-geometry';
import {color, font} from '../design/tokens';
import {E, TransitionIn, useScene, useSceneFrame, type SfxCue} from '../shared';
import {G3Screen} from './_parts/G3/G3Screen';
import {boxTopForBaseline, METRICS, ramp, Ring, Stage, StageTop, Words} from './_parts/G3/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'whip_2.wav', atFrame: 0, gainDb: -8, note: 'Whip pass-by on the cut abs 450; file starts 2 f earlier (master track).'},
	{ref: 'type-1.wav', atFrame: 8, gainDb: -22},
	{ref: 'type-2.wav', atFrame: 12, gainDb: -22},
	{ref: 'type-3.wav', atFrame: 16, gainDb: -22},
	{ref: 'type-4.wav', atFrame: 20, gainDb: -22},
	{ref: 'type-1.wav', atFrame: 24, gainDb: -22},
	{ref: 'type-2.wav', atFrame: 28, gainDb: -22},
	{ref: 'type-3.wav', atFrame: 32, gainDb: -22},
	{ref: 'type-4.wav', atFrame: 36, gainDb: -22},
	{ref: 'type-1.wav', atFrame: 40, gainDb: -22},
	{ref: 'type-2.wav', atFrame: 44, gainDb: -22},
	{ref: 'type-3.wav', atFrame: 48, gainDb: -22},
];

const FILE = 'ui/categories.png';

/**
 * The description textarea, measured on ui/categories.png (image px):
 * 2-px border #303f5b at x 1340–1341 / 2788–2789, y 690–691 / 880–881, outer
 * radius ≈ 16; fill #121a2b. Captured text: Inter 400, cap height 21 px
 * (D rows 717–737) → 28 px, colour #eaf0fa, first baseline y 738, left x 1369,
 * line pitch 40.
 */
const TEXTAREA = {x: 1340, y: 690, w: 1450, h: 192, r: 16};
const TA_FILL = '#121a2b';
/** Patch over the textarea interior (the storyboard's rect): hides the captured value. */
const TA_PATCH = {x: 1356, y: 705, w: 1418, h: 170};
const TYPE = {left: 1369, baseline: 736, size: 28, color: '#eaf0fa'};
/** The capture's own description value, first line (88 chars). */
const TYPED = 'Docência no IFRO Campus Porto Velho Calama: aulas de Programação Web, orientação de TCC,';
const TYPE_START = 8;
const CPF = 2; // chars per frame

/** Field block lit from f6 (label + textarea + hint). */
const FIELD = {x: 1330, y: 630, w: 1470, h: 280};

/** Typed layer (image space): the patch, the re-typed value and the caret. */
const Typing: React.FC<{f: number}> = ({f}) => {
	const n = f < TYPE_START ? 0 : Math.min(TYPED.length, (f - TYPE_START) * CPF);
	const caretOn = f >= TYPE_START && (f <= TYPE_START + Math.ceil(TYPED.length / CPF) || Math.floor((f - (TYPE_START + TYPED.length / CPF + 1)) / 8) % 2 === 0);
	const top = boxTopForBaseline(METRICS.inter, TYPE.size, TYPE.baseline);
	const capH = METRICS.inter.cap * TYPE.size;
	return (
		<>
			<div style={{position: 'absolute', left: TA_PATCH.x, top: TA_PATCH.y, width: TA_PATCH.w, height: TA_PATCH.h, background: TA_FILL}} />
			<div
				style={{
					position: 'absolute',
					left: TYPE.left,
					top,
					fontFamily: font.text,
					fontWeight: 400,
					fontSize: TYPE.size,
					lineHeight: 1,
					color: TYPE.color,
					whiteSpace: 'pre',
				}}
			>
				{TYPED.slice(0, n)}
				{caretOn ? (
					<span
						style={{
							display: 'inline-block',
							width: 3,
							height: capH + 12,
							marginLeft: 2,
							verticalAlign: `${-5}px`,
							background: color.volt,
							borderRadius: 1.5,
							boxShadow: '0 0 10px rgba(77,141,255,0.55)',
						}}
					/>
				) : null}
			</div>
		</>
	);
};

const S07AsSuasCategorias: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const [k0, k1] = scene.camera; // f0 (arrives with the whip-in) and f59 (drift)
	const tilt = k0.tilt ?? {rx: 3, ry: -3};
	// Framing: the storyboard's zoom 2.8 put the end of the typed line ("…TCC,") at x ≈ 1905 once the −3° yaw
	// is applied (measured on a still), clipping the caret. Zoom 2.7 (bitmap 0.9× on the @3x twin; label
	// 26 → 35 px, typed text 28 → 38 px) with the anchor 40 px further left centres the typed line
	// (x ≈ 94–1825 at the end). The storyboard's 20-px drift to the left (following the caret) is kept.
	const ZOOM = 2.7;
	const DX = -40;
	const camera: CameraKey[] = [
		{at: k0.atFrame, zoom: ZOOM, focus: k0.focus!, anchor: {x: k0.anchor!.x + DX, y: k0.anchor!.y}, duration: 0},
		{at: k1.atFrame, zoom: ZOOM, focus: k1.focus!, anchor: {x: k1.anchor!.x + DX, y: k1.anchor!.y}, duration: k1.atFrame - k0.atFrame, easing: E.linear},
	];
	const shot: ScreenConfig = {
		src: FILE,
		width: 1440,
		radius: 18,
		float: 3,
		glow: false,
		camera,
		rotateX: tilt.rx,
		rotateY: tilt.ry,
	};

	// spotlight on the field block: dim 0 → 0.55 over 12 f from f6 (E.enter); the textarea takes focus with it
	const lit = ramp(f, 6, 18, E.enter);
	const dim = 0.55 * lit;
	const focusRing = ramp(f, 6, 14, E.enter);
	const onScreen = (1440 / 2880) * ZOOM;

	const headline = scene.copy[0];
	const [w1, w2] = headline.text.split(' ');
	const scrim = ramp(f, 0, 6, E.enter);

	return (
		<Stage>
			<TransitionIn>
				<G3Screen {...shot} style={{zIndex: 'auto'}}>
					<Typing f={f} />
					{/* spotlight: dim outside the field block (no outline — the focus ring is the textarea's own) */}
					{dim > 0.001 ? (
						<div
							style={{
								position: 'absolute',
								left: FIELD.x,
								top: FIELD.y,
								width: FIELD.w,
								height: FIELD.h,
								borderRadius: 20,
								boxShadow: `0 0 0 6000px rgba(6,9,16,${dim.toFixed(4)})`,
							}}
						/>
					) : null}
					<Ring rect={TEXTAREA} radius={TEXTAREA.r} onScreen={onScreen} opacity={focusRing} line={2} glow={28} glowAlpha={0.28} />
				</G3Screen>
			</TransitionIn>
			{/* top canvas scrim under the headline: 94 % 0–244 px → 0 at 304 (90 % left the editor title "IFRO" ghosting through the headline) */}
			<AbsoluteFill
				style={{
					opacity: scrim,
					background: 'linear-gradient(180deg, rgba(10,13,22,0.94) 0px, rgba(10,13,22,0.94) 244px, rgba(10,13,22,0) 304px)',
				}}
			/>
			<Words
				f={f}
				size={96}
				left={144}
				capTop={110}
				letterSpacing="-0.035em"
				units={[
					{text: w1, at: 3, volt: headline.emphasis.includes(w1)},
					{text: w2, at: 6, volt: headline.emphasis.includes(w2)},
				]}
			/>
			<StageTop seed="s07" />
		</Stage>
	);
};

export default S07AsSuasCategorias;
