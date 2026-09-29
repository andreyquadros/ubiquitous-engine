/**
 * G1 (s01–s04) shared parts: the problem-act backdrop, the vector "ANTES"
 * timesheet plane (+ a projector that replicates its CSS 3D maths so screen-
 * space overlays can be glued to it), the volt caret, type metrics and the
 * slam shake. Scene-local — not shared code.
 */
import React from 'react';
import {AbsoluteFill, interpolate, useCurrentFrame, useVideoConfig} from 'remotion';
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

const LINE = 1.5;
const LINE_C = '#26324b'; // #1f2a40 lifted one step: after the ANTES filter (brightness .8) it lands on the board's line colour

export type Mark = {col: number; row: number; p: number /* 0→1 spring */; o: number /* opacity */};

export const TimesheetPlane: React.FC<{
	pose: PlanePose;
	/** 0 → 1: the four non-TER columns go to 40 %. */
	dim?: number;
	/** In-plane caret (TER/10h). */
	caretOn?: boolean;
	/** Rose "?" guesses. */
	marks?: Mark[];
	P?: number;
}> = ({pose, dim = 0, caretOn = false, marks = [], P = PERSPECTIVE}) => {
	const colOpacity = (i: number) => (i === TER ? 1 : 1 - 0.6 * dim);
	const caret = cellCenter(TER, 1);
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
				{/* ANTES treatment: desaturate 60 %, brightness 0.8 — the caret and the guesses stay out of it */}
				<div style={{position: 'absolute', inset: 0, filter: 'saturate(0.4) brightness(0.8)'}}>
					{/* card */}
					<div
						style={{
							position: 'absolute',
							inset: 0,
							borderRadius: 24,
							background: 'linear-gradient(180deg, rgba(23,32,51,0.72) 0%, rgba(17,23,38,0.66) 100%)',
							boxShadow: `inset 0 1px 0 rgba(255,255,255,0.06), 0 0 0 ${LINE}px ${LINE_C}, 0 40px 90px -30px rgba(0,0,0,0.7)`,
						}}
					/>
					{/* header band */}
					<div
						style={{
							position: 'absolute',
							left: 0,
							top: 0,
							width: TS.w,
							height: TS.header,
							borderRadius: '24px 24px 0 0',
							background: 'rgba(255,255,255,0.025)',
						}}
					/>
					{/* gutter: hour labels + the header rule across the gutter */}
					<div style={{position: 'absolute', left: 0, top: TS.header - LINE / 2, width: TS.gutter, height: LINE, background: LINE_C}} />
					{HOURS.map((h, j) => (
						<React.Fragment key={h}>
							<div
								style={{
									position: 'absolute',
									left: 0,
									width: TS.gutter - 28,
									top: TS.header + j * TS.row,
									height: TS.row,
									display: 'flex',
									alignItems: 'center',
									justifyContent: 'flex-end',
									fontFamily: font.text,
									fontWeight: 500,
									fontSize: 26,
									color: color.ink3,
									fontVariantNumeric: 'tabular-nums',
								}}
							>
								{h}
							</div>
							{j > 0 ? (
								<div style={{position: 'absolute', left: 24, top: TS.header + j * TS.row - LINE / 2, width: TS.gutter - 24, height: LINE, background: alpha(LINE_C, 0.5)}} />
							) : null}
						</React.Fragment>
					))}
					{/* day columns */}
					{DAYS.map((d, i) => {
						const left = TS.gutter + i * TS.col;
						const last = i === DAYS.length - 1;
						return (
							<div key={d} style={{position: 'absolute', left, top: 0, width: TS.col, height: TS.h, opacity: colOpacity(i)}}>
								{/* left rule (TER also owns its right rule so the spotlit column stays closed) */}
								<div style={{position: 'absolute', left: -LINE / 2, top: 0, width: LINE, height: TS.h, background: LINE_C}} />
								{i === TER ? <div style={{position: 'absolute', left: TS.col - LINE / 2, top: 0, width: LINE, height: TS.h, background: LINE_C}} /> : null}
								{/* header label */}
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
										fontWeight: 600,
										fontSize: 30,
										letterSpacing: '0.14em',
										paddingLeft: '0.14em',
										color: i === TER ? interpolateColor(dim) : color.ink2,
									}}
								>
									{d}
								</div>
								{/* row rules (header rule + 8 inner) */}
								{Array.from({length: TS.rows}, (_, j) => (
									<div
										key={j}
										style={{
											position: 'absolute',
											left: 0,
											width: last ? TS.col - 1 : TS.col,
											top: TS.header + j * TS.row - LINE / 2,
											height: LINE,
											background: LINE_C,
										}}
									/>
								))}
							</div>
						);
					})}
				</div>
				{/* rose guesses */}
				{marks.map((m, i) => {
					if (m.o <= 0) return null;
					const c = cellCenter(m.col, m.row);
					const sc = 0.9 + 0.1 * m.p;
					return (
						<div
							key={i}
							style={{
								position: 'absolute',
								left: c.x - 40,
								top: c.y - 30,
								width: 80,
								height: 60,
								display: 'flex',
								alignItems: 'center',
								justifyContent: 'center',
								fontFamily: font.text,
								fontWeight: 600,
								fontSize: 44,
								lineHeight: 1,
								color: color.rose,
								opacity: m.o,
								transform: `scale(${sc})`,
							}}
						>
							?
						</div>
					);
				})}
				{caretOn ? (
					<div style={{position: 'absolute', left: caret.x - CARET_W / 2, top: caret.y - CARET_H / 2, ...caretStyle()}} />
				) : null}
			</div>
		</AbsoluteFill>
	);
};

/** TER header: ink-2 → ink as the spotlight comes on. */
const interpolateColor = (t: number) => {
	const a = [0xa8, 0xb4, 0xcd];
	const b = [0xe8, 0xed, 0xf9];
	const c = a.map((v, i) => Math.round(v + (b[i] - v) * Math.max(0, Math.min(1, t))));
	return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
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
}> = ({progress, color: c, topEm, thicknessEm, overhangEm = 0}) =>
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
				transformOrigin: '0% 50%',
				transform: `scaleX(${progress})`,
			}}
		/>
	);

export const useFrame = () => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	return {frame, fps};
};

export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const ramp = (frame: number, a: number, b: number, easing?: (t: number) => number) =>
	interpolate(frame, [a, b], [0, 1], {...CLAMP, easing});
