/**
 * s16-sua-ia — S16 · abs 1125–1229 (105 f) · proof · bar 19.4 → 21.3
 *
 * f0–3 whip-left in-half (T2) on the UI plane: a tight, tilted crop of the real
 * Configurações › IA "Provedor de IA" picker (zoom 2.6, rx 3° ry −7°, the @3x
 * twin → bitmap scale 0.87). The picker is a vector re-set with the app's own
 * ProviderPicker tokens (Inter 500 28 image px, px-3 / gap-1.5, lucide KeyRound
 * in --signal, active = bg-panel + inset line-2, radius 7 css px) so hover and
 * active states are crisp and can move. Line 1 "Claude, OpenAI ou Grok." staggers
 * in f2/4/6 over the bottom band. The cursor hovers each provider on the beat
 * (f15 Anthropic, f30 OpenAI, f45 xAI Grok: text → ink + a 4-f hover fill),
 * line 2 "Ou deixe com o Ubi." f48–60, cursor on "IA do Ubi" f71, CLICK f75
 * (abs 1200 downbeat), the active pill slides to "IA do Ubi" (SNAPPY) while the
 * capture swaps hard to settings-ai-ubi at f77 (same camera), f78 shimmer across
 * the pill. f77–104 10-px pan drift. f99–104 blur-dissolve out-half on the
 * UI/background; the type hard-cuts at f105.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {Screen, type ScreenConfig} from '../components/Screen';
import {mapWithGeometry, screenGeometry, type Rect} from '../components/screen-geometry';
import {alpha, color, font} from '../design/tokens';
import {E, hotspot, Patches, springAt, storyboardCamera, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {
	APP,
	arcPoint,
	ArrowCursor,
	Backdrop,
	ClickRipple,
	cursorScale,
	Finish,
	H,
	Headline,
	Icon,
	lerp,
	mixHex,
	pressAt,
	ramp,
	unitsOf,
	W,
} from './_parts/G5/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'whip_3.wav', atFrame: 0, gainDb: -8, note: 'Whip on the cut abs 1125; file starts 3 f earlier (master track).'},
	{ref: 'ui_tick_2.wav', atFrame: 30, gainDb: -22, note: 'Hover.'},
	{ref: 'ui_tick_3.wav', atFrame: 45, gainDb: -22, note: 'Hover.'},
	{ref: 'click.wav', atFrame: 75, gainDb: -12, note: 'IA do Ubi.'},
	{ref: 'shimmer_2.wav', atFrame: 78, gainDb: -14, note: 'State change.'},
];

const BEFORE = 'ui/settings-ai.png';
const AFTER = 'ui/settings-ai-ubi.png';
const SWAP = 77;
const CLICK = 75;

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

/** Hover windows (the cursor rests on each label from its arrival − 3 f to its departure). */
const HOVER: Record<ProviderId, [number, number]> = {
	anthropic: [12, 17],
	openai: [27, 32],
	xai: [42, 58],
	ubi: [68, 999],
};

/** Cursor tips (image px): just below each label's centre, never on the words' x-height. */
const TIPS: Record<ProviderId, {x: number; y: number}> = {
	anthropic: {x: 1330, y: 474},
	openai: {x: 1560, y: 474},
	xai: {x: 1716, y: 474},
	ubi: {x: 1872, y: 474},
};
const REST_IN = {x: 820, y: 640};
const REST_OUT = {x: 1660, y: 650};

const Picker: React.FC<{f: number}> = ({f}) => {
	// active pill: Anthropic → IA do Ubi, SNAPPY from the click's release
	const t = f < CLICK + 1 ? 0 : springAt(f, CLICK + 1, 'SNAPPY');
	const a = rectOf('anthropic');
	const b = rectOf('ubi');
	const pill = {x: lerp(a.x, b.x, t), y: a.y, w: lerp(a.w, b.w, t), h: a.h};
	const stretch = Math.sin(Math.PI * Math.min(1, Math.max(0, t))) * 0.06; // a hair of stretch mid-travel
	const sheen = ramp(f, 78, 94, E.glide);
	return (
		<>
			{/* the control's interior, re-filled so the re-set owns every state */}
			<div style={{position: 'absolute', left: PICKER.x + 2, top: PICKER.y + 2, width: PICKER.w - 4, height: PICKER.h - 4, borderRadius: 16, background: APP.panel2}} />
			{PROVIDERS.map((p) => {
				const [h0, h1] = HOVER[p.id];
				const hover = ramp(f, h0, h0 + 4, E.enter) * (1 - ramp(f, h1, h1 + 4, E.enter));
				return hover > 0.001 ? (
					<div
						key={`h-${p.id}`}
						style={{position: 'absolute', left: p.rect.x, top: p.rect.y, width: p.rect.w, height: p.rect.h, borderRadius: 14, background: `rgba(255,255,255,${(0.045 * hover).toFixed(4)})`}}
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
					background: APP.panel,
					boxShadow: `inset 0 0 0 2px ${APP.line2}`,
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
							background: `linear-gradient(90deg, transparent, ${alpha(color.volt, 0.22)}, rgba(255,255,255,0.10), ${alpha(color.volt, 0.22)}, transparent)`,
						}}
					/>
				) : null}
			</div>
			{PROVIDERS.map((p) => {
				const [h0, h1] = HOVER[p.id];
				const hover = ramp(f, h0, h0 + 4, E.enter) * (1 - ramp(f, h1, h1 + 4, E.enter));
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
							color: mixHex(APP.ink2, APP.ink, ink),
							whiteSpace: 'nowrap',
						}}
					>
						<span>{p.label}</span>
						{p.key ? <Icon name="key-round" size={28} stroke={APP.signal} strokeWidth={1.75} /> : null}
					</div>
				);
			})}
		</>
	);
};

const S16SuaIa: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const comp = {width: W, height: H};
	const [l1, l2] = scene.copy;
	const line1 = unitsOf(l1.text, [
		{text: 'Claude,', at: 2},
		{text: 'OpenAI', at: 4},
		{text: 'ou Grok.', at: 6},
	]);
	const line2 = unitsOf(l2.text, [
		{text: 'Ou', at: 48},
		{text: 'deixe', at: 50},
		{text: 'com', at: 52},
		{text: 'o Ubi.', at: 54, volt: l2.emphasis.includes('Ubi.') ? 'Ubi.' : undefined},
	]);

	const file = f < SWAP ? BEFORE : AFTER;
	const shot: ScreenConfig = {
		src: file,
		...storyboardCamera(scene),
		// alive while the camera holds: ×1.00 → ×1.012 over the scene
		scale: [
			[0, 1],
			[104, 1.012, E.linear],
		],
		glow: false,
		radius: 18,
	};
	const g = screenGeometry(shot, f, comp);
	const tip = (fr: number, id: ProviderId) => mapWithGeometry(screenGeometry(shot, fr, comp), TIPS[id]);

	// cursor: rest → Anthropic f15 → OpenAI f30 → xAI f45 → IA do Ubi f71 → CLICK f75 → drifts off
	let cur = REST_IN;
	if (f > 8 && f < 15) cur = arcPoint(REST_IN, tip(f, 'anthropic'), E.cursor(ramp(f, 8, 15)), 0.12);
	else if (f >= 15 && f <= 16) cur = tip(f, 'anthropic');
	else if (f > 16 && f < 30) cur = arcPoint(tip(16, 'anthropic'), tip(f, 'openai'), E.cursor(ramp(f, 16, 30)), 0.12);
	else if (f >= 30 && f <= 31) cur = tip(f, 'openai');
	else if (f > 31 && f < 45) cur = arcPoint(tip(31, 'openai'), tip(f, 'xai'), E.cursor(ramp(f, 31, 45)), 0.12);
	else if (f >= 45 && f <= 57) cur = tip(f, 'xai');
	else if (f > 57 && f < 71) cur = arcPoint(tip(57, 'xai'), tip(f, 'ubi'), E.cursor(ramp(f, 57, 71)), 0.12);
	else if (f >= 71 && f <= 86) cur = tip(f, 'ubi');
	else if (f > 86) cur = arcPoint(tip(86, 'ubi'), REST_OUT, E.cursor(ramp(f, 86, 100)), 0.12);
	const press = pressAt(f, CLICK);
	const cSize = 30 * cursorScale(g.k) * (1 - 0.15 * press);
	const cOpacity = ramp(f, 2, 8, E.enter);
	const clickPt = tip(CLICK, 'ubi');

	return (
		<AbsoluteFill style={{backgroundColor: color.canvas}}>
			<TransitionOut>
				<Backdrop seed="s16" ember={0.04} volt={0.14}>
					<TransitionIn>
						<Screen {...shot} style={{zIndex: 'auto'}}>
							<Patches patches={storyboardPatches(scene, file)} />
							<Picker f={f} />
							{/* continuity: the review queue is empty since s13 — no pending-count badge (seen during the whip) */}
							<div style={{position: 'absolute', left: BADGE.x - 3, top: BADGE.y - 3, width: BADGE.w + 6, height: BADGE.h + 6, background: '#0a101c'}} />
						</Screen>
						<ClickRipple x={clickPt.x} y={clickPt.y} at={CLICK} scale={cursorScale(g.k)} />
						{f >= 2 ? <ArrowCursor x={cur.x} y={cur.y} size={cSize} opacity={cOpacity} /> : null}
					</TransitionIn>
					{/* bottom band: solid canvas from y 700 (storyboard scrim), feathered 60 px above it */}
					<AbsoluteFill
						style={{
							background: 'linear-gradient(180deg, rgba(10,13,22,0) 630px, rgba(10,13,22,0.85) 680px, #0a0d16 700px, #0a0d16 1080px)',
						}}
					/>
				</Backdrop>
			</TransitionOut>
			<Headline units={line1} size={88} left={144} capTop={736} />
			{f >= 48 ? <Headline units={line2} size={88} left={144} capTop={840} /> : null}
			<Finish seed="s16" vignette={0.45} />
		</AbsoluteFill>
	);
};

export default S16SuaIa;
