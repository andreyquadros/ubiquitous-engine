/**
 * s17-privacidade — S17 · abs 1230–1379 (150 f) · proof · bar 21.3 → 24.1
 *
 * v2. The breather and the trust proof. Z layout: line 1 "Seus dados ficam com
 * você." (104 px) across the top, UBI large on the left, line 2 "A IA vê só o
 * mínimo." (88 px) and a REDACTION PANEL on the right: a floating window with
 * four fictional samples (the exact tokens of crates/ubiqx-core/src/redact.rs).
 * Left column = your data (it stays), a volt "mask gate", right column = what
 * the AI gets. On a beat-locked cascade each value passes through the gate and
 * comes out as its token:
 *   ana@example.com        → [email]      f90  (abs 1320, downbeat)
 *   (00) 90000-0000        → [telefone]   f98  (8th; DDD 00 does not exist)
 *   123.456.789-00         → [cpf]        f105 (beat)
 *   example.com/?ref=ana   → example.com  f113 (8th; query stripped)
 *
 * f0–5 blur-dissolve in-half on the stage lights + UBI; s16's headline plate
 * finishes its dissolve over f0–4 (v2 review). Line 1 masks up from f2 through
 * a soft-edged mask (≥ 0.9 by f8). f4–30 the panel
 * rises and its rows fill in. f45 (beat 2) internal jump-cut: the stage + UBI
 * jump to 1.08 (the type/panel layer to 1.02: parallax), UBI's look clip starts.
 * f60 line 2 masks up. f76–88 the gate arms (draws down) and a volt marker
 * sweeps the address. f90 CONTACT (row 1), f98/105/113 rows 2–4. f92–104 a 2-px
 * volt line draws from the [email] token up under "IA"; "mínimo." turns volt.
 * f105–149 the build: stage/UBI push to 1.16 (E.exit), type/panel to 1.04,
 * grid floor 0 → 30 %, volt orb up, the tokens breathe on beats 120/135.
 * f150: hard cut on the final hit.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {evolvePath} from '@remotion/paths';
import {StageBase} from '../components/Stage';
import {alpha, color} from '../design/tokens';
import {E, springAt, TransitionIn, TransitionOut, UbiTrack, useSceneFrame, type SfxCue} from '../shared';
import {clamp01, FilmGrain, lerp, MaskLine, ramp, Stage, TYPE, Vignette} from './_parts/G6/common';
import {gradientFill, mixHex, voltGlow} from './_parts/G6/kit';
import {S16HeadlineGhost} from './s16-sua-ia';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'ui_tick_1.wav', atFrame: 76, gainDb: -22, note: 'Marker sweep starts (the mask gate arms).'},
	{ref: 'pop.wav', atFrame: 90, gainDb: -14, note: 'Mask contact (row 1 → [email]).'},
	{ref: 'pop+1.wav', atFrame: 98, gainDb: -17, note: 'v2 cascade: row 2 → [telefone].'},
	{ref: 'pop+2.wav', atFrame: 105, gainDb: -17, note: 'v2 cascade: row 3 → [cpf].'},
	{ref: 'pop+3.wav', atFrame: 113, gainDb: -17, note: 'v2 cascade: row 4 → bare domain.'},
];

/* ------------------------------------------------------------------------ */
/* Layout (canvas px at camera scale 1)                                      */
/* ------------------------------------------------------------------------ */

const L1 = {left: 150, capTop: 122, size: 104} as const;
const COL_X = 704;
const L2 = {left: COL_X, capTop: 290, size: 88} as const;
const L2_BASELINE = L2.capTop + (TYPE.sora.baseline - TYPE.sora.capTop) * L2.size; // ≈ 354
/** "A " and "IA" advances at Sora 700 88 px −0.04em (HarfBuzz widths at 72 px −0.03em rescaled). */
const IA = {left: COL_X + 88, w: 90};
const IA_CX = IA.left + IA.w / 2;
const UNDER_Y = Math.round(L2_BASELINE + 14);

/** Redaction panel (canvas px). */
const PANEL = {x: COL_X - 4, y: 400, w: 1080, h: 556, radius: 30} as const;
const BAR_H = 58;
const ROW = {top: 74, h: 104, gap: 16, inset: 18, radius: 20} as const;
const ICON = {x: 40, size: 60} as const;
const VALUE_X = 124;
const GATE_X = 668; // panel-local x of the mask gate
const TOKEN_X = 712;
const FS = 44; // chip text, canvas px (≥ 44)

type Row = {icon: 'mail' | 'phone' | 'id' | 'link'; value: string; token: string; flip: number};
const FLIPS = [90, 98, 105, 113];

/** UBI: 820-px frame, feet on (400, 985). */
const UBI = {x: 400, y: 985, size: 820} as const;

/* ------------------------------------------------------------------------ */
/* Timing                                                                    */
/* ------------------------------------------------------------------------ */

const L1_AT = 2;
const JUMP = 45;
const LINE2 = 60;
const SWEEP = 76;
const SWEEP_END = 88;
const DRAW = 92;
const DRAW_END = 104;
const BUILD = 105;

/**
 * Two cameras (parallax). Stage + UBI: 1.0 (drift) → hard 1.08 at f45 → 1.10
 * by f119 → 1.16 E.exit by f149 (storyboard), origin on UBI's feet so he grows
 * up and left. Type + panel: 1.0 → 1.02 at f45 → 1.025 → 1.04, origin at the
 * canvas centre: everything stays inside title-safe at the end.
 */
const stageZoom = (f: number) => {
	if (f < JUMP) return lerp(1, 1.008, f / (JUMP - 1));
	if (f <= 119) return lerp(1.08, 1.1, (f - JUMP) / (119 - JUMP));
	return lerp(1.1, 1.16, E.exit(clamp01((f - 119) / 30)));
};
const typeZoom = (f: number) => {
	if (f < JUMP) return lerp(1, 1.003, f / (JUMP - 1));
	if (f <= 119) return lerp(1.02, 1.025, (f - JUMP) / (119 - JUMP));
	return lerp(1.025, 1.04, E.exit(clamp01((f - 119) / 30)));
};

/* ------------------------------------------------------------------------ */
/* Pieces                                                                    */
/* ------------------------------------------------------------------------ */

const PANEL_BG = '#131c30';
const ROW_BG = '#19243b';
const INK = '#eef3fc';
const TOKEN_INK = '#8ab4ff';
/** v2 review: the landed token is the proof → near-white on a volt-tinted fill (brighter than the raw value). */
const TOKEN_ON = '#f3f7ff';
/** The raw value dims to this once its token has landed, so the eye travels left → right with the cascade. */
const VALUE_DIM = 0.45;

/** lucide icons (mail, phone, id-card, link), stroke 2 in a 24 box. */
const RowIcon: React.FC<{name: Row['icon']; size: number; stroke: string}> = ({name, size, stroke}) => {
	const c = {fill: 'none', stroke, strokeWidth: 1.9, strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const};
	return (
		<svg width={size} height={size} viewBox="0 0 24 24" style={{display: 'block'}}>
			{name === 'mail' ? (
				<>
					<rect {...c} x="2" y="4" width="20" height="16" rx="2" />
					<path {...c} d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7" />
				</>
			) : name === 'phone' ? (
				<path
					{...c}
					d="M22 16.92v3a2 2 0 0 1-2.18 2 19.79 19.79 0 0 1-8.63-3.07 19.5 19.5 0 0 1-6-6 19.79 19.79 0 0 1-3.07-8.67A2 2 0 0 1 4.11 2h3a2 2 0 0 1 2 1.72 12.84 12.84 0 0 0 .7 2.81 2 2 0 0 1-.45 2.11L8.09 9.91a16 16 0 0 0 6 6l1.27-1.27a2 2 0 0 1 2.11-.45 12.84 12.84 0 0 0 2.81.7A2 2 0 0 1 22 16.92z"
				/>
			) : name === 'id' ? (
				<>
					<rect {...c} x="2" y="5" width="20" height="14" rx="2" />
					<circle {...c} cx="8" cy="11" r="2" />
					<path {...c} d="M5.17 16a3 3 0 0 1 5.66 0" />
					<path {...c} d="M14 10h4" />
					<path {...c} d="M14 14h4" />
				</>
			) : (
				<>
					<path {...c} d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
					<path {...c} d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
				</>
			)}
		</svg>
	);
};

const textStyle = (weight: number, c: string): React.CSSProperties => ({
	fontFamily: '"Inter", sans-serif',
	fontWeight: weight,
	fontSize: FS,
	lineHeight: 1,
	letterSpacing: '-0.012em',
	color: c,
	whiteSpace: 'nowrap',
});

const RowView: React.FC<{f: number; row: Row; i: number}> = ({f, row, i}) => {
	const top = ROW.top + i * (ROW.h + ROW.gap);
	const cy = ROW.h / 2;
	// fill-in: rise + fade, 5 f apart from f8
	const inAt = 8 + 5 * i;
	const rin = springAt(f, inAt, 'SNAPPY');
	const ro = ramp(f, inAt, inAt + 6);
	const F = row.flip;
	// marker (row 1: f76–88 like v1; rows 2–4: 6 f before the flip)
	const m0 = i === 0 ? SWEEP + 4 : F - 7;
	const m1 = i === 0 ? SWEEP_END : F - 1;
	const sweep = ramp(f, m0, m1, E.glide);
	const markerO = f < F ? 1 : 1 - ramp(f, F, F + 3);
	// ghost: a copy of the value slides into the gate (clipped at the gate) over the last 7 f
	const g = ramp(f, F - 7, F, E.push);
	const ghostO = f < F - 7 || f > F + 1 ? 0 : Math.sin(Math.PI * clamp01((f - (F - 7)) / 9)) * 0.75;
	// token: pops out of the gate on the flip (SNAPPY), blur 6 → 0 by F + 2
	// v2 review: readable ON the pop's frame (opacity 0.8, blur ≤ 1.6 px at F), settled by F + 2
	const tokIn = f < F ? 0 : springAt(f, F, 'SNAPPY');
	const tokO = f < F ? 0 : lerp(0.8, 1, ramp(f, F, F + 2));
	const tokSharp = f < F ? 0 : lerp(0.73, 1, ramp(f, F, F + 2, E.enter));
	const flash = f >= F ? 1 - ramp(f, F, F + 16, E.glide) : 0;
	const done = ramp(f, F, F + 5, E.enter);
	const valueO = f < F ? 1 : lerp(1, VALUE_DIM, ramp(f, F, F + 6, E.glide));
	// beat breath on the landed tokens (the build)
	const breath = f >= 118 ? 0.5 + 0.5 * Math.cos((2 * Math.PI * (f - 120)) / 15) : 0;
	const breathA = f >= 118 ? ramp(f, 118, 124) * breath : 0;
	const markW = Math.min(520, 26 * row.value.length) + 20;
	return (
		<div
			style={{
				position: 'absolute',
				left: ROW.inset,
				top,
				width: PANEL.w - 2 * ROW.inset,
				height: ROW.h,
				borderRadius: ROW.radius,
				background: ROW_BG,
				boxShadow: `inset 0 1px 0 rgba(255,255,255,0.05), inset 0 0 0 1px rgba(140,170,230,${(0.1 + 0.25 * flash).toFixed(3)})`,
				opacity: ro,
				transform: rin < 0.999 ? `translateY(${(22 * (1 - rin)).toFixed(2)}px)` : undefined,
			}}
		>
			{/* icon tile */}
			<div
				style={{
					position: 'absolute',
					left: ICON.x - ROW.inset,
					top: cy - ICON.size / 2,
					width: ICON.size,
					height: ICON.size,
					borderRadius: 16,
					background: '#22304c',
					display: 'flex',
					alignItems: 'center',
					justifyContent: 'center',
				}}
			>
				<RowIcon name={row.icon} size={32} stroke={mixHex('#aebbd6', TOKEN_INK, done)} />
			</div>
			{/* marker behind the value */}
			{f >= m0 && markerO > 0.01 ? (
				<div
					style={{
						position: 'absolute',
						left: VALUE_X - ROW.inset - 10,
						top: cy - 32,
						width: markW * sweep,
						height: 64,
						borderRadius: 12,
						background: alpha(color.volt, 0.26),
						opacity: markerO,
					}}
				/>
			) : null}
			{/* the value: it stays (your data) */}
			<div style={{position: 'absolute', left: VALUE_X - ROW.inset, top: cy - FS / 2, height: FS, display: 'flex', alignItems: 'center', ...textStyle(500, INK), opacity: valueO}}>{row.value}</div>
			{/* ghost copy sliding into the gate, clipped at it */}
			{ghostO > 0.01 ? (
				<div style={{position: 'absolute', left: 0, top: 0, width: GATE_X - ROW.inset, height: ROW.h, overflow: 'hidden'}}>
					<div
						style={{
							position: 'absolute',
							left: VALUE_X - ROW.inset + lerp(0, GATE_X - VALUE_X + 40, g),
							top: cy - FS / 2,
							height: FS,
							display: 'flex',
							alignItems: 'center',
							...textStyle(500, TOKEN_INK),
							opacity: ghostO,
							filter: `blur(${(1 + 3 * g).toFixed(2)}px)`,
						}}
					>
						{row.value}
					</div>
				</div>
			) : null}
			{/* right cell: empty slot → token. The slot is sized by the (invisible) token itself, with the token
			    chip's own padding, so each dashed slot is exactly the chip that lands in it (critique r1: the
			    fixed 330 px slots left the column half-empty around short tokens like [cpf]). */}
			<div
				style={{
					position: 'absolute',
					left: TOKEN_X - ROW.inset,
					top: cy - 36,
					height: 72,
					display: 'flex',
					alignItems: 'center',
					padding: '0 20.5px', // + the 1.5 px border = the chip's 22 px padding
					borderRadius: 16,
					boxSizing: 'border-box',
					border: `1.5px dashed rgba(140,170,230,${(0.22 * (1 - done)).toFixed(3)})`,
					...textStyle(600, 'transparent'),
				}}
			>
				{row.token}
			</div>
			{tokO > 0 ? (
				<div
					style={{
						position: 'absolute',
						left: TOKEN_X - ROW.inset,
						top: cy - 36,
						height: 72,
						display: 'flex',
						alignItems: 'center',
						padding: '0 22px',
						borderRadius: 16,
						boxSizing: 'border-box',
						background: `linear-gradient(${alpha(color.volt, 0.3 + 0.12 * flash + 0.06 * breathA)}, ${alpha(color.volt, 0.3 + 0.12 * flash + 0.06 * breathA)}), #0f1a33`,
						boxShadow: `inset 0 0 0 1.5px ${alpha(color.volt, 0.6 + 0.3 * flash)}, 0 0 ${(12 + 30 * flash + 10 * breathA).toFixed(1)}px ${alpha(color.volt, 0.26 + 0.4 * flash + 0.12 * breathA)}`,
						opacity: tokO,
						filter: tokSharp < 1 ? `blur(${(6 * (1 - tokSharp)).toFixed(2)}px)` : undefined,
						transformOrigin: '0% 50%',
						transform: tokIn < 0.999 ? `translateX(${(-24 * (1 - tokIn)).toFixed(2)}px) scale(${lerp(0.9, 1, tokIn).toFixed(4)})` : undefined,
						...textStyle(600, TOKEN_ON),
					}}
				>
					{row.token}
				</div>
			) : null}
		</div>
	);
};

/** The mask gate: a volt hairline down the rows with a chevron per row; arms f76–88, flashes per flip. */
const Gate: React.FC<{f: number}> = ({f}) => {
	const top = ROW.top - 6;
	const bottom = ROW.top + 4 * ROW.h + 3 * ROW.gap + 6;
	const arm = ramp(f, SWEEP, SWEEP_END, E.glide);
	const idle = ramp(f, 14, 30) * 0.28;
	return (
		<>
			<div
				style={{
					position: 'absolute',
					left: GATE_X - 1.5,
					top,
					width: 3,
					height: bottom - top,
					borderRadius: 2,
					background: `rgba(140,170,230,${idle.toFixed(3)})`,
				}}
			/>
			{arm > 0 ? (
				<div
					style={{
						position: 'absolute',
						left: GATE_X - 1.5,
						top,
						width: 3,
						height: (bottom - top) * arm,
						borderRadius: 2,
						background: color.volt,
						boxShadow: `0 0 14px ${alpha(color.volt, 0.7)}, 0 0 36px ${alpha(color.volt, 0.35)}`,
					}}
				/>
			) : null}
			{FLIPS.map((F, i) => {
				const cy = ROW.top + i * (ROW.h + ROW.gap) + ROW.h / 2;
				const flash = f >= F - 1 ? 1 - ramp(f, F - 1, F + 12, E.glide) : 0;
				const lit = Math.max(idle * 1.6, arm >= 1 ? 0.9 : 0, flash);
				return (
					<React.Fragment key={i}>
						<svg width={26} height={26} viewBox="0 0 24 24" style={{position: 'absolute', left: GATE_X + 8, top: cy - 13, overflow: 'visible'}}>
							<path d="m9 6 6 6-6 6" fill="none" stroke={f >= F ? TOKEN_INK : color.volt} strokeOpacity={lit} strokeWidth={2.6} strokeLinecap="round" strokeLinejoin="round" />
						</svg>
						{flash > 0.01 ? (
							<div
								style={{
									position: 'absolute',
									left: GATE_X - 60,
									top: cy - 60,
									width: 120,
									height: 120,
									borderRadius: '50%',
									background: `radial-gradient(closest-side, ${alpha(color.volt, 0.55 * flash)} 0%, ${alpha(color.volt, 0.18 * flash)} 50%, transparent 100%)`,
								}}
							/>
						) : null}
					</React.Fragment>
				);
			})}
		</>
	);
};

const Panel: React.FC<{f: number; rows: Row[]}> = ({f, rows}) => {
	const rise = springAt(f, 4, 'SMOOTH');
	const o = ramp(f, 4, 14);
	const float = Math.sin((2 * Math.PI * f) / 110) * 4;
	const ry = lerp(-7, -4, clamp01(f / 149));
	const rx = lerp(4, 2.5, clamp01(f / 149));
	return (
		<div style={{position: 'absolute', left: 0, top: 0, width: 1920, height: 1080, perspective: 2200, perspectiveOrigin: `${PANEL.x + PANEL.w / 2}px ${PANEL.y + PANEL.h / 2}px`}}>
			<div
				style={{
					position: 'absolute',
					left: PANEL.x,
					top: PANEL.y + 40 * (1 - rise) + float,
					width: PANEL.w,
					height: PANEL.h,
					opacity: o,
					transform: `rotateX(${rx.toFixed(3)}deg) rotateY(${ry.toFixed(3)}deg)`,
				}}
			>
				{/* ambient volt glow under the panel */}
				<div style={{position: 'absolute', left: '-18%', right: '-18%', top: '-22%', bottom: '-30%', background: `radial-gradient(closest-side, ${alpha(color.volt, 0.3)} 0%, ${alpha(color.volt, 0.1)} 55%, transparent 100%)`}} />
				{/* rim + shadow */}
				<div
					style={{
						position: 'absolute',
						inset: -1.5,
						borderRadius: PANEL.radius + 1.5,
						background: `linear-gradient(135deg, ${alpha(color.volt, 0.75)} 0%, rgba(77,141,255,0.18) 35%, rgba(77,141,255,0) 60%)`,
						boxShadow: '0 22px 44px -10px rgba(0,0,0,0.6), 0 60px 120px -30px rgba(0,0,0,0.7)',
					}}
				/>
				<div style={{position: 'absolute', inset: 0, borderRadius: PANEL.radius, background: PANEL_BG, overflow: 'hidden', boxShadow: 'inset 0 1px 0 rgba(255,255,255,0.10)'}}>
					{/* title bar: neutral dots (platform-agnostic window) */}
					<div style={{position: 'absolute', left: 0, right: 0, top: 0, height: BAR_H, borderBottom: '1px solid rgba(140,170,230,0.10)', background: 'rgba(255,255,255,0.02)'}}>
						{[0, 1, 2].map((d) => (
							<div key={d} style={{position: 'absolute', left: 28 + d * 26, top: BAR_H / 2 - 7, width: 14, height: 14, borderRadius: 7, background: '#3a4560'}} />
						))}
					</div>
					{rows.map((r, i) => (
						<RowView key={i} f={f} row={r} i={i} />
					))}
					<Gate f={f} />
				</div>
			</div>
		</div>
	);
};

const Connector: React.FC<{f: number}> = ({f}) => {
	if (f < DRAW) return null;
	// from the [email] token's top (panel row 1, right column) up under "IA"
	const sx = PANEL.x + TOKEN_X + 70;
	const sy = PANEL.y + ROW.top + 14;
	const path = `M ${sx} ${sy} C ${sx} ${sy - 40} ${IA_CX} ${UNDER_Y + 44} ${IA_CX} ${UNDER_Y}`;
	const p = ramp(f, DRAW, DRAW + 9, E.glide);
	const u = ramp(f, DRAW + 8, DRAW_END, E.push);
	const dot = springAt(f, DRAW, 'SNAPPY');
	const ev = evolvePath(p, path);
	const half = (IA.w / 2 + 6) * u;
	return (
		<svg width={1920} height={1080} style={{position: 'absolute', left: 0, top: 0, overflow: 'visible'}}>
			<path d={path} fill="none" stroke={color.volt} strokeWidth={3} strokeLinecap="round" strokeDasharray={ev.strokeDasharray} strokeDashoffset={ev.strokeDashoffset} style={{filter: `drop-shadow(0 0 6px ${alpha(color.volt, 0.7)})`}} />
			<circle cx={sx} cy={sy} r={5 * Math.min(1.1, dot)} fill={color.volt} />
			{u > 0 ? <line x1={IA_CX - half} y1={UNDER_Y} x2={IA_CX + half} y2={UNDER_Y} stroke={color.volt} strokeWidth={4} strokeLinecap="round" /> : null}
		</svg>
	);
};

/* ------------------------------------------------------------------------ */
/* Scene                                                                     */
/* ------------------------------------------------------------------------ */

const S17Privacidade: React.FC = () => {
	const {frame: f, scene} = useSceneFrame();
	const [line1, line2, address, token] = scene.copy.map((c) => c.text);
	const emph = scene.copy[1].emphasis[0] ?? 'mínimo.';
	const cut = line2.lastIndexOf(emph);
	const line2a = cut >= 0 ? line2.slice(0, cut) : line2;
	const line2b = cut >= 0 ? line2.slice(cut) : '';

	// fictional samples; the address and its token are the storyboard's copy, the rest follow redact.rs exactly
	const rows: Row[] = [
		{icon: 'mail', value: address, token, flip: FLIPS[0]},
		// v2 review: DDD 00 does not exist (provably fictional); still matches redact.rs's PHONE regex → [telefone]
		{icon: 'phone', value: '(00) 90000-0000', token: '[telefone]', flip: FLIPS[1]},
		{icon: 'id', value: '123.456.789-00', token: '[cpf]', flip: FLIPS[2]},
		{icon: 'link', value: 'example.com/?ref=ana', token: 'example.com', flip: FLIPS[3]},
	];

	const zs = stageZoom(f);
	const zt = typeZoom(f);
	const build = ramp(f, BUILD, 149, E.enter);
	const orbVolt = lerp(0.2, 0.3, build);
	const floor = 0.3 * ramp(f, BUILD, 149, E.glide);
	const volt = ramp(f, DRAW + 6, DRAW_END + 2, E.enter); // "mínimo." turns volt as the line lands
	const ghostK = 1 - clamp01(f / 5); // 1 on the cut frame → 0 at f5

	return (
		<TransitionOut>
			<AbsoluteFill style={{backgroundColor: '#0a0f20'}}>
				<StageBase />
				{/* stage + UBI camera (storyboard push) */}
				<AbsoluteFill style={{transform: `scale(${zs.toFixed(5)})`, transformOrigin: `${UBI.x}px ${UBI.y}px`}}>
					{/* storyboard in-half (blur-dissolve 6 f) on the stage lights + UBI only */}
					<TransitionIn>
						<Stage
							seed="s17"
							base="transparent"
							orbs={[
								// the volt bloom sits behind UBI (the lit subject), off the volt "mínimo." (GUARD RULE)
								{c: color.volt, x: 420, y: 560, d: 1100, opacity: orbVolt},
								{c: color.ember, x: 260, y: 1000, d: 760, opacity: 0.1},
							]}
							floor={floor}
							lineAlpha={0.5}
							horizon={0.72}
							gridSpeed={0.6}
							look={{
								keyPool: {x: 0.5, y: 0.7, w: 0.95, h: 0.9, opacity: 0.4},
								keyLight: {x: 0.21, y: 0.6, w: 0.34, h: 0.6, opacity: 0.17},
							}}
						/>
						<UbiTrack segments={scene.ubiTrack} x={UBI.x} y={UBI.y} size={UBI.size} float={8} floatPeriod={126} floorGlow={0.34} shadow={0.4} />
					</TransitionIn>
				</AbsoluteFill>

				{/* type + panel camera (parallax: moves less than the stage) */}
				<AbsoluteFill style={{transform: `scale(${zt.toFixed(5)})`, transformOrigin: '960px 540px'}}>
					{/* v2 review: line 1 rises from f2 (≥ 0.9 by f8, as the storyboard) through a soft-edged mask, once the dissolve has mostly settled */}
					<MaskLine frame={f} at={L1_AT} duration={16} feather capTop={L1.capTop} left={L1.left} family="sora" size={L1.size} weight={700} tracking="-0.04em" color={color.ink} ariaLabel={line1}>
						<span style={gradientFill(0)}>{line1}</span>
					</MaskLine>
					<Panel f={f} rows={rows} />
					<MaskLine frame={f} at={LINE2} capTop={L2.capTop} left={L2.left} family="sora" size={L2.size} weight={700} tracking="-0.04em" color={color.ink} ariaLabel={line2}>
						<span style={gradientFill(0)}>{line2a}</span>
						<span style={{...gradientFill(volt), filter: volt > 0.01 ? voltGlow(volt) : undefined}}>{line2b}</span>
					</MaskLine>
					<Connector f={f} />
				</AbsoluteFill>

				{/* v2 review: the s16 side of the blur-dissolve continues here — s16's headline plate (0.4, 12 px on s16's
				    last frame) finishes dissolving over f0–4 (screen space, the cut is not a camera move) */}
				{f < 5 ? <S16HeadlineGhost opacity={0.4 * Math.pow(ghostK, 1.4)} blur={12 + 8 * (1 - ghostK)} scale={1 + 0.015 * ghostK} /> : null}

				<Vignette strength={0.5} />
				<FilmGrain opacity={0.045} seed="s17" />
			</AbsoluteFill>
		</TransitionOut>
	);
};

export default S17Privacidade;
