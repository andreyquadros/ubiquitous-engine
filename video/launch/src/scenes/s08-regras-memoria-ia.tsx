/**
 * s08-regras-memoria-ia — S08 · abs 510–584 (75 f) · features · bar 9.3 → 10.4
 *
 * Three hits on three beats. Left: a stacked slam "Regras," / "memória," / "IA."
 * (Sora 800 150 px, x 144, cap-tops 250 / 393 / 536) on a canvas scrim. Right:
 * the real "Classificados neste dia (19)" rows, tilted, jump-cutting badge to
 * badge on each hit:
 *   f0  (abs 510) hard cut into the first contact: "Regras," at full opacity,
 *       scale 1.06 → 1 (SLAM), 6-px camera shake over 8 f; the "regra" badge
 *       (row 1, sei.ifro.edu.br) is lit (dim 0 → 0.62 over 8 f) and ringed.
 *  f15  (abs 525) jump-cut down to the "memória" badge (row 3, docs.google.com)
 *       as "memória," slams on line 2; "Regras," settles to ink-2.
 *  f30  (abs 540, bar-10 downbeat) jump-cut to the "IA" badge (row 5, Terminal)
 *       as "IA." slams in volt; canvas lift #0a0d16 → #121a2c → #0a0d16 over 3 f.
 * Each ring pops on its hit (scale 1.12 → 1, SLAM, glow flash) and stays drawn
 * at 60 % after the hop. f30–74 hold on row 5: glow breathes 0.3 → 0.22, the
 * plane drifts (y −12 image px, rx 4 → 3°) and pushes 1.00 → 1.025. f75 cut.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import type {CameraKey, ScreenConfig} from '../components/screen-geometry';
import type {Keyframe} from '../design/motion';
import {E, springAt, useScene, useSceneFrame, type SfxCue, type StoryboardCameraKey} from '../shared';
import {G3Screen} from './_parts/G3/G3Screen';
import {
	boxTopForCapTop,
	clamp01,
	DimMask,
	lerp,
	METRICS,
	mixHex,
	Patch,
	ramp,
	Ring,
	shakeTransform,
	slamShake,
	Stage,
	StageTop,
} from './_parts/G3/common';
import {color, font} from '../design/tokens';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'impact_soft_1.wav', atFrame: 0, gainDb: 0, note: '“Regras,” — duck −5 dB.'},
	{ref: 'impact_soft_2.wav', atFrame: 15, gainDb: 0, note: '“memória,” — duck −5 dB.'},
	{ref: 'impact_soft_3.wav', atFrame: 30, gainDb: 2, note: '“IA.” on the downbeat — duck −5 dB.'},
];

const FILE = 'ui/review-settled-expanded.png';
const HITS = [0, 15, 30] as const;

/**
 * Keys legend of the assign card, measured on the capture: chips x 2128–2789,
 * y ≈ 662–699 (the storyboard's 640 × 32 rect left a sliver of the "↑" chip and
 * the chip bottoms). Solid card colour.
 */
const LEGEND_PATCH = {x: 2124, y: 656, w: 670, h: 48, fill: '#0c1220'};

/** A slam word: full opacity on its contact frame, scale 1.06 → 1 (SLAM), a 4-f white-hot → colour settle. */
const SlamWord: React.FC<{f: number; text: string; at: number; capTop: number; rest: string; dimAt?: number}> = ({f, text, at, capTop, rest, dimAt}) => {
	if (f < at) return null;
	const size = 150;
	const s = springAt(f, at, 'SLAM');
	const scale = 1.06 - 0.06 * s;
	const hot = 1 - ramp(f, at, at + 4, E.enter);
	const settle = dimAt !== undefined ? ramp(f, dimAt, dimAt + 4, E.enter) : 0;
	const base = settle > 0 ? mixHex(rest, color.ink2, settle) : rest;
	const c = hot > 0 ? mixHex(rest, '#ffffff', 0.55 * hot) : settle > 0 ? base : rest;
	return (
		<div
			style={{
				position: 'absolute',
				left: 144,
				top: boxTopForCapTop(METRICS.sora, size, capTop),
				fontFamily: font.display,
				fontWeight: 800,
				fontSize: size,
				lineHeight: 1,
				letterSpacing: '-0.045em',
				whiteSpace: 'pre',
				color: c,
				transformOrigin: '0% 70%',
				transform: Math.abs(scale - 1) > 0.0005 ? `scale(${scale.toFixed(4)})` : undefined,
			}}
		>
			{text}
		</div>
	);
};

/** Canvas lift colour (#121a2c) mixed into an rgb triple → "r,g,b". */
const liftRgb = (base: [number, number, number], t: number) => {
	const to = [18, 26, 44];
	return base.map((v, i) => Math.round(v + (to[i] - v) * clamp01(t))).join(',');
};

const S08RegrasMemoriaIa: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const keys = scene.camera.filter((k) => k.file && k.target !== 'canvas');
	const [kA, kB, kC, kD] = keys as [StoryboardCameraKey, StoryboardCameraKey, StoryboardCameraKey, StoryboardCameraKey];

	/* ---- camera: the storyboard's three hard framings + a micro push inside each hold ---- */
	const PUSH = 1.012; // per 14-f hold (style S11: micro drift ×1.00 → ×1.02)
	const at = (k: StoryboardCameraKey, frame: number, zoom: number, duration: number, easing = E.linear): CameraKey => ({
		at: frame,
		zoom,
		focus: k.focus!,
		anchor: k.anchor!,
		duration,
		easing,
	});
	const camera: CameraKey[] = [
		at(kA, kA.atFrame, kA.zoom, 0),
		at(kA, kB.atFrame - 1, kA.zoom * PUSH, kB.atFrame - 1 - kA.atFrame),
		at(kB, kB.atFrame, kB.zoom, 0),
		at(kB, kC.atFrame - 1, kB.zoom * PUSH, kC.atFrame - 1 - kB.atFrame),
		at(kC, kC.atFrame, kC.zoom, 0),
		at(kD, kD.atFrame, kD.zoom * 1.025, kD.atFrame - kC.atFrame),
	];
	const tiltKeys = (axis: 'rx' | 'ry'): Keyframe[] => [
		[kA.atFrame, kA.tilt![axis]],
		[kB.atFrame - 1, kA.tilt![axis]],
		[kB.atFrame, kB.tilt![axis]],
		[kC.atFrame - 1, kB.tilt![axis]],
		[kC.atFrame, kC.tilt![axis]],
		[kD.atFrame, kD.tilt![axis], E.linear],
	];
	const shot: ScreenConfig = {
		src: FILE,
		width: 1440,
		radius: 18,
		glow: false,
		camera,
		rotateX: tiltKeys('rx'),
		rotateY: tiltKeys('ry'),
	};

	/* ---- badge spotlights (storyboard rects = badge + 16 px), dim 0.62 ---- */
	const spots = scene.spotlights.map((s) => ({rect: s.rect!, from: s.from, to: s.to, dim: s.dim ?? 0.62}));
	const cur = spots.findIndex((s) => f >= s.from && f <= s.to);
	const d = ramp(f, 0, 8, E.enter);
	const onScreen = (1440 / 2880) * kA.zoom;

	// camera hit shake on each contact (whole picture, like a camera bump)
	const hit = [...HITS].reverse().find((h) => f >= h) ?? 0;
	const shake = slamShake(f, hit, `s08-${hit}`, 6, 0.4, 8);

	// canvas lift on the downbeat (abs 540): #0a0d16 → #121a2c → #0a0d16 over 3 f
	// (applied to the CANVAS only — the scrim under the stack and the dim's tint — so the lit badge pops;
	// a full-frame "lighten" veil flattened the whole picture)
	const lift = f === 30 ? 1 : f === 31 ? 0.55 : f === 32 ? 0.18 : 0;
	const canvasRgb = liftRgb([10, 13, 22], lift);

	const [cRegras, cMemoria, cIa] = scene.copy;

	return (
		<Stage>
			<AbsoluteFill style={{transform: shakeTransform(shake)}}>
				<G3Screen {...shot} style={{zIndex: 'auto'}}>
					<Patch {...LEGEND_PATCH} />
					<DimMask dim={0.62 * d} tint={liftRgb([6, 9, 16], lift)} holes={cur >= 0 ? [{...spots[cur].rect, r: 14}] : []} />
					{spots.map((s, i) => {
						if (f < s.from) return null;
						const active = i === cur;
						// pop on the hit: scale 1.12 → 1 (SLAM), glow flash 0.55 → 0.3, then breathe 0.3 → 0.22 on the last hold
						const pop = springAt(f, s.from, 'SLAM');
						const scale = active ? 1.12 - 0.12 * pop : 1;
						const flash = active ? 0.25 * (1 - ramp(f, s.from, s.from + 8, E.enter)) : 0;
						const breathe = active && i === spots.length - 1 ? lerp(0.3, 0.22, 0.5 - 0.5 * Math.cos(Math.PI * ramp(f, 38, 74, E.linear) * 2)) : 0.3;
						const opacity = active ? (i === 0 ? d : 1) : 0.6;
						return (
							<Ring
								key={i}
								rect={s.rect}
								radius={14}
								onScreen={onScreen}
								opacity={opacity}
								scale={scale}
								line={2}
								glow={40}
								glowAlpha={active ? breathe + flash : 0.3}
							/>
						);
					})}
				</G3Screen>
				{/* left canvas scrim under the slam stack: 85 % → 0 at x 1000 */}
				<AbsoluteFill
					style={{
						background: `linear-gradient(90deg, rgba(${canvasRgb},0.85) 0px, rgba(${canvasRgb},0.85) 520px, rgba(${canvasRgb},0.5) 780px, rgba(${canvasRgb},0) 1000px)`,
					}}
				/>
				<SlamWord f={f} text={cRegras.text} at={cRegras.landFrame} capTop={250} rest={color.ink} dimAt={cMemoria.landFrame} />
				<SlamWord f={f} text={cMemoria.text} at={cMemoria.landFrame} capTop={393} rest={color.ink} dimAt={cIa.landFrame} />
				<SlamWord f={f} text={cIa.text} at={cIa.landFrame} capTop={536} rest={color.volt} />
			</AbsoluteFill>
			<StageTop seed="s08" />
		</Stage>
	);
};

export default S08RegrasMemoriaIa;
