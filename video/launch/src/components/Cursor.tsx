import React from 'react';
import {AbsoluteFill, useCurrentFrame, useVideoConfig} from 'remotion';
import {alpha, ease, layer, resolveColor, type Accent} from '../design/tokens';
import {clamp, lerp, progress, type EaseFn} from '../design/motion';
import {mapWithGeometry, onScreenScale, screenGeometry, type Point, type ScreenConfig} from './screen-geometry';

export type CursorKey = {
	/** Frame the cursor ARRIVES at this point. */
	at: number;
	/** X in image px (when `screen` is given) or composition px. */
	x: number;
	/** Y in image px (when `screen` is given) or composition px. */
	y: number;
	/** Click on arrival. */
	click?: boolean;
	/** Show the pointing-hand variant while resting here (e.g. over a button). */
	hand?: boolean;
	/** Override travel duration into this key (frames). */
	move?: number;
	/** Override easing of the travel into this key. */
	easing?: EaseFn;
};

export type CursorProps = {
	/** Keyframed path. Before the first key the cursor rests at the first point. */
	path: CursorKey[];
	/** Share the Screen's config to use image-pixel coordinates that follow tilt + camera. */
	screen?: ScreenConfig;
	/** Extra click frames (in addition to `click: true` keys). */
	clicks?: number[];
	/** Default travel duration into each key, in frames. Default 20. */
	moveDuration?: number;
	/** Fade-in frame. Default: 6 frames before the first key's travel starts (or 0). */
	appearAt?: number;
	/** Fade-out frame. Omit to keep visible. */
	hideAt?: number;
	/** Base size (height of the arrow) in composition px. Default 40. */
	size?: number;
	/** Scale the cursor with the camera zoom (feels "in" the UI). Default true when `screen` is set. */
	scaleWithCamera?: boolean;
	/** Path curvature, fraction of travel distance (0 = straight). Default 0.12. */
	arc?: number;
	/** Click ripple colour. Default volt. */
	rippleColor?: Accent | string;
	/** Default pointer variant. Default "arrow". */
	variant?: 'arrow' | 'hand';
};

/** Position along the path at `frame` (in the path's own coordinate space). */
export const cursorPathPoint = (
	path: CursorKey[],
	frame: number,
	moveDuration = 20,
	arc = 0.12,
): {p: Point; key: CursorKey; moving: boolean} => {
	if (path.length === 0) return {p: {x: 0, y: 0}, key: {at: 0, x: 0, y: 0}, moving: false};
	if (frame <= path[0].at || path.length === 1) return {p: {x: path[0].x, y: path[0].y}, key: path[0], moving: false};
	for (let i = 1; i < path.length; i++) {
		const a = path[i - 1];
		const b = path[i];
		if (frame <= b.at) {
			const dur = Math.min(b.move ?? moveDuration, b.at - a.at);
			const start = b.at - dur;
			if (frame <= start) return {p: {x: a.x, y: a.y}, key: a, moving: false};
			const t = progress(frame, start, dur, b.easing ?? ease.inOut);
			// quadratic bezier with a perpendicular control offset → natural arc
			const dx = b.x - a.x;
			const dy = b.y - a.y;
			const mx = (a.x + b.x) / 2 - dy * arc;
			const my = (a.y + b.y) / 2 + dx * arc;
			const u = 1 - t;
			return {
				p: {x: u * u * a.x + 2 * u * t * mx + t * t * b.x, y: u * u * a.y + 2 * u * t * my + t * t * b.y},
				key: t > 0.85 ? b : a,
				moving: t > 0 && t < 1,
			};
		}
	}
	const last = path[path.length - 1];
	return {p: {x: last.x, y: last.y}, key: last, moving: false};
};

/**
 * macOS-style pointer with eased, slightly arced travel, press-scale and a
 * click ripple. Place it AFTER <Screen> in the same Sequence.
 *
 * @example
 * <Cursor screen={shot} path={[
 *   {at: 0, x: 2400, y: 1500},
 *   {at: 30, x: 1320, y: 640, click: true},
 *   {at: 60, x: 1500, y: 900, hand: true},
 * ]} />
 */
export const Cursor: React.FC<CursorProps> = ({
	path,
	screen,
	clicks = [],
	moveDuration = 20,
	appearAt,
	hideAt,
	size = 40,
	scaleWithCamera,
	arc = 0.12,
	rippleColor = 'volt',
	variant = 'arrow',
}) => {
	const frame = useCurrentFrame();
	const {width, height} = useVideoConfig();
	const comp = {width, height};
	const {p: raw, key} = cursorPathPoint(path, frame, moveDuration, arc);

	let pos = raw;
	let camScale = 1;
	let geo: ReturnType<typeof screenGeometry> | null = null;
	if (screen) {
		geo = screenGeometry(screen, frame, comp);
		pos = mapWithGeometry(geo, raw);
		// normalise so a 1:1 un-zoomed window keeps the base size
		if (scaleWithCamera ?? true) camScale = geo.k * geo.scale;
	}

	const allClicks = [...clicks, ...path.filter((k) => k.click).map((k) => k.at)].sort((a, b) => a - b);

	// press: down over 3 frames, release over 7
	let press = 0;
	for (const c of allClicks) {
		const d = frame - c;
		if (d >= -3 && d < 0) press = Math.max(press, (d + 3) / 3);
		else if (d >= 0 && d < 7) press = Math.max(press, 1 - d / 7);
	}

	const first = path[0]?.at ?? 0;
	const secondStart = path.length > 1 ? path[1].at - Math.min(path[1].move ?? moveDuration, path[1].at - first) : first;
	const inAt = appearAt ?? Math.max(0, Math.min(first, secondStart) - 6);
	let opacity = progress(frame, inAt, 6, ease.settle);
	if (hideAt !== undefined) opacity *= 1 - progress(frame, hideAt, 6, ease.exit);
	if (opacity <= 0) return null;

	const s = size * camScale * (1 - 0.16 * press);
	const hand = key.hand ?? variant === 'hand';
	const rc = resolveColor(rippleColor);

	return (
		<AbsoluteFill style={{zIndex: layer.cursor, pointerEvents: 'none'}}>
			{allClicks.map((c) => {
				const d = frame - c;
				if (d < 0 || d > 22) return null;
				// ripple is anchored to the click point, which may move with the camera
				const kAt = path.find((k) => k.at === c) ?? {x: raw.x, y: raw.y};
				const cp = geo ? mapWithGeometry(geo, {x: kAt.x, y: kAt.y}) : {x: kAt.x, y: kAt.y};
				const t = clamp(d / 22);
				const r = lerp(8, 64, ease.push(t)) * camScale;
				const r2 = lerp(4, 38, ease.push(clamp(d / 16))) * camScale;
				return (
					<React.Fragment key={c}>
						<div
							style={{
								position: 'absolute',
								left: cp.x - r,
								top: cp.y - r,
								width: r * 2,
								height: r * 2,
								borderRadius: '50%',
								border: `${3 * camScale}px solid ${alpha(rc, 0.9 * (1 - t))}`,
								boxShadow: `0 0 24px ${alpha(rc, 0.5 * (1 - t))}`,
							}}
						/>
						<div
							style={{
								position: 'absolute',
								left: cp.x - r2,
								top: cp.y - r2,
								width: r2 * 2,
								height: r2 * 2,
								borderRadius: '50%',
								background: alpha(rc, 0.28 * (1 - clamp(d / 16))),
							}}
						/>
					</React.Fragment>
				);
			})}
			<div
				style={{
					position: 'absolute',
					left: pos.x,
					top: pos.y,
					width: s,
					height: s,
					opacity,
					filter: `drop-shadow(0 ${s * 0.08}px ${s * 0.12}px rgba(0,0,0,0.55))`,
				}}
			>
				{hand ? <HandSvg size={s} /> : <ArrowSvg size={s} />}
			</div>
		</AbsoluteFill>
	);
};

/** Arrow pointer; hotspot at the tip = top-left of the box. */
const ArrowSvg: React.FC<{size: number}> = ({size}) => (
	<svg width={size} height={size} viewBox="0 0 28 28" style={{position: 'absolute', left: -size * (3 / 28), top: -size * (2 / 28), overflow: 'visible'}}>
		<path
			d="M3 2 L3 22.6 L8.2 17.8 L11.6 25.6 L15.4 24 L12.1 16.4 L19.2 16.4 Z"
			fill="#0b0d12"
			stroke="#ffffff"
			strokeWidth={1.7}
			strokeLinejoin="round"
		/>
	</svg>
);

/** Pointing hand; hotspot at the index fingertip. */
const HandSvg: React.FC<{size: number}> = ({size}) => (
	<svg width={size} height={size} viewBox="0 0 28 28" style={{position: 'absolute', left: -size * (10.5 / 28), top: -size * (2 / 28), overflow: 'visible'}}>
		<path
			d="M9 13.2 V4.4 a1.9 1.9 0 0 1 3.8 0 V11 a1.8 1.8 0 0 1 3.6 0.2 a1.8 1.8 0 0 1 3.5 0.5 a1.8 1.8 0 0 1 3.4 0.8 V18 c0 4.2 -2.8 7.4 -7 7.4 h-1.6 c-2.6 0 -4.3 -1.1 -5.8 -3.2 L4.6 17.4 a1.9 1.9 0 0 1 3 -2.3 Z"
			fill="#ffffff"
			stroke="#0b0d12"
			strokeWidth={1.5}
			strokeLinejoin="round"
		/>
		<path d="M12.8 13.5 v4.5 M16.4 13.8 v4.2 M19.9 14.2 v3.8" stroke="#0b0d12" strokeWidth={1.2} strokeLinecap="round" />
	</svg>
);

/**
 * Hook: map an image-pixel point of a Screen to composition px at the current
 * frame. Useful for custom overlays that should track the UI.
 */
export const useImageToComp = (screen: ScreenConfig): ((pt: Point) => Point) => {
	const frame = useCurrentFrame();
	const {width, height} = useVideoConfig();
	const g = screenGeometry(screen, frame, {width, height});
	return (pt: Point) => mapWithGeometry(g, pt);
};

export {onScreenScale};
