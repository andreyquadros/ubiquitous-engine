/**
 * Launch-video transitions as @remotion/transitions presentations
 * (use inside <TransitionSeries>) plus wrappers (<TransitionIn>/<TransitionOut>)
 * for use inside a plain <Sequence>.
 *
 * NOTE: TransitionSeries overlaps the two scenes, so the series gets SHORTER
 * by each transition's duration. Budget for that in the timeline.
 */
import React, {useId} from 'react';
import {AbsoluteFill, useCurrentFrame, useVideoConfig} from 'remotion';
import {linearTiming} from '@remotion/transitions';
import type {TransitionPresentation, TransitionPresentationComponentProps, TransitionTiming} from '@remotion/transitions';
import {alpha, color, ease, resolveColor} from '../design/tokens';
import {clamp, lerp, progress, type EaseFn} from '../design/motion';

type Dir = 'left' | 'right' | 'up' | 'down';

const cleanId = (id: string) => id.replace(/[^a-zA-Z0-9_-]/g, '');

/* ------------------------------------------------------------------------ */
/* DirectionalBlur                                                           */
/* ------------------------------------------------------------------------ */

export type DirectionalBlurProps = {
	/** Blur radius (px std-deviation) along the motion direction. 0 = no-op. */
	amount: number;
	/** Direction of motion in degrees (0 = horizontal, 90 = vertical). Default 0. */
	angle?: number;
	children: React.ReactNode;
	/** Extra style on the outer fill. */
	style?: React.CSSProperties;
};

/**
 * Motion blur along one direction (SVG feGaussianBlur with an anisotropic
 * std-deviation). Cheap stand-in for real motion blur on whips and fast moves.
 */
export const DirectionalBlur: React.FC<DirectionalBlurProps> = ({amount, angle = 0, children, style}) => {
	const id = `dblur-${cleanId(useId())}`;
	if (amount < 0.3) return <AbsoluteFill style={style}>{children}</AbsoluteFill>;
	const a = ((angle % 180) + 180) % 180;
	const axisAligned = a === 0 || a === 90;
	const std = axisAligned ? (a === 0 ? `${amount} 0` : `0 ${amount}`) : `${amount} 0`;
	const svg = (
		<svg width={0} height={0} style={{position: 'absolute'}}>
			<defs>
				<filter id={id} x="-50%" y="-50%" width="200%" height="200%" colorInterpolationFilters="sRGB">
					<feGaussianBlur stdDeviation={std} edgeMode="duplicate" />
				</filter>
			</defs>
		</svg>
	);
	if (axisAligned) {
		return (
			<AbsoluteFill style={style}>
				{svg}
				<AbsoluteFill style={{filter: `url(#${id})`}}>{children}</AbsoluteFill>
			</AbsoluteFill>
		);
	}
	return (
		<AbsoluteFill style={style}>
			{svg}
			<AbsoluteFill style={{transform: `rotate(${a}deg)`, filter: `url(#${id})`}}>
				<AbsoluteFill style={{transform: `rotate(${-a}deg)`}}>{children}</AbsoluteFill>
			</AbsoluteFill>
		</AbsoluteFill>
	);
};

/* ------------------------------------------------------------------------ */
/* whipPan                                                                   */
/* ------------------------------------------------------------------------ */

export type WhipPanProps = {
	/** Direction the CONTENT travels (a "left" whip moves the old scene out to the left). Default "left". */
	direction?: Dir;
	/** Peak motion-blur std-deviation in px. Default 42. */
	blur?: number;
	/** Travel as a fraction of the frame. Default 1. */
	distance?: number;
};

const WhipPanPresentation: React.FC<TransitionPresentationComponentProps<WhipPanProps>> = ({
	children,
	presentationDirection,
	presentationProgress: p,
	passedProps,
}) => {
	const {width, height} = useVideoConfig();
	const dir = passedProps.direction ?? 'left';
	const dist = passedProps.distance ?? 1;
	const horizontal = dir === 'left' || dir === 'right';
	const sign = dir === 'left' || dir === 'up' ? -1 : 1;
	const span = (horizontal ? width : height) * dist;
	const off = presentationDirection === 'exiting' ? sign * p * span : -sign * (1 - p) * span;
	const blur = (passedProps.blur ?? 42) * Math.sin(Math.PI * clamp(p));
	return (
		<DirectionalBlur amount={blur} angle={horizontal ? 0 : 90}>
			<AbsoluteFill style={{transform: horizontal ? `translateX(${off}px)` : `translateY(${off}px)`}}>{children}</AbsoluteFill>
		</DirectionalBlur>
	);
};

/** Whip pan with directional motion blur. Pair with `beatTiming(8–10, ease.whip)`. */
export const whipPan = (props: WhipPanProps = {}): TransitionPresentation<WhipPanProps> => ({
	component: WhipPanPresentation,
	props,
});

/* ------------------------------------------------------------------------ */
/* zoomThrough                                                               */
/* ------------------------------------------------------------------------ */

export type ZoomThroughProps = {
	/** How far the outgoing scene flies toward camera. Default 2.6. */
	zoom?: number;
	/** Peak blur px. Default 22. */
	blur?: number;
	/** Scale the incoming scene starts from. Default 0.55. */
	fromScale?: number;
	/** Transform origin (% of frame) — zoom INTO this point. Default {x: 50, y: 50}. */
	origin?: {x: number; y: number};
};

const ZoomThroughPresentation: React.FC<TransitionPresentationComponentProps<ZoomThroughProps>> = ({
	children,
	presentationDirection,
	presentationProgress: p,
	passedProps,
}) => {
	const o = passedProps.origin ?? {x: 50, y: 50};
	const B = passedProps.blur ?? 22;
	if (presentationDirection === 'exiting') {
		const s = lerp(1, passedProps.zoom ?? 2.6, Math.pow(p, 1.6));
		return (
			<AbsoluteFill
				style={{
					transform: `scale(${s})`,
					transformOrigin: `${o.x}% ${o.y}%`,
					opacity: 1 - clamp((p - 0.35) / 0.5),
					filter: p > 0.01 ? `blur(${p * B}px)` : undefined,
				}}
			>
				{children}
			</AbsoluteFill>
		);
	}
	const e = ease.push(p);
	return (
		<AbsoluteFill
			style={{
				transform: `scale(${lerp(passedProps.fromScale ?? 0.55, 1, e)})`,
				transformOrigin: `${o.x}% ${o.y}%`,
				opacity: clamp((p - 0.2) / 0.45),
				filter: p < 0.99 ? `blur(${(1 - e) * B}px)` : undefined,
			}}
		>
			{children}
		</AbsoluteFill>
	);
};

/** Fly through the old scene into the new one. */
export const zoomThrough = (props: ZoomThroughProps = {}): TransitionPresentation<ZoomThroughProps> => ({
	component: ZoomThroughPresentation,
	props,
});

/* ------------------------------------------------------------------------ */
/* maskWipe                                                                  */
/* ------------------------------------------------------------------------ */

export type MaskWipeProps = {
	/** "circle" iris from `origin`, or a "diagonal" blade sweeping across. Default "circle". */
	shape?: 'circle' | 'diagonal';
	/** Circle origin in % of the frame. Default centre. */
	origin?: {x: number; y: number};
	/** Diagonal sweep direction. Default "right". */
	direction?: 'left' | 'right';
	/** Diagonal blade slant in px (horizontal offset top vs bottom). Default 360. */
	slant?: number;
	/** Glowing edge colour, or false. Default volt. */
	edge?: string | false;
};

const MaskWipePresentation: React.FC<TransitionPresentationComponentProps<MaskWipeProps>> = ({
	children,
	presentationDirection,
	presentationProgress: p,
	passedProps,
}) => {
	const {width, height} = useVideoConfig();
	const shape = passedProps.shape ?? 'circle';
	const edge = passedProps.edge === false ? null : resolveColor(passedProps.edge ?? 'volt');
	if (presentationDirection === 'exiting') {
		// darken + gentle push-in under the incoming mask (scale ≥ 1 so no edges are exposed)
		return (
			<AbsoluteFill style={{transform: `scale(${lerp(1, 1.04, p)})`, filter: `brightness(${lerp(1, 0.55, p)})`}}>{children}</AbsoluteFill>
		);
	}
	if (shape === 'circle') {
		const o = passedProps.origin ?? {x: 50, y: 50};
		const ox = (o.x / 100) * width;
		const oy = (o.y / 100) * height;
		const maxR = Math.max(
			Math.hypot(ox, oy),
			Math.hypot(width - ox, oy),
			Math.hypot(ox, height - oy),
			Math.hypot(width - ox, height - oy),
		);
		const r = maxR * p;
		return (
			<AbsoluteFill>
				<AbsoluteFill style={{clipPath: `circle(${r}px at ${ox}px ${oy}px)`}}>{children}</AbsoluteFill>
				{edge && p > 0 && p < 1 ? (
					<div
						style={{
							position: 'absolute',
							left: ox - r,
							top: oy - r,
							width: r * 2,
							height: r * 2,
							borderRadius: '50%',
							boxShadow: `0 0 0 2px ${alpha(edge, 0.9)}, 0 0 40px 6px ${alpha(edge, 0.5)}`,
							opacity: 1 - p * 0.5,
						}}
					/>
				) : null}
			</AbsoluteFill>
		);
	}
	const slant = passedProps.slant ?? 360;
	const rightward = (passedProps.direction ?? 'right') === 'right';
	const e = -slant + p * (width + slant * 2); // blade centre x
	const topX = rightward ? e + slant / 2 : width - e - slant / 2;
	const botX = rightward ? e - slant / 2 : width - e + slant / 2;
	const poly = rightward
		? `polygon(0px 0px, ${topX}px 0px, ${botX}px ${height}px, 0px ${height}px)`
		: `polygon(${topX}px 0px, ${width}px 0px, ${width}px ${height}px, ${botX}px ${height}px)`;
	return (
		<AbsoluteFill>
			<AbsoluteFill style={{clipPath: poly}}>{children}</AbsoluteFill>
			{edge && p > 0 && p < 1 ? (
				<svg width={width} height={height} style={{position: 'absolute', inset: 0}}>
					<line x1={topX} y1={0} x2={botX} y2={height} stroke={alpha(edge, 0.35)} strokeWidth={18} />
					<line x1={topX} y1={0} x2={botX} y2={height} stroke={edge} strokeWidth={3} />
				</svg>
			) : null}
		</AbsoluteFill>
	);
};

/** Circle iris or diagonal blade reveal of the incoming scene. */
export const maskWipe = (props: MaskWipeProps = {}): TransitionPresentation<MaskWipeProps> => ({
	component: MaskWipePresentation,
	props,
});

/* ------------------------------------------------------------------------ */
/* flashCut                                                                  */
/* ------------------------------------------------------------------------ */

export type FlashCutProps = {
	/** Flash colour. Default white. */
	color?: string;
	/** Peak opacity 0–1. Default 0.9. */
	peak?: number;
	/** Add a small scale punch on the incoming scene. Default true. */
	punch?: boolean;
};

const FlashCutPresentation: React.FC<TransitionPresentationComponentProps<FlashCutProps>> = ({
	children,
	presentationDirection,
	presentationProgress: p,
	passedProps,
}) => {
	const c = passedProps.color ?? '#ffffff';
	const flash = Math.pow(1 - Math.abs(p - 0.5) * 2, 1.5) * (passedProps.peak ?? 0.9);
	const entering = presentationDirection === 'entering';
	const visible = entering ? p >= 0.5 : p < 0.5;
	const punch = entering && passedProps.punch !== false ? lerp(1.06, 1, clamp((p - 0.5) * 2)) : 1;
	return (
		<AbsoluteFill style={{opacity: visible ? 1 : 0}}>
			<AbsoluteFill style={{transform: `scale(${punch})`}}>{children}</AbsoluteFill>
			{visible ? <AbsoluteFill style={{background: c, opacity: flash, mixBlendMode: 'screen'}} /> : null}
		</AbsoluteFill>
	);
};

/** Hard cut hidden behind a bright flash. Keep it short: beatTiming(4–6). */
export const flashCut = (props: FlashCutProps = {}): TransitionPresentation<FlashCutProps> => ({
	component: FlashCutPresentation,
	props,
});

/* ------------------------------------------------------------------------ */
/* blurDissolve                                                              */
/* ------------------------------------------------------------------------ */

export type BlurDissolveProps = {
	/** Peak blur px. Default 18. */
	blur?: number;
	/** Scale drift of the incoming scene (1 = none). Default 1.04. */
	fromScale?: number;
};

const BlurDissolvePresentation: React.FC<TransitionPresentationComponentProps<BlurDissolveProps>> = ({
	children,
	presentationDirection,
	presentationProgress: p,
	passedProps,
}) => {
	const B = passedProps.blur ?? 18;
	if (presentationDirection === 'exiting') {
		return <AbsoluteFill style={{opacity: 1 - p, filter: p > 0.01 ? `blur(${p * B}px)` : undefined}}>{children}</AbsoluteFill>;
	}
	return (
		<AbsoluteFill
			style={{
				opacity: p,
				filter: p < 0.99 ? `blur(${(1 - p) * B}px)` : undefined,
				transform: `scale(${lerp(passedProps.fromScale ?? 1.04, 1, ease.settle(p))})`,
			}}
		>
			{children}
		</AbsoluteFill>
	);
};

/** Soft cross-dissolve through blur. */
export const blurDissolve = (props: BlurDissolveProps = {}): TransitionPresentation<BlurDissolveProps> => ({
	component: BlurDissolvePresentation,
	props,
});

/* ------------------------------------------------------------------------ */
/* Timing + wrappers                                                         */
/* ------------------------------------------------------------------------ */

/**
 * Transition timing snapped to the grid: `beatTiming(8)` = half a beat,
 * `beatTiming(15)` = one beat. Default easing ease.whip.
 */
export const beatTiming = (frames = 8, easing: EaseFn = ease.whip): TransitionTiming =>
	linearTiming({durationInFrames: frames, easing});

type AnyPresentation = TransitionPresentation<Record<string, unknown>>;

export type TransitionWrapperProps = {
	/** A presentation from this file, e.g. `whipPan({direction: 'up'})`. */
	presentation: TransitionPresentation<any>;
	/** Start frame (local to the Sequence). Default 0 for In. */
	at?: number;
	/** Duration in frames. Default 10. */
	duration?: number;
	/** Easing of progress. Default ease.whip. */
	easing?: EaseFn;
	children: React.ReactNode;
};

const noop = () => undefined;

/**
 * Plays the ENTERING half of a presentation on its children (no series needed).
 *
 * @example <TransitionIn presentation={zoomThrough()} duration={12}><MyScene/></TransitionIn>
 */
export const TransitionIn: React.FC<TransitionWrapperProps> = ({presentation, at = 0, duration = 10, easing = ease.whip, children}) => {
	const frame = useCurrentFrame();
	const p = progress(frame, at, duration, easing);
	const P = presentation as AnyPresentation;
	const C = P.component as React.ComponentType<TransitionPresentationComponentProps<Record<string, unknown>>>;
	if (p >= 1) return <AbsoluteFill>{children}</AbsoluteFill>;
	return (
		<C
			presentationDirection="entering"
			presentationProgress={p}
			passedProps={P.props}
			presentationDurationInFrames={duration}
			onElementImage={noop}
			onUnmount={noop}
			bothEnteringAndExiting={false}
		>
			{children}
		</C>
	);
};

/**
 * Plays the EXITING half of a presentation on its children, starting at `at`.
 *
 * @example <TransitionOut presentation={whipPan({direction: 'left'})} at={50} duration={8}>…</TransitionOut>
 */
export const TransitionOut: React.FC<TransitionWrapperProps> = ({presentation, at = 0, duration = 10, easing = ease.whip, children}) => {
	const frame = useCurrentFrame();
	const p = progress(frame, at, duration, easing);
	const P = presentation as AnyPresentation;
	const C = P.component as React.ComponentType<TransitionPresentationComponentProps<Record<string, unknown>>>;
	if (p <= 0) return <AbsoluteFill>{children}</AbsoluteFill>;
	return (
		<C
			presentationDirection="exiting"
			presentationProgress={p}
			passedProps={P.props}
			presentationDurationInFrames={duration}
			onElementImage={noop}
			onUnmount={noop}
			bothEnteringAndExiting={false}
		>
			{children}
		</C>
	);
};

/** Full-frame flash overlay you can drop anywhere (e.g. on a beat hit). */
export const Flash: React.FC<{at: number; duration?: number; color?: string; peak?: number}> = ({at, duration = 8, color: c = color.white, peak = 0.8}) => {
	const frame = useCurrentFrame();
	const d = frame - at;
	if (d < 0 || d > duration) return null;
	const v = Math.pow(1 - d / duration, 2) * peak;
	return <AbsoluteFill style={{background: c, opacity: v, mixBlendMode: 'screen', pointerEvents: 'none'}} />;
};
