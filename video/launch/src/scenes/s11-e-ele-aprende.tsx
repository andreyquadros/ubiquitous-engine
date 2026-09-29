/**
 * s11-e-ele-aprende — S11 · abs 690–779 (90 f) · features · bar 12.3 → 14.1
 *
 * v2. f0 (abs 690, beat 3) T5 match cut: s10's keycap, drawn at its exact s10
 * pose (S10_FINAL), flies on an arc and flattens into the "1" chip of the
 * FLOATING picker card (lifted off the assign card since s09 f56), 12 f:
 * rect, tilt → card tilt, radius, skirt → 0, silver face → chip fill, the volt
 * legend stays volt (the anchor of the cut). Meanwhile the backdrop racks into
 * focus: s10 held this exact framing blurred (REVIEW_END), so blur 16 → 0,
 * veil 0.34 → 0.22 and the s10 push 1.04 → 1 are one rack focus.
 * f12 the IFRO row of the card takes the pressed tint; the "1" chip stays lit.
 * f15 (abs 705, beat) state change → review-after-assign in the window: the
 * Calendário row slides out left, the list closes up, the new active row
 * flashes mint.
 * f12–27 "Uma tecla. / E ele aprende." (116 px, units at f12/15/18/21, "aprende."
 * volt) on a navy scrim at the left.
 * f30 (abs 720, downbeat) the payoff: the rule chips "Criar regra: Sempre:
 * Calendário → IFRO" pop OUT of the window as a second lifted card (bouncy lift
 * from their in-window rect, 900 px wide, chip text ≈ 40 px), a spark burst and
 * a light sweep across the chips (f38–54). f30–89 slow pull (zoom 2.5 → 2.44),
 * both cards float and drift against it. Cut out.
 */
import React from 'react';
import {AbsoluteFill, useVideoConfig} from 'remotion';
import {LiftCard, LiftHole, type LiftCardProps} from '../components/LiftCard';
import {navyDim} from '../components/Stage';
import type {Rect, ScreenConfig} from '../components/screen-geometry';
import {sceneById} from '../storyboard';
import {alpha, color, font} from '../design/tokens';
import {E, hotspot, Patches, springAt, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {arcPoint, Backdrop, CaptureImg, clamp01, G4Plane, insetOf, lerp, ramp, UI, widenLegend} from './_parts/G4/common';
import {KeyCap3D, mixPose, type KeyPose} from './_parts/G4/KeyCap3D';
import {S10_FINAL} from './_parts/G4/keyTimeline';
import {
	AFTER_FILE,
	CARD_BG,
	cardPoint,
	PICKER,
	PICKER_AT,
	pickerLift,
	planeRect,
	REVIEW_END,
	REVIEW_FILE,
	S09_LEN,
	S09_SUBTITLE_PATCH,
	S10_LEN,
	V2Headline,
} from './_parts/G4/review';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'ui_tick_1.wav', atFrame: 12, gainDb: -18, note: 'Chip lights.'},
	{ref: 'success_chime_1.wav', atFrame: 18, gainDb: -14, note: 'Group classified (C6→E6).'},
	{ref: 'pop.wav', atFrame: 30, gainDb: -16, note: 'Rule chips pop out of the window (v2: as a lifted card).'},
	{ref: 'shimmer_3.wav', atFrame: 38, gainDb: -20, note: 'v2: light sweep across the lifted rule chips (onset on the sweep).'},
];

/** The keycap legend is s10's copy line ("1"). */
const KEY_LABEL = sceneById('s10-tecla-1').copy[0].text;

const MORPH = 12; // match-cut half, frames (storyboard transitionIn)
const SWAP = 15;
const SWAP_LEN = 6;
const RULES = 30;
const SWEEP = 38;

const KEY1 = hotspot(REVIEW_FILE, 'assign-key-1'); // 2746,402 40×40
const IFRO_ROW = hotspot(REVIEW_FILE, 'assign-option-1');
const ROW1 = hotspot(REVIEW_FILE, 'queue-row-1'); // Calendário (selected)
const LIST_BELOW: Rect = {x: 514, y: 462, w: 1529, h: 1092}; // rows 2… + settled header + card bottom
const NEW_ACTIVE = hotspot(AFTER_FILE, 'queue-row-1');

/** Rule-chip reveal cover: the storyboard's (2127,800,663×150) leaves 3 px of the second chip's border → 160 tall. */
const RULE_COVER: Rect = {x: 2127, y: 800, w: 663, h: 160};
/** The lifted rule block: "Da última decisão", "Criar regra: [Sempre: Calendário → IFRO]", "[Sempre: calendário → IFRO]". */
const RULE: Rect = {x: 2110, y: 796, w: 512, h: 172};

/** v2 headline scrim: a lifted navy (#141f3c), lighter than the stage so the left half is not a hole. */
const SCRIM = (a: number) => `rgba(20, 31, 60, ${a.toFixed(3)})`;

/** Plane push carried over from s10 (1.04 at its last frame), relaxed during the rack. */
const PUSH_O = {x: 960, y: 540};

const S11EEleAprende: React.FC = () => {
	const scene = useScene();
	const {fps} = useVideoConfig();
	const {frame: f} = useSceneFrame();
	const copy = scene.copy[0];
	const [l1, l2] = copy.text.split('\n');
	const [w1, w2] = l1.split(' ');
	const cut = l2.lastIndexOf(' ');
	const w3 = l2.slice(0, cut); // "E ele" (glued)
	const w4 = l2.slice(cut + 1); // "aprende."

	// --- rack focus s10 → s11 -------------------------------------------------
	const rack = f < MORPH + 2 ? springAt(f, 0, 'SMOOTH', MORPH + 2) : 1;
	const push = lerp(1.04, 1, rack);
	const blur = 16 * (1 - rack);
	const veil = lerp(0.34, 0.05, rack) + 0.05 * ramp(f, RULES, RULES + 12, E.enter);

	// --- plane camera: REVIEW_END held through the swap, then a slow pull ------------
	const cam: Omit<ScreenConfig, 'src'> = {
		width: 1440,
		radius: 18,
		glow: false,
		camera: [
			{at: 0, zoom: REVIEW_END.zoom, focus: REVIEW_END.focus, anchor: REVIEW_END.anchor, duration: 0},
			{at: 89, zoom: 2.42, focus: {x: REVIEW_END.focus.x + 20, y: REVIEW_END.focus.y + 16}, anchor: REVIEW_END.anchor, duration: 89 - SWAP, easing: E.linear},
		],
		rotateX: REVIEW_END.tilt.rx,
		rotateY: [
			[SWAP, REVIEW_END.tilt.ry],
			[89, REVIEW_END.tilt.ry - 1.5],
		],
	};
	const pushed = (r: Rect): Rect => ({x: PUSH_O.x + (r.x - PUSH_O.x) * push, y: PUSH_O.y + (r.y - PUSH_O.y) * push, w: r.w * push, h: r.h * push});

	// --- the floating picker (same props as s09 / s10, frame-shifted) ----------
	const picker: LiftCardProps = {src: REVIEW_FILE, ...pickerLift(PICKER_AT - S09_LEN - S10_LEN, PICKER)};

	// --- match cut: keycap → the "1" chip on the picker card --------------------------
	const m = E.cursor(ramp(f, 0, MORPH));
	const chip = cardPoint(picker, f, fps, {x: KEY1.x + KEY1.w / 2, y: KEY1.y + KEY1.h / 2});
	const chipC = {x: PUSH_O.x + (chip.x - PUSH_O.x) * push, y: PUSH_O.y + (chip.y - PUSH_O.y) * push};
	const k = chip.k * push;
	const target: KeyPose = {
		...S10_FINAL,
		cx: chipC.x,
		cy: chipC.y,
		size: KEY1.w * k,
		rx: 7,
		ry: -9,
		rz: 0,
		perspective: 1700,
		radius: 8 * k,
		skirt: 0,
		press: 0,
		scale: 1,
		dy: 0,
		legendSize: 23 * k,
		legendVolt: 1,
		legendInk: color.volt,
		legendLit: color.volt,
		faceTop: UI.chipFill,
		faceBottom: UI.chipFill,
		border: color.volt,
		borderAlpha: 0.9,
		borderWidth: 1.5,
		highlight: 0,
		underglow: 0,
		shadow: 0,
		ring: 1,
		dish: 0,
		skirtTop: UI.chipFill,
		skirtBottom: UI.chipFill,
		rim: 0,
		shock: 0,
		floorGlow: 0,
		opacity: 1,
	};
	const p = arcPoint({x: S10_FINAL.cx, y: S10_FINAL.cy}, {x: target.cx, y: target.cy}, clamp01(m), 0.1);
	const pose: KeyPose = {...mixPose(S10_FINAL, target, clamp01(m)), cx: p.x, cy: p.y, size: lerp(S10_FINAL.size, target.size, m)};
	// shape / colour settle a touch earlier than the travel so the chip is flat when it arrives
	const flat = ramp(f, 3, 11, E.glide);
	pose.skirt = lerp(S10_FINAL.skirt, 0, flat);
	pose.shadow = lerp(1, 0, flat);
	pose.dish = lerp(1, 0, flat);
	pose.rim = lerp(S10_FINAL.rim, 0, flat);
	pose.floorGlow = lerp(S10_FINAL.floorGlow, 0, flat);
	pose.shock = 0;

	// --- plane state ---------------------------------------------------------
	const before = widenLegend(storyboardPatches(scene, REVIEW_FILE));
	const afterPatches = widenLegend(storyboardPatches(scene, AFTER_FILE)).filter((q) => !/rule chips/.test(q.covers ?? ''));
	const swapped = f >= SWAP;
	const e = swapped ? springAt(f, SWAP, 'SNAPPY') : 0;
	const inSwap = swapped && f < SWAP + SWAP_LEN;
	const subtitle = <div style={{position: 'absolute', left: S09_SUBTITLE_PATCH.x, top: S09_SUBTITLE_PATCH.y, width: S09_SUBTITLE_PATCH.w, height: S09_SUBTITLE_PATCH.h, background: S09_SUBTITLE_PATCH.fill}} />;
	const pickerHole = <LiftHole rect={PICKER} at={picker.at} enter="lift" color={CARD_BG} pad={4} feather={10} radius={14} />;
	const beforePatchEls = (
		<>
			<Patches patches={before} />
			{subtitle}
		</>
	);
	const mint = ramp(f, SWAP, SWAP + 2) * (1 - ramp(f, SWAP + 2, SWAP + 10, E.enter));

	// --- the rule card ---------------------------------------------------------
	const rule: LiftCardProps = {
		src: AFTER_FILE,
		rect: RULE,
		at: RULES,
		enter: 'lift',
		spring: 'subtleBounce',
		from: (fr) => pushed(planeRect(cam, fr, RULE)),
		x: 1418,
		y: 808,
		width: 900,
		rotateX: [
			[RULES, 10],
			[89, 7],
		],
		rotateY: [
			[RULES, -12],
			[89, -8],
		],
		perspective: 1700,
		radius: 22,
		float: 4,
		floatPeriod: 90,
		drift: {x: -0.18, y: -0.05},
		glow: 'volt',
		glowOpacity: 0.55,
	};
	const coverOn = swapped && f < RULES;

	const afterLayer = (
		<>
			<Patches patches={afterPatches} />
			{subtitle}
			{pickerHole}
			{coverOn ? <div style={{position: 'absolute', left: RULE_COVER.x - 2, top: RULE_COVER.y - 2, width: RULE_COVER.w + 4, height: RULE_COVER.h + 4, background: UI.card}} /> : null}
			<LiftHole rect={RULE} at={RULES} enter="lift" color={CARD_BG} pad={6} feather={12} radius={16} />
			{inSwap ? (
				<>
					<BeforeSlice rect={LIST_BELOW} ty={-112 * e} opacity={1 - ramp(f, SWAP + 1, SWAP + SWAP_LEN)} patches={beforePatchEls} />
					<BeforeSlice rect={ROW1} tx={-140 * e} opacity={1 - ramp(f, SWAP, SWAP + SWAP_LEN, E.enter)} patches={beforePatchEls} />
				</>
			) : null}
			{mint > 0.001 ? (
				<div style={{position: 'absolute', left: NEW_ACTIVE.x, top: NEW_ACTIVE.y, width: NEW_ACTIVE.w, height: NEW_ACTIVE.h, background: 'rgba(46,204,143,0.12)', opacity: mint}} />
			) : null}
		</>
	);

	// --- picker card overlays: pressed IFRO row + the lit "1" chip -------------------
	const tint = ramp(f, MORPH, MORPH + 4, E.enter);
	const flash = f >= MORPH ? Math.exp(-(f - MORPH) / 3) : 0;
	const chipLit = f >= MORPH ? 1 : 0;
	const pickerKids = (
		<>
			{tint > 0.001 ? (
				// the pressed option takes the selected state: a volt-filled row; its icon tile + label are re-set on top
				<div
					style={{
						position: 'absolute',
						left: IFRO_ROW.x + 2,
						top: IFRO_ROW.y + 2,
						width: IFRO_ROW.w - 4,
						height: IFRO_ROW.h - 4,
						borderRadius: 14,
						background: `linear-gradient(180deg, rgba(92,154,255,${(0.8 * tint + 0.15 * flash).toFixed(3)}), rgba(61,124,240,${(0.72 * tint + 0.15 * flash).toFixed(3)}))`,
						boxShadow: `inset 0 1.5px 0 rgba(255,255,255,${(0.35 * tint).toFixed(3)}), 0 0 ${(26 + 30 * flash).toFixed(1)}px rgba(77,141,255,${(0.45 * tint + 0.3 * flash).toFixed(3)})`,
					}}
				/>
			) : null}
			{tint > 0.001 ? <IfroSelected opacity={tint} /> : null}
			<LitChip opacity={chipLit} label={KEY_LABEL} />
		</>
	);

	const lines = [
		[
			{text: w1, at: 12},
			{text: w2, at: 15},
		],
		[
			{text: w3.replace(/ /g, ' '), at: 18},
			{text: w4, at: 21, volt: true},
		],
	];
	if (`${w1} ${w2}\n${w3} ${w4}` !== copy.text) throw new Error(`s11 copy drift: ${copy.text}`);

	return (
		<TransitionOut>
			<TransitionIn>
				{() => (
					<Backdrop seed="s11" vignette={0.5} look={{keyPool: {x: 0.72, y: 0.55, w: 0.7, h: 0.9, opacity: 0.4}, keyLight: {x: 0.73, y: 0.52, w: 0.46, h: 0.7, opacity: 0.16}}}>
						<AbsoluteFill
							style={{
								filter: blur > 0.2 ? `blur(${blur.toFixed(2)}px)` : undefined,
								transform: Math.abs(push - 1) > 1e-4 ? `translate(${PUSH_O.x}px, ${PUSH_O.y}px) scale(${push.toFixed(5)}) translate(${-PUSH_O.x}px, ${-PUSH_O.y}px)` : undefined,
								transformOrigin: '0 0',
							}}
						>
							<G4Plane {...cam} layers={swapped ? [{src: AFTER_FILE, children: afterLayer}] : [{src: REVIEW_FILE, children: <>{beforePatchEls}{pickerHole}</>}]} />
							<AbsoluteFill style={{background: navyDim(veil)}} />
							{/* key light over the stepped-back window, behind the two cards */}
							<AbsoluteFill
								style={{
									background: `radial-gradient(ellipse 38% 56% at 73% 52%, ${alpha('#cfe0ff', 0.2)} 0%, ${alpha(color.volt, 0.15)} 50%, ${alpha(color.volt, 0)} 100%)`,
								}}
							/>
							<LiftCard {...picker} patches={before}>
								{pickerKids}
							</LiftCard>
						</AbsoluteFill>
						{/* s10's key light travels with the key into the chip and dies into the IFRO row's glow */}
						{f < MORPH + 8 ? (
							<AbsoluteFill
								style={{
									opacity: 1 - ramp(f, MORPH - 2, MORPH + 7, E.enter),
									background: `radial-gradient(ellipse ${lerp(34, 9, m).toFixed(2)}% ${lerp(44, 14, m).toFixed(2)}% at ${((pose.cx / 1920) * 100).toFixed(2)}% ${((pose.cy / 1080) * 100).toFixed(2)}%, ${alpha('#cfe0ff', 0.2)} 0%, ${alpha(color.volt, 0.16)} 45%, ${alpha(color.volt, 0)} 100%)`,
								}}
							/>
						) : null}
						{/* left scrim under the headline (navy, v2) */}
						<AbsoluteFill
							style={{
								opacity: ramp(f, 8, 18, E.enter),
								background: `linear-gradient(90deg, ${SCRIM(0.92)} 0px, ${SCRIM(0.84)} 620px, ${SCRIM(0.36)} 900px, ${SCRIM(0)} 1060px)`,
							}}
						/>
						<RuleSparks f={f} card={rule} fps={fps} />
						<LiftCard {...rule}>
							<Sweep f={f} rect={RULE} />
						</LiftCard>
						{f < MORPH ? <KeyCap3D pose={pose} label={KEY_LABEL} /> : null}
						<V2Headline f={f} lines={lines} size={124} left={100} capTops={[384, 540]} />
					</Backdrop>
				)}
			</TransitionIn>
		</TransitionOut>
	);
};

/** The lit chip on the picker card once the key has landed (image space of the capture). */
const LitChip: React.FC<{opacity: number; label: string}> = ({opacity, label}) =>
	opacity <= 0.001 ? null : (
		<div
			style={{
				position: 'absolute',
				left: KEY1.x,
				top: KEY1.y,
				width: KEY1.w,
				height: KEY1.h,
				borderRadius: 8,
				background: UI.chipFill,
				boxShadow: `inset 0 0 0 1.15px rgba(77,141,255,0.9), 0 0 14px rgba(77,141,255,0.45)`,
				display: 'flex',
				alignItems: 'center',
				justifyContent: 'center',
				opacity,
			}}
		>
			<div style={{fontFamily: font.text, fontWeight: 600, fontSize: 23, lineHeight: 1, color: color.volt, fontVariantNumeric: 'tabular-nums'}}>{label}</div>
		</div>
	);

/**
 * The IFRO option re-set above its volt fill: the icon tile is a rounded crop of the capture (x 2132–2180, y 400–446), the label is
 * vector type (Inter 500, 29 image px, like the app), white, vertically centred in the 72-px row.
 */
const IfroSelected: React.FC<{opacity: number}> = ({opacity}) => (
	<div style={{position: 'absolute', left: 0, top: 0, width: 2880, height: 1800, opacity}}>
		<div style={{position: 'absolute', left: 0, top: 0, width: 2880, height: 1800, clipPath: `${insetOf({x: 2132, y: 400, w: 48, h: 46}).slice(0, -1)} round 10px)`}}>
			<CaptureImg src={REVIEW_FILE} />
		</div>
		<div
			style={{
				position: 'absolute',
				left: 2199,
				top: IFRO_ROW.y,
				height: IFRO_ROW.h,
				display: 'flex',
				alignItems: 'center',
				fontFamily: font.text,
				fontWeight: 500,
				fontSize: 29,
				color: '#ffffff',
				letterSpacing: '0.005em',
			}}
		>
			IFRO
		</div>
	</div>
);

/** A clipped piece of the BEFORE capture (+ its patches) that can move and fade: the list re-layout. */
const BeforeSlice: React.FC<{rect: Rect; tx?: number; ty?: number; opacity: number; patches: React.ReactNode}> = ({rect, tx = 0, ty = 0, opacity, patches}) =>
	opacity <= 0.001 ? null : (
		<div
			style={{
				position: 'absolute',
				left: 0,
				top: 0,
				width: 2880,
				height: 1800,
				clipPath: insetOf(rect),
				transform: tx || ty ? `translate(${tx.toFixed(2)}px, ${ty.toFixed(2)}px)` : undefined,
				opacity,
			}}
		>
			<CaptureImg src={REVIEW_FILE} />
			{patches}
		</div>
	);

/** A soft diagonal light band sweeping across the rule chips (image px of the capture). */
const Sweep: React.FC<{f: number; rect: Rect}> = ({f, rect}) => {
	const t = ramp(f, SWEEP, SWEEP + 16, E.glide);
	if (t <= 0 || t >= 1) return null;
	const x = lerp(rect.x - 260, rect.x + rect.w + 60, t);
	return (
		<div
			style={{
				position: 'absolute',
				left: x,
				top: rect.y - 20,
				width: 200,
				height: rect.h + 40,
				transform: 'skewX(-18deg)',
				background: 'linear-gradient(90deg, rgba(207,224,255,0) 0%, rgba(207,224,255,0.22) 45%, rgba(255,255,255,0.3) 50%, rgba(207,224,255,0.22) 55%, rgba(207,224,255,0) 100%)',
				mixBlendMode: 'screen',
			}}
		/>
	);
};

/** 14 volt sparks thrown out of the rule card as it pops (canvas px), f30–46. */
const RuleSparks: React.FC<{f: number; card: LiftCardProps; fps: number}> = ({f, card, fps}) => {
	const t0 = RULES + 2;
	if (f < t0 || f > t0 + 16) return null;
	const c = cardPoint(card, f, fps, {x: RULE.x + RULE.w / 2, y: RULE.y + RULE.h / 2});
	const u = ramp(f, t0, t0 + 16, E.push);
	return (
		<AbsoluteFill style={{pointerEvents: 'none'}}>
			{Array.from({length: 14}, (_, i) => {
				const a = (i / 14) * Math.PI * 2 + 0.3 * Math.sin(i * 7.1);
				const r0 = 0.5 * (i % 3 === 0 ? 1.1 : 0.95);
				const dist = lerp(0, 130 + 60 * ((i * 37) % 5) / 5, u);
				const x = c.x + Math.cos(a) * (470 * r0 + dist);
				const y = c.y + Math.sin(a) * (160 * r0 + dist * 0.55);
				const sz = (i % 2 ? 7 : 5) * (1 - 0.5 * u);
				const o = 0.95 * (1 - u);
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: x - sz / 2,
							top: y - sz / 2,
							width: sz,
							height: sz,
							borderRadius: '50%',
							background: i % 3 === 0 ? '#e6eeff' : color.volt,
							boxShadow: `0 0 12px ${alpha(color.volt, 0.9 * o)}`,
							opacity: o,
						}}
					/>
				);
			})}
		</AbsoluteFill>
	);
};

export default S11EEleAprende;
