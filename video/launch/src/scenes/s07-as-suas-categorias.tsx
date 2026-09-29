/**
 * s07-as-suas-categorias — S07 · abs 450–509 (60 f) · features · bar 8.3 → 9.3
 *
 * v2. The real Categorias editor (IFRO) arrives on the whip-left in-half (f0–3,
 * TransitionIn: the whole shot travels +960 → 0 with the horizontal blur that
 * pairs with s06's out-half). The window sits low and back on the navy stage;
 * its description field (label + textarea + hint) LIFTS out of it as a big
 * floating card (f4, snappy spring, lands ≈ f14 with the headline) and the
 * window keeps an empty socket where it was (LiftHole).
 *
 *  - "Suas categorias." Sora 700 184 px (−0.04em, top-lit gradient, "Suas" volt
 *    gradient + glow), S03 stagger f3 / f6 → landed f15, on the stage above the
 *    card (no scrim: the window sits below the headline band).
 *  - The typing is the subject: the capture's own value is re-typed live at
 *    2 chars/f (f8–52) inside the card, re-set at 44 image px (≈ 50 px on the
 *    canvas, two lines), under the field label re-set at 36 image px (≈ 41 px).
 *    The textarea takes a volt focus ring and the card's volt glow swells while
 *    typing, then breathes back once the caret blinks.
 *  - Secondary motion in the hold: window push + pan (camera), card float, tilt
 *    settle, parallax drift, glow breath, stage aurora.
 * f60 hard cut to s08.
 */
import React from 'react';
import {useVideoConfig} from 'remotion';
import type {ScreenConfig, CameraKey, Rect} from '../components/screen-geometry';
import {mapImageRect} from '../components/screen-geometry';
import {LiftCard, LiftHole, liftCardPose, type LiftCardProps} from '../components/LiftCard';
import type {Keyframe} from '../design/motion';
import {color, font} from '../design/tokens';
import {E, TransitionIn, useSceneFrame, useScene, type SfxCue} from '../shared';
import {G3Screen} from './_parts/G3/G3Screen';
import {boxTopForBaseline, DimMask, METRICS, ramp, Ring, Stage, StageTop, Words} from './_parts/G3/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'whip_2.wav', atFrame: 0, gainDb: -8, note: 'Whip pass-by on the cut abs 450; file starts 2 f earlier (master track).'},
	{ref: 'whoosh_in_3.wav', atFrame: 12, gainDb: -22, note: 'v2: the description field lifts out of the window toward the camera (lands ≈ f12–14).'},
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
const COMP = {width: 1920, height: 1080};

/** Panel colour around the field (sampled at (1335, 640), (2000, 900)). */
const PANEL = '#0c1220';
/**
 * The description textarea, measured on ui/categories.png (image px):
 * 2-px border #303f5b at x 1340–1341 / 2788–2789, y 690–691 / 880–881, outer
 * radius ≈ 16; fill #121a2b.
 */
const TEXTAREA = {x: 1340, y: 690, w: 1450, h: 192, r: 16};
const TA_FILL = '#121a2b';
/** Patch over the textarea interior (hides the captured value; leaves the resize grip at 2772–2786). */
const TA_PATCH = {x: 1356, y: 704, w: 1412, h: 160};
/** The field label ("O que conta como trabalho desta categoria", Inter 500, cap-top ≈ 645, baseline ≈ 667). */
const LABEL_PATCH = {x: 1336, y: 634, w: 840, h: 48};
const LABEL = {left: 1342, baseline: 673, size: 36, color: '#e3e9f4', text: 'O que conta como trabalho desta categoria'};
/** Re-typed value: Inter 400 at 44 image px (capture: 28), wrapping inside the textarea interior. */
const TYPE = {left: 1369, firstBaseline: 756, size: 44, lineHeight: 58, width: 1390, color: '#f2f6fc'};
/** The capture's own description value, first line (88 chars). */
const TYPED = 'Docência no IFRO Campus Porto Velho Calama: aulas de Programação Web, orientação de TCC,';
const TYPE_START = 8;
const CPF = 2; // chars per frame
const TYPE_END = TYPE_START + Math.ceil(TYPED.length / CPF); // 52

/** The lifted crop: label + textarea + hint line (image px). */
const FIELD: Rect = {x: 1322, y: 612, w: 1490, h: 324};

/** Typed layer (image space, drawn inside the card): patches, label re-set, the re-typed value and the caret. */
const Typing: React.FC<{f: number}> = ({f}) => {
	const n = f < TYPE_START ? 0 : Math.min(TYPED.length, (f - TYPE_START) * CPF);
	const caretOn = f >= TYPE_START && (f <= TYPE_END || Math.floor((f - (TYPE_END + 1)) / 8) % 2 === 0);
	const top = boxTopForBaseline(METRICS.inter, TYPE.size, TYPE.firstBaseline, TYPE.lineHeight / TYPE.size);
	const capH = METRICS.inter.cap * TYPE.size;
	return (
		<>
			<div style={{position: 'absolute', left: LABEL_PATCH.x, top: LABEL_PATCH.y, width: LABEL_PATCH.w, height: LABEL_PATCH.h, background: PANEL}} />
			<div
				style={{
					position: 'absolute',
					left: LABEL.left,
					top: boxTopForBaseline(METRICS.inter, LABEL.size, LABEL.baseline),
					fontFamily: font.text,
					fontWeight: 500,
					fontSize: LABEL.size,
					lineHeight: 1,
					letterSpacing: '-0.01em',
					color: LABEL.color,
					whiteSpace: 'pre',
				}}
			>
				{LABEL.text}
			</div>
			<div style={{position: 'absolute', left: TA_PATCH.x, top: TA_PATCH.y, width: TA_PATCH.w, height: TA_PATCH.h, background: TA_FILL}} />
			<div
				style={{
					position: 'absolute',
					left: TYPE.left,
					top,
					width: TYPE.width,
					fontFamily: font.text,
					fontWeight: 400,
					fontSize: TYPE.size,
					lineHeight: `${TYPE.lineHeight}px`,
					letterSpacing: '-0.005em',
					color: TYPE.color,
					whiteSpace: 'normal',
					overflowWrap: 'normal',
				}}
			>
				{TYPED.slice(0, n)}
				{caretOn ? (
					<span
						style={{
							display: 'inline-block',
							width: 4,
							height: capH + 14,
							marginLeft: 3,
							verticalAlign: -7,
							background: color.volt,
							borderRadius: 2,
							boxShadow: '0 0 12px rgba(77,141,255,0.7)',
						}}
					/>
				) : null}
			</div>
		</>
	);
};

const S07AsSuasCategorias: React.FC = () => {
	const scene = useScene();
	const {fps} = useVideoConfig();
	const {frame: f} = useSceneFrame();

	/* ---- window: low and back, a slow push + pan through the hold ---- */
	const WZ = 1.0;
	const FOCUS = {x: FIELD.x + FIELD.w / 2, y: FIELD.y + FIELD.h / 2};
	const camera: CameraKey[] = [
		{at: 0, zoom: WZ, focus: FOCUS, anchor: {x: 1273, y: 810}, duration: 0},
		{at: 59, zoom: WZ * 1.05, focus: FOCUS, anchor: {x: 1250, y: 798}, duration: 59, easing: E.linear},
	];
	const shot: ScreenConfig = {
		src: FILE,
		width: 1440,
		radius: 18,
		float: 3,
		camera,
		rotateX: [[0, 7], [59, 5, E.linear]] as Keyframe[],
		rotateY: [[0, -9], [59, -7, E.linear]] as Keyframe[],
	};

	/* ---- the lifted field card ---- */
	const LIFT_AT = 4;
	const lift = {rect: FIELD, at: LIFT_AT, enter: 'lift' as const, spring: 'snappy' as const};
	const typing = ramp(f, TYPE_START, TYPE_START + 6, E.enter) * (1 - 0.55 * ramp(f, TYPE_END, TYPE_END + 8, E.enter));
	const breathe = f > TYPE_END ? 0.04 * Math.sin(((f - TYPE_END) / 16) * Math.PI) : 0;
	const cardProps: LiftCardProps = {
		src: FILE,
		...lift,
		from: (fr) => mapImageRect(shot, fr, COMP, FIELD),
		x: [[LIFT_AT, 960], [59, 948, E.linear]] as Keyframe[],
		y: 548,
		width: [[LIFT_AT, 1700], [59, 1730, E.linear]] as Keyframe[],
		rotateX: [[LIFT_AT, 6], [59, 3, E.linear]] as Keyframe[],
		rotateY: [[LIFT_AT, -7], [59, -3.5, E.linear]] as Keyframe[],
		float: 4,
		floatPeriod: 80,
		radius: 22,
		glow: 'volt',
		glowOpacity: 0.4 + 0.3 * typing + breathe,
		style: {zIndex: 30},
	};
	const pose = liftCardPose(cardProps, f, fps);
	const focusRing = ramp(f, TYPE_START - 2, TYPE_START + 4, E.enter);

	// the window steps back a little once the card is out (a light navy dim, not a blackout)
	const back = 0.1 * ramp(f, LIFT_AT, LIFT_AT + 10, E.enter);

	const headline = scene.copy[0];
	const [w1, w2] = headline.text.split(' ');

	return (
		<Stage
			seed="s07"
			look={{
				keyPool: {x: 0.52, y: 0.46, w: 0.92, h: 0.9, opacity: 0.48},
				keyLight: {x: 0.5, y: 0.54, w: 0.62, h: 0.5, opacity: 0.16},
			}}
		>
			<TransitionIn>
				<G3Screen {...shot} style={{zIndex: 'auto'}}>
					<DimMask dim={back} holes={[]} />
					<LiftHole {...lift} color={PANEL} radius={24} />
				</G3Screen>
				<LiftCard {...cardProps}>
					<Typing f={f} />
					<Ring rect={TEXTAREA} radius={TEXTAREA.r} onScreen={pose.k * pose.s} opacity={focusRing} line={2.5} glow={30} glowAlpha={0.22 + 0.2 * typing} />
				</LiftCard>
			</TransitionIn>
			<Words
				f={f}
				size={184}
				left={108}
				capTop={98}
				letterSpacing="-0.04em"
				units={[
					{text: w1, at: 3, volt: headline.emphasis.includes(w1)},
					{text: w2, at: 6, volt: headline.emphasis.includes(w2)},
				]}
			/>
			<StageTop seed="s07" vignette={0.5} />
		</Stage>
	);
};

export default S07AsSuasCategorias;
