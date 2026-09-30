/**
 * s08-regras-memoria-ia — S08 · abs 510–584 (75 f) · features · bar 9.3 → 10.4
 *
 * v2. Three hits on three beats, word ↔ proof. Left: the slam stack "Regras," /
 * "memória," / "IA." (Sora 800 184 px, −0.045em, top-lit gradient; "IA." volt
 * gradient + glow), x 104, cap-tops 236 / 432 / 628. Right: the real
 * "Classificados neste dia (19)" list in a tilted window that sits back on the
 * stage; on each hit the matching ORIGIN BADGE lifts out of its row as a big
 * floating card (≈ 2.4×, badge text ≈ 53 px) and flies left to sit beside its
 * word; the row keeps an empty socket (LiftHole).
 *   f0  (abs 510) hard cut straight into the first contact: "Regras," SLAM
 *       (scale 1.06 → 1, 4-f white-hot settle) + 6-px camera shake; the "regra"
 *       badge (row 1, sei.ifro.edu.br) lifts.
 *  f15  (abs 525) jump-cut down the list (the window re-frames on row 3) as
 *       "memória," slams; the "memória" badge (row 3, docs.google.com) lifts;
 *       "Regras," and its card step back.
 *  f30  (abs 540, bar-10 downbeat) jump-cut to row 5 as "IA." slams in volt; the
 *       "IA" badge (row 5, Terminal) lifts; the stage lights flash (level
 *       1 → 1.35 → 1 over 4 f, the v1 canvas lift).
 * Each card's volt ring pops on its hit (scale 1.12 → 1, glow flash) and stays
 * at 45 % after the next hit. f30–74 hold: the IA card's glow breathes, every
 * card floats on its own phase, the window pushes 1.00 → 1.03 with rx 4 → 3°,
 * the aurora sweeps. f75 hard cut to s09.
 */
import React from 'react';
import {AbsoluteFill, useVideoConfig} from 'remotion';
import type {CameraKey, Rect, ScreenConfig} from '../components/screen-geometry';
import {mapImageRect} from '../components/screen-geometry';
import {LiftCard, LiftHole, liftCardPose, type LiftCardProps} from '../components/LiftCard';
import type {Keyframe} from '../design/motion';
import {E, springAt, useScene, useSceneFrame, type SfxCue} from '../shared';
import {G3Screen} from './_parts/G3/G3Screen';
import {
	boxTopForCapTop,
	gradientInk,
	lerp,
	METRICS,
	Patch,
	ramp,
	Ring,
	shakeTransform,
	slamShake,
	Stage,
	StageTop,
	voltGlow,
} from './_parts/G3/common';
import {font} from '../design/tokens';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'impact_soft_1.wav', atFrame: 0, gainDb: 0, note: '“Regras,” — duck −5 dB.'},
	{ref: 'impact_soft_2.wav', atFrame: 15, gainDb: 0, note: '“memória,” — duck −5 dB.'},
	{ref: 'impact_soft_3.wav', atFrame: 30, gainDb: 2, note: '“IA.” on the downbeat — duck −5 dB.'},
];

const FILE = 'ui/review-settled-expanded.png';
const COMP = {width: 1920, height: 1080};
const HITS = [0, 15, 30] as const;

/**
 * Keys legend of the assign card, measured on the capture: chips x 2128–2789,
 * y ≈ 662–699 (the storyboard's 640 × 32 rect left a sliver of the "↑" chip and
 * the chip bottoms). Solid card colour.
 */
const LEGEND_PATCH = {x: 2124, y: 656, w: 670, h: 48, fill: '#0c1220'};
/** List panel colour around the badges (sampled at (1700, 560), (2000, 586)). */
const PANEL = '#0c1220';
/**
 * Confidence percentages (texture; at the window's size they are ≈ 11 px, but a
 * legible "100%" beside "regra" would read as an accuracy claim — facts §6.13).
 * Measured glyph boxes + 4 px: rows 1–5.
 */
const PCT_PATCHES = [
	// the two rows above the group header
	{x: 1781, y: 134, w: 52, h: 24},
	{x: 1781, y: 246, w: 52, h: 24},
	// rows 1–5
	{x: 1735, y: 572, w: 66, h: 24},
	{x: 1781, y: 686, w: 52, h: 24},
	{x: 1714, y: 800, w: 49, h: 24},
	{x: 1713, y: 914, w: 50, h: 24},
	{x: 1781, y: 1028, w: 52, h: 24},
	// rows 6–11 (legible at ≈ 13 px now that the window is shown whole)
	{x: 1713, y: 1142, w: 50, h: 24},
	{x: 1751, y: 1256, w: 50, h: 24},
	{x: 1713, y: 1370, w: 50, h: 24},
	{x: 1785, y: 1484, w: 48, h: 24},
	{x: 1781, y: 1598, w: 52, h: 24},
	{x: 1785, y: 1712, w: 48, h: 24},
];
/** Sidebar "Revisão" pending-count badge ("8", orange disc at 376–413 × 378–413) on the active-row fill: the film never shows a pending number. */
const REVIEW_COUNT_PATCH = {x: 370, y: 372, w: 50, h: 48, fill: '#14223c'};

/** Origin badges (storyboard rects, image px): row 1 "regra", row 3 "memória", row 5 "IA". */
const BADGES: Rect[] = [
	{x: 1826, y: 566, w: 114, h: 40},
	{x: 1788, y: 792, w: 152, h: 40},
	{x: 1858, y: 1020, w: 82, h: 40},
];
/** Card crop = badge + 22 px (x) / 16 px (y) of the row around it. */
const crop = (b: Rect): Rect => ({x: b.x - 22, y: b.y - 16, w: b.w + 44, h: b.h + 32});
const CROPS = BADGES.map(crop);
/** Card scale: comp px per image px (badge text 22 image px → ≈ 53 px). */
const K = 2.4;
/** Stack geometry. */
const SLAM = {size: 188, left: 100, capTops: [232, 432, 632]};
const CAP = METRICS.sora.cap * SLAM.size;
/** Card column: left edge + vertical centre on each word's cap band. */
const CARD_LEFT = [880, 1115, 470];
const GLOWS = ['volt', '#8a78ff', 'volt'] as const;

/** A slam word: full opacity on its contact frame, scale 1.06 → 1 (SLAM), a 4-f white-hot → colour settle, then steps back. */
const SlamWord: React.FC<{f: number; text: string; at: number; capTop: number; volt?: boolean; dimAt?: number}> = ({f, text, at, capTop, volt = false, dimAt}) => {
	if (f < at) return null;
	const s = springAt(f, at, 'SLAM');
	const scale = 1.06 - 0.06 * s;
	const hot = 1 - ramp(f, at, at + 4, E.enter);
	const settle = dimAt !== undefined ? ramp(f, dimAt, dimAt + 4, E.enter) : 0;
	const ink = gradientInk(volt ? 1 - 0.7 * hot : 0, 0.62 * settle);
	return (
		<div
			style={{
				position: 'absolute',
				left: SLAM.left,
				top: boxTopForCapTop(METRICS.sora, SLAM.size, capTop),
				fontFamily: font.display,
				fontWeight: 800,
				fontSize: SLAM.size,
				lineHeight: 1,
				letterSpacing: '-0.045em',
				whiteSpace: 'pre',
				transformOrigin: '0% 70%',
				transform: Math.abs(scale - 1) > 0.0005 ? `scale(${scale.toFixed(4)})` : undefined,
				filter: [volt ? voltGlow(1) : '', hot > 0.01 ? `brightness(${(1 + 0.6 * hot).toFixed(3)})` : ''].join(' ').trim() || undefined,
			}}
		>
			<span style={{...ink, display: 'inline-block', padding: '0 0.04em'}}>{text}</span>
		</div>
	);
};

const S08RegrasMemoriaIa: React.FC = () => {
	const scene = useScene();
	const {fps} = useVideoConfig();
	const {frame: f} = useSceneFrame();

	/* ---- window: back on the stage, right; hard re-frame on each hit (jump-cut down the list) ----
	 * ANCHOR x 1860 / Z 0.86 (was 1790 / 0.9): the window's left border stays ≥ 50 px clear of the
	 * "memória," comma (right edge ≈ 1024 on the f15 contact, 971 settled) on every frame f15–74,
	 * while the badge column (focus x 1883) stays inside the frame. */
	const ROW_Y = BADGES.map((b) => b.y + b.h / 2);
	const ANCHOR = {x: 1860, y: 560};
	const Z = 0.86;
	const at = (frame: number, row: number, zoom: number, duration: number): CameraKey => ({
		at: frame,
		zoom,
		focus: {x: 1883, y: ROW_Y[row]},
		anchor: ANCHOR,
		duration,
		easing: E.linear,
	});
	const camera: CameraKey[] = [
		at(0, 0, Z, 0),
		at(14, 0, Z * 1.012, 14),
		at(15, 1, Z * 1.03, 0),
		at(29, 1, Z * 1.042, 14),
		at(30, 2, Z * 1.06, 0),
		at(74, 2, Z * 1.09, 44),
	];
	const shot: ScreenConfig = {
		src: FILE,
		width: 1440,
		radius: 18,
		float: 3,
		camera,
		rotateX: [[0, 5], [14, 5], [15, 4], [29, 4], [30, 5], [74, 3.5, E.linear]] as Keyframe[],
		rotateY: [[0, -14], [14, -14], [15, -12], [29, -12], [30, -14], [74, -12, E.linear]] as Keyframe[],
	};

	// camera hit shake on each contact (whole picture, like a camera bump)
	const hit = [...HITS].reverse().find((h) => f >= h) ?? 0;
	const shake = slamShake(f, hit, `s08-${hit}`, 6, 0.4, 8);
	// downbeat light flash (the v1 canvas lift): stage level 1 → 1.35 → 1 over f30–33
	const flash = f === 30 ? 1 : f === 31 ? 0.6 : f === 32 ? 0.25 : f === 33 ? 0.08 : 0;

	/* ---- the three badge cards ---- */
	const cards = BADGES.map((b, i) => {
		const h = HITS[i];
		const rect = CROPS[i];
		const w = rect.w * K;
		const cy = SLAM.capTops[i] + CAP / 2;
		const cx = CARD_LEFT[i] + w / 2;
		const next = HITS[i + 1];
		const back = next !== undefined ? ramp(f, next, next + 6, E.enter) : 0;
		const breathe = i === 2 ? 0.08 * Math.sin((Math.max(0, f - 38) / 22) * Math.PI) : 0;
		const props: LiftCardProps = {
			src: FILE,
			rect,
			at: h,
			enter: 'lift',
			spring: 'snappy',
			from: (fr) => mapImageRect(shot, fr, COMP, rect),
			x: [[h, cx], [74, cx - 10 - 4 * i, E.linear]] as Keyframe[],
			y: cy,
			width: w,
			rotateX: [[h, 8], [74, 5, E.linear]] as Keyframe[],
			rotateY: [[h, -14], [74, -9, E.linear]] as Keyframe[],
			float: 5,
			floatPeriod: 70 + 11 * i,
			radius: 20,
			glow: GLOWS[i],
			glowOpacity: lerp(0.55, 0.26, back) + breathe,
			// the cards sit in the key light, closer to the camera than the window: a touch brighter than its grade
			grade: {brightness: 1.3, contrast: 1.05, saturate: 1.2, lift: 0.07},
			style: {zIndex: 30 + i},
		};
		return {props, badge: b, h, back};
	});

	const [cRegras, cMemoria, cIa] = scene.copy;

	return (
		<Stage
			seed="s08"
			look={{
				level: 1.12 + 0.3 * flash,
				keyPool: {x: 0.52, y: 0.48, w: 0.86, h: 0.98, opacity: 0.45},
				keyLight: {x: 0.58, y: 0.5, w: 0.5, h: 0.62, opacity: 0.18},
			}}
		>
			<AbsoluteFill style={{transform: shakeTransform(shake)}}>
				<G3Screen {...shot} style={{zIndex: 'auto'}}>
					<Patch {...LEGEND_PATCH} />
					<Patch {...REVIEW_COUNT_PATCH} />
					{PCT_PATCHES.map((p, i) => (
						<Patch key={i} {...p} fill={PANEL} />
					))}
					{cards.map((c, i) => (
						<LiftHole key={i} rect={c.badge} at={c.h} enter="lift" color={PANEL} radius={12} pad={8} feather={14} />
					))}
				</G3Screen>
				{cards.map((c, i) => {
					const pose = liftCardPose(c.props, f, fps);
					const pop = springAt(f, c.h, 'SLAM');
					const flashRing = 0.3 * (1 - ramp(f, c.h, c.h + 8, E.enter));
					return (
						<LiftCard key={i} {...c.props}>
							<Ring
								rect={c.badge}
								radius={10}
								onScreen={pose.k * pose.s}
								opacity={f < c.h ? 0 : lerp(1, 0.45, c.back)}
								scale={1.12 - 0.12 * pop}
								line={2.5}
								glow={34}
								glowAlpha={0.3 + flashRing}
							/>
						</LiftCard>
					);
				})}
				<SlamWord f={f} text={cRegras.text} at={cRegras.landFrame} capTop={SLAM.capTops[0]} dimAt={cMemoria.landFrame} />
				<SlamWord f={f} text={cMemoria.text} at={cMemoria.landFrame} capTop={SLAM.capTops[1]} dimAt={cIa.landFrame} />
				<SlamWord f={f} text={cIa.text} at={cIa.landFrame} capTop={SLAM.capTops[2]} volt />
			</AbsoluteFill>
			<StageTop seed="s08" vignette={0.5} />
		</Stage>
	);
};

export default S08RegrasMemoriaIa;
