/**
 * <LiftCard>: lift one element of a UI capture (the 88 score ring, a review
 * row, the "Confirmar os 19" button, the provider pills, the redaction chip)
 * out of the screenshot into composition space as a floating 3D card
 * (brief/v2-look.md §3 "Depth: lift cards").
 *
 * - The crop `rect` is in the capture's 2x image px (the same space as
 *   hotspots, <Screen> children and patches); the @3x twin is drawn when it is
 *   shipped (public/ui/hires.json), exactly like <Screen>.
 * - The card is graded like the window (GRADE), so it reads as the same UI.
 *   Claim-safety patches that fall inside the crop must be re-applied as
 *   `children` (image px of the FULL capture, like <Screen> children).
 *   Or pass them as `patches` (StoryboardPatch[], e.g. storyboardPatches(scene, file)):
 *   drawn exactly like <Patches> (same frames, colour and 2 px bleed).
 * - `from` may be a function of the frame (e.g. `(f) => mapImageRect(shot, f, comp, rect)`)
 *   so the lift origin / drop target tracks a moving camera or a floating window.
 * - <LiftHole> (child of the <Screen>, image space) hides the source element in
 *   the window while it is lifted, so the element never appears twice.
 * - Deterministic and frame-driven: springs from design tokens, sine float,
 *   linear drift, keyframed pose. Frames are local to the enclosing Sequence.
 *
 * @example
 * // lift the 88 focus ring off the dashboard, 1.4× its in-window size
 * const ring = hotspot('ui/dashboard.png', 'focus-dial');
 * <LiftCard src="ui/dashboard.png" rect={ring} x={1320} y={520} scale={1.4}
 *   at={20} enter="lift" from={mapImageRect(shot, 20, {width: 1920, height: 1080}, ring)}
 *   rotateX={8} rotateY={-12} float={6} drift={{x: -0.15, y: 0}} glow="mint" />
 */
import React from 'react';
import {AbsoluteFill, Img, interpolate, spring, useCurrentFrame, useVideoConfig} from 'remotion';
import {alpha, color, ease, resolveColor, springs, type Accent, type SpringPreset} from '../design/tokens';
import {kf, lerp, oscillate, progress, type Keyframed} from '../design/motion';
import {useHires} from '../shared/ui';
import type {StoryboardPatch} from '../storyboard';
import {gradeFilter, resolveGrade, type Grade, type Rect} from './screen-geometry';
import {resolveSrc, rimBackground, GLASS_INSET, RIM_PX} from './Screen';

/** A comp-px rect, or one per frame (local frames) for a moving window. */
export type LiftFrom = Rect | ((frame: number) => Rect);

export type LiftCardEnter = 'lift' | 'rise' | 'pop' | 'fade' | 'none';
export type LiftCardExit = 'sink' | 'drop' | 'fade' | 'none';

export type LiftCardProps = {
	/** Capture path under public/ ("ui/dashboard.png"): the 2x file; its @3x twin is drawn when shipped. */
	src: string;
	/** Crop in the capture's 2x image px. */
	rect: Rect;
	/** Natural size of the 2x capture. Default 2880x1800. */
	imageSize?: {w: number; h: number};
	/** Hi-res twin: auto (default), false = never, or a path under public/. */
	hires?: boolean | string;
	/** Card centre on the canvas, composition px (keyframeable). */
	x: Keyframed;
	y: Keyframed;
	/**
	 * Size relative to how the element reads inside a 1440-wide <Screen> at
	 * camera zoom 1 (comp px per image px = 1440 / imageSize.w). 1.2–1.6 per the
	 * brief. Keyframeable. Default 1.4. Ignored when `width` is set.
	 */
	scale?: Keyframed;
	/** Explicit on-canvas card width in comp px (keyframeable); overrides `scale`. */
	width?: Keyframed;
	/** Tilt, degrees (keyframeable). +X = top leans away, +Y = right side leans away. Default 8 / −10. */
	rotateX?: Keyframed;
	rotateY?: Keyframed;
	rotateZ?: Keyframed;
	/** Perspective px. Default 1600. */
	perspective?: number;
	/** Corner radius, comp px. Default 16. */
	radius?: number;
	/** Entrance start frame. Default 0. */
	at?: number;
	/**
	 * Entrance. `lift`: from its in-window position (`from`) up to the target pose;
	 * `rise`: from 80 px lower; `pop`: scale 0.7 → 1 (bouncy); `fade`; `none`. Default `rise`.
	 */
	enter?: LiftCardEnter;
	/** Spring preset of the entrance (tokens.springs). Default smooth (pop: subtleBounce). */
	spring?: SpringPreset;
	/**
	 * Where the element sits in the window (comp px rect), used by `lift` and `drop`.
	 * Static (`mapImageRect(shot, at, …)`) or a function of the frame
	 * (`(f) => mapImageRect(shot, f, …)`) so the origin tracks a moving camera.
	 */
	from?: LiftFrom;
	/** Exit start frame and style. */
	exitAt?: number;
	exit?: LiftCardExit;
	/** Exit duration, frames. Default 12. */
	exitDuration?: number;
	/** Float bob amplitude px (0 = off). Default 5. */
	float?: number;
	/** Float period, frames. Default 96. */
	floatPeriod?: number;
	/** Parallax drift, comp px per frame from `at` (move against the window's camera). Default none. */
	drift?: {x: number; y: number};
	/** Coloured glow under the card (accent / CSS) or false. Default volt. */
	glow?: Accent | string | false;
	/** Glow strength 0–1. Default 0.35. */
	glowOpacity?: number;
	/** Rim light (volt top-left gradient border + white top highlight). Default true. */
	rim?: boolean;
	/** Drop shadow strength 0–1. Default 1. */
	shadow?: number;
	/** Colour grade, as <Screen grade>. Default on (GRADE). */
	grade?: boolean | Grade;
	/** Overall opacity multiplier (keyframeable). Default 1. */
	opacity?: Keyframed;
	/** Claim-safety patches (image px of the FULL capture), e.g. storyboardPatches(scene, file); drawn like <Patches>. */
	patches?: readonly StoryboardPatch[];
	/** Image-space overlays (patches, live values): image px of the FULL capture, like <Screen> children. */
	children?: React.ReactNode;
	/** Extra style on the outer full-frame layer (e.g. zIndex). */
	style?: React.CSSProperties;
};

const DEFAULT_IMAGE = {w: 2880, h: 1800};

/** Pose of a LiftCard at a frame (pure; useful to glue overlays or a cursor to the card). */
export const liftCardPose = (p: LiftCardProps, frame: number, fps: number) => {
	const img = p.imageSize ?? DEFAULT_IMAGE;
	const at = p.at ?? 0;
	const enter = p.enter ?? 'rise';
	const preset: SpringPreset = p.spring ?? (enter === 'pop' ? 'subtleBounce' : 'smooth');
	const e = enter === 'none' ? 1 : spring({frame: frame - at, fps, config: springs[preset]});
	const eIn = enter === 'none' ? 1 : Math.max(0, Math.min(1, e));
	// target size: comp px per image px
	const kTarget = p.width !== undefined ? kf(p.width, frame) / p.rect.w : (1440 / img.w) * kf(p.scale, frame, 1.4);
	let cx = kf(p.x, frame);
	let cy = kf(p.y, frame);
	let k = kTarget;
	let rx = kf(p.rotateX, frame, 8);
	let ry = kf(p.rotateY, frame, -10);
	const rz = kf(p.rotateZ, frame, 0);
	let s = 1;
	let o = kf(p.opacity, frame, 1);
	let lift = 1; // shadow / glow / rim presence
	const from = typeof p.from === 'function' ? p.from(frame) : p.from;
	if (enter === 'lift' && from) {
		const fx = from.x + from.w / 2;
		const fy = from.y + from.h / 2;
		const kFrom = from.w / p.rect.w;
		cx = lerp(fx, cx, e);
		cy = lerp(fy, cy, e);
		k = Math.exp(lerp(Math.log(kFrom), Math.log(kTarget), e));
		rx = lerp(0, rx, e);
		ry = lerp(0, ry, e);
		lift = eIn;
		o *= frame < at ? 0 : 1;
	} else if (enter === 'rise') {
		cy += (1 - e) * 80;
		o *= interpolate(frame, [at, at + 8], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
	} else if (enter === 'pop') {
		s = lerp(0.7, 1, e);
		o *= interpolate(frame, [at, at + 5], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
	} else if (enter === 'fade') {
		o *= progress(frame, at, 12, ease.settle);
	}
	// float + parallax drift (from `at`)
	const t = Math.max(0, frame - at);
	if (p.float ?? 5) cy += oscillate(t, p.floatPeriod ?? 96, p.float ?? 5) * eIn;
	if (p.drift) {
		cx += p.drift.x * t;
		cy += p.drift.y * t;
	}
	// exit
	if (p.exitAt !== undefined && (p.exit ?? 'fade') !== 'none') {
		const x = progress(frame, p.exitAt, p.exitDuration ?? 12, ease.exit);
		const ex = p.exit ?? 'fade';
		if (ex === 'sink') {
			cy += x * 60;
			s *= lerp(1, 0.92, x);
		} else if (ex === 'drop') {
			// back into the window: flatten + shrink toward `from`
			if (from) {
				cx = lerp(cx, from.x + from.w / 2, x);
				cy = lerp(cy, from.y + from.h / 2, x);
				k = lerp(k, from.w / p.rect.w, x);
			}
			rx = lerp(rx, 0, x);
			ry = lerp(ry, 0, x);
			lift *= 1 - x;
		}
		o *= ex === 'drop' ? 1 - ease.exit(Math.max(0, (x - 0.7) / 0.3)) : 1 - x;
	}
	return {cx, cy, k, w: p.rect.w * k, h: p.rect.h * k, rx, ry, rz, s, o, lift};
};

export const LiftCard: React.FC<LiftCardProps> = (props) => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	const {src, rect, hires: hiresProp, radius = 16, perspective = 1600, glowOpacity = 0.35, rim = true, shadow = 1, patches, children, style} = props;
	const img = props.imageSize ?? DEFAULT_IMAGE;
	const hires = useHires(hiresProp === false || typeof hiresProp === 'string' ? null : src);
	const drawSrc = typeof hiresProp === 'string' ? hiresProp : hires ? `ui/${hires.hires}` : src;
	const pose = liftCardPose(props, frame, fps);
	if (pose.o <= 0.002) return null;
	const glowC = props.glow === false ? null : resolveColor(props.glow ?? 'volt');
	const grade = gradeFilter(resolveGrade(props.grade));
	const {w, h, k} = pose;
	const sh = shadow * pose.lift;

	return (
		<AbsoluteFill style={{pointerEvents: 'none', perspective, perspectiveOrigin: `${pose.cx}px ${pose.cy}px`, ...style}}>
			<div
				style={{
					position: 'absolute',
					left: pose.cx - w / 2,
					top: pose.cy - h / 2,
					width: w,
					height: h,
					opacity: pose.o < 0.999 ? pose.o : undefined,
					transformOrigin: '50% 50%',
					transform: `rotateX(${pose.rx.toFixed(3)}deg) rotateY(${pose.ry.toFixed(3)}deg) rotateZ(${pose.rz.toFixed(3)}deg) scale(${pose.s.toFixed(4)})`,
				}}
			>
				{glowC && pose.lift > 0.01 ? (
					<div
						style={{
							position: 'absolute',
							left: '-30%',
							right: '-30%',
							top: '-35%',
							bottom: '-45%',
							background: `radial-gradient(closest-side, ${alpha(glowC, glowOpacity * pose.lift)} 0%, ${alpha(glowC, glowOpacity * 0.35 * pose.lift)} 55%, transparent 100%)`,
						}}
					/>
				) : null}
				{/* shadow + rim, behind the opaque card */}
				<div
					style={{
						position: 'absolute',
						inset: rim ? -RIM_PX : 0,
						borderRadius: radius + (rim ? RIM_PX : 0),
						background: rim ? rimBackground(glowC ?? color.volt) : undefined,
						opacity: rim ? Math.max(0.35, pose.lift) : undefined,
						boxShadow:
							sh > 0.01
								? `0 ${(18 * sh).toFixed(1)}px ${(36 * sh).toFixed(1)}px -8px rgba(0,0,0,${(0.7 * sh).toFixed(3)}), 0 ${(46 * sh).toFixed(1)}px ${(90 * sh).toFixed(1)}px -18px rgba(0,0,0,${(0.75 * sh).toFixed(3)})`
								: undefined,
					}}
				/>
				<div style={{position: 'absolute', inset: 0, borderRadius: radius, overflow: 'hidden', background: color.panel}}>
					<div style={{position: 'absolute', left: -rect.x * k, top: -rect.y * k, width: img.w * k, height: img.h * k, filter: grade || undefined}}>
						<Img src={resolveSrc(drawSrc)} style={{position: 'absolute', left: 0, top: 0, width: img.w * k, height: img.h * k, maxWidth: 'none', display: 'block'}} />
						{children || patches?.length ? (
							<div style={{position: 'absolute', left: 0, top: 0, width: img.w, height: img.h, transformOrigin: '0 0', transform: `scale(${k})`}}>
								{patches ? <PatchLayer patches={patches} frame={frame} rect={rect} /> : null}
								{children}
							</div>
						) : null}
					</div>
					<div style={{position: 'absolute', inset: 0, borderRadius: radius, boxShadow: GLASS_INSET}} />
				</div>
			</div>
		</AbsoluteFill>
	);
};

/** Storyboard patches as <Patches> draws them (inlined to keep components/ free of shared/ cycles); skips ones outside the crop. */
const PatchLayer: React.FC<{patches: readonly StoryboardPatch[]; frame: number; rect: Rect; bleed?: number}> = ({patches, frame, rect, bleed = 2}) => (
	<>
		{patches
			.filter((q) => frame >= q.from && frame <= q.to)
			.filter((q) => q.rect.x < rect.x + rect.w && q.rect.x + q.rect.w > rect.x && q.rect.y < rect.y + rect.h && q.rect.y + q.rect.h > rect.y)
			.map((q, i) => (
				<div key={i} style={{position: 'absolute', left: q.rect.x - bleed, top: q.rect.y - bleed, width: q.rect.w + bleed * 2, height: q.rect.h + bleed * 2, background: q.color}} />
			))}
	</>
);

export type LiftHoleProps = Pick<LiftCardProps, 'rect' | 'at' | 'enter' | 'spring' | 'exitAt' | 'exit' | 'exitDuration'> & {
	/**
	 * Fill, sampled from the capture's surface around the element (hex, like the
	 * storyboard patch colours) so the element simply vanishes from the window.
	 * Omitted: a navy dim (rgba(7,11,20,0.78)) that leaves a faint ghost, which
	 * reads fine on any surface but is less clean. Prefer a sampled colour.
	 */
	color?: string;
	/** Extra coverage around `rect`, image px (covers anti-aliased edges / glow). Default 6. */
	pad?: number;
	/** Soft edge width, image px. Default 18. */
	feather?: number;
	/** Corner radius, image px. Default 24. */
	radius?: number;
	/** Empty-socket hint: faint inner rim + inset shade, 0–1. Default 0.5 (0 = pure fill). */
	socket?: number;
	/** Opacity multiplier (keyframeable). Default 1. */
	opacity?: Keyframed;
};

/** Presence 0–1 of the hole at a frame (pure): on when the card leaves, off as a `drop` lands or a card fades away. */
export const liftHolePresence = (p: LiftHoleProps, frame: number, fps: number): number => {
	const at = p.at ?? 0;
	const enter = p.enter ?? 'rise';
	if (frame < at) return 0;
	let o = 1;
	if (enter !== 'lift' && enter !== 'none') {
		// the card appears elsewhere: the element leaves the window with the same spring
		const e = spring({frame: frame - at, fps, config: springs[p.spring ?? (enter === 'pop' ? 'subtleBounce' : 'smooth')]});
		o = Math.max(0, Math.min(1, e * 1.4));
	}
	if (p.exitAt !== undefined && (p.exit ?? 'fade') !== 'none') {
		const x = progress(frame, p.exitAt, p.exitDuration ?? 12, ease.exit);
		// drop: hole stays until the card has landed (the card fades over its last 30 %);
		// fade/sink: the element returns to the window as the card leaves
		o *= (p.exit ?? 'fade') === 'drop' ? (x >= 1 ? 0 : 1) : 1 - x;
	}
	return o * kf(p.opacity, frame, 1);
};

/**
 * <LiftHole>: hides the lifted element in the window underneath. Place it as a
 * CHILD of the <Screen> (image space, graded + tilted with the window) and pass
 * the card's rect/timing (spreading the LiftCard props works).
 *
 * @example
 * const ring = {src: 'ui/dashboard.png', rect: RING, at: 14, enter: 'lift' as const};
 * <Screen {...shot}><LiftHole {...ring} color="#111c33" /></Screen>
 * <LiftCard {...ring} x={1380} y={500} from={(f) => mapImageRect(shot, f, COMP, RING)} style={{zIndex: 30}} />
 */
export const LiftHole: React.FC<LiftHoleProps> = (p) => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	const o = liftHolePresence(p, frame, fps);
	if (o <= 0.002) return null;
	const {rect, pad = 6, feather = 18, radius = 24, socket = 0.5} = p;
	const fill = p.color ?? 'rgba(7,11,20,0.78)';
	return (
		<div
			style={{
				position: 'absolute',
				left: rect.x - pad,
				top: rect.y - pad,
				width: rect.w + pad * 2,
				height: rect.h + pad * 2,
				borderRadius: radius,
				background: fill,
				opacity: o < 0.999 ? o : undefined,
				// feathered edge: the same colour blurred outward
				boxShadow: [
					`0 0 ${feather}px ${feather * 0.35}px ${fill}`,
					socket > 0 ? `inset 0 ${(6 * socket).toFixed(1)}px ${(22 * socket).toFixed(1)}px rgba(0,0,0,${(0.45 * socket).toFixed(3)})` : '',
					socket > 0 ? `inset 0 0 0 1.5px rgba(255,255,255,${(0.05 * socket).toFixed(3)})` : '',
				]
					.filter(Boolean)
					.join(', '),
			}}
		/>
	);
};
