/**
 * G4 v2 (s09–s11) review-sequence contracts, shared by the three scenes so the
 * hand-offs cannot drift:
 *
 *  - REVIEW_END: the camera s09 arrives on at f74. s10 holds it as its blurred
 *    backdrop and s11 opens on it (a pure rack focus s10 → s11).
 *  - PICKER_LIFT: the picker ("IFRO · 1" on top) lifts off the assign card at
 *    s09 f56 as a LiftCard and stays up through s10 and s11 (same props, the
 *    frame offset of each scene applied to `at`), so s11's key lands on the
 *    "1" chip of the floating card.
 *  - cardPoint(): projects an image-px point of a LiftCard crop to the canvas
 *    (the same CSS 3D maths as LiftCard), so the keycap lands exactly on the chip.
 *  - <V2Headline>: the v2 display line (Sora 700, −0.04em, white → cool-grey
 *    gradient, volt-gradient emphasis with a soft glow, optional marker sweep).
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import type {LiftCardProps} from '../../../components/LiftCard';
import {liftCardPose} from '../../../components/LiftCard';
import type {Point, Rect, ScreenConfig} from '../../../components/screen-geometry';
import {color, font} from '../../../design/tokens';
import {springAt} from '../../../shared/motion';
import {clamp01, mapPt, planeGeometry, ramp} from './common';

export const REVIEW_FILE = 'ui/review-queue-selected.png';
export const AFTER_FILE = 'ui/review-after-assign.png';

/** s09 / s10 / s11 frame offsets (film: s09 585, s10 660, s11 690). */
export const S09_LEN = 75;
export const S10_LEN = 30;

/** Camera s09 lands on at f74 (image px focus on canvas anchor). The picker sits at comp ≈ (1300, 470), eff 1.25. */
export const REVIEW_END = {zoom: 2.5, focus: {x: 2458, y: 566}, anchor: {x: 1300, y: 470}, tilt: {rx: 3, ry: -5}} as const;

/**
 * v2 claim patch: the page subtitle "Terça-feira, 29 de setembro: 10 grupos esperam sua decisão, 19 min ainda sem
 * categoria." (ink x 512–1700, y 148–188) carries two counts; covered on the page colour #060a14 in both captures.
 */
export const S09_SUBTITLE_PATCH = {x: 504, y: 140, w: 1216, h: 56, fill: '#060a14'};

/** Card surface colour of the review captures (the socket fill). */
export const CARD_BG = '#0c1220';

/** The picker (5 options + key chips), image px (identical in both captures to < 0.4 levels). */
export const PICKER: Rect = {x: 2106, y: 380, w: 704, h: 372};
/** The picker's lift, s09 frames. */
export const PICKER_AT = 56;

/** Pose of the lifted picker (canvas px): 880 wide → option labels ≈ 36 px, IFRO row on top. */
export const pickerLift = (at: number, from: LiftCardProps['from']): Omit<LiftCardProps, 'src'> => ({
	rect: PICKER,
	at,
	enter: 'lift',
	spring: 'smooth',
	from,
	x: 1428,
	y: 372,
	width: 880,
	rotateX: 7,
	rotateY: -9,
	rotateZ: 0,
	perspective: 1700,
	radius: 22,
	float: 5,
	floatPeriod: 110,
	drift: {x: -0.12, y: 0},
	glow: 'volt',
	glowOpacity: 0.3,
});

/** Canvas rect of an image rect inside a G4 plane at a frame (bounding box of the 4 mapped corners). */
export const planeRect = (cfg: Omit<ScreenConfig, 'src'>, frame: number, r: Rect): Rect => {
	const g = planeGeometry(cfg, frame);
	const pts = [
		{x: r.x, y: r.y},
		{x: r.x + r.w, y: r.y},
		{x: r.x, y: r.y + r.h},
		{x: r.x + r.w, y: r.y + r.h},
	].map((p) => mapPt(g, p));
	const xs = pts.map((p) => p.x);
	const ys = pts.map((p) => p.y);
	return {x: Math.min(...xs), y: Math.min(...ys), w: Math.max(...xs) - Math.min(...xs), h: Math.max(...ys) - Math.min(...ys)};
};

const rad = (d: number) => (d * Math.PI) / 180;

/**
 * Canvas position of an image-px point on a LiftCard at a frame, and the on-screen scale there
 * (CSS: perspective P with origin at the card centre; transform rotateX · rotateY · rotateZ · scale).
 */
export const cardPoint = (props: LiftCardProps, frame: number, fps: number, pt: Point): {x: number; y: number; k: number} => {
	const pose = liftCardPose(props, frame, fps);
	const P = props.perspective ?? 1600;
	let x = (pt.x - (props.rect.x + props.rect.w / 2)) * pose.k * pose.s;
	let y = (pt.y - (props.rect.y + props.rect.h / 2)) * pose.k * pose.s;
	let z = 0;
	// rotateZ
	const cz = Math.cos(rad(pose.rz));
	const sz = Math.sin(rad(pose.rz));
	[x, y] = [x * cz - y * sz, x * sz + y * cz];
	// rotateY
	const cy = Math.cos(rad(pose.ry));
	const sy = Math.sin(rad(pose.ry));
	[x, z] = [x * cy + z * sy, -x * sy + z * cy];
	// rotateX
	const cx = Math.cos(rad(pose.rx));
	const sx = Math.sin(rad(pose.rx));
	[y, z] = [y * cx - z * sx, y * sx + z * cx];
	const w = P / (P - z);
	return {x: pose.cx + x * w, y: pose.cy + y * w, k: pose.k * pose.s * w};
};

/* ------------------------------------------------------------------------ */
/* v2 headline                                                                */
/* ------------------------------------------------------------------------ */

/** Sora: ascent 0.97, descent 0.29, cap 0.73 (upm 1000); line-height 1 → baseline at 0.84 em. */
const SORA_BASE = (1 - (0.97 + 0.29)) / 2 + 0.97;
const SORA_CAPH = 0.73;

export type V2Unit = {text: string; at: number; volt?: boolean};

const hexMix = (a: string, b: string, t: number) => {
	const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
	const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
	return '#' + pa.map((v, i) => Math.round(v + (pb[i] - v) * clamp01(t)).toString(16).padStart(2, '0')).join('');
};

const inkFill = (volt: number): React.CSSProperties => {
	const top = hexMix('#ffffff', '#9cc0ff', volt);
	const bottom = hexMix('#c9d3e6', color.volt, volt);
	return {
		backgroundImage: `linear-gradient(180deg, ${top} 16%, ${bottom} 94%)`,
		WebkitBackgroundClip: 'text',
		backgroundClip: 'text',
		color: 'transparent',
		WebkitTextFillColor: 'transparent',
	};
};

/**
 * One or more lines of v2 display type with the S03 word stagger (SNAPPY, rise 30 → 0, blur 8 → 0).
 * Lines are positioned by cap-top (canvas px). A `marker` sweeps a volt highlight behind a unit range.
 */
export const V2Headline: React.FC<{
	f: number;
	lines: V2Unit[][];
	size: number;
	left: number;
	capTops: number[];
	weight?: number;
	tracking?: string;
	marker?: {line: number; units: [number, number]; from: number; to: number; easing?: (t: number) => number};
	shadow?: boolean;
}> = ({f, lines, size, left, capTops, weight = 700, tracking = '-0.04em', marker, shadow = true}) => {
	const unit = (u: V2Unit, key: React.Key) => {
		const s = f < u.at ? 0 : springAt(f, u.at, 'SNAPPY');
		const o = clamp01((f - u.at + 1) / 6);
		const blur = 8 * clamp01(1 - s);
		const volt = u.volt ? clamp01((s - 0.5) / 0.45) : 0;
		const y = 30 * (1 - s);
		const filters = [blur > 0.15 ? `blur(${blur.toFixed(2)}px)` : '', volt > 0.01 ? `drop-shadow(0 0 22px rgba(77,141,255,${(0.5 * volt).toFixed(3)}))` : ''].filter(Boolean);
		return (
			<span
				key={key}
				style={{
					display: 'inline-block',
					opacity: o,
					transform: Math.abs(y) > 0.01 ? `translateY(${y.toFixed(2)}px)` : undefined,
					filter: filters.length ? filters.join(' ') : undefined,
					position: 'relative',
					zIndex: 1,
					...inkFill(volt),
				}}
			>
				{u.text}
			</span>
		);
	};
	return (
		<AbsoluteFill style={{pointerEvents: 'none'}}>
			{lines.map((line, li) => {
				const kids: React.ReactNode[] = [];
				let i = 0;
				while (i < line.length) {
					if (marker && marker.line === li && i === marker.units[0]) {
						const [a, b] = marker.units;
						const p = ramp(f, marker.from, marker.to, marker.easing);
						const group: React.ReactNode[] = [];
						for (let j = a; j <= b; j++) {
							group.push(unit(line[j], j));
							if (j < b) group.push(' ');
						}
						kids.push(
							<span key={`m${i}`} style={{position: 'relative', display: 'inline-block'}}>
								{p > 0 ? (
									<span
										style={{
											position: 'absolute',
											left: '-0.08em',
											top: '0.08em',
											height: '0.9em',
											width: `calc((100% + 0.16em) * ${p.toFixed(4)})`,
											borderRadius: '0.1em',
											background: 'linear-gradient(90deg, rgba(77,141,255,0.30), rgba(77,141,255,0.22))',
											boxShadow: '0 0 30px rgba(77,141,255,0.18)',
											zIndex: 0,
										}}
									/>
								) : null}
								{group}
							</span>,
						);
						i = b + 1;
					} else {
						kids.push(unit(line[i], i));
						i += 1;
					}
					if (i < line.length) kids.push(' ');
				}
				return (
					<div
						key={li}
						style={{
							position: 'absolute',
							left,
							top: Math.round(capTops[li] - (SORA_BASE - SORA_CAPH) * size),
							fontFamily: font.display,
							fontWeight: weight,
							fontSize: size,
							lineHeight: 1,
							letterSpacing: tracking,
							whiteSpace: 'pre',
							filter: shadow ? 'drop-shadow(0 6px 22px rgba(4,8,20,0.55))' : undefined,
						}}
					>
						{kids}
					</div>
				);
			})}
		</AbsoluteFill>
	);
};
