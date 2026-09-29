/**
 * s09-so-pergunta — S09 · abs 585–659 (75 f) · features · bar 10.4 → 12.1
 *
 * Revisão: the queue only holds what the chain could not settle.
 *  f0  (abs 585) hard cut, medium framing on the real Revisão page (rx 4° ry −6°):
 *      the selected Calendário group and three Finder groups ("Sem categoria ·
 *      0 % · pendente") and the assign card. Micro push ×1.02 over the hold.
 *  f2–16  "Só pergunta o que não sabe." word stagger (5 units, "o que" glued, 2 f)
 *      on a solid bottom band; f18–28 a volt marker sweeps behind "não sabe.".
 *  f30 (abs 615) internal jump-cut closer on rows 1–4; f34 everything else dims
 *      to 0.5; f44–50 the four "pendente" pills pulse an ember ring (2-f row stagger).
 *  f44–74 E.glide span push toward the picker (IFRO · 1 on top), which is un-dimmed
 *      as the camera arrives; the scene cuts on the downbeat abs 660 on exactly the
 *      storyboard's f74 key (s10 holds it as its blurred backdrop).
 * Claim patches (solid, image space): the selected row's key-range hint line and
 * the keys legend. The bottom band hides rows 6+ (incl. the mail.google.com group).
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import type {CameraKey, ScreenConfig} from '../components/screen-geometry';
import type {Keyframe} from '../design/motion';
import {E, useScene, useSceneFrame, type SfxCue, type StoryboardCameraKey} from '../shared';
import {G3Screen} from './_parts/G3/G3Screen';
import {DimMask, lerp, Patch, ramp, Stage, StageTop, Words, type WordUnit} from './_parts/G3/common';

/** None: music only (≥ 30 % of the features act stays SFX-free). */
export const sfx: SfxCue[] = [];

const FILE = 'ui/review-queue-selected.png';

/**
 * Claim-safety patches, measured on the capture (image px):
 *  - the selected row's second line "Pressione 1 a 9 …" (ink x 636–1846, y 398–437 incl.
 *    descenders; the storyboard's 1222 × 32 rect left the descenders) on the row colour #15233f;
 *  - the keys legend (chips x 2128–2789, y ≈ 804–842) on the card colour #0c1220.
 */
const HINT_PATCH = {x: 626, y: 394, w: 1232, h: 48, fill: '#15233f'};
const LEGEND_PATCH = {x: 2124, y: 800, w: 670, h: 48, fill: '#0c1220'};
/** Integrator: the sidebar's orange 'Revisão' pending-count badge (nav-revisao-badge), covered in the
 *  active-row colour as s12, s14, s15 and s16 do, so no second count of the dataset reads in shot A. */
const BADGE_PATCH = {x: 373, y: 375, w: 44, h: 42, fill: '#14223c'};

/** The four "pendente" pills of rows 1–4 (measured ink boxes, image px). */
const PILLS = [274, 498, 610, 722].map((y) => ({x: 1812, y, w: 128, h: 40}));
const PULSE_AT = 44; // onset: first moving frame one before the beat (f45)

const PendingPulse: React.FC<{f: number}> = ({f}) => (
	<>
		{PILLS.map((p, i) => {
			const t0 = PULSE_AT + 2 * i;
			if (f < t0 || f > t0 + 16) return null;
			const u = ramp(f, t0, t0 + 15, E.push);
			const pad = lerp(2, 20, u);
			const a = 0.9 * (1 - ramp(f, t0 + 2, t0 + 16, E.enter));
			const fill = 0.22 * (1 - ramp(f, t0, t0 + 10, E.enter));
			return (
				<React.Fragment key={i}>
					<div style={{position: 'absolute', left: p.x, top: p.y, width: p.w, height: p.h, borderRadius: 10, background: `rgba(255,122,31,${fill.toFixed(3)})`, boxShadow: `inset 0 0 0 2px rgba(255,122,31,${(0.8 * (1 - u)).toFixed(3)})`}} />
					<div
						style={{
							position: 'absolute',
							left: p.x - pad,
							top: p.y - pad,
							width: p.w + 2 * pad,
							height: p.h + 2 * pad,
							borderRadius: 10 + pad,
							border: `3px solid rgba(255,122,31,${a.toFixed(3)})`,
							boxShadow: `0 0 18px rgba(255,122,31,${(0.35 * a).toFixed(3)})`,
						}}
					/>
				</React.Fragment>
			);
		})}
	</>
);

const S09SoPergunta: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const [kA, kB, kC] = scene.camera.filter((k) => k.file) as [StoryboardCameraKey, StoryboardCameraKey, StoryboardCameraKey];

	/* ---- camera: shot A (medium) → jump-cut shot B (rows 1–4) → E.glide span to the picker ---- */
	const glideStart = kC.atFrame - (kC.duration ?? 30); // f44
	const camera: CameraKey[] = [
		{at: kA.atFrame, zoom: kA.zoom, focus: kA.focus!, anchor: kA.anchor!, duration: 0},
		// micro push over shot A so the medium hold is never a still
		{at: kB.atFrame - 1, zoom: kA.zoom * 1.02, focus: kA.focus!, anchor: kA.anchor!, duration: kB.atFrame - 1 - kA.atFrame, easing: E.linear},
		{at: kB.atFrame, zoom: kB.zoom, focus: kB.focus!, anchor: kB.anchor!, duration: 0},
		{at: glideStart, zoom: kB.zoom * 1.008, focus: kB.focus!, anchor: kB.anchor!, duration: glideStart - kB.atFrame, easing: E.linear},
		// exactly the storyboard's f74 key (s10's backdrop)
		{at: kC.atFrame, zoom: kC.zoom, focus: kC.focus!, anchor: kC.anchor!, duration: kC.atFrame - glideStart, easing: E.glide},
	];
	const tilt = (axis: 'rx' | 'ry'): Keyframe[] => [
		[kA.atFrame, kA.tilt![axis]],
		[kB.atFrame - 1, kA.tilt![axis]],
		[kB.atFrame, kB.tilt![axis]],
		[glideStart, kB.tilt![axis]],
		[kC.atFrame, kC.tilt![axis], E.glide],
	];
	const shot: ScreenConfig = {
		src: FILE,
		width: 1440,
		radius: 18,
		glow: false,
		camera,
		rotateX: tilt('rx'),
		rotateY: tilt('ry'),
	};

	/* ---- dim: rows 1–4 lit from f34 (storyboard spotlight); the IFRO option opens as the push lands ---- */
	const sp = scene.spotlights[0];
	const dim = (sp.dim ?? 0.5) * ramp(f, sp.from, sp.from + 12, E.enter);
	const pickerOpen = ramp(f, 56, 68, E.enter);

	/* ---- headline ---- */
	const line = scene.copy[0];
	const parts = line.text.split(' '); // "o que" stays one unit
	const emph = line.emphasis.join(' ').split(' ');
	const units: WordUnit[] = parts.map((t, i) => ({text: t, at: line.inFrame + 2 * i}));
	const m0 = parts.findIndex((t) => t === emph[0]);
	const marker = m0 >= 0 ? {units: [m0, m0 + emph.length - 1] as [number, number], from: 18, to: 28, easing: E.glide} : undefined;

	return (
		<Stage>
			<G3Screen {...shot} style={{zIndex: 'auto'}}>
				<Patch {...HINT_PATCH} />
				<Patch {...LEGEND_PATCH} />
				<Patch {...BADGE_PATCH} />
				<DimMask
					dim={dim}
					holes={[
						{...sp.rect!, r: 16},
						{x: 2115, y: 386, w: 687, h: 72, r: 14, open: pickerOpen},
					]}
				/>
				<PendingPulse f={f} />
			</G3Screen>
			{/* shot B: a soft top scrim keeps the eye off the page header (subtitle counts, the volt
			    "Classificar agora" button) while the camera works the rows and the picker */}
			{f >= kB.atFrame ? (
				<AbsoluteFill style={{background: 'linear-gradient(180deg, rgba(10,13,22,0.94) 0px, rgba(10,13,22,0.8) 120px, rgba(10,13,22,0) 210px)'}} />
			) : null}
			{/* solid bottom band (comp y 800–1080, gradient from 700) under the headline */}
			<AbsoluteFill
				style={{
					background: 'linear-gradient(180deg, rgba(10,13,22,0) 700px, rgba(10,13,22,0.72) 760px, rgba(10,13,22,1) 800px, rgba(10,13,22,1) 1080px)',
				}}
			/>
			<Words f={f} size={96} left={144} baseline={905} letterSpacing="-0.035em" units={units} marker={marker} />
			<StageTop seed="s09" />
		</Stage>
	);
};

export default S09SoPergunta;
