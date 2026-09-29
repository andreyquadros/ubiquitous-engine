/**
 * s18 end-card lockup pieces: the wordmark (brand SVG paths, per-letter masked
 * rise + glint + decaying volt bloom), the "AI" pill, the CTA pill and the
 * platform row with monochrome OS glyphs.
 *
 * Geometry (storyboard s18): wordmark drawn 165 px tall (Sora ≈ 170 px),
 * x 501–998; its x-height band is centred on the lockup row y 432.
 */
import React from 'react';
import {color} from '../../../design/tokens';
import {E, springAt} from '../../../shared/motion';
import {clamp01, lerp, ramp, W} from './common';
import {OS_GLYPHS} from './os-glyphs';
import {WM_BASELINE, WM_LETTERS, WM_VIEWBOX, WM_XHEIGHT_TOP} from './wordmark-paths';

export const ROW_Y = 432;

export const WM = (() => {
	const h = 165;
	const k = h / WM_VIEWBOX.h;
	const w = WM_VIEWBOX.w * k;
	const left = 501;
	// x-height band (221 → 769) centred on the row
	const top = ROW_Y - ((WM_XHEIGHT_TOP + WM_BASELINE) / 2) * k;
	return {h, k, w, left, top, right: left + w, baseline: top + WM_BASELINE * k};
})();

/** Per-letter rise timing: letter i starts at LETTER0 + i (1-f stagger, style §3.4 "wordmark only"). */
const LETTER0 = -3;
const LETTER_DUR = 10;

export const Wordmark: React.FC<{frame: number}> = ({frame}) => {
	const drop = WM_VIEWBOX.h + 60; // viewBox units below the clip edge
	const glint = frame < 10 ? 0 : ramp(frame, 10, 28, (t) => t * t * (3 - 2 * t));
	const showGlint = glint > 0 && glint < 1;
	// volt bloom: peaks on the hit, gone by beat 4 (≤ 1 bar, style §5.4)
	const bloom = 1 - ramp(frame, 4, 48, E.glide);
	const letters = WM_LETTERS.map((p, i) => {
		const u = ramp(frame, LETTER0 + i, LETTER0 + i + LETTER_DUR, E.push);
		if (u <= 0) return null;
		const dy = (1 - u) * drop;
		return <path key={p.id} d={p.d} fill={p.fill} transform={dy > 0.05 ? `translate(0 ${dy.toFixed(2)})` : undefined} />;
	});
	const band = 700;
	const gx = lerp(-900, WM_VIEWBOX.w + 900, glint);
	return (
		<svg
			width={WM.w}
			height={WM.h}
			viewBox={`0 0 ${WM_VIEWBOX.w} ${WM_VIEWBOX.h}`}
			style={{
				position: 'absolute',
				left: WM.left,
				top: WM.top,
				overflow: 'visible',
				filter:
					bloom > 0.01
						? `drop-shadow(0 0 ${(22 * bloom).toFixed(1)}px rgba(77, 141, 255, ${(0.3 * bloom).toFixed(3)})) drop-shadow(0 0 ${(60 * bloom).toFixed(1)}px rgba(77, 141, 255, ${(0.14 * bloom).toFixed(3)}))`
						: undefined,
			}}
		>
			<defs>
				<clipPath id="g6-wm-window">
					<rect x={-400} y={-600} width={WM_VIEWBOX.w + 800} height={WM_VIEWBOX.h + 600 + 24} />
				</clipPath>
				<clipPath id="g6-wm-glyphs">
					{WM_LETTERS.map((p) => (
						<path key={p.id} d={p.d} />
					))}
				</clipPath>
				<linearGradient id="g6-glint" x1="0" y1="0" x2="1" y2="0">
					<stop offset="0" stopColor="#fff" stopOpacity="0" />
					<stop offset="0.5" stopColor="#fff" stopOpacity="0.2" />
					<stop offset="1" stopColor="#fff" stopOpacity="0" />
				</linearGradient>
			</defs>
			<g clipPath="url(#g6-wm-window)">{letters}</g>
			{showGlint ? (
				<g clipPath="url(#g6-wm-glyphs)">
					<rect x={gx - band / 2} y={-600} width={band} height={WM_VIEWBOX.h + 1200} fill="url(#g6-glint)" transform={`rotate(20 ${gx} ${WM_VIEWBOX.h / 2})`} />
				</g>
			) : null}
		</svg>
	);
};

/** "AI" pill: Sora 600 52 px, volt on volt 14 %, radius 999, padding 0.2em 0.5em; masked rise after the X. */
export const AI_PILL = {size: 52, gap: 24, left: WM.left + WM.w + 24} as const;

export const AiPill: React.FC<{frame: number}> = ({frame}) => {
	const at = LETTER0 + 5;
	const u = ramp(frame, at, at + LETTER_DUR, E.push);
	if (u <= 0) return null;
	const s = AI_PILL.size;
	const h = s * 1 + 2 * 0.2 * s;
	return (
		<div style={{position: 'absolute', left: AI_PILL.left - 4, top: ROW_Y - h / 2 - 4, overflow: 'hidden', padding: 4}}>
			<div
				style={{
					transform: u < 1 ? `translateY(${((1 - u) * 120).toFixed(2)}%)` : undefined,
					fontFamily: '"Sora", sans-serif',
					fontWeight: 600,
					fontSize: s,
					lineHeight: 1,
					letterSpacing: '-0.01em',
					color: color.volt,
					background: 'rgba(77, 141, 255, 0.14)',
					boxShadow: 'inset 0 0 0 1.5px rgba(77, 141, 255, 0.24)',
					borderRadius: 999,
					padding: '0.2em 0.5em',
					whiteSpace: 'nowrap',
				}}
			>
				AI
			</div>
		</div>
	);
};

/* ------------------------------------------------------------------------ */
/* CTA pill                                                                  */
/* ------------------------------------------------------------------------ */

export const CTA = {cy: 740, size: 44, padY: 22, padX: 44, lineH: 48} as const;

/**
 * "Baixe em ubiqx.com.br": Sora 600 44 px #0a0d16 on volt (6.1:1), radius 999,
 * padding 22 × 44 → 92 px tall, centred on y 720. Pops with BOUNCY_SUBTLE (the
 * shot's one bouncy element); a light sheen crosses it on `sheenAt`.
 */
export const CtaPill: React.FC<{frame: number; at: number; sheenAt: number; text: string}> = ({frame, at, sheenAt, text}) => {
	if (frame < at) return null;
	const s = springAt(frame, at, 'BOUNCY_SUBTLE');
	const o = ramp(frame, at, at + 3);
	const scale = lerp(0.72, 1, s);
	const sheen = ramp(frame, sheenAt, sheenAt + 20, E.glide);
	const showSheen = sheen > 0 && sheen < 1;
	const h = CTA.lineH + 2 * CTA.padY;
	return (
		<div style={{position: 'absolute', left: 0, width: W, top: CTA.cy - h / 2, height: h, display: 'flex', justifyContent: 'center'}}>
			<div
				style={{
					position: 'relative',
					height: h,
					boxSizing: 'border-box',
					padding: `${CTA.padY}px ${CTA.padX}px`,
					borderRadius: 999,
					background: color.volt,
					overflow: 'hidden',
					opacity: o,
					transform: Math.abs(scale - 1) > 0.0005 ? `scale(${scale.toFixed(4)})` : undefined,
					boxShadow: `inset 0 1px 0 rgba(255,255,255,0.28), 0 1px 0 rgba(255,255,255,0.05), 0 14px 36px -10px rgba(77, 141, 255, ${(0.35 * o).toFixed(3)}), 0 10px 24px -8px rgba(0,0,0,0.55)`,
				}}
			>
				<div
					style={{
						fontFamily: '"Sora", sans-serif',
						fontWeight: 600,
						fontSize: CTA.size,
						lineHeight: `${CTA.lineH}px`,
						letterSpacing: '-0.01em',
						color: color.canvas,
						whiteSpace: 'nowrap',
						position: 'relative',
						top: -1,
					}}
				>
					{text}
				</div>
				{showSheen ? (
					<div
						style={{
							position: 'absolute',
							top: -20,
							bottom: -20,
							width: 140,
							left: `${lerp(-25, 110, sheen)}%`,
							transform: 'skewX(-20deg)',
							background: 'linear-gradient(90deg, rgba(255,255,255,0) 0%, rgba(255,255,255,0.32) 50%, rgba(255,255,255,0) 100%)',
						}}
					/>
				) : null}
			</div>
		</div>
	);
};

/* ------------------------------------------------------------------------ */
/* Platform row                                                              */
/* ------------------------------------------------------------------------ */

export const ROW = {cy: 850, size: 36, glyph: 30, glyphGap: 12, sepGap: 22} as const;

const Glyph: React.FC<{name: keyof typeof OS_GLYPHS; size: number; fill: string}> = ({name, size, fill}) => {
	const g = OS_GLYPHS[name];
	// optical sizing: Windows' four panes read larger than the Apple/Tux silhouettes
	const k = name === 'windows' ? 0.86 : 1;
	return (
		<svg width={size} height={size} viewBox={g.viewBox} style={{display: 'block', flex: 'none'}}>
			<g transform={k !== 1 ? `translate(${(parseFloat(g.viewBox.split(' ')[2]) * (1 - k)) / 2} ${(parseFloat(g.viewBox.split(' ')[3]) * (1 - k)) / 2}) scale(${k})` : undefined}>
				<path d={g.d} fill={fill} />
			</g>
		</svg>
	);
};

/**
 * "macOS · Windows · Linux": Inter 500 36 px ink-2 with 30-px monochrome glyphs,
 * centred on y 850. Items rise y 16 → 0, blur 6 → 0, opacity 0 → 1 (E.enter
 * 14 f), 3 f apart from `at` (style S20).
 */
export const PlatformRow: React.FC<{frame: number; at: number; text: string}> = ({frame, at, text}) => {
	if (frame < at) return null;
	// the storyboard string, split on the middots ('macOS · Windows · Linux')
	const names = text.split('·').map((s) => s.replace(/ /g, ' ').trim());
	const glyphs: (keyof typeof OS_GLYPHS)[] = ['apple', 'windows', 'linux'];
	const item = (i: number) => {
		const p = ramp(frame, at + 3 * i, at + 3 * i + 14, E.enter);
		return {
			opacity: clamp01(p * 1.4),
			transform: p < 1 ? `translateY(${(16 * (1 - p)).toFixed(2)}px)` : undefined,
			filter: p < 0.98 ? `blur(${(6 * (1 - p)).toFixed(2)}px)` : undefined,
		} as React.CSSProperties;
	};
	const ink = color.ink2;
	const lineTop = ROW.cy - ROW.size / 2;
	return (
		<div
			aria-label={text}
			style={{position: 'absolute', left: 0, width: W, top: lineTop, height: ROW.size, display: 'flex', justifyContent: 'center', alignItems: 'center'}}
		>
			{names.map((n, i) => (
				<React.Fragment key={n}>
					{i > 0 ? (
						<span
							style={{
								...item(i),
								margin: `0 ${ROW.sepGap}px`,
								fontFamily: '"Inter", sans-serif',
								fontWeight: 500,
								fontSize: ROW.size,
								lineHeight: 1,
								color: color.ink3,
							}}
						>
							·
						</span>
					) : null}
					<span style={{...item(i), display: 'flex', alignItems: 'center', gap: ROW.glyphGap}}>
						<span style={{position: 'relative', top: glyphs[i] === 'apple' ? -2 : -1}}>
							<Glyph name={glyphs[i] ?? 'linux'} size={ROW.glyph} fill={ink} />
						</span>
						<span
							style={{
								fontFamily: '"Inter", sans-serif',
								fontWeight: 500,
								fontSize: ROW.size,
								lineHeight: 1,
								letterSpacing: '-0.005em',
								color: ink,
								whiteSpace: 'nowrap',
								// Inter caps centre sits on the box centre (cap top 0.136 em, baseline 0.864 em)
							}}
						>
							{n}
						</span>
					</span>
				</React.Fragment>
			))}
		</div>
	);
};

export const TAGLINE = {capTop: 610, size: 48} as const;
