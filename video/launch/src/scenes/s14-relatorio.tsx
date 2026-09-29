/**
 * s14-relatorio — S14 · abs 930–1034 (105 f) · features · bar 16.3 → 18.2
 *
 * Part A (f0–29): hard cut to a big tabular "17:59" on the orbs; f10–15 the last
 * three digits roll like an odometer (staggered 1 f, right to left, vertical
 * motion blur) and make contact on "18:00" at f15 (abs 945, beat 2): pulse
 * 1 → 1.05 → 1, volt, a glow breath.
 * Part B (f30–104): on the downbeat (abs 960) the clock flies up into the volt
 * kicker pill (SNAPPY 12 f, Sora → Inter crossfade in the last 5 f) while the
 * real Relatórios window rises (y +220 → 0, rotateX 20° → 6°, E.push 20 f) into
 * a wide on the IFRO report (f45), then punches in on "Copiar Markdown"
 * (E.push 12 f, arrives f57). The button is a vector re-set with the app's own
 * tokens (Button sm: 24 image px label, lucide Copy 28 px, line-2 border),
 * lifted 1.16 off the plane so its label reads at ≥ 36 px. Cursor arcs in, hover
 * f54 (the app's hover:bg-panel-2), CLICK f60 (abs 990, beat 3), the icon turns
 * into a mint check (draw-on 10 f), the "Markdown copiado" pill pops under the
 * button with a 1.5 px leader. The model name in both report meta lines is
 * patched; the rest of each meta line is re-composed from the capture's own
 * pixels ("Gerado 29/09 18:00" + "3h22 registradas · 6.120 tokens…").
 */
import React from 'react';
import {AbsoluteFill, Easing} from 'remotion';
import {measureText} from '@remotion/layout-utils';
import {Screen, type ScreenConfig} from '../components/Screen';
import {mapWithGeometry, screenGeometry, type CameraKey} from '../components/screen-geometry';
import {alpha, color, font} from '../design/tokens';
import {E, hotspot, Patches, springAt, storyboardPatches, TransitionIn, TransitionOut, useScene, useSceneFrame, type SfxCue} from '../shared';
import {
	APP,
	arcPoint,
	ArrowCursor,
	Backdrop,
	ClickRipple,
	clamp01,
	Crop,
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
	{ref: 'ding_1.wav', atFrame: 15, gainDb: -12, note: '18:00 lands (E6); rhymes with s06’s ding.'},
	{ref: 'whoosh-soft.wav', atFrame: 36, gainDb: -14, note: 'Report window rises; loudest point ≈ fastest frame of the rise.'},
	{ref: 'click.wav', atFrame: 60, gainDb: -12, note: 'Copiar Markdown.'},
	{ref: 'success_chime_2.wav', atFrame: 63, gainDb: -14, note: 'Mint check.'},
];

const FILE = 'ui/reports.png';
const CONTACT = 15;
const MORPH = 30;
const MORPH_END = 42;
const RISE = 30;
const PUNCH = 57;
const CLICK = 60;
const OK = 62;

const BTN = hotspot(FILE, 'report-1-copy'); // 2282, 392, 288 × 64
const LIFT = 1.16;
const BADGE = hotspot(FILE, 'nav-revisao-badge');

/* ------------------------------------------------------------------------ */
/* Part A — the clock                                                        */
/* ------------------------------------------------------------------------ */

const CLOCK = {size: 240, cx: 960, cy: 470};
const KICK = {left: 144, top: 88, h: 48, size: 40, padX: 20};

/** One digit cell: rolls from `from` to `to` (new digit enters from below). */
const Digit: React.FC<{from: string; to: string; p: number; v: number}> = ({from, to, p, v}) => {
	const h = CLOCK.size;
	if (from === to) return <span style={{display: 'inline-block'}}>{to}</span>;
	// vertical motion blur ∝ speed, released over the last 15 % so the digit is crisp at contact
	const blur = Math.min(10, Math.abs(v) * h * 0.12) * clamp01((1 - p) / 0.15);
	return (
		<span
			style={{
				display: 'inline-block',
				position: 'relative',
				height: h * 1.0,
				overflow: 'hidden',
				verticalAlign: 'top',
				WebkitMaskImage: p > 0.001 && p < 0.999 ? 'linear-gradient(180deg, transparent 0%, #000 16%, #000 84%, transparent 100%)' : undefined,
			}}
		>
			<span
				style={{
					display: 'block',
					transform: p > 0.0005 ? `translateY(${(-p * h).toFixed(2)}px)` : undefined,
					filter: blur > 0.5 ? `url(#s14-vblur-${Math.round(blur)})` : undefined,
				}}
			>
				<span style={{display: 'block', height: h}}>{from}</span>
				<span style={{display: 'block', height: h}}>{to}</span>
			</span>
		</span>
	);
};

const VBlurDefs: React.FC = () => (
	<svg width={0} height={0} style={{position: 'absolute'}}>
		<defs>
			{Array.from({length: 11}, (_, b) => (
				<filter key={b} id={`s14-vblur-${b}`} x="0" y="-20%" width="100%" height="140%">
					<feGaussianBlur stdDeviation={`0 ${b}`} />
				</filter>
			))}
		</defs>
	</svg>
);

const Clock: React.FC<{f: number}> = ({f}) => {
	// odometer: right to left, contact on f15
	const rolls = [
		{i: 4, from: '9', to: '0', start: 8},
		{i: 3, from: '5', to: '0', start: 9},
		{i: 1, from: '7', to: '8', start: 10},
	];
	const cells = ['1', '7', ':', '5', '9'];
	const prog = (start: number, fr: number) => E.glide(clamp01((fr - start) / (CONTACT - start)));

	const pulse = 1 + 0.05 * (ramp(f, CONTACT - 1, CONTACT + 2, E.push) - ramp(f, CONTACT + 2, CONTACT + 13, E.glide));
	const voltT = ramp(f, CONTACT - 1, CONTACT);
	const drift = 1 + 0.02 * ramp(f, 0, MORPH);

	// morph to the kicker pill (SNAPPY 12 f)
	const m = f < MORPH ? 0 : springAt(f, MORPH, 'SNAPPY', 12);
	const soraW = measureText({text: '18:00', fontFamily: font.display, fontSize: CLOCK.size, fontWeight: '700', letterSpacing: '-0.04em', additionalStyles: {fontVariantNumeric: 'tabular-nums'}}).width;
	const interW = measureText({text: '18:00', fontFamily: font.text, fontSize: KICK.size, fontWeight: '600', additionalStyles: {fontVariantNumeric: 'tabular-nums'}}).width;
	const pillW = interW + KICK.padX * 2;
	const tx = KICK.left + pillW / 2;
	const ty = KICK.top + KICK.h / 2;
	const endScale = interW / soraW;
	const cx = lerp(CLOCK.cx, tx, m);
	const cy = lerp(CLOCK.cy, ty, m);
	const sc = Math.exp(lerp(0, Math.log(endScale), m)) * (f < MORPH ? pulse * drift : lerp(drift, 1, clamp01(m)));
	const bigO = 1 - ramp(f, MORPH_END - 5, MORPH_END - 1);
	const pillO = ramp(f, MORPH_END - 5, MORPH_END - 1);
	const pillBg = f < MORPH ? 0 : springAt(f, MORPH + 3, 'SNAPPY', 10);

	// glow breath behind the clock at contact
	const glow = 0.14 + 0.1 * (ramp(f, CONTACT - 1, CONTACT + 2) - ramp(f, CONTACT + 2, CONTACT + 24, E.glide));
	const glowO = 1 - ramp(f, MORPH, MORPH + 10, E.enter);

	return (
		<>
			<VBlurDefs />
			{glowO > 0.001 ? (
				<div
					style={{
						position: 'absolute',
						left: CLOCK.cx - 700,
						top: CLOCK.cy - 420,
						width: 1400,
						height: 840,
						borderRadius: '50%',
						opacity: glowO,
						background: `radial-gradient(closest-side, ${alpha(color.volt, glow)} 0%, ${alpha(color.volt, glow * 0.35)} 45%, transparent 100%)`,
					}}
				/>
			) : null}
			{bigO > 0.001 ? (
				<div
					style={{
						position: 'absolute',
						left: cx,
						top: cy,
						transform: `translate(-50%, -50%) scale(${sc.toFixed(5)})`,
						transformOrigin: '50% 50%',
						opacity: bigO,
						fontFamily: font.display,
						fontWeight: 700,
						fontSize: CLOCK.size,
						lineHeight: 1,
						height: CLOCK.size,
						letterSpacing: '-0.04em',
						fontVariantNumeric: 'tabular-nums',
						color: mixHex(color.ink, color.volt, voltT),
						whiteSpace: 'nowrap',
						display: 'flex',
					}}
				>
					{cells.map((c, i) => {
						const r = rolls.find((x) => x.i === i);
						if (!r) return (
							<span key={i} style={{display: 'inline-block'}}>
								{c}
							</span>
						);
						const p = prog(r.start, f);
						const v = p - prog(r.start, f - 1);
						return <Digit key={i} from={r.from} to={r.to} p={p} v={v} />;
					})}
				</div>
			) : null}
			{pillO > 0.001 || pillBg > 0.001 ? (
				<div
					style={{
						position: 'absolute',
						left: KICK.left,
						top: KICK.top,
						width: pillW,
						height: KICK.h,
						borderRadius: 999,
						background: alpha(color.volt, 0.14 * clamp01(pillBg)),
						transform: pillBg < 0.999 ? `scale(${lerp(0.7, 1, pillBg).toFixed(4)})` : undefined,
						transformOrigin: '50% 50%',
					}}
				/>
			) : null}
			{pillO > 0.001 ? (
				<div
					style={{
						position: 'absolute',
						left: KICK.left,
						top: KICK.top,
						width: pillW,
						height: KICK.h,
						display: 'flex',
						alignItems: 'center',
						justifyContent: 'center',
						opacity: pillO,
						fontFamily: font.text,
						fontWeight: 600,
						fontSize: KICK.size,
						lineHeight: 1,
						fontVariantNumeric: 'tabular-nums',
						color: color.volt,
					}}
				>
					18:00
				</div>
			) : null}
		</>
	);
};

/* ------------------------------------------------------------------------ */
/* Part B — the report                                                       */
/* ------------------------------------------------------------------------ */

const PUNCH_EASE = Easing.bezier(0.42, 0, 0.12, 1);

const CAMERA: CameraKey[] = [
	{at: RISE, zoom: 1.35, focus: {x: 1672, y: 840}, anchor: {x: 1100, y: 640}, duration: 0},
	{at: 45, zoom: 1.6, focus: {x: 1672, y: 840}, anchor: {x: 1100, y: 640}, duration: 15, easing: E.push},
	// punch-in: leaves the wide at rest (the rise just settled), arrives hard — E.push from a standstill read as a 1-frame jump
	{at: PUNCH, zoom: 2.6, focus: {x: 2426, y: 424}, anchor: {x: 1340, y: 600}, duration: 12, easing: PUNCH_EASE},
	// hold: micro drift ×1.02 (float only; bitmap scale 1.33, @3x twin → 0.88)
	{at: 104, zoom: 2.65, focus: {x: 2426, y: 424}, anchor: {x: 1340, y: 600}, duration: 44, easing: E.linear},
];

const SHOT: ScreenConfig = {
	src: FILE,
	camera: CAMERA,
	rotateX: [
		[RISE, 20],
		[50, 6, E.push],
		[PUNCH, 4, E.push],
	],
	rotateY: -3,
	glow: false,
	radius: 18,
};

/** Rise offset (screen space, applied outside the camera so the camera cannot cancel it). */
const riseY = (f: number) => 220 * (1 - E.push(ramp(f, RISE, RISE + 20)));

/** Meta lines: patch (storyboard) + the capture's own words minus the model name. */
const META_DY = [0, 1262];
const MetaRecompose: React.FC = () => (
	<>
		{META_DY.map((dy) => (
			<React.Fragment key={dy}>
				{/* "Gerado 29/09 18:00" stays where it is */}
				<Crop src={FILE} rect={{x: 648, y: 434 + dy, w: 244, h: 36}} />
				{/* "3h22 registradas   6.120 tokens de entrada, 1.480 de saída" slides in after it (the 26-px group gap kept) */}
				<Crop src={FILE} rect={{x: 1160, y: 434 + dy, w: 700, h: 36}} at={{x: 912, y: 434 + dy}} />
			</React.Fragment>
		))}
	</>
);

/** The lifted "Copiar Markdown" button, vector re-set with the app's Button sm tokens. */
const CopyButton: React.FC<{f: number}> = ({f}) => {
	const lift = lerp(1, LIFT, E.push(ramp(f, 45, PUNCH)));
	const press = pressAt(f, CLICK);
	const hover = ramp(f, 54, 58, E.enter);
	const scale = lift * (1 - 0.04 * press);
	const flash = ramp(f, OK, OK + 2) * (1 - ramp(f, OK + 2, OK + 10, E.enter));
	const iconOut = ramp(f, CLICK, CLICK + 3, E.exit);
	const check = ramp(f, OK, OK + 10, E.push);
	const bg = mixHex(mixHex(APP.panel, APP.panel2, hover), color.mint, 0.12 * flash);
	const border = mixHex(APP.line2, color.mint, 0.55 * ramp(f, OK, OK + 6));
	const shadowY = 4 + 14 * (lift - 1) / (LIFT - 1);
	return (
		<div
			style={{
				position: 'absolute',
				left: BTN.x,
				top: BTN.y,
				width: BTN.w,
				height: BTN.h,
				transform: `scale(${scale.toFixed(4)})`,
				// grows to the left (open card space), away from "Regenerar" 16 px to its right
				transformOrigin: '100% 50%',
				boxSizing: 'border-box',
				borderRadius: 16,
				background: bg,
				boxShadow: `inset 0 0 0 2px ${border}, 0 ${shadowY.toFixed(1)}px ${(12 + shadowY * 1.6).toFixed(1)}px -6px rgba(0,0,0,${(0.35 + 0.3 * (lift - 1) / (LIFT - 1)).toFixed(3)})`,
				display: 'flex',
				alignItems: 'center',
				paddingLeft: 22,
				gap: 12,
				fontFamily: font.text,
				fontWeight: 500,
				fontSize: 24,
				lineHeight: 1,
				color: APP.ink,
				whiteSpace: 'nowrap',
			}}
		>
			<div style={{position: 'relative', width: 28, height: 28, flex: '0 0 28px'}}>
				{iconOut < 0.999 ? (
					<Icon name="copy" size={28} stroke={APP.ink} strokeWidth={1.75} style={{position: 'absolute', left: 0, top: 0, opacity: 1 - iconOut, transform: `scale(${lerp(1, 0.6, iconOut)})`}} />
				) : null}
				{f >= OK ? <Icon name="check" size={28} stroke={color.mint} strokeWidth={2.25} draw={check} style={{position: 'absolute', left: 0, top: 0}} /> : null}
			</div>
			<span>Copiar Markdown</span>
		</div>
	);
};

/** Image-space spotlight on the lifted button (dim 0.62, volt outline ≈ 2.5 comp px). */
const ButtonSpot: React.FC<{f: number; onScreen: number}> = ({f, onScreen}) => {
	const d = ramp(f, 45, 57, E.enter);
	if (d <= 0.001) return null;
	const lift = lerp(1, LIFT, E.push(ramp(f, 45, PUNCH)));
	const grow = BTN.w * (lift - 1);
	const padY = 12 + (BTN.h * (lift - 1)) / 2;
	const bw = 2.5 / Math.max(0.05, onScreen);
	const gl = 26 / Math.max(0.05, onScreen);
	const ok = ramp(f, OK, OK + 6);
	const ring = mixHex(color.volt, color.mint, ok);
	return (
		<div
			style={{
				position: 'absolute',
				left: BTN.x - grow - 12,
				top: BTN.y - padY,
				width: BTN.w + grow + 12 + 7,
				height: BTN.h + padY * 2,
				borderRadius: 26,
				boxShadow: [
					`0 0 0 ${bw}px ${alpha(ring, 0.9 * d)}`,
					`0 0 ${gl}px ${gl * 0.25}px ${alpha(ring, 0.4 * d)}`,
					`0 0 0 6000px rgba(6, 9, 16, ${(0.62 * d).toFixed(3)})`,
				].join(', '),
			}}
		/>
	);
};

/** "Markdown copiado": the app's own toast string as a mint callout pill under the button. */
const Copied: React.FC<{f: number; x: number; top: number; text: string}> = ({f, x, top, text}) => {
	if (f < OK) return null;
	const line = ramp(f, OK, OK + 10, E.push);
	const s = springAt(f, OK + 1, 'SNAPPY');
	const o = ramp(f, OK + 1, OK + 4);
	const L = 56;
	const pillTop = top + L + 6;
	return (
		<>
			<div style={{position: 'absolute', left: x - 0.75, top, width: 1.5, height: L * line, background: alpha(color.ink3, 0.9), borderRadius: 1}} />
			<div
				style={{
					position: 'absolute',
					left: x,
					top: pillTop,
					transform: `translateX(-50%) translateY(${((1 - s) * -10).toFixed(2)}px) scale(${lerp(0.92, 1, s).toFixed(4)})`,
					transformOrigin: '50% 0%',
					opacity: o,
					height: 64,
					padding: '0 30px 0 22px',
					display: 'flex',
					alignItems: 'center',
					gap: 12,
					borderRadius: 999,
					// solid under-layer so the pill reads over the dimmed UI, then the 16 % mint tint
					background: `linear-gradient(${alpha(color.mint, 0.16)}, ${alpha(color.mint, 0.16)}), #0a0d16`,
					boxShadow: `inset 0 0 0 1.5px ${alpha(color.mint, 0.45)}, 0 12px 28px -8px rgba(0,0,0,0.6)`,
					fontFamily: font.text,
					fontWeight: 600,
					fontSize: 36,
					lineHeight: 1,
					color: color.ink,
					whiteSpace: 'nowrap',
				}}
			>
				<Icon name="check" size={30} stroke={color.mint} strokeWidth={2.5} draw={ramp(f, OK + 2, OK + 10, E.push)} />
				<span>{text}</span>
			</div>
		</>
	);
};

/* ------------------------------------------------------------------------ */

const REST_IN = {x: 1750, y: 940};
const REST_OUT = {x: 1700, y: 900};
/** Cursor tip on the label, left of centre (image px). */
const TIP = {x: BTN.x + 120, y: BTN.y + 38};

const S14Relatorio: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const comp = {width: W, height: H};
	const kicker = scene.copy.find((c) => c.role === 'kicker')!;
	const head = scene.copy.find((c) => c.role === 'headline')!;
	const caption = scene.copy.find((c) => c.role === 'ui-caption')!;
	if (kicker.text !== '18:00') throw new Error(`s14 kicker drift: ${kicker.text}`);
	const units = unitsOf(head.text, [
		{text: 'O\u00a0relatório', at: 32},
		{text: 'sai', at: 34},
		{text: 'pronto.', at: 36, volt: head.emphasis.includes('pronto.') ? 'pronto.' : undefined},
	]);

	const partB = f >= RISE;
	const rise = riseY(f);
	const g = screenGeometry(SHOT, f, comp);
	const onScreen = g.k * g.scale * g.s0;
	const toComp = (fr: number, p: {x: number; y: number}) => {
		const q = mapWithGeometry(screenGeometry(SHOT, fr, comp), p);
		return {x: q.x, y: q.y + riseY(fr)};
	};

	// cursor
	const tipAt = (fr: number) => toComp(fr, TIP);
	let cur = REST_IN;
	if (f > 40 && f < 56) cur = arcPoint(REST_IN, tipAt(f), E.cursor(ramp(f, 40, 56)), 0.12);
	else if (f >= 56 && f <= 66) cur = tipAt(f);
	else if (f > 66) cur = arcPoint(tipAt(66), REST_OUT, E.cursor(ramp(f, 66, 80)), 0.12);
	const press = pressAt(f, CLICK);
	const cSize = 30 * cursorScale(g.k) * (1 - 0.15 * press);
	const cOpacity = ramp(f, 34, 40, E.enter) * (1 - ramp(f, 78, 84, E.exit));
	const clickPt = tipAt(CLICK);

	// callout anchor: bottom-centre of the lifted button
	const btnBottom = toComp(f, {x: BTN.x + BTN.w / 2 - (BTN.w * (LIFT - 1)) / 2, y: BTN.y + BTN.h / 2});
	const halfH = (BTN.h / 2) * LIFT * onScreen;

	const scrim = ramp(f, RISE, RISE + 2);

	return (
		<TransitionOut>
			<TransitionIn>
				<Backdrop seed="s14" ember={0.05}>
					{partB ? (
						<AbsoluteFill style={{transform: rise > 0.05 ? `translateY(${rise.toFixed(2)}px)` : undefined, opacity: ramp(f, RISE, RISE + 4)}}>
							<Screen {...SHOT} style={{zIndex: 'auto'}}>
								<Patches patches={storyboardPatches(scene, FILE)} />
								<MetaRecompose />
								{/* continuity: s13 just emptied the review queue — the sidebar's pending-count badge goes (nav colour) */}
								<div style={{position: 'absolute', left: BADGE.x - 3, top: BADGE.y - 3, width: BADGE.w + 6, height: BADGE.h + 6, background: '#0a101c'}} />
								<ButtonSpot f={f} onScreen={onScreen} />
								<CopyButton f={f} />
							</Screen>
						</AbsoluteFill>
					) : null}
					{scrim > 0.001 ? (
						<AbsoluteFill
							style={{
								opacity: scrim,
								background: 'linear-gradient(180deg, rgba(10,13,22,0.97) 0px, rgba(10,13,22,0.95) 250px, rgba(10,13,22,0.82) 290px, rgba(10,13,22,0) 390px)',
							}}
						/>
					) : null}
					<Clock f={f} />
					{f >= 32 ? <Headline units={units} size={88} left={144} capTop={170} /> : null}
					<Copied f={f} x={Math.round(btnBottom.x)} top={Math.round(btnBottom.y + halfH + 8)} text={caption.text} />
					<ClickRipple x={clickPt.x} y={clickPt.y} at={CLICK} scale={cursorScale(g.k)} />
					{f >= 34 ? <ArrowCursor x={cur.x} y={cur.y} size={cSize} opacity={cOpacity} /> : null}
					<Finish seed="s14" />
				</Backdrop>
			</TransitionIn>
		</TransitionOut>
	);
};

export default S14Relatorio;
