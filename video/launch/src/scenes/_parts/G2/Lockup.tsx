/**
 * The s05 lockup type: the real ubiqX wordmark (brand/ubiqx-wordmark-bold.svg,
 * per-letter paths inlined verbatim so the X can pop on its own) + the "AI" pill.
 *
 * Geometry (v2: LOCKUP_SCALE 1.23 × the v1 194 px): viewBox 2917 × 969 drawn
 * 238.6 px tall (k ≈ 0.2463, width 718 px). The X glyph box is (2213–2917,
 * 39–769): its centre (2565, 404) stays pinned on the caret spot (960, 520), so
 * the wordmark spans x 328.3–1046.7, y 420.5–659.1 (baseline 609.9).
 */
import React from 'react';
import {color, font} from '../../../design/tokens';
import {CARET, clamp01, lerp} from './common';

/** v2: the lockup is 1.23× the v1 size (brief v2-scenes G2: 1.15–1.3×). */
export const LOCKUP_SCALE = 1.23;

export const WM = {
	vbW: 2917,
	vbH: 969,
	h: 194 * LOCKUP_SCALE,
	get k() {
		return this.h / this.vbH;
	},
	get w() {
		return this.vbW * this.k;
	},
	xCentre: {x: 2565, y: 404},
	get left() {
		return CARET.x - this.xCentre.x * this.k;
	},
	get top() {
		return CARET.y - this.xCentre.y * this.k;
	},
	get baseline() {
		return this.top + 769 * this.k;
	},
	get xHeightTop() {
		return this.top + 221 * this.k;
	},
};

/** Paths copied from public/brand/ubiqx-wordmark-bold.svg (ids wm-u/b/i/q/X). */
export const WM_PATHS: {id: string; fill: string; d: string}[] = [
	{
		id: 'u',
		fill: '#e8edf9',
		d: 'M197 786Q102 786 51 724.5Q0 663 0 539V221H160V547Q160 591 185 617Q210 643 252 643Q295 643 322 616Q349 589 349 543V221H509V769H382V641Q378 660 371 676Q350 732 308 759Q266 786 204 786Z',
	},
	{
		id: 'b',
		fill: '#e8edf9',
		d: 'M964 787Q897 787 846 758.5Q795 730 766 676Q759 663 754 648V769H627V39H787V307Q815 259 859 234Q909 205 973 205Q1029 205 1074.5 225.5Q1120 246 1152 283.5Q1184 321 1201.5 372Q1219 423 1219 483V505Q1219 565 1201.5 616Q1184 667 1150.5 705.5Q1117 744 1070 765.5Q1023 787 964 787ZM922 653Q963 653 993.5 633Q1024 613 1041 576.5Q1058 540 1058 494Q1058 447 1041 412Q1024 377 993.5 357.5Q963 338 922 338Q886 338 854 355Q822 372 802.5 403.5Q783 435 783 480V517Q783 560 803.5 590.5Q824 621 856 637Q888 653 922 653Z',
	},
	{
		id: 'i',
		fill: '#e8edf9',
		d: 'M1326 769V339H1253V221H1486V769ZM1388 167Q1343 167 1321.5 143.5Q1300 120 1300 83.5Q1300 47 1321.5 23.5Q1343 0 1388 0Q1433 0 1454.5 23.5Q1476 47 1476 83.5Q1476 120 1454.5 143.5Q1433 167 1388 167Z',
	},
	{
		id: 'q',
		fill: '#e8edf9',
		d: 'M2001 969V698Q1976 736 1938 758Q1889 787 1823 787Q1766 787 1719 765.5Q1672 744 1638.5 706.5Q1605 669 1587.5 618Q1570 567 1570 508V485Q1570 426 1588.5 375Q1607 324 1641.5 286Q1676 248 1723.5 226.5Q1771 205 1829 205Q1897 205 1946.5 234.5Q1996 264 2024 319Q2030 332 2035 345V221H2162V969ZM1870 652Q1907 652 1937.5 635.5Q1968 619 1986.5 587Q2005 555 2005 510V473Q2005 429 1986 399Q1967 369 1936 353Q1905 337 1870 337Q1829 337 1797.5 357.5Q1766 378 1748 413.5Q1730 449 1730 496.5Q1730 544 1748 579Q1766 614 1797.5 633Q1829 652 1870 652Z',
	},
	{id: 'X', fill: '#4d8dff', d: 'M2213 769 2457 388 2234 39H2414L2556 267H2579L2719 39H2894L2667 393L2917 769H2736L2570 511H2546L2388 769Z'},
];

export type WordmarkState = {
	/** Extra letter spacing, px on screen, per gap (tracking animation). */
	spread: number;
	/** X pop: 0→1 spring (may overshoot). */
	xPop: number;
	/** Glint band progress 0→1 (≤0 or ≥1: hidden). */
	glint: number;
	/** Volt glow strength 0–1 (text-shadow 0.30 at 1). */
	glow: number;
};

/** The wordmark as SVG, absolutely positioned in canvas px (left/top from WM). */
export const Wordmark: React.FC<WordmarkState> = ({spread, xPop, glint, glow}) => {
	const k = WM.k;
	const dxVb = spread / k; // viewBox units per gap
	const letterDx = (i: number) => -(4 - i) * dxVb; // X (index 4) is the pivot
	const xs = Math.max(0, xPop);
	const xRot = lerp(-90, 0, clamp01(xPop));
	const letters = WM_PATHS.map((p, i) =>
		i < 4 ? (
			<path key={p.id} d={p.d} fill={p.fill} transform={dxVb ? `translate(${letterDx(i).toFixed(2)} 0)` : undefined} />
		) : xs > 0.001 ? (
			<path
				key={p.id}
				d={p.d}
				fill={p.fill}
				transform={`translate(${WM.xCentre.x} ${WM.xCentre.y}) rotate(${xRot.toFixed(2)}) scale(${xs.toFixed(4)}) translate(${-WM.xCentre.x} ${-WM.xCentre.y})`}
			/>
		) : null,
	);
	const band = 700; // 172 px on screen
	const gx = lerp(-900, WM.vbW + 900, glint);
	const showGlint = glint > 0 && glint < 1;
	return (
		<svg
			width={WM.w}
			height={WM.h}
			viewBox={`0 0 ${WM.vbW} ${WM.vbH}`}
			style={{
				position: 'absolute',
				left: WM.left,
				top: WM.top,
				overflow: 'visible',
				filter:
					glow > 0.01
						? `drop-shadow(0 0 ${(26 * glow).toFixed(1)}px rgba(77, 141, 255, ${(0.3 * glow).toFixed(3)})) drop-shadow(0 0 ${(72 * glow).toFixed(1)}px rgba(77, 141, 255, ${(0.14 * glow).toFixed(3)}))`
						: undefined,
			}}
		>
			<defs>
				<clipPath id="g2-wm-clip">{letters}</clipPath>
				<linearGradient id="g2-glint" x1="0" y1="0" x2="1" y2="0">
					<stop offset="0" stopColor="#fff" stopOpacity="0" />
					<stop offset="0.5" stopColor="#fff" stopOpacity="0.18" />
					<stop offset="1" stopColor="#fff" stopOpacity="0" />
				</linearGradient>
			</defs>
			{letters}
			{showGlint ? (
				<g clipPath="url(#g2-wm-clip)">
					<rect x={gx - band / 2} y={-600} width={band} height={WM.vbH + 1200} fill="url(#g2-glint)" transform={`rotate(20 ${gx} ${WM.vbH / 2})`} />
				</g>
			) : null}
		</svg>
	);
};

/** "AI" pill: Sora 600 56 px × LOCKUP_SCALE (69 px), volt on a navy-backed volt tint, radius 999, padding 0.2em 0.5em. */
export const AI_PILL = {
	size: Math.round(56 * LOCKUP_SCALE),
	gap: Math.round(24 * LOCKUP_SCALE),
	get left() {
		return WM.left + WM.w + this.gap;
	},
	/** vertically centred on the lowercase x-height band of the wordmark */
	get centreY() {
		return (WM.xHeightTop + WM.baseline) / 2;
	},
};

export const AiPill: React.FC<{p: number}> = ({p}) => {
	if (p <= 0.001) return null;
	const s = lerp(0.72, 1, p);
	return (
		<div
			style={{
				position: 'absolute',
				left: AI_PILL.left,
				top: AI_PILL.centreY,
				transform: `translateY(-50%) translateX(${lerp(-14, 0, p).toFixed(2)}px) scale(${s.toFixed(4)})`,
				transformOrigin: '0% 50%',
				opacity: clamp01(p * 1.6),
				fontFamily: font.display,
				fontWeight: 600,
				fontSize: AI_PILL.size,
				lineHeight: 1,
				letterSpacing: '-0.01em',
				color: color.volt,
				// v2 round 2: the pill is a surface with its own navy backing (0.88) under the volt tint, so the volt
				// "AI" holds ≥ 4.5:1 wherever the stage light falls (a 0.14 tint alone over a lit stage measured 3.7:1)
				background: 'linear-gradient(rgba(77, 141, 255, 0.11), rgba(77, 141, 255, 0.11)), rgba(10, 16, 36, 0.88)',
				boxShadow: 'inset 0 0 0 1.5px rgba(77, 141, 255, 0.22)',
				borderRadius: 999,
				padding: '0.2em 0.5em',
				whiteSpace: 'nowrap',
			}}
		>
			AI
		</div>
	);
};
