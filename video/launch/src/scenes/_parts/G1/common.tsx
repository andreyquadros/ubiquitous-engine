/**
 * G1 (s01–s04) shared parts: the problem-act backdrop, the vector "ANTES"
 * timesheet plane (+ a projector that replicates its CSS 3D maths so screen-
 * space overlays can be glued to it), the volt caret, type metrics and the
 * slam shake. Scene-local — not shared code.
 */
import React from 'react';
import {AbsoluteFill, interpolate, random, useCurrentFrame, useVideoConfig} from 'remotion';
import {noise2D} from '@remotion/noise';
import {StageBase, StageFinish, StageGuards, StageLights, STAGE, type StageLook} from '../../../components/Stage';
import {alpha, color, font} from '../../../design/tokens';

export const W = 1920;
export const H = 1080;
export const CLAMP = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

/* ------------------------------------------------------------------------ */
/* Type metrics (hhea of the shipped woff2: Sora 970/−290/cap 730, Inter     */
/* 1984/−494/cap 1490 @ 2048). With line-height = font-size the cap top sits */
/* `k × size` below the line box top.                                        */
/* ------------------------------------------------------------------------ */

export const CAP_OFFSET = {display: 0.11, text: 0.1358} as const;
export const BASELINE = {display: 0.84, text: 0.8633} as const;

/** Line-box top (line-height 1) that puts the cap top of `size`-px type at `capTop`. */
export const boxTopForCap = (role: 'display' | 'text', size: number, capTop: number) => capTop - CAP_OFFSET[role] * size;

/* ------------------------------------------------------------------------ */
/* Slam shake: noise2D × amp × exp(−t/3) for 8 f, ±0.4° (style S02).         */
/* ------------------------------------------------------------------------ */

export const slamShake = (frame: number, contact: number, seed: string, amp = 6, rotAmp = 0.4, len = 8) => {
	const t = frame - contact;
	if (t < 0 || t >= len) return {x: 0, y: 0, r: 0};
	const d = Math.exp(-t / 3);
	return {
		x: noise2D(`${seed}-sx`, t * 0.9, 0) * amp * d,
		y: noise2D(`${seed}-sy`, t * 0.9, 3.3) * amp * d,
		r: noise2D(`${seed}-sr`, t * 0.9, 7.1) * rotAmp * d,
	};
};

export const shakeTransform = (s: {x: number; y: number; r: number}) =>
	s.x === 0 && s.y === 0 && s.r === 0 ? undefined : `translate(${s.x.toFixed(2)}px, ${s.y.toFixed(2)}px) rotate(${s.r.toFixed(3)}deg)`;

/* ------------------------------------------------------------------------ */
/* Backdrop: the v2 stage (navy gradient + key/indigo/ember pools + aurora + */
/* key light, components/Stage.tsx) + the problem act's drifting orbs +      */
/* vignette (≤ 0.35) + grain.                                                */
/* ------------------------------------------------------------------------ */

export type Orb = {c: string; x: number; y: number; size: number; opacity: number};

export const G1_ORBS: Orb[] = [
	// rose, bottom-left (problem palette)
	{c: color.rose, x: 0.12, y: 0.94, size: 0.62, opacity: 0.06},
	// faint volt, top-centre
	{c: color.volt, x: 0.5, y: 0.04, size: 0.72, opacity: 0.08},
];

/**
 * G1 stage rig: the key pool sits behind the headline + timesheet (upper
 * centre), the problem act swaps the ember pool for a rose one.
 */
export const G1_LOOK: StageLook = {
	// round 1: a real pool (was w 1.3 × h 1.45, a frame-wide wash)
	keyPool: {x: 0.5, y: 0.44, w: 0.84, h: 0.94, opacity: 0.4},
	pools: [
		{color: STAGE.indigo, x: 0.86, y: 0.14, w: 0.8, opacity: 0.18},
		{color: color.rose, x: 0.1, y: 0.94, w: 0.7, opacity: 0.15},
	],
	keyLight: {x: 0.5, y: 0.46, w: 0.52, h: 0.62, opacity: 0.14},
};

export const Backdrop: React.FC<{
	orbs?: Orb[];
	seed: string;
	grain?: number;
	vignette?: number;
	/**
	 * Stage rig overrides (merged over G1_LOOK). Lights out = `look={{level: 0}}`
	 * (s04, the designed silence: navy base + vignette + grain only).
	 */
	look?: StageLook;
	children?: React.ReactNode;
}> = ({orbs = G1_ORBS, seed, grain = 0.045, vignette = 0.6, look, children}) => {
	const frame = useCurrentFrame();
	const t = frame * 0.004;
	const rig: StageLook = {...G1_LOOK, ...look};
	return (
		<AbsoluteFill style={{backgroundColor: STAGE.bottom, overflow: 'hidden'}}>
			<StageBase />
			<StageLights seed={seed} {...rig} />
			{orbs.map((o, i) => {
				const d = o.size * W;
				// ≤ 0.3 px/f drift, ±3 % breathing
				const nx = noise2D(`${seed}-ox-${i}`, t, i * 3.1) * 0.03 * W;
				const ny = noise2D(`${seed}-oy-${i}`, t, i * 7.7) * 0.02 * W;
				const b = 1 + noise2D(`${seed}-ob-${i}`, t * 0.7, i) * 0.03;
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: o.x * W + nx - (d * b) / 2,
							top: o.y * H + ny - (d * b) / 2,
							width: d * b,
							height: d * b,
							borderRadius: '50%',
							background: `radial-gradient(closest-side, ${alpha(o.c, o.opacity)} 0%, ${alpha(o.c, o.opacity * 0.5)} 35%, ${alpha(o.c, o.opacity * 0.14)} 65%, ${alpha(o.c, 0)} 100%)`,
						}}
					/>
				);
			})}
			<StageGuards guard={rig.guard} level={rig.level} keyPool={rig.keyPool} keyLight={rig.keyLight} />
			{children}
			<StageFinish vignette={vignette} grain={grain} seed={seed} />
		</AbsoluteFill>
	);
};

/* ------------------------------------------------------------------------ */
/* The volt caret                                                            */
/* ------------------------------------------------------------------------ */

export const CARET_W = 4;
export const CARET_H = 56;
/** Where the caret (and s05's X) lives in the drop-out: screen centre of the caret. */
export const CARET_SPOT = {x: 960, y: 520} as const;

/** 8 on / 8 off from `phase` (scene frame where an "on" run starts). */
export const blinkOn = (frame: number, phase = 0) => (((frame - phase) % 16) + 16) % 16 < 8;

export const caretStyle = (w = CARET_W, h = CARET_H): React.CSSProperties => ({
	width: w,
	height: h,
	borderRadius: 2,
	background: color.volt,
	boxShadow: `0 0 14px ${alpha(color.volt, 0.55)}, 0 0 2px ${alpha(color.volt, 0.9)}`,
});

/** Screen-space caret centred on (x, y). */
export const Caret: React.FC<{x: number; y: number; on: boolean; w?: number; h?: number}> = ({x, y, on, w = CARET_W, h = CARET_H}) =>
	on ? <div style={{position: 'absolute', left: x - w / 2, top: y - h / 2, ...caretStyle(w, h)}} /> : null;

/* ------------------------------------------------------------------------ */
/* Timesheet geometry                                                        */
/* ------------------------------------------------------------------------ */

export const TS = {
	w: 1440,
	header: 60,
	row: 64,
	rows: 9,
	gutter: 128,
	get h() {
		return this.header + this.row * this.rows; // 636
	},
	get col() {
		return (this.w - this.gutter) / 5; // 262.4
	},
};

export const DAYS = ['SEG', 'TER', 'QUA', 'QUI', 'SEX'] as const;
export const HOURS = ['09h', '10h', '11h', '12h', '13h', '14h', '15h', '16h', '17h'] as const;
export const TER = 1;

/** Cell centre in plane-local px (origin = plane top-left). */
export const cellCenter = (col: number, row: number) => ({
	x: TS.gutter + (col + 0.5) * TS.col,
	y: TS.header + (row + 0.5) * TS.row,
});

export type PlanePose = {
	/** Plane centre on the canvas (before perspective). */
	cx: number;
	cy: number;
	rx: number; // deg
	rz: number; // deg
	s: number;
};

export const PERSPECTIVE = 1800;

/**
 * Project a plane-local point through `perspective(P) [origin 960 540]` and the
 * plane transform `rotateX(rx) rotateZ(rz) scale(s)` (origin: plane centre) —
 * exactly what <TimesheetPlane> renders.
 */
export const project = (pose: PlanePose, px: number, py: number, P = PERSPECTIVE) => {
	const x0 = (px - TS.w / 2) * pose.s;
	const y0 = (py - TS.h / 2) * pose.s;
	const rz = (pose.rz * Math.PI) / 180;
	const x1 = x0 * Math.cos(rz) - y0 * Math.sin(rz);
	const y1 = x0 * Math.sin(rz) + y0 * Math.cos(rz);
	const rx = (pose.rx * Math.PI) / 180;
	const y2 = y1 * Math.cos(rx);
	const z2 = y1 * Math.sin(rx);
	const k = P / (P - z2);
	return {x: W / 2 + (pose.cx + x1 - W / 2) * k, y: H / 2 + (pose.cy + y2 - H / 2) * k, k};
};

/** Camera: focus F (canvas px) goes to anchor A, scaled by zoom around it. */
export type Cam2D = {fx: number; fy: number; ax: number; ay: number; zoom: number};
export const camTransform = (c: Cam2D) => `translate(${c.ax}px, ${c.ay}px) scale(${c.zoom}) translate(${-c.fx}px, ${-c.fy}px)`;
export const camApply = (c: Cam2D, p: {x: number; y: number}) => ({x: c.ax + (p.x - c.fx) * c.zoom, y: c.ay + (p.y - c.fy) * c.zoom});

/* ------------------------------------------------------------------------ */
/* <TimesheetPlane>                                                          */
/* ------------------------------------------------------------------------ */

const LINE = 1.25;
/**
 * v2 review fix: the ANTES sheet is a material object, not a flat mid-grey
 * slab. Deep slate cells (#232b3a → #1b2230), brighter 1 px rules, a lit
 * header band, a glass sheen + specular band, a gradient rim highlight and a
 * real drop shadow onto the stage; a few cells hold ghosted, half-erased
 * pencil entries and eraser smudges so "empty" reads as "forgotten". Stays
 * desaturated (ANTES) against the lit volt product world from s05 on.
 */
const SHEET = {
	top: '#252e3e',
	bottom: '#1b2230',
	header: '#4a5672',
	headerBottom: '#353f55',
	headerRule: '#8d9bbb',
	line: 'rgba(146, 162, 198, 0.62)',
	rowLine: 'rgba(146, 162, 198, 0.36)',
	zebra: 'rgba(255, 255, 255, 0.018)',
	graphite: '190, 202, 228',
};

/**
 * Ghosted pencil entries: (col, row, length 0–1 of the cell, erased 0–1).
 * Never in TER (the forgotten day stays blank) and never under s03's "?"
 * cells (SEG/15h, QUI/09h, TER/*).
 */
const GHOSTS: {col: number; row: number; len: number; erased: number}[] = [
	{col: 0, row: 0, len: 0.72, erased: 0.2},
	{col: 0, row: 2, len: 0.46, erased: 0.55},
	{col: 2, row: 1, len: 0.64, erased: 0.35},
	{col: 2, row: 4, len: 0.52, erased: 0.7},
	{col: 3, row: 3, len: 0.7, erased: 0.25},
	{col: 4, row: 0, len: 0.58, erased: 0.45},
	{col: 4, row: 5, len: 0.66, erased: 0.3},
	{col: 0, row: 7, len: 0.5, erased: 0.6},
	{col: 3, row: 7, len: 0.62, erased: 0.4},
	{col: 2, row: 7, len: 0.4, erased: 0.75},
];
/** Eraser smudges (cell-relative): some over the ghosts, some alone. */
const SMUDGES: {col: number; row: number; dx: number; w: number; rot: number; o: number}[] = [
	{col: 0, row: 2, dx: 0.12, w: 0.8, rot: -6, o: 0.09},
	{col: 2, row: 4, dx: 0.05, w: 0.9, rot: 4, o: 0.1},
	{col: 2, row: 6, dx: -0.05, w: 0.75, rot: -3, o: 0.08},
	{col: 4, row: 2, dx: 0.08, w: 0.7, rot: 7, o: 0.08},
	{col: 4, row: 0, dx: 0.2, w: 0.55, rot: -8, o: 0.07},
	{col: 3, row: 5, dx: -0.1, w: 0.6, rot: 5, o: 0.07},
];

/** Illegible cursive scribble (slanted e/l loops, 1–2 pen lifts) across `len` of a cell: pure texture, no letters or digits. */
const scribble = (seed: string, x0: number, y0: number, len: number) => {
	const loops = Math.max(4, Math.round(len * 11));
	const a = (len * TS.col * 0.8) / (loops * Math.PI * 2);
	const b = a * 2.3; // b > a: the stroke runs back on itself = a loop
	const breaks = new Set<number>();
	const nb = 1 + Math.floor(random(`${seed}-nb`) * 2);
	for (let i = 0; i < nb; i++) breaks.add(2 + Math.floor(random(`${seed}-b${i}`) * (loops - 3)));
	const N = 16;
	let d = '';
	let pen = false;
	let drift = 0;
	for (let k = 0; k <= loops * N; k++) {
		const t = (k / N) * Math.PI * 2;
		const loop = Math.min(loops - 1, Math.floor(k / N));
		if (breaks.has(loop) && k % N < N * 0.55) {
			pen = false;
			continue;
		}
		const r = random(`${seed}-h${loop}`);
		const h = r > 0.72 ? 24 + r * 8 : 7 + r * 9; // mostly x-height loops, some ascenders
		drift += (random(`${seed}-d${k}`) - 0.5) * 0.25;
		const up = (h * (1 - Math.cos(t))) / 2; // 0 on the baseline → h at the top of the loop
		const y = y0 + 8 - up + drift;
		const x = x0 + a * t - b * Math.sin(t) + up * 0.32; // italic slant
		d += `${pen ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)} `;
		pen = true;
	}
	return d;
};

export type Mark = {col: number; row: number; p: number /* 0→1 spring (may overshoot) */; o: number /* opacity */};

export const TimesheetPlane: React.FC<{
	pose: PlanePose;
	/** 0 → 1: the four non-TER columns step back to 55 %. */
	dim?: number;
	/** 0 → 1: the TER column lights up volt (wash + rim + glow + lit header chip). */
	glow?: number;
	/** In-plane caret (TER/10h). */
	caretOn?: boolean;
	/** Rose "?" guesses. */
	marks?: Mark[];
	/** Light sweep across the sheet: 0 → 1 = left → right (undefined = none). */
	sweep?: number;
	P?: number;
}> = ({pose, dim = 0, glow = 0, caretOn = false, marks = [], sweep, P = PERSPECTIVE}) => {
	const colOpacity = (i: number) => (i === TER ? 1 : 1 - 0.45 * dim);
	const caret = cellCenter(TER, 1);
	const terLeft = TS.gutter + TER * TS.col;
	return (
		<AbsoluteFill style={{perspective: P, perspectiveOrigin: `${W / 2}px ${H / 2}px`}}>
			<div
				style={{
					position: 'absolute',
					left: pose.cx - TS.w / 2,
					top: pose.cy - TS.h / 2,
					width: TS.w,
					height: TS.h,
					transformOrigin: '50% 50%',
					transform: `rotateX(${pose.rx}deg) rotateZ(${pose.rz}deg) scale(${pose.s})`,
				}}
			>
				{/* drop shadow onto the stage + a faint volt bounce (outside the ANTES filter) */}
				<div
					style={{
						position: 'absolute',
						left: 30,
						right: 30,
						top: 40,
						bottom: -30,
						borderRadius: 40,
						background: 'rgba(2, 4, 12, 0.7)',
						filter: 'blur(46px)',
					}}
				/>
				<div
					style={{
						position: 'absolute',
						inset: 0,
						borderRadius: 24,
						boxShadow: `0 40px 90px -18px rgba(2, 4, 12, 0.9), 0 10px 26px -6px rgba(2, 4, 12, 0.75), 0 0 110px -10px ${alpha(color.volt, 0.14)}`,
					}}
				/>
				{/* ANTES treatment: desaturated; the caret, the volt column and the guesses stay out of it */}
				<div style={{position: 'absolute', inset: 0, filter: 'saturate(0.6)'}}>
					{/* card: deep slate */}
					<div
						style={{
							position: 'absolute',
							inset: 0,
							borderRadius: 24,
							overflow: 'hidden',
							background: `linear-gradient(180deg, ${SHEET.top} 0%, ${SHEET.bottom} 100%)`,
						}}
					>
						{/* the key light pooled on the story (TER / 10h), falling off to deep slate at the edges: a lit object, not a flat slab */}
						<div
							style={{
								position: 'absolute',
								left: cellCenter(TER, 1).x - 620,
								top: cellCenter(TER, 1).y - 430,
								width: 1240,
								height: 860,
								background: 'radial-gradient(closest-side, rgba(207,224,255,0.15), rgba(207,224,255,0.07) 45%, rgba(207,224,255,0.02) 75%, rgba(207,224,255,0) 100%)',
							}}
						/>
						{/* zebra rows */}
						{Array.from({length: TS.rows}, (_, j) =>
							j % 2 === 1 ? (
								<div key={j} style={{position: 'absolute', left: 0, right: 0, top: TS.header + j * TS.row, height: TS.row, background: SHEET.zebra}} />
							) : null,
						)}
					</div>
					{/* header band */}
					<div
						style={{
							position: 'absolute',
							left: 0,
							top: 0,
							width: TS.w,
							height: TS.header,
							borderRadius: '24px 24px 0 0',
							background: `linear-gradient(180deg, ${SHEET.header}, ${SHEET.headerBottom})`,
							boxShadow: `inset 0 -${LINE}px 0 ${SHEET.headerRule}, inset 0 1px 0 rgba(255,255,255,0.12)`,
						}}
					/>
					{/* ghosted, half-erased pencil entries + eraser smudges: "empty" reads as "forgotten" */}
					<svg width={TS.w} height={TS.h} style={{position: 'absolute', left: 0, top: 0, overflow: 'visible'}}>
						<defs>
							<filter id="g1-graphite" x="-5%" y="-40%" width="110%" height="180%">
								<feGaussianBlur stdDeviation="0.6" />
							</filter>
							<filter id="g1-smudge" x="-30%" y="-80%" width="160%" height="260%">
								<feGaussianBlur stdDeviation="9" />
							</filter>
							{GHOSTS.map((g, i) => {
								const c = cellCenter(g.col, g.row);
								const x0 = c.x - TS.col * 0.4;
								const x1 = x0 + g.len * TS.col * 0.82;
								return (
									<linearGradient key={i} id={`g1-erase-${i}`} gradientUnits="userSpaceOnUse" x1={x0} y1={0} x2={x1} y2={0}>
										<stop offset="0" stopColor={`rgb(${SHEET.graphite})`} stopOpacity={0.2} />
										<stop offset={Math.max(0.05, 1 - g.erased).toFixed(2)} stopColor={`rgb(${SHEET.graphite})`} stopOpacity={0.16} />
										<stop offset="1" stopColor={`rgb(${SHEET.graphite})`} stopOpacity={0.03} />
									</linearGradient>
								);
							})}
						</defs>
						{SMUDGES.map((m, i) => {
							const c = cellCenter(m.col, m.row);
							return (
								<ellipse
									key={`s${i}`}
									cx={c.x + m.dx * TS.col}
									cy={c.y}
									rx={(m.w * TS.col) / 2}
									ry={TS.row * 0.26}
									transform={`rotate(${m.rot} ${c.x} ${c.y})`}
									fill={`rgba(${SHEET.graphite}, ${m.o})`}
									filter="url(#g1-smudge)"
									opacity={colOpacity(m.col)}
								/>
							);
						})}
						{GHOSTS.map((g, i) => {
							const c = cellCenter(g.col, g.row);
							return (
								<path
									key={`g${i}`}
									d={scribble(`g1-ghost-${i}`, c.x - TS.col * 0.4, c.y + 4, g.len)}
									fill="none"
									stroke={`url(#g1-erase-${i})`}
									strokeWidth={2.2}
									strokeLinecap="round"
									strokeLinejoin="round"
									filter="url(#g1-graphite)"
									opacity={colOpacity(g.col)}
								/>
							);
						})}
					</svg>
					{/* gutter: hour labels + rules */}
					{HOURS.map((h, j) => (
						<React.Fragment key={h}>
							<div
								style={{
									position: 'absolute',
									left: 0,
									width: TS.gutter - 22,
									top: TS.header + j * TS.row,
									height: TS.row,
									display: 'flex',
									alignItems: 'center',
									justifyContent: 'flex-end',
									fontFamily: font.text,
									fontWeight: 600,
									fontSize: 26,
									color: color.ink2,
									fontVariantNumeric: 'tabular-nums',
								}}
							>
								{h}
							</div>
							{j > 0 ? <div style={{position: 'absolute', left: 20, top: TS.header + j * TS.row - LINE / 2, width: TS.gutter - 20, height: LINE, background: SHEET.rowLine}} /> : null}
						</React.Fragment>
					))}
					{/* day columns */}
					{DAYS.map((d, i) => {
						const left = TS.gutter + i * TS.col;
						const last = i === DAYS.length - 1;
						return (
							<div key={d} style={{position: 'absolute', left, top: 0, width: TS.col, height: TS.h, opacity: colOpacity(i)}}>
								<div style={{position: 'absolute', left: -LINE / 2, top: 0, width: LINE, height: TS.h, background: SHEET.line}} />
								<div
									style={{
										position: 'absolute',
										left: 0,
										top: 0,
										width: TS.col,
										height: TS.header,
										display: 'flex',
										alignItems: 'center',
										justifyContent: 'center',
										fontFamily: font.text,
										fontWeight: 700,
										fontSize: 32,
										letterSpacing: '0.12em',
										paddingLeft: '0.12em',
										color: color.ink,
										opacity: i === TER ? 1 - glow : 0.92,
									}}
								>
									{d}
								</div>
								{Array.from({length: TS.rows - 1}, (_, j) => (
									<div
										key={j}
										style={{
											position: 'absolute',
											left: 0,
											width: last ? TS.col - 1 : TS.col,
											top: TS.header + (j + 1) * TS.row - LINE / 2,
											height: LINE,
											background: SHEET.rowLine,
										}}
									/>
								))}
							</div>
						);
					})}
					{/* glass sheen: a soft top-left falloff + a thin specular band */}
					<div
						style={{
							position: 'absolute',
							inset: 0,
							borderRadius: 24,
							pointerEvents: 'none',
							background:
								'linear-gradient(160deg, rgba(220,232,255,0.075) 0%, rgba(220,232,255,0.03) 22%, rgba(220,232,255,0) 42%, rgba(220,232,255,0) 58%, rgba(220,232,255,0.035) 63%, rgba(220,232,255,0) 70%), linear-gradient(180deg, rgba(0,0,0,0) 55%, rgba(2,4,12,0.22) 100%)',
						}}
					/>
					{/* rim highlight: a lit gradient edge, brightest along the top */}
					<div
						style={{
							position: 'absolute',
							inset: 0,
							borderRadius: 24,
							padding: 1.75,
							pointerEvents: 'none',
							background: 'linear-gradient(172deg, rgba(222,232,255,0.8) 0%, rgba(170,186,222,0.42) 12%, rgba(130,146,184,0.26) 45%, rgba(110,126,166,0.16) 100%)',
							WebkitMask: 'linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)',
							WebkitMaskComposite: 'xor',
							maskComposite: 'exclude',
						}}
					/>
				</div>
				{/* light sweep (secondary motion in the holds) */}
				{sweep !== undefined && sweep > 0 && sweep < 1 ? (
					<div style={{position: 'absolute', inset: 0, borderRadius: 24, overflow: 'hidden', pointerEvents: 'none'}}>
						<div
							style={{
								position: 'absolute',
								top: -TS.h * 0.3,
								height: TS.h * 1.6,
								width: 360,
								left: -360 + sweep * (TS.w + 720) - 360,
								transform: 'rotate(18deg)',
								background: 'linear-gradient(90deg, rgba(207,224,255,0) 0%, rgba(207,224,255,0.10) 45%, rgba(230,238,255,0.16) 50%, rgba(207,224,255,0.10) 55%, rgba(207,224,255,0) 100%)',
							}}
						/>
					</div>
				) : null}
				{/* the TER column lights up volt */}
				{glow > 0 ? (
					<>
						<div
							style={{
								position: 'absolute',
								left: terLeft,
								top: 0,
								width: TS.col,
								height: TS.h,
								borderRadius: 14,
								opacity: glow,
								background: `linear-gradient(180deg, ${alpha(color.volt, 0.3)} 0%, ${alpha(color.volt, 0.16)} 28%, ${alpha(color.volt, 0.1)} 100%)`,
								boxShadow: `inset 0 0 0 2.5px ${alpha(color.volt, 0.95)}, inset 0 0 36px ${alpha(color.volt, 0.35)}, 0 0 48px ${alpha(color.volt, 0.55)}, 0 0 140px ${alpha(color.volt, 0.3)}`,
							}}
						/>
						{/* lit header chip */}
						<div
							style={{
								position: 'absolute',
								left: terLeft + 34,
								top: 8,
								width: TS.col - 68,
								height: TS.header - 16,
								borderRadius: 999,
								opacity: glow,
								background: `linear-gradient(180deg, #6aa0ff, ${color.volt})`,
								boxShadow: `0 0 24px ${alpha(color.volt, 0.7)}, inset 0 1px 0 rgba(255,255,255,0.35)`,
								display: 'flex',
								alignItems: 'center',
								justifyContent: 'center',
								fontFamily: font.text,
								fontWeight: 800,
								fontSize: 32,
								letterSpacing: '0.12em',
								paddingLeft: '0.12em',
								color: '#ffffff',
							}}
						>
							{DAYS[TER]}
						</div>
					</>
				) : null}
				{/* rose guesses */}
				{marks.map((m, i) => {
					if (m.o <= 0) return null;
					const c = cellCenter(m.col, m.row);
					const sc = Math.max(0, m.p);
					return (
						<React.Fragment key={i}>
							<div
								style={{
									position: 'absolute',
									left: c.x - TS.col / 2 + 3,
									top: c.y - TS.row / 2 + 3,
									width: TS.col - 6,
									height: TS.row - 6,
									borderRadius: 10,
									opacity: m.o * Math.min(1, sc),
									// lit from within (on the deep slate a flat 0.2 wash read muddy)
									background: `linear-gradient(180deg, ${alpha(color.rose, 0.32)} 0%, ${alpha(color.rose, 0.18)} 100%)`,
									boxShadow: `inset 0 0 0 2px ${alpha(color.rose, 0.8)}, inset 0 0 22px ${alpha(color.rose, 0.28)}, 0 0 30px ${alpha(color.rose, 0.38)}`,
								}}
							/>
							<div
								style={{
									position: 'absolute',
									left: c.x - 60,
									// centre the cap-height glyph (Sora: cap centre 0.475 em below the line-box top)
									top: c.y - 50 + (0.5 - 0.475) * 68,
									width: 120,
									height: 100,
									display: 'flex',
									alignItems: 'center',
									justifyContent: 'center',
									fontFamily: font.display,
									fontWeight: 800,
									fontSize: 68,
									lineHeight: 1,
									color: '#ff6b70',
									textShadow: `0 0 22px ${alpha(color.rose, 0.7)}, 0 2px 0 rgba(60, 8, 12, 0.5)`,
									opacity: m.o,
									transform: `scale(${sc.toFixed(4)})`,
								}}
							>
								?
							</div>
						</React.Fragment>
					);
				})}
				{caretOn ? (
					<div style={{position: 'absolute', left: caret.x - CARET_W / 2, top: caret.y - CARET_H / 2, ...caretStyle(), boxShadow: `0 0 18px ${alpha(color.volt, 0.9)}, 0 0 4px #bcd4ff`}} />
				) : null}
			</div>
		</AbsoluteFill>
	);
};

/* ------------------------------------------------------------------------ */
/* Emphasis bars (underline sweep / strike-through), in em of the parent     */
/* ------------------------------------------------------------------------ */

export const SweepBar: React.FC<{
	progress: number;
	color: string;
	/** top edge, em below the span's line-box top */
	topEm: number;
	thicknessEm: number;
	/** extend beyond the word, em each side */
	overhangEm?: number;
	/** soft glow in the bar's colour */
	glow?: boolean;
}> = ({progress, color: c, topEm, thicknessEm, overhangEm = 0, glow = false}) =>
	progress <= 0 ? null : (
		<span
			style={{
				position: 'absolute',
				left: `${-overhangEm}em`,
				right: `${-overhangEm}em`,
				top: `${topEm}em`,
				height: `${thicknessEm}em`,
				borderRadius: 999,
				background: c,
				boxShadow: glow ? `0 0 18px ${c.startsWith('#') ? alpha(c, 0.7) : c}` : undefined,
				transformOrigin: '0% 50%',
				transform: `scaleX(${progress})`,
			}}
		/>
	);

/** v2 headline ink: white with a slight top-to-bottom gradient (brief §3 Headlines). */
export const inkGradient: React.CSSProperties = {
	backgroundImage: 'linear-gradient(180deg, #ffffff 0%, #ffffff 35%, #cdd7ee 100%)',
	WebkitBackgroundClip: 'text',
	backgroundClip: 'text',
	color: 'transparent',
};

export const useFrame = () => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	return {frame, fps};
};

export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const ramp = (frame: number, a: number, b: number, easing?: (t: number) => number) =>
	interpolate(frame, [a, b], [0, 1], {...CLAMP, easing});
