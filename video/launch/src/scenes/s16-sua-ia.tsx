/**
 * s16-sua-ia — S16 · abs 1125–1229 (105 f) · proof · bar 19.4 → 21.3
 *
 * v2. The real Configurações › IA window (settings-ai.png) floats set back on a
 * lit navy stage (zoom 0.9, ry −15° → −11°, depth blur 1.2 px) and its
 * "Provedor de IA" picker LIFTS out of it as a big 3D card (LiftCard, 1380 px
 * wide: pill text 44 px on the canvas) that tilts toward the camera over the
 * scene; a LiftHole leaves an empty socket where it sat. The picker is the
 * app's own ProviderPicker re-set as vectors inside the card (Inter 500 28 image
 * px, lucide KeyRound in --signal, active = bg-panel + inset line-2), so hover
 * and active states are crisp and can move.
 *
 * f0–3 whip-left in-half (T2, pairs with s15's out-half) on window + card.
 * f4 the card lifts (smooth spring, flies from the in-window rect).
 * Line 1 "Claude, OpenAI ou Grok." (110 px) staggers in f2/4/6 at the bottom.
 * The cursor hovers each provider ON the beat (f15 Anthropic, f30 OpenAI, f45 xAI
 * Grok: text → ink + a 4-f hover fill) and the matching word of line 1 lifts.
 * Line 2 "Ou deixe com o Ubi." f48–60. Cursor on "IA do Ubi" f71, CLICK f75 (abs
 * 1200 downbeat): shockwave off the pill, the active pill slides to "IA do Ubi"
 * (SNAPPY) and takes a volt selection ring; the window swaps hard to
 * settings-ai-ubi at f77 (same camera); f78 shimmer across the pill; "Ubi."
 * glows. f78–98 hold: card float + tilt settle + window drift. f99–104
 * blur-dissolve out-half on UI/background; the type hard-cuts at f105.
 *
 * Claim safety: the helper line under the picker (a cost estimate before the
 * click, the plan price after it) is patched in BOTH captures, as are the model
 * fields (storyboard) and the monthly-budget value; the window is also set back
 * behind a depth blur, so none of its small print is legible.
 */
import React from 'react';
import {AbsoluteFill, useVideoConfig} from 'remotion';
import {LiftCard, LiftHole, liftCardPose, type LiftCardProps} from '../components/LiftCard';
import {Screen, type ScreenConfig} from '../components/Screen';
import {navyDim, STAGE, StageBase} from '../components/Stage';
import {mapImageRect, type Rect} from '../components/screen-geometry';
import {alpha, color, font} from '../design/tokens';
import {E, hotspot, springAt, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import type {StoryboardPatch} from '../storyboard';
import {FilmGrain, H, lerp, ramp, Stage, Vignette, W} from './_parts/G6/common';
import {APP, arcPoint, ArrowCursor, ClickRipple, Headline, KeyIcon, mixHex, pressAt, projectCardPoint, Shockwave, unitsOf} from './_parts/G6/kit';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'whip_3.wav', atFrame: 0, gainDb: -8, note: 'Whip on the cut abs 1125; file starts 3 f earlier (master track).'},
	{ref: 'whoosh_in_3.wav', atFrame: 12, gainDb: -16, note: 'v2: the picker card lifts out of the window (arrives ≈ f12).'},
	{ref: 'ui_tick_2.wav', atFrame: 30, gainDb: -22, note: 'Hover.'},
	{ref: 'ui_tick_3.wav', atFrame: 45, gainDb: -22, note: 'Hover.'},
	{ref: 'click.wav', atFrame: 75, gainDb: -12, note: 'IA do Ubi.'},
	{ref: 'shimmer_2.wav', atFrame: 78, gainDb: -14, note: 'State change.'},
];

const BEFORE = 'ui/settings-ai.png';
const AFTER = 'ui/settings-ai-ubi.png';
const SWAP = 77;
const CLICK = 75;
const COMP = {width: W, height: H};

type ProviderId = 'anthropic' | 'openai' | 'xai' | 'ubi';
const PROVIDERS: {id: ProviderId; label: string; key: boolean; rect: Rect}[] = [
	{id: 'anthropic', label: 'Anthropic Claude', key: true, rect: hotspot(BEFORE, 'provider-anthropic')},
	{id: 'openai', label: 'OpenAI', key: false, rect: hotspot(BEFORE, 'provider-openai')},
	{id: 'xai', label: 'xAI Grok', key: false, rect: hotspot(BEFORE, 'provider-xai')},
	{id: 'ubi', label: 'IA do Ubi', key: true, rect: hotspot(BEFORE, 'provider-ia-do-ubi')},
];
const rectOf = (id: ProviderId) => PROVIDERS.find((p) => p.id === id)!.rect;
const BADGE = hotspot(BEFORE, 'nav-revisao-badge');
const PICKER = hotspot(BEFORE, 'provider-picker'); // 1146, 411, 874 × 84 (2-px border)
/** The capture's panel colour around the picker (sampled #0c1220 at (1140, 450) in both captures). */
const PANEL = '#0c1220';

/**
 * v2 claim-safety patches (image px, both captures): the helper line under the
 * picker (before: "…Com Claude, US$ 3–6/mês estimados…", after: "…(R$ 49/mês)…";
 * glyph bbox x 1146–2446, y 512–536 measured) and the monthly-budget value +
 * unit ("5 / 6 US$ por mês"). Plus the storyboard's model-field patches.
 */
const V2_PATCHES: StoryboardPatch[] = [
	{file: BEFORE, rect: {x: 1140, y: 504, w: 1320, h: 40}, color: PANEL, from: 0, to: SWAP - 1, covers: 'provider cost estimate'},
	{file: AFTER, rect: {x: 1140, y: 504, w: 1320, h: 40}, color: PANEL, from: SWAP, to: 104, covers: 'plan price'},
	{file: BEFORE, rect: {x: 1160, y: 1430, w: 60, h: 46}, color: '#131a2a', from: 0, to: SWAP - 1, covers: 'budget value'},
	{file: BEFORE, rect: {x: 1796, y: 1430, w: 170, h: 46}, color: PANEL, from: 0, to: SWAP - 1, covers: 'budget unit'},
	{file: AFTER, rect: {x: 1160, y: 1230, w: 60, h: 46}, color: '#10172a', from: SWAP, to: 104, covers: 'budget value'},
	{file: AFTER, rect: {x: 1796, y: 1230, w: 170, h: 46}, color: PANEL, from: SWAP, to: 104, covers: 'budget unit'},
];

/** Hover windows (the cursor rests on each label from its arrival − 3 f to its departure). */
const HOVER: Record<ProviderId, [number, number]> = {
	anthropic: [12, 17],
	openai: [27, 32],
	xai: [42, 58],
	ubi: [68, 999],
};
const hoverOf = (f: number, id: ProviderId) => {
	const [h0, h1] = HOVER[id];
	return ramp(f, h0, h0 + 4, E.enter) * (1 - ramp(f, h1, h1 + 4, E.enter));
};
/** Headline word lift: follows the hover, with a slower release. */
const litOf = (id: ProviderId) => (f: number) => {
	const [h0, h1] = HOVER[id];
	return ramp(f, h0, h0 + 4, E.enter) * (1 - ramp(f, h1 + 2, h1 + 12, E.glide));
};

/** Cursor tips (image px): just below each label's centre, never on the words' x-height. */
const TIPS: Record<ProviderId, {x: number; y: number}> = {
	anthropic: {x: 1330, y: 474},
	openai: {x: 1560, y: 474},
	xai: {x: 1716, y: 474},
	ubi: {x: 1880, y: 474},
};
const REST_IN = {x: 760, y: 640};
const REST_OUT = {x: 1640, y: 640};

/* ------------------------------------------------------------------------ */
/* Picker re-set (image px of the full capture; drawn inside the card)       */
/* ------------------------------------------------------------------------ */

const Picker: React.FC<{f: number}> = ({f}) => {
	// active pill: Anthropic → IA do Ubi, SNAPPY from the click's release
	const t = f < CLICK + 1 ? 0 : springAt(f, CLICK + 1, 'SNAPPY');
	const a = rectOf('anthropic');
	const b = rectOf('ubi');
	const pill = {x: lerp(a.x, b.x, t), y: a.y, w: lerp(a.w, b.w, t), h: a.h};
	const stretch = Math.sin(Math.PI * Math.min(1, Math.max(0, t))) * 0.06;
	const sheen = ramp(f, 78, 94, E.glide);
	// v2: the chosen pill takes a volt selection ring after the click (+ a flash that decays)
	const sel = ramp(f, CLICK + 2, CLICK + 8, E.enter);
	const flash = f >= CLICK ? 1 - ramp(f, CLICK, CLICK + 16, E.glide) : 0;
	return (
		<>
			<div style={{position: 'absolute', left: PICKER.x + 2, top: PICKER.y + 2, width: PICKER.w - 4, height: PICKER.h - 4, borderRadius: 16, background: APP.panel2}} />
			{PROVIDERS.map((p) => {
				const hover = hoverOf(f, p.id);
				return hover > 0.001 ? (
					<div
						key={`h-${p.id}`}
						style={{position: 'absolute', left: p.rect.x, top: p.rect.y, width: p.rect.w, height: p.rect.h, borderRadius: 14, background: `rgba(255,255,255,${(0.07 * hover).toFixed(4)})`}}
					/>
				) : null;
			})}
			<div
				style={{
					position: 'absolute',
					left: pill.x,
					top: pill.y,
					width: pill.w,
					height: pill.h,
					borderRadius: 14,
					background: sel > 0 ? mixHex(APP.panel, '#13213f', sel) : APP.panel,
					boxShadow: [
						`inset 0 0 0 2px ${sel > 0 ? mixHex(APP.line2, color.volt, sel * 0.85) : APP.line2}`,
						sel > 0 ? `0 0 ${(10 + 16 * flash).toFixed(1)}px ${alpha(color.volt, 0.35 * sel + 0.4 * flash)}` : '',
					]
						.filter(Boolean)
						.join(', '),
					transform: stretch > 0.001 ? `scaleX(${(1 + stretch).toFixed(4)})` : undefined,
					overflow: 'hidden',
				}}
			>
				{sheen > 0 && sheen < 1 ? (
					<div
						style={{
							position: 'absolute',
							top: -20,
							bottom: -20,
							width: 90,
							left: lerp(-120, pill.w + 30, sheen),
							transform: 'skewX(-20deg)',
							background: `linear-gradient(90deg, transparent, ${alpha(color.volt, 0.3)}, rgba(255,255,255,0.16), ${alpha(color.volt, 0.3)}, transparent)`,
						}}
					/>
				) : null}
			</div>
			{PROVIDERS.map((p) => {
				const hover = hoverOf(f, p.id);
				const active = p.id === 'anthropic' ? 1 - Math.min(1, Math.max(0, t)) : p.id === 'ubi' ? Math.min(1, Math.max(0, t)) : 0;
				const ink = Math.max(active, hover);
				return (
					<div
						key={p.id}
						style={{
							position: 'absolute',
							left: p.rect.x,
							top: p.rect.y,
							height: p.rect.h,
							display: 'flex',
							alignItems: 'center',
							gap: 12,
							paddingLeft: 24,
							fontFamily: font.text,
							fontWeight: 500,
							fontSize: 28,
							lineHeight: 1,
							color: mixHex(APP.ink2, '#ffffff', ink),
							whiteSpace: 'nowrap',
						}}
					>
						<span>{p.label}</span>
						{p.key ? <KeyIcon size={28} stroke={APP.signal} strokeWidth={1.75} /> : null}
					</div>
				);
			})}
		</>
	);
};

/* ------------------------------------------------------------------------ */
/* Scene                                                                     */
/* ------------------------------------------------------------------------ */

/** Window set back: zoom 0.9 around the picker, placed up-right; slow turn + push. */
const WINDOW_FOCUS = {x: 1583, y: 453};

/** The lifted picker card: 1380 px wide (pill text 28 × 1380/874 = 44 px). */
const CARD_W = 1380;
const CARD = {x: 930, y: 452} as const;

const S16SuaIa: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const {fps} = useVideoConfig();
	const [l1, l2] = scene.copy;
	const line1 = unitsOf(l1.text, [
		{text: 'Claude,', at: 2, lit: litOf('anthropic')},
		{text: 'OpenAI', at: 4, lit: litOf('openai')},
		{text: 'ou Grok.', at: 6, lit: litOf('xai')},
	]);
	const ubiGlow = (fr: number) => (fr < CLICK ? 0 : 1 - ramp(fr, CLICK + 10, CLICK + 28, E.glide)) * ramp(fr, CLICK, CLICK + 4);
	const line2 = unitsOf(l2.text, [
		{text: 'Ou', at: 48},
		{text: 'deixe', at: 50},
		{text: 'com', at: 52},
		{text: 'o Ubi.', at: 54, volt: l2.emphasis.includes('Ubi.') ? 'Ubi.' : undefined, lit: ubiGlow},
	]);

	const file = f < SWAP ? BEFORE : AFTER;
	const shot: ScreenConfig = {
		src: file,
		camera: [
			{at: 0, zoom: 0.9, focus: WINDOW_FOCUS, anchor: {x: 1130, y: 236}, duration: 0},
			{at: 104, zoom: 0.95, focus: WINDOW_FOCUS, anchor: {x: 1112, y: 240}, duration: 104, easing: E.linear},
		],
		rotateX: [
			[0, 7],
			[104, 5, E.linear],
		],
		rotateY: [
			[0, -15],
			[104, -11, E.linear],
		],
		radius: 18,
		// set-back context window: a brighter grade (it is lit by the stage key; the small print is blurred + patched)
		grade: {brightness: 1.32, lift: 0.08},
	};
	const patches = [...storyboardPatches(scene, file), ...V2_PATCHES.filter((p) => p.file === file)];

	const lift: Pick<LiftCardProps, 'rect' | 'at' | 'enter'> = {rect: PICKER, at: 4, enter: 'lift'};
	const cardProps: LiftCardProps = {
		src: file,
		...lift,
		from: (fr: number) => mapImageRect(shot, fr, COMP, PICKER),
		x: CARD.x,
		y: CARD.y,
		width: CARD_W,
		rotateX: [
			[4, 14],
			[104, 7, E.linear],
		],
		rotateY: [
			[4, -10],
			[104, -4, E.linear],
		],
		float: 5,
		floatPeriod: 90,
		drift: {x: -0.1, y: 0},
		radius: 24,
		glowOpacity: 0.55,
		grade: {brightness: 1.26, lift: 0.07},
		patches,
	};
	const pose = liftCardPose(cardProps, f, fps);
	const pcx = PICKER.x + PICKER.w / 2;
	const pcy = PICKER.y + PICKER.h / 2;
	const tipAt = (fr: number, id: ProviderId) => {
		const p = liftCardPose(cardProps, fr, fps);
		return projectCardPoint(p, (TIPS[id].x - pcx) * p.k, (TIPS[id].y - pcy) * p.k);
	};

	// cursor: rest → Anthropic f15 → OpenAI f30 → xAI f45 → IA do Ubi f71 → CLICK f75 → drifts off
	let cur = REST_IN;
	if (f > 8 && f < 15) cur = arcPoint(REST_IN, tipAt(f, 'anthropic'), E.cursor(ramp(f, 8, 15)), 0.12);
	else if (f >= 15 && f <= 16) cur = tipAt(f, 'anthropic');
	else if (f > 16 && f < 30) cur = arcPoint(tipAt(16, 'anthropic'), tipAt(f, 'openai'), E.cursor(ramp(f, 16, 30)), 0.12);
	else if (f >= 30 && f <= 31) cur = tipAt(f, 'openai');
	else if (f > 31 && f < 45) cur = arcPoint(tipAt(31, 'openai'), tipAt(f, 'xai'), E.cursor(ramp(f, 31, 45)), 0.12);
	else if (f >= 45 && f <= 57) cur = tipAt(f, 'xai');
	else if (f > 57 && f < 71) cur = arcPoint(tipAt(57, 'xai'), tipAt(f, 'ubi'), E.cursor(ramp(f, 57, 71)), 0.12);
	else if (f >= 71 && f <= 86) cur = tipAt(f, 'ubi');
	else if (f > 86) cur = arcPoint(tipAt(86, 'ubi'), REST_OUT, E.cursor(ramp(f, 86, 100)), 0.12);
	const press = pressAt(f, CLICK);
	const cSize = 44 * (1 - 0.15 * press);
	const cOpacity = ramp(f, 2, 8, E.enter);
	const ubiRect = rectOf('ubi');
	const clickPt = projectCardPoint(liftCardPose(cardProps, CLICK, fps), (ubiRect.x + ubiRect.w / 2 - pcx) * pose.k, (ubiRect.y + ubiRect.h / 2 - pcy) * pose.k);

	return (
		<AbsoluteFill style={{backgroundColor: STAGE.bottom}}>
			{/* the navy base stays under the blur-dissolve out-half (s17's in-half dissolves over the same base) */}
			<StageBase />
			<TransitionOut>
				<Stage
					seed="s16"
					orbs={[
						{c: color.volt, x: 1180, y: 250, d: 1300, opacity: 0.2},
						{c: color.ember, x: 1760, y: 980, d: 700, opacity: 0.08},
					]}
					look={{
						// the key pool + key light sit behind the window and the lifted card (the subject); the volt "Ubi." (bottom left) is on their falloff
						keyPool: {x: 0.54, y: 0.46, w: 1.0, h: 1.0, opacity: 0.44},
						keyLight: {x: 0.5, y: 0.42, w: 0.62, h: 0.4, opacity: 0.18},
					}}
				>
					<TransitionIn>
						{/* the window, set back: a touch of depth blur */}
						<AbsoluteFill style={{filter: 'blur(1.2px)'}}>
							<Screen {...shot} style={{zIndex: 'auto'}}>
								{patches.map((p, i) => (
									<div key={i} style={{position: 'absolute', left: p.rect.x - 2, top: p.rect.y - 2, width: p.rect.w + 4, height: p.rect.h + 4, background: p.color}} />
								))}
								{/* continuity: the review queue is empty since s13 — no pending-count badge */}
								<div style={{position: 'absolute', left: BADGE.x - 3, top: BADGE.y - 3, width: BADGE.w + 6, height: BADGE.h + 6, background: '#0a101c'}} />
								<LiftHole {...lift} color={PANEL} radius={22} pad={8} socket={0.7} />
							</Screen>
						</AbsoluteFill>
						{/* navy falloff under the type (the stage, not a black band) */}
						<AbsoluteFill style={{background: `linear-gradient(180deg, ${navyDim(0)} 580px, ${navyDim(0.5)} 700px, ${navyDim(0.55)} 1080px)`}} />
						<LiftCard {...cardProps} style={{zIndex: 30}}>
							<Picker f={f} />
						</LiftCard>
						<AbsoluteFill style={{zIndex: 31}}>
							<Shockwave x={clickPt.x} y={clickPt.y} at={CLICK} radius={260} len={18} squash={0.55} strength={0.9} />
							<ClickRipple x={clickPt.x} y={clickPt.y} at={CLICK} scale={1.4} />
							{f >= 2 ? <ArrowCursor x={cur.x} y={cur.y} size={cSize} opacity={cOpacity} /> : null}
						</AbsoluteFill>
					</TransitionIn>
				</Stage>
			</TransitionOut>
			<Headline units={line1} size={116} left={136} capTop={706} />
			{f >= 48 ? <Headline units={line2} size={116} left={136} capTop={848} /> : null}
			<Vignette strength={0.5} />
			<FilmGrain opacity={0.045} seed="s16" />
		</AbsoluteFill>
	);
};

export default S16SuaIa;
