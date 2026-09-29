/**
 * G4 (s10–s13) shared parts. Scene-local — not shared code.
 *
 *  - helpers: ramp / lerp / clamp01 / mixHex / arcPoint, the measured colours
 *  - <Backdrop>: canvas + two slow orbs + vignette + grain (same recipe as G2's)
 *  - <G4Plane>: a local copy of <Screen> (same screenGeometry maths, so every
 *    hotspot / camera key / cursor mapping is identical) that adds what the
 *    review scenes need and the shared primitive does not have:
 *      · several capture layers in ONE window (s11's 6-f crossfade, s12's
 *        same-camera swap), each with its own image-space children (patches);
 *      · a screen-space plane filter (rack blur / dim) and a composition push;
 *      · neutral #3a4560 title-bar dots (style §S10, like G2's copy).
 *  - <GlideSpot>: a spotlight whose rect can glide (s11 IFRO row → rule chips).
 *  - <Crop>: an image-space bitmap crop drawn from the @3x twin (crisp replicas).
 *  - <StaggerHeadline>: the S03 word stagger at exact cap-top / left positions.
 *  - <ScreenCursor>: the S13 pointer (white fill, dark stroke) + click ripple.
 */
import React, {createContext, useContext} from 'react';
import {AbsoluteFill, Img, interpolate, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {noise2D} from '@remotion/noise';
import {Grain} from '../../../components/Grain';
import {mapWithGeometry, screenGeometry, type Point, type Rect, type ScreenConfig, type ScreenGeometry} from '../../../components/screen-geometry';
import {alpha, color, font, shadow} from '../../../design/tokens';
import {E, springAt} from '../../../shared/motion';
import {useHires} from '../../../shared/ui';

export const W = 1920;
export const H = 1080;
export const CLAMP = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;

export const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
export const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
/** 0→1 between frames a and b (clamped), optional easing. */
export const ramp = (frame: number, a: number, b: number, easing?: (t: number) => number) =>
	b <= a ? (frame >= a ? 1 : 0) : interpolate(frame, [a, b], [0, 1], {...CLAMP, easing});

/** Mix two #rrggbb colours. */
export const mixHex = (a: string, b: string, t: number) => {
	const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
	const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
	const c = pa.map((v, i) => Math.round(v + (pb[i] - v) * clamp01(t)));
	return `rgb(${c[0]}, ${c[1]}, ${c[2]})`;
};

/** Point on a quadratic arc from a to b whose control point sits `bend` × distance off the chord (left-hand normal). */
export const arcPoint = (a: Point, b: Point, t: number, bend = 0.12): Point => {
	const dx = b.x - a.x;
	const dy = b.y - a.y;
	const mx = (a.x + b.x) / 2 + dy * bend;
	const my = (a.y + b.y) / 2 - dx * bend;
	const u = 1 - t;
	return {x: u * u * a.x + 2 * u * t * mx + t * t * b.x, y: u * u * a.y + 2 * u * t * my + t * t * b.y};
};

/* ------------------------------------------------------------------------ */
/* Measured colours (sampled from the review captures, 2x px)                */
/* ------------------------------------------------------------------------ */

export const UI = {
	/** Card / page body of every review capture. */
	card: '#0c1220',
	/** Selected queue row. */
	rowActive: '#15233f',
	/** Key chip fill and its 1-px border (assign-key-*). */
	chipFill: '#121a2b',
	chipLine: '#303f5b',
	/** Confirm bar band (review-settled-expanded). */
	confirmBar: '#0e1524',
	/** Button hairline. */
	buttonLine: '#2b3650',
} as const;

/* ------------------------------------------------------------------------ */
/* Backdrop                                                                  */
/* ------------------------------------------------------------------------ */

export const Backdrop: React.FC<{seed: string; children?: React.ReactNode; grain?: number; vignette?: number; ember?: number}> = ({
	seed,
	children,
	grain = 0.045,
	vignette = 0.55,
	ember = 0.06,
}) => {
	const frame = useCurrentFrame();
	const t = frame * 0.004;
	const orbs = [
		{c: color.volt, x: 0.2, y: 0.12, d: 1100, o: 0.16},
		{c: color.ember, x: 0.86, y: 0.9, d: 760, o: ember},
	];
	return (
		<AbsoluteFill style={{backgroundColor: color.canvas, overflow: 'hidden'}}>
			{orbs.map((o, i) => {
				const nx = noise2D(`${seed}-ox-${i}`, t, i * 3.1) * 0.025 * W;
				const ny = noise2D(`${seed}-oy-${i}`, t, i * 7.7) * 0.018 * W;
				const d = o.d * (1 + noise2D(`${seed}-ob-${i}`, t * 0.7, i) * 0.03);
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: o.x * W + nx - d / 2,
							top: o.y * H + ny - d / 2,
							width: d,
							height: d,
							borderRadius: '50%',
							background: `radial-gradient(closest-side, ${alpha(o.c, o.o)} 0%, ${alpha(o.c, o.o * 0.45)} 38%, ${alpha(o.c, 0)} 100%)`,
						}}
					/>
				);
			})}
			{children}
			{vignette > 0 ? (
				<AbsoluteFill
					style={{pointerEvents: 'none', background: `radial-gradient(ellipse 85% 80% at 50% 50%, transparent 55%, rgba(3,5,10,${0.75 * vignette}) 100%)`}}
				/>
			) : null}
			{grain > 0 ? <Grain opacity={grain} seed={`${seed}-grain`} /> : null}
		</AbsoluteFill>
	);
};

/* ------------------------------------------------------------------------ */
/* G4Plane — Screen with capture layers, plane filter and push               */
/* ------------------------------------------------------------------------ */

const GeoCtx = createContext<ScreenGeometry | null>(null);
/** Inside G4Plane children: the plane geometry at the current frame. */
export const useG4Geometry = () => useContext(GeoCtx);

export type PlaneLayer = {
	/** 2x capture path ("ui/review-done.png"); the @3x twin is drawn when shipped. */
	src: string;
	opacity?: number;
	/** Image-space children of this layer only (its patches, its replicas). */
	children?: React.ReactNode;
};

export type G4PlaneProps = Omit<ScreenConfig, 'src'> & {
	layers: PlaneLayer[];
	/** Image-space children above every layer (spotlights, overlays). */
	children?: React.ReactNode;
	/** Screen-space whole-plane blur, px. */
	blur?: number;
	/** Screen-space brightness multiplier (1 = none). */
	brightness?: number;
	/** Canvas-colour veil over the plane, 0–1. */
	dim?: number;
	/** Composition push: extra scale around `pushOrigin` (screen space). */
	push?: number;
	pushOrigin?: Point;
	/** Screen-space translation (shake). */
	shift?: Point;
};

const LayerImg: React.FC<{layer: PlaneLayer; g: ScreenGeometry}> = ({layer, g}) => {
	const hires = useHires(layer.src);
	const src = hires ? `ui/${hires.hires}` : layer.src;
	const o = layer.opacity ?? 1;
	if (o <= 0.001) return null;
	return (
		<div style={{position: 'absolute', inset: 0, opacity: o}}>
			<Img src={staticFile(src)} style={{position: 'absolute', left: 0, top: 0, width: g.contentW, height: g.contentH, display: 'block'}} />
			{layer.children ? (
				<div style={{position: 'absolute', left: 0, top: 0, width: g.imgW, height: g.imgH, transformOrigin: '0 0', transform: `scale(${g.s0})`}}>
					{layer.children}
				</div>
			) : null}
		</div>
	);
};

export const G4Plane: React.FC<G4PlaneProps> = ({layers, children, blur = 0, brightness = 1, dim = 0, push = 1, pushOrigin, shift, ...cfg}) => {
	const frame = useCurrentFrame();
	const {width, height} = useVideoConfig();
	const g = screenGeometry({...cfg, src: layers[0]?.src ?? ''}, frame, {width, height});
	const r = (cfg.radius ?? 14) * (g.winW / 1440);
	const filters = [blur > 0.2 ? `blur(${blur.toFixed(2)}px)` : '', Math.abs(brightness - 1) > 0.002 ? `brightness(${brightness.toFixed(3)})` : '']
		.filter(Boolean)
		.join(' ');
	const po = pushOrigin ?? {x: W / 2, y: H / 2};
	const tf = [
		shift && (shift.x !== 0 || shift.y !== 0) ? `translate(${shift.x.toFixed(2)}px, ${shift.y.toFixed(2)}px)` : '',
		Math.abs(push - 1) > 1e-4 ? `translate(${po.x}px, ${po.y}px) scale(${push.toFixed(5)}) translate(${-po.x}px, ${-po.y}px)` : '',
	]
		.filter(Boolean)
		.join(' ');
	return (
		<AbsoluteFill style={{pointerEvents: 'none', filter: filters || undefined}}>
			<AbsoluteFill style={{transform: tf || undefined, transformOrigin: '0 0'}}>
				<AbsoluteFill style={{transformOrigin: '0 0', transform: `translate(${g.Cx - g.k * g.fx}px, ${g.Cy - g.k * g.fy}px) scale(${g.k})`}}>
					<AbsoluteFill style={{perspective: g.perspective, perspectiveOrigin: `${g.cx}px ${g.cy}px`}}>
						<div
							style={{
								position: 'absolute',
								left: g.left,
								top: g.top,
								width: g.winW,
								height: g.winH,
								transformOrigin: '50% 50%',
								transform: `translateY(${g.bob}px) rotateX(${g.rx}deg) rotateY(${g.ry}deg) rotateZ(${g.rz}deg) scale(${g.scale})`,
							}}
						>
							<div style={{position: 'absolute', inset: 0, borderRadius: r, overflow: 'hidden', background: color.panel, boxShadow: shadow.window}}>
								<TitleBar height={g.titleH} />
								<div style={{position: 'absolute', left: 0, top: g.titleH, width: g.contentW, height: g.contentH, overflow: 'hidden'}}>
									{layers.map((l, i) => (
										<LayerImg key={`${l.src}-${i}`} layer={l} g={g} />
									))}
									<div style={{position: 'absolute', left: 0, top: 0, width: g.imgW, height: g.imgH, transformOrigin: '0 0', transform: `scale(${g.s0})`}}>
										<GeoCtx.Provider value={g}>{children}</GeoCtx.Provider>
									</div>
								</div>
								<div
									style={{
										position: 'absolute',
										inset: 0,
										borderRadius: r,
										boxShadow: 'inset 0 0 0 1px rgba(255,255,255,0.08), inset 0 1px 0 rgba(255,255,255,0.06)',
									}}
								/>
							</div>
						</div>
					</AbsoluteFill>
				</AbsoluteFill>
			</AbsoluteFill>
			{dim > 0.001 ? <AbsoluteFill style={{background: color.canvas, opacity: dim}} /> : null}
		</AbsoluteFill>
	);
};

const TitleBar: React.FC<{height: number}> = ({height}) => {
	const d = Math.round(height * 0.32);
	return (
		<div
			style={{
				position: 'absolute',
				left: 0,
				right: 0,
				top: 0,
				height,
				background: 'linear-gradient(180deg, #1a2236 0%, #131a2a 100%)',
				borderBottom: '1px solid rgba(255,255,255,0.06)',
				display: 'flex',
				alignItems: 'center',
				paddingLeft: Math.round(height * 0.42),
				gap: Math.round(d * 0.66),
			}}
		>
			{[0, 1, 2].map((i) => (
				<div key={i} style={{width: d, height: d, borderRadius: '50%', background: '#3a4560'}} />
			))}
		</div>
	);
};

/** Plane geometry for a config at a frame (outside the plane: cursor, keycap, bubble). */
export const planeGeometry = (cfg: Omit<ScreenConfig, 'src'>, frame: number): ScreenGeometry =>
	screenGeometry({...cfg, src: ''}, frame, {width: W, height: H});

export const mapPt = (g: ScreenGeometry, p: Point) => mapWithGeometry(g, p);

/* ------------------------------------------------------------------------ */
/* Spotlight with a gliding rect                                             */
/* ------------------------------------------------------------------------ */

/** Image-space spotlight: dims everything outside `rect`, volt outline ≈ 2.5 comp px. Child of G4Plane. */
export const GlideSpot: React.FC<{rect: Rect; dim: number; outline?: number; pad?: number; radius?: number}> = ({
	rect,
	dim,
	outline = 1,
	pad = 14,
	radius = 20,
}) => {
	const g = useG4Geometry();
	if (dim <= 0.001 && outline <= 0.001) return null;
	const onScreen = g ? g.k * g.scale * g.s0 : 1;
	const bw = 2.5 / Math.max(0.05, onScreen);
	const glowPx = 26 / Math.max(0.05, onScreen);
	return (
		<div
			style={{
				position: 'absolute',
				left: rect.x - pad,
				top: rect.y - pad,
				width: rect.w + pad * 2,
				height: rect.h + pad * 2,
				borderRadius: radius,
				boxShadow: [
					outline > 0.001 ? `0 0 0 ${bw}px ${alpha(color.volt, 0.95 * outline)}` : null,
					outline > 0.001 ? `0 0 ${glowPx}px ${glowPx * 0.25}px ${alpha(color.volt, 0.45 * outline)}` : null,
					`0 0 0 6000px rgba(6, 9, 16, ${dim})`,
				]
					.filter(Boolean)
					.join(', '),
			}}
		/>
	);
};

export const lerpRect = (a: Rect, b: Rect, t: number): Rect => ({x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t), w: lerp(a.w, b.w, t), h: lerp(a.h, b.h, t)});

/* ------------------------------------------------------------------------ */
/* Crop: a piece of a capture (from its @3x twin) as an image-space element   */
/* ------------------------------------------------------------------------ */

export const Crop: React.FC<{src: string; rect: Rect; style?: React.CSSProperties; imgW?: number; imgH?: number}> = ({src, rect, style, imgW = 2880, imgH = 1800}) => {
	const hires = useHires(src);
	const file = hires ? `ui/${hires.hires}` : src;
	return (
		<div style={{position: 'absolute', left: rect.x, top: rect.y, width: rect.w, height: rect.h, overflow: 'hidden', ...style}}>
			<Img src={staticFile(file)} style={{position: 'absolute', left: -rect.x, top: -rect.y, width: imgW, height: imgH, maxWidth: 'none'}} />
		</div>
	);
};

/* ------------------------------------------------------------------------ */
/* Scrim                                                                     */
/* ------------------------------------------------------------------------ */

export const Scrim: React.FC<{background: string; opacity?: number; mask?: string}> = ({background, opacity = 1, mask}) =>
	opacity <= 0.001 ? null : <AbsoluteFill style={{background, opacity, pointerEvents: 'none', WebkitMaskImage: mask, maskImage: mask}} />;

/* ------------------------------------------------------------------------ */
/* Stagger headline (style S03)                                              */
/* ------------------------------------------------------------------------ */

/** Sora cap top sits 0.11 × size below the line box top when line-height = 1 (hhea of the shipped woff2). */
export const SORA_CAP = 0.11;

export type HeadUnit = {text: string; at: number; emphasis?: boolean};

/**
 * Split `text` (the storyboard's exact string, lines by \n) into the given
 * units; throws if the units do not rebuild the text byte-exact (copy drift).
 * Units inside a line are separated by single spaces; a unit may contain
 * spaces (glued words, rendered with U+00A0).
 */
export const unitsOf = (text: string, units: HeadUnit[][]): HeadUnit[][] => {
	const rebuilt = units.map((line) => line.map((u) => u.text).join(' ')).join('\n');
	if (rebuilt !== text) throw new Error(`G4 headline copy drift: "${rebuilt}" ≠ storyboard "${text}"`);
	return units;
};

export const StaggerHeadline: React.FC<{
	lines: HeadUnit[][];
	size: number;
	left: number;
	capTops: number[];
	tracking?: string;
}> = ({lines, size, left, capTops, tracking = '-0.03em'}) => {
	const frame = useCurrentFrame();
	return (
		<AbsoluteFill style={{pointerEvents: 'none'}}>
			{lines.map((line, li) => (
				<div
					key={li}
					style={{
						position: 'absolute',
						left,
						top: Math.round(capTops[li] - SORA_CAP * size),
						fontFamily: font.display,
						fontWeight: 700,
						fontSize: size,
						lineHeight: 1,
						letterSpacing: tracking,
						color: color.ink,
						whiteSpace: 'nowrap',
					}}
				>
					{line.map((u, i) => {
						const s = frame < u.at ? 0 : springAt(frame, u.at, 'SNAPPY');
						const o = clamp01((frame - u.at + 1) / 6);
						const settled = frame >= u.at + 14;
						const ty = settled ? 0 : 28 * (1 - s);
						const blur = settled ? 0 : 8 * clamp01(1 - s);
						const volt = u.emphasis ? clamp01((s - 0.55) / 0.4) : 0;
						return (
							<React.Fragment key={i}>
								<span
									style={{
										display: 'inline-block',
										opacity: o,
										transform: Math.abs(ty) > 0.05 ? `translateY(${ty.toFixed(2)}px)` : undefined,
										filter: blur > 0.15 ? `blur(${blur.toFixed(2)}px)` : undefined,
										color: volt > 0 ? mixHex(color.ink, color.volt, volt) : undefined,
									}}
								>
									{u.text.replace(/ /g, ' ')}
								</span>
								{i < line.length - 1 ? ' ' : null}
							</React.Fragment>
						);
					})}
				</div>
			))}
		</AbsoluteFill>
	);
};

/* ------------------------------------------------------------------------ */
/* Cursor (style S13): white arrow, 1.5 px dark stroke, press + ripple        */
/* ------------------------------------------------------------------------ */

export const ArrowCursor: React.FC<{x: number; y: number; size: number; opacity?: number}> = ({x, y, size, opacity = 1}) => {
	if (opacity <= 0.001) return null;
	// viewBox 0 0 20 30: tip at (2, 1.6)
	const k = size / 30;
	return (
		<svg
			width={20 * k + 8}
			height={30 * k + 8}
			viewBox={`0 0 ${20 + 8 / k} ${30 + 8 / k}`}
			style={{position: 'absolute', left: x - 2 * k, top: y - 1.6 * k, opacity, overflow: 'visible', filter: `drop-shadow(0 ${2 * k}px ${3 * k}px rgba(0,0,0,0.5))`}}
		>
			<path
				d="M2 1.6 L2 24.2 L7.3 19.3 L10.9 27.6 L14.9 25.9 L11.4 17.8 L18.6 17.8 Z"
				fill="#ffffff"
				stroke="#0a0d16"
				strokeWidth={1.5 / k}
				strokeLinejoin="round"
			/>
		</svg>
	);
};

/** Screen-space click ripple centred on (x, y) (style S13: 0→44 px, 14 f E.push, + ring 3 f later → 64 px). */
export const ClickRipple: React.FC<{x: number; y: number; at: number; scale?: number}> = ({x, y, at, scale = 1}) => {
	const frame = useCurrentFrame();
	const rings = [
		{start: at, r: 44, o: 0.6, len: 14},
		{start: at + 3, r: 64, o: 0.3, len: 14},
	];
	return (
		<>
			{rings.map((g, i) => {
				const d = frame - g.start;
				if (d < 0 || d > g.len) return null;
				const t = clamp01(d / g.len);
				const e = E.push(t);
				const r = g.r * e * scale;
				const sw = lerp(2, 0.5, t) * scale;
				return (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: x - r,
							top: y - r,
							width: r * 2,
							height: r * 2,
							borderRadius: '50%',
							border: `${sw.toFixed(2)}px solid ${alpha(color.volt, g.o * (1 - t))}`,
							boxSizing: 'border-box',
						}}
					/>
				);
			})}
		</>
	);
};

/** Soft volt pool behind the s10 keycap (an orb, not a UI glow): separates the key from the dimmed plane. */
export const KeyPool: React.FC<{x?: number; y?: number; opacity?: number}> = ({x = 960, y = 600, opacity = 1}) =>
	opacity <= 0.001 ? null : (
		<div
			style={{
				position: 'absolute',
				left: x - 520,
				top: y - 420,
				width: 1040,
				height: 840,
				borderRadius: '50%',
				opacity,
				background: `radial-gradient(closest-side, ${alpha(color.volt, 0.14)} 0%, ${alpha(color.volt, 0.05)} 45%, ${alpha(color.volt, 0)} 100%)`,
				pointerEvents: 'none',
			}}
		/>
	);

/** A whole capture (its @3x twin when shipped) drawn in image space (2880×1800 box). */
export const CaptureImg: React.FC<{src: string; imgW?: number; imgH?: number}> = ({src, imgW = 2880, imgH = 1800}) => {
	const hires = useHires(src);
	return <Img src={staticFile(hires ? `ui/${hires.hires}` : src)} style={{position: 'absolute', left: 0, top: 0, width: imgW, height: imgH, maxWidth: 'none'}} />;
};

/** CSS clip-path inset for an image-space rect inside a 2880×1800 box. */
export const insetOf = (r: Rect, imgW = 2880, imgH = 1800) => `inset(${r.y}px ${imgW - r.x - r.w}px ${imgH - r.y - r.h}px ${r.x}px)`;

/**
 * The storyboard's keys-legend patches (x 2136, h 32) leave the left edge of the first key box and the bottom
 * 3 px of every key box visible (measured on the @3x twins: the boxes span x 2128–2786 and 39 px in height).
 * Grow them to the whole legend line so no key-box fragments peek out. Other patches pass through unchanged.
 */
export const widenLegend = <T extends {rect: Rect; covers?: string}>(patches: T[]): T[] =>
	patches.map((p) => (/keys legend/.test(p.covers ?? '') ? {...p, rect: {x: 2118, y: p.rect.y - 4, w: 2800 - 2118, h: p.rect.h + 10}} : p));
