/**
 * s09-so-pergunta — S09 · abs 585–659 (75 f) · features · bar 10.4 → 12.1
 *
 * Revisão, v2: the queue only holds what the chain could not settle.
 *  f0  (abs 585) hard cut, rows 1–4 of the real Revisão page (the selected
 *      Calendário group + three Finder groups, "Sem categoria · 0 % · pendente"),
 *      zoom 2.44 (row titles ≈ 35 px), rx 4° ry −6°, micro push ×1.02 over the hold.
 *      The window fades out at comp y ≈ 740–820 into the lit navy stage, where
 *      the headline sits on a footlight.
 *  f2–16  "Só pergunta o que não sabe." 124 px, word stagger (5 units, "o que"
 *      glued, 2 f); f18–28 a volt underline swipes in under "não sabe." (v2 review fix: was a box).
 *  f30 (abs 615) internal jump-cut: a new shot, zoom 3.2 on the right half of rows 1–4 (yaw flip,
 *      pills ≈ 45 px); f44–60 the four "pendente" pills pulse an ember ring (2-f
 *      row stagger).
 *  f44–74 E.glide span to the picker (REVIEW_END); f56 the picker ("IFRO · 1" on
 *      top) LIFTS off the assign card as a floating card (it stays up through
 *      s10 and s11, where the key lands on its "1"); the window steps back under
 *      a light navy veil. Cut on the downbeat abs 660 (s10 holds this framing).
 * Claim patches (image space, graded with the window): the selected row's
 * key-range hint line, the keys legend, the sidebar badge and (v2) the page
 * subtitle, whose "10 grupos … 19 min" counts would otherwise read.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {LiftCard, LiftHole, type LiftCardProps} from '../components/LiftCard';
import {navyDim} from '../components/Stage';
import type {CameraKey, ScreenConfig} from '../components/screen-geometry';
import type {Keyframe} from '../design/motion';
import {E, useScene, useSceneFrame, type SfxCue} from '../shared';
import {Backdrop, G4Plane, lerp, ramp} from './_parts/G4/common';
import {ASSIGN_SUBTITLE_PATCH, CARD_BG, PICKER, PICKER_AT, pickerLift, planeRect, REVIEW_END, REVIEW_FILE, S09_SUBTITLE_PATCH, V2Headline, type V2Unit} from './_parts/G4/review';

/** None: music only (≥ 30 % of the features act stays SFX-free). */
export const sfx: SfxCue[] = [];

const FILE = REVIEW_FILE;

/**
 * Claim-safety patches, measured on the capture (image px):
 *  - the selected row's second line "Pressione 1 a 9 …" (ink x 636–1846, y 398–437 incl.
 *    descenders; the storyboard's 1222 × 32 rect left the descenders) on the row colour #15233f;
 *  - the keys legend (chips x 2128–2789, y ≈ 804–842) on the card colour #0c1220;
 *  - the sidebar's orange 'Revisão' pending-count badge (nav-revisao-badge), active-row colour;
 *  - v2: the page subtitle (two counts).
 */
const PATCHES = [
	{x: 626, y: 394, w: 1232, h: 48, fill: '#15233f'},
	{x: 2124, y: 800, w: 670, h: 48, fill: '#0c1220'},
	{x: 373, y: 375, w: 44, h: 42, fill: '#14223c'},
	S09_SUBTITLE_PATCH,
];
/** Storyboard-shaped copies for the lifted picker (the legend patch falls inside its crop's neighbourhood). */
const CARD_PATCHES = PATCHES.map((p) => ({file: FILE, rect: {x: p.x, y: p.y, w: p.w, h: p.h}, color: p.fill, from: 0, to: 9999}));

/** The four "pendente" pills of rows 1–4 (measured ink boxes, image px). */
const PILLS = [274, 498, 610, 722].map((y) => ({x: 1812, y, w: 128, h: 40}));
const PULSE_AT = 44; // onset: first moving frame one before the beat (f45)

const PendingPulse: React.FC<{f: number}> = ({f}) => (
	<>
		{PILLS.map((p, i) => {
			const t0 = PULSE_AT + 2 * i;
			if (f < t0 || f > t0 + 16) return null;
			const u = ramp(f, t0, t0 + 15, E.push);
			const pad = lerp(2, 22, u);
			const a = 0.95 * (1 - ramp(f, t0 + 2, t0 + 16, E.enter));
			const fill = 0.26 * (1 - ramp(f, t0, t0 + 10, E.enter));
			return (
				<React.Fragment key={i}>
					<div style={{position: 'absolute', left: p.x, top: p.y, width: p.w, height: p.h, borderRadius: 10, background: `rgba(255,122,31,${fill.toFixed(3)})`, boxShadow: `inset 0 0 0 2px rgba(255,122,31,${(0.85 * (1 - u)).toFixed(3)})`}} />
					<div
						style={{
							position: 'absolute',
							left: p.x - pad,
							top: p.y - pad,
							width: p.w + 2 * pad,
							height: p.h + 2 * pad,
							borderRadius: 10 + pad,
							border: `3px solid rgba(255,122,31,${a.toFixed(3)})`,
							boxShadow: `0 0 20px rgba(255,122,31,${(0.4 * a).toFixed(3)})`,
						}}
					/>
				</React.Fragment>
			);
		})}
	</>
);

/* ---- camera: shot A (rows 1–4) → jump-cut shot B (closer) → E.glide span to the picker ---- */
const A = {zoom: 2.44, focus: {x: 1280, y: 518}, anchor: {x: 960, y: 400}};
/**
 * Critique fix: B is a clearly different shot (the old 2.49 → 2.65 step read as a stutter): +29 % closer on the RIGHT half of
 * rows 1–4, so "Sem categoria · 0 % · pendente" fill the frame (≈ 45 px) and the half-cut C/F icon tiles leave it; a harder
 * angle too (rx 4 → 7, ry −6 → +4, a yaw flip). The f44–74 glide to REVIEW_END becomes a pull-back-and-pan to the picker.
 */
const B = {zoom: 3.2, focus: {x: 1660, y: 500}, anchor: {x: 960, y: 392}};
const JUMP = 30;
const GLIDE = 44;
const END = 74;

const camera: CameraKey[] = [
	{at: 0, zoom: A.zoom, focus: A.focus, anchor: A.anchor, duration: 0},
	{at: JUMP - 1, zoom: A.zoom * 1.02, focus: A.focus, anchor: A.anchor, duration: JUMP - 1, easing: E.linear},
	{at: JUMP, zoom: B.zoom, focus: B.focus, anchor: B.anchor, duration: 0},
	{at: GLIDE, zoom: B.zoom * 1.01, focus: B.focus, anchor: B.anchor, duration: GLIDE - JUMP, easing: E.linear},
	// exactly REVIEW_END (s10's backdrop, s11's first frame)
	{at: END, zoom: REVIEW_END.zoom, focus: REVIEW_END.focus, anchor: REVIEW_END.anchor, duration: END - GLIDE, easing: E.glide},
];
const tilt = (a: number, b: number, c: number): Keyframe[] => [
	[0, a],
	[JUMP - 1, a],
	[JUMP, b],
	[GLIDE, b],
	[END, c, E.glide],
];
const SHOT: Omit<ScreenConfig, 'src'> = {
	width: 1440,
	radius: 18,
	glow: false,
	camera,
	rotateX: tilt(4, 7, REVIEW_END.tilt.rx),
	rotateY: tilt(-6, 4, REVIEW_END.tilt.ry),
};

const S09SoPergunta: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();

	/* ---- the picker lift (continues in s10 / s11) ---- */
	const picker: LiftCardProps = {src: FILE, ...pickerLift(PICKER_AT, (fr) => planeRect(SHOT, fr, PICKER))};
	const veil = 0.14 * ramp(f, PICKER_AT, PICKER_AT + 14, E.enter);

	/* ---- headline ---- */
	const line = scene.copy[0];
	const glued = line.text.split(' '); // "o que" is one unit: the storyboard glues it with a NBSP
	const emph = line.emphasis.join(' ').split(' ');
	// v2-look §3: the emphasised words take the volt-gradient ink (+ soft glow) over a lighter marker
	const units: V2Unit[] = glued.map((t, i) => ({text: t, at: line.inFrame + 2 * i, volt: emph.includes(t)}));
	const m0 = glued.findIndex((t) => t === emph[0]);
	const marker = m0 >= 0 ? {line: 0, units: [m0, m0 + emph.length - 1] as [number, number], from: 18, to: 28, easing: E.glide} : undefined;

	// the window fades into the stage above the headline band (comp y 730 → 820)
	// (critique fix: 30 px higher, so the half-visible WhatsApp row's chips stay pure texture; a soft 22-px top fade keeps the
	// volt "Classificar agora" button's glow off the frame edge)
	const mask = 'linear-gradient(180deg, rgba(0,0,0,0.25) 0px, #000 22px, #000 700px, rgba(0,0,0,0.35) 760px, rgba(0,0,0,0) 800px)';

	return (
		<Backdrop
			seed="s09"
			vignette={0.45}
			look={{
				keyPool: {x: 0.46, y: 0.98, w: 1.0, h: 0.5, opacity: 0.6},
				keyLight: {x: 0.42, y: 0.86, w: 0.7, h: 0.34, opacity: 0.14},
			}}
		>
			<AbsoluteFill style={{WebkitMaskImage: mask, maskImage: mask}}>
				<G4Plane
					{...SHOT}
					layers={[
						{
							src: FILE,
							children: (
								<>
									{PATCHES.map((p, i) => (
										<div key={i} style={{position: 'absolute', left: p.x, top: p.y, width: p.w, height: p.h, background: p.fill}} />
									))}
									<PendingPulse f={f} />
									{/* v2 review fix: the assign subtitle would be sliced by the lifting card's left edge */}
									<div style={{position: 'absolute', left: ASSIGN_SUBTITLE_PATCH.x, top: ASSIGN_SUBTITLE_PATCH.y, width: ASSIGN_SUBTITLE_PATCH.w, height: ASSIGN_SUBTITLE_PATCH.h, background: ASSIGN_SUBTITLE_PATCH.fill, opacity: ramp(f, PICKER_AT - 2, PICKER_AT + 4, E.enter)}} />
									<LiftHole rect={PICKER} at={PICKER_AT} enter="lift" color={CARD_BG} pad={4} feather={10} radius={14} />
								</>
							),
						},
					]}
				/>
				{veil > 0.001 ? <AbsoluteFill style={{background: navyDim(veil)}} /> : null}
			</AbsoluteFill>
			<LiftCard {...picker} patches={CARD_PATCHES} style={{zIndex: 5}} />
			<V2Headline f={f} lines={[units]} size={124} left={128} capTops={[868]} marker={marker} />
		</Backdrop>
	);
};

export default S09SoPergunta;
