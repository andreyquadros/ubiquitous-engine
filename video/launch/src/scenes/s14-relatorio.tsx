/**
 * s14-relatorio — S14 · abs 930–1034 (105 f) · features · bar 16.3 → 18.2
 *
 * Part A (f0–29): hard cut to a big tabular "17:59" (Sora 700 290 px, v1 240) on the
 * stage; f10–15 the last three digits roll like an odometer (staggered 1 f,
 * right to left, vertical motion blur) and make contact on "18:00" at f15
 * (abs 945, beat 2): pulse 1 → 1.05 → 1, volt, a glow breath. Footlight pool
 * under the clock + a light type guard (GUARD RULE).
 * Part B (f30–104, v2): on the downbeat (abs 960) the clock flies up into the
 * volt kicker pill (Inter 600 44 px, SNAPPY 12 f) while the real Relatórios
 * window rises (y +220 → 0, rotateX 20° → 6°, E.push 20 f) into a wide on the
 * IFRO report; "O relatório sai pronto." (Sora 700 120 px, "pronto." volt)
 * f32–42. f45 (abs 975, beat) the report's activity table + its "Copiar
 * Markdown" button LIFT out of the window as a big tilted card (LiftCard
 * `lift`, crop x 1452–2600 y 376–882 at 1.42 comp px per image px: row text
 * ≈ 37 px, the button label ≈ 34 → 36 px on hover), the window pulls back and
 * steps behind it (camera 1.6 → 1.05, light blur), leaving an empty socket.
 * The button is a vector re-set with the app's own tokens inside the card.
 * Cursor arcs in, hover f54, CLICK f60 (abs 990, beat 3): press, shockwave,
 * the icon becomes a mint check (draw-on 10 f), the card's rim goes mint and
 * the "Markdown copiado" pill (Inter 600 44 px) pops out to the left of the
 * button on a short leader, in the card's header strip. f66–104 hold: card float + drift + tilt settle, light sweep
 * across the card, window drift. The model name in both report meta lines is
 * patched (window + card); the window's meta line is re-composed from the
 * capture's own pixels ("Gerado 29/09 18:00" + "3h22 registradas · …").
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {measureText} from '@remotion/layout-utils';
import {Screen, type ScreenConfig} from '../components/Screen';
import {GUARD_OPACITY, guardFor, navyDim} from '../components/Stage';
import {mapImageRect, type CameraKey} from '../components/screen-geometry';
import {LiftCard, LiftHole, liftCardPose, type LiftCardProps} from '../components/LiftCard';
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
	Finish,
	H,
	Headline,
	Icon,
	gradientFill,
	VOLT_TOP,
	voltGlow,
	LightSweep,
	lerp,
	mixHex,
	pressAt,
	ramp,
	Shockwave,
	unitsOf,
	W,
} from './_parts/G5/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'ding_1.wav', atFrame: 15, gainDb: -12, note: '18:00 lands (E6); rhymes with s06’s ding.'},
	{ref: 'whoosh-soft.wav', atFrame: 36, gainDb: -14, note: 'Report window rises; loudest point ≈ fastest frame of the rise.'},
	{ref: 'whoosh_in_3.wav', atFrame: 53, gainDb: -18, note: 'v2: the report card lifts out of the window (SNAPPY, lands ≈ f53). v2 review: −22 → −18 dB (the film-wide LiftCard lift level).'},
	{ref: 'click.wav', atFrame: 60, gainDb: -12, note: 'Copiar Markdown.'},
	{ref: 'success_chime_2.wav', atFrame: 63, gainDb: -14, note: 'Mint check.'},
];

const FILE = 'ui/reports.png';
const CONTACT = 15;
const MORPH = 30;
const MORPH_END = 42;
const RISE = 30;
const CLICK = 60;
const OK = 62;

const BTN = hotspot(FILE, 'report-1-copy'); // 2282, 392, 288 × 64
const BADGE = hotspot(FILE, 'nav-revisao-badge');

/* ------------------------------------------------------------------------ */
/* Part A — the clock                                                        */
/* ------------------------------------------------------------------------ */

const CLOCK = {size: 290, cx: 960, cy: 470};
const KICK = {left: 106, top: 44, h: 62, size: 44, padX: 26};
/** v2 type guard behind the volt "18:00" (glyph box ≈ x 565–1355, y 348–576 at 290 px); fades with the big clock. */
const CLOCK_GUARD = guardFor({x: CLOCK.cx, y: 462, w: 790, h: 228});

/** One digit cell: rolls from `from` to `to` (new digit enters from below). */
const Digit: React.FC<{from: string; to: string; p: number; v: number; fill: React.CSSProperties}> = ({from, to, p, v, fill}) => {
	const h = CLOCK.size;
	if (from === to) return <span style={{display: 'inline-block', ...fill}}>{to}</span>;
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
				<span style={{display: 'block', height: h, ...fill}}>{from}</span>
				<span style={{display: 'block', height: h, ...fill}}>{to}</span>
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
	// v2: top-lit fill (white → volt gradient at contact)
	const fill = gradientFill(voltT);
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
	// v2: steady level 0.14 → 0.09 so the volt 18:00 holds ≥ 4.5:1 on its stage guard (the contact breath is unchanged)
	const glow = 0.09 + 0.1 * (ramp(f, CONTACT - 1, CONTACT + 2) - ramp(f, CONTACT + 2, CONTACT + 24, E.glide));
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
						whiteSpace: 'nowrap',
						display: 'flex',
						// v2: glyph glow once volt (the top-lit fill is set per glyph: background-clip text does not reach into the rolling cells)
						filter: voltT > 0.01 ? voltGlow(voltT * 0.8) : undefined,
					}}
				>
					{cells.map((c, i) => {
						const r = rolls.find((x) => x.i === i);
						if (!r) return (
							<span key={i} style={{display: 'inline-block', ...fill}}>
								{c}
							</span>
						);
						const p = prog(r.start, f);
						const v = p - prog(r.start, f - 1);
						return <Digit key={i} from={r.from} to={r.to} p={p} v={v} fill={fill} />;
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
						// a pill is a surface (GUARD RULE 4): its own navy backing under the 14 % volt tint
						background: `linear-gradient(${alpha(color.volt, 0.14)}, ${alpha(color.volt, 0.14)}), rgba(10,16,36,0.88)`,
						opacity: clamp01(pillBg),
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
						color: mixHex(color.volt, VOLT_TOP, 0.45),
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

const LIFT_AT = 45;
const COMP = {width: W, height: H};
/** v2 grade: a touch brighter than the default window grade (window + card share it, patches keep matching). */
const GRADE_S14 = {brightness: 1.3, lift: 0.07};

const CAMERA: CameraKey[] = [
	{at: RISE, zoom: 1.35, focus: {x: 1672, y: 840}, anchor: {x: 1000, y: 660}, duration: 0},
	{at: LIFT_AT, zoom: 1.55, focus: {x: 1800, y: 760}, anchor: {x: 1000, y: 660}, duration: 15, easing: E.push},
	// the card leaves: the window pulls back and steps behind it
	{at: 66, zoom: 1.02, focus: {x: 1500, y: 820}, anchor: {x: 1010, y: 690}, duration: 21, easing: E.push},
	{at: 104, zoom: 1.06, focus: {x: 1480, y: 820}, anchor: {x: 1010, y: 690}, duration: 38, easing: E.linear},
];

const SHOT: ScreenConfig = {
	src: FILE,
	camera: CAMERA,
	rotateX: [
		[RISE, 20],
		[50, 6, E.push],
		[104, 8, E.glide],
	],
	rotateY: [
		[RISE, -3],
		[104, -9, E.glide],
	],
	glow: false,
	radius: 18,
	dots: 'neutral',
	grade: GRADE_S14,
};

/** Rise offset (screen space, applied outside the camera so the camera cannot cancel it). */
const riseY = (f: number) => 220 * (1 - E.push(ramp(f, RISE, RISE + 20)));

/** The lifted piece of the report: buttons strip, table header, rows 1–2 (Atividade · Minutos · Tipo). */
const CROP = {x: 1452, y: 376, w: 1148, h: 506};
const CARD_K = 1.42;
const CARD_AT = {x: 1010, y: 676}; // v2 crit: −24 px so the card's bottom edge keeps a margin in the hold
const CARD: LiftCardProps = {
	src: FILE,
	rect: CROP,
	at: LIFT_AT,
	enter: 'lift',
	spring: 'snappy',
	from: (f) => {
		const r = mapImageRect(SHOT, f, COMP, CROP);
		return {...r, y: r.y + riseY(f)};
	},
	x: [
		[LIFT_AT, CARD_AT.x],
		[104, CARD_AT.x - 26],
	],
	y: CARD_AT.y,
	width: [
		[LIFT_AT, CROP.w * CARD_K],
		[104, CROP.w * CARD_K * 1.025],
	],
	rotateX: [
		[LIFT_AT, 11],
		[104, 6, E.glide],
	],
	rotateY: [
		[LIFT_AT, -12],
		[104, -6, E.glide],
	],
	radius: 22,
	float: 5,
	floatPeriod: 80,
	glow: 'volt',
	glowOpacity: 0.6,
	grade: GRADE_S14,
	style: {zIndex: 30},
};

/** Meta lines: patch (storyboard) + the capture's own words minus the model name. */
const META_DY = [0, 1262];
const MetaRecompose: React.FC = () => (
	<>
		{META_DY.map((dy) => (
			<React.Fragment key={dy}>
				{/* "Gerado 29/09 18:00" stays where it is */}
				<Crop src={FILE} rect={{x: 648, y: 434 + dy, w: 244, h: 36}} />
				{/* "3h22 registradas" slides in after it (the 26-px group gap kept); v2 review: the token counts (x 1386–1843) are left out */}
				<Crop src={FILE} rect={{x: 1160, y: 434 + dy, w: 210, h: 36}} at={{x: 912, y: 434 + dy}} />
			</React.Fragment>
		))}
	</>
);

/** Page background of the Relatórios capture. */
const PAGE_BG = '#060a14';
/**
 * v2 review: the page heading block. "Relatórios" + its subline (x 513–1441, y 81–183) ghost behind the headline,
 * and the status line "Terça-feira, 29 de setembro: 2 de 3 relatórios prontos." (x 513–1225, y 260–287) reads
 * as "2 of 3 ready" right under "O relatório sai pronto.". Both go (page colour).
 */
const PAGE_PATCHES = [
	{x: 500, y: 68, w: 960, h: 128},
	{x: 500, y: 248, w: 750, h: 52},
];
const PagePatches: React.FC = () => (
	<>
		{PAGE_PATCHES.map((r, i) => (
			<div key={i} style={{position: 'absolute', left: r.x, top: r.y, width: r.w, height: r.h, background: PAGE_BG}} />
		))}
	</>
);
/** Input fill of the report's activity fields (#121a2b); fields span x 1468–2074. */
const FIELD_BG = '#121a2b';
/** Inner top of each activity field (measured: 610, 756, 902, 1048, 1226, 1372; inner height 64). */
const FIELD_TOPS = [610, 756, 902, 1048, 1226, 1372];
/** v2 review: a real text field's overflow fade at the right edge, so "…ementas e ca" / "…000042/2(" read as overflow, not a clip. */
const FieldFades: React.FC = () => (
	<>
		{FIELD_TOPS.map((top, i) => (
			<div
				key={i}
				style={{
					position: 'absolute',
					left: 1986,
					top: top + 1,
					width: 86,
					height: 62,
					background: `linear-gradient(90deg, rgba(18,26,43,0) 0%, ${FIELD_BG} 72%)`,
				}}
			/>
		))}
	</>
);

/** The "Copiar Markdown" button, vector re-set with the app's Button sm tokens (image space, inside the card). */
const CopyButton: React.FC<{f: number}> = ({f}) => {
	const press = pressAt(f, CLICK);
	const hover = ramp(f, 54, 58, E.enter);
	const scale = (1 + 0.06 * hover) * (1 - 0.05 * press);
	const flash = ramp(f, OK, OK + 2) * (1 - ramp(f, OK + 2, OK + 10, E.enter));
	const iconOut = ramp(f, CLICK, CLICK + 3, E.exit);
	const check = ramp(f, OK, OK + 10, E.push);
	const ok = ramp(f, OK, OK + 6);
	const bg = mixHex(mixHex(APP.panel, APP.panel2, hover), color.mint, 0.12 * flash + 0.06 * ok);
	const border = mixHex(mixHex(APP.line2, color.volt, 0.8 * hover), color.mint, ok);
	const ring = mixHex(color.volt, color.mint, ok);
	const ringA = Math.max(0.55 * hover * (1 - ok), 0.6 * ok);
	return (
		<div
			style={{
				position: 'absolute',
				left: BTN.x,
				top: BTN.y,
				width: BTN.w,
				height: BTN.h,
				transform: `scale(${scale.toFixed(4)})`,
				transformOrigin: '50% 50%',
				boxSizing: 'border-box',
				borderRadius: 16,
				background: bg,
				boxShadow: `inset 0 0 0 2px ${border}, 0 0 ${(22 * ringA).toFixed(1)}px ${(4 * ringA).toFixed(1)}px ${alpha(ring, 0.6 * ringA)}, 0 8px 18px -6px rgba(0,0,0,0.5)`,
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

/** "Markdown copiado": the app's own toast string as a mint pill popping out to the LEFT of the button, on a short leader, in the card's empty header strip. */
const Copied: React.FC<{f: number; right: number; cy: number; text: string}> = ({f, right, cy, text}) => {
	if (f < OK) return null;
	const line = ramp(f, OK, OK + 8, E.push);
	const s = springAt(f, OK + 1, 'SNAPPY');
	const o = ramp(f, OK + 1, OK + 4);
	const L = 30;
	const pillH = 76;
	const bob = Math.sin(((f - OK) / 70) * Math.PI * 2) * 3 * ramp(f, OK + 10, OK + 20);
	return (
		<>
			<div style={{position: 'absolute', left: right - L * line, top: cy - 1, width: L * line, height: 2, background: alpha(color.mint, 0.8), borderRadius: 1}} />
			<div
				style={{
					position: 'absolute',
					left: right - L - 4,
					top: cy - pillH / 2 + bob,
					transform: `translateX(-100%) translateX(${((1 - s) * 16).toFixed(2)}px) scale(${lerp(0.86, 1, s).toFixed(4)})`,
					transformOrigin: '100% 50%',
					opacity: o,
					height: pillH,
					padding: '0 36px 0 28px',
					display: 'flex',
					alignItems: 'center',
					gap: 14,
					borderRadius: 999,
					// solid under-layer so the pill reads over anything, then the mint tint
					background: `linear-gradient(${alpha(color.mint, 0.2)}, ${alpha(color.mint, 0.2)}), #0d1426`,
					boxShadow: `inset 0 0 0 2px ${alpha(color.mint, 0.6)}, 0 0 28px ${alpha(color.mint, 0.3)}, 0 16px 32px -8px rgba(0,0,0,0.6)`,
					fontFamily: font.text,
					fontWeight: 600,
					fontSize: 44,
					lineHeight: 1,
					color: color.ink,
					whiteSpace: 'nowrap',
				}}
			>
				<Icon name="check" size={38} stroke={color.mint} strokeWidth={2.5} draw={ramp(f, OK + 2, OK + 10, E.push)} />
				<span>{text}</span>
			</div>
		</>
	);
};

/* ------------------------------------------------------------------------ */

const REST_IN = {x: 1750, y: 960};
const REST_OUT = {x: 1720, y: 940};
/** Cursor tip on the label, left of centre (image px). */
const TIP = {x: BTN.x + 150, y: BTN.y + 40};

const S14Relatorio: React.FC = () => {
	const scene = useScene();
	const {frame: f} = useSceneFrame();
	const fps = 30;
	const kicker = scene.copy.find((c) => c.role === 'kicker')!;
	const head = scene.copy.find((c) => c.role === 'headline')!;
	const caption = scene.copy.find((c) => c.role === 'ui-caption')!;
	if (kicker.text !== '18:00') throw new Error(`s14 kicker drift: ${kicker.text}`);
	const units = unitsOf(head.text, [
		// +2 f (v2 crit): the flying 18:00 clears the headline's first words before they stagger in
		{text: 'O relatório', at: 34},
		{text: 'sai', at: 36},
		{text: 'pronto.', at: 38, volt: head.emphasis.includes('pronto.') ? 'pronto.' : undefined},
	]);

	const partB = f >= RISE;
	const rise = riseY(f);

	// image px of the capture → comp px ON THE CARD (ignores the card's tilt: the cursor tip and callout sit on the button)
	const onCard = (fr: number, p: {x: number; y: number}) => {
		const c = liftCardPose(CARD, fr, fps);
		const k = c.k * c.s;
		return {x: c.cx + (p.x - (CROP.x + CROP.w / 2)) * k, y: c.cy + (p.y - (CROP.y + CROP.h / 2)) * k, k};
	};

	// cursor
	const tipAt = (fr: number) => onCard(fr, TIP);
	let cur = REST_IN;
	if (f > 40 && f < 56) cur = arcPoint(REST_IN, tipAt(56), E.cursor(ramp(f, 40, 56)), 0.12);
	else if (f >= 56 && f <= 66) cur = tipAt(f);
	else if (f > 66) cur = arcPoint(tipAt(66), REST_OUT, E.cursor(ramp(f, 66, 80)), 0.12);
	const press = pressAt(f, CLICK);
	const cSize = 30 * 1.4 * (1 - 0.15 * press);
	const cOpacity = ramp(f, 34, 40, E.enter) * (1 - ramp(f, 78, 84, E.exit));
	const clickPt = tipAt(CLICK);

	// callout anchor: left-middle of the button on the card; shockwave from its centre
	const btnLeft = onCard(f, {x: BTN.x, y: BTN.y + BTN.h / 2});
	const btnTop = onCard(f, {x: BTN.x + BTN.w / 2, y: BTN.y});

	const band = ramp(f, RISE, RISE + 2) * (1 - 0.6 * ramp(f, LIFT_AT, 58, E.glide));
	// denser under the headline while the window's own heading sits right behind it (until the pull-back)
	const bandTop = lerp(0.95, 0.86, ramp(f, 48, 58, E.glide));
	const poolUp = ramp(f, MORPH, MORPH_END + 6, E.glide);
	const back = ramp(f, LIFT_AT + 2, 64, E.glide);

	return (
		<TransitionOut>
			<TransitionIn>
				<Backdrop
					seed="s14"
					ember={0.05}
					look={{
						// v2: the key pool is a footlight UNDER the volt 18:00 (not on it), then rises behind the report card
						keyPool: {x: lerp(0.5, 0.54, poolUp), y: lerp(0.82, 0.62, poolUp), w: lerp(0.92, 0.82, poolUp), h: lerp(0.66, 0.92, poolUp), opacity: lerp(0.55, 0.46, poolUp)},
						keyLight: {x: lerp(0.5, 0.54, poolUp), y: lerp(0.86, 0.64, poolUp), w: lerp(0.6, 0.5, poolUp), h: lerp(0.4, 0.62, poolUp), opacity: lerp(0.22, 0.16, poolUp)},
						guard: [{...CLOCK_GUARD, opacity: GUARD_OPACITY * (1 - poolUp)}] /* footlight falloff only (GUARD RULE) */,
					}}
				>
					{partB ? (
						<AbsoluteFill
							style={{
								transform: rise > 0.05 ? `translateY(${rise.toFixed(2)}px)` : undefined,
								opacity: ramp(f, RISE, RISE + 4),
								filter: back > 0.01 ? `blur(${(2.5 * back).toFixed(2)}px)` : undefined,
							}}
						>
							<Screen {...SHOT} style={{zIndex: 'auto'}}>
								<Patches patches={storyboardPatches(scene, FILE)} />
								<MetaRecompose />
								<PagePatches />
								<FieldFades />
								{/* continuity: s13 just emptied the review queue — the sidebar's pending-count badge goes (nav colour) */}
								<div style={{position: 'absolute', left: BADGE.x - 3, top: BADGE.y - 3, width: BADGE.w + 6, height: BADGE.h + 6, background: '#0a101c'}} />
								<LiftHole rect={CROP} at={LIFT_AT} enter="lift" color={APP.panel} pad={4} feather={16} radius={20} socket={0.6} />
							</Screen>
						</AbsoluteFill>
					) : null}
					{band > 0.001 ? (
						<AbsoluteFill
							style={{
								opacity: band,
								background: `linear-gradient(180deg, ${navyDim(bandTop)} 0px, ${navyDim(bandTop)} 270px, ${navyDim(0.55)} 310px, ${navyDim(0)} 380px)`,
							}}
						/>
					) : null}
					{f >= LIFT_AT ? (
						<LiftCard {...CARD} patches={storyboardPatches(scene, FILE)}>
							{/* Regenerar's left edge peeks in at the crop's right side: card colour */}
							<div style={{position: 'absolute', left: 2582, top: CROP.y, width: 40, height: 110, background: APP.panel}} />
							{/* the card's own title, re-composed from the capture's pixels: the IFRO icon + "IFRO" and "Gerado 29/09 18:00". */}
							{/* The header strip is card colour at FULL opacity from the lift's first frame (it covers the window meta line's */}
							{/* sliced "…480 de saída" at the crop's left edge); the re-composed title then fades in on it: never two layers. */}
							<div style={{position: 'absolute', left: CROP.x, top: 380, width: 820, height: 92, background: APP.panel}} />
							<div style={{position: 'absolute', inset: 0, opacity: ramp(f, LIFT_AT, LIFT_AT + 4)}}>
								<Crop src={FILE} rect={{x: 548, y: 382, w: 200, h: 84}} at={{x: CROP.x + 30, y: 382}} />
								<Crop src={FILE} rect={{x: 648, y: 434, w: 244, h: 36}} at={{x: CROP.x + 132, y: 434}} />
							</div>
							<FieldFades />
							<CopyButton f={f} />
							<div style={{position: 'absolute', left: CROP.x, top: CROP.y, width: CROP.w, height: CROP.h, overflow: 'hidden'}}>
								<LightSweep from={70} to={100} strength={0.1} />
							</div>
						</LiftCard>
					) : null}
					<AbsoluteFill style={{zIndex: 31, pointerEvents: 'none'}}>
						<Clock f={f} />
						{f >= 32 ? <Headline units={units} size={124} left={102} capTop={150} /> : null}
						<Copied f={f} right={Math.round(btnLeft.x - 10)} cy={Math.round(btnLeft.y)} text={caption.text} />
						<Shockwave x={btnTop.x} y={btnTop.y + (BTN.h / 2) * btnTop.k} at={CLICK} radius={300} len={16} tint={color.mint} strength={0.8} />
						<ClickRipple x={clickPt.x} y={clickPt.y} at={CLICK} scale={1.4} />
						{f >= 34 ? <ArrowCursor x={cur.x} y={cur.y} size={cSize} opacity={cOpacity} /> : null}
					</AbsoluteFill>
					<Finish seed="s14" />
				</Backdrop>
			</TransitionIn>
		</TransitionOut>
	);
};

export default S14Relatorio;
