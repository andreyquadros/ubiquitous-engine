/**
 * Split transitions (storyboard convention): every boundary is a HARD cut in
 * the timeline; a transition is split into an OUTGOING half at the end of
 * scene N and an INCOMING half at the start of scene N+1, named identically.
 * No overlap, no TransitionSeries (the film keeps its 1560 frames).
 *
 * Both halves are computed by ONE function, `splitTransition`, from the
 * "proximity to the cut" k ∈ [0, 1] (1 = at the cut):
 *
 *   out half, frames [D − N, D − 1]:  u = (i + 1) / N   k = u      → the last out frame is the cut state
 *   in  half, frames [0, M − 1]:      u = j / M         k = 1 − u  → the first in frame is the cut state
 *
 * so the two halves of a boundary are mirror images of the same curve and
 * match exactly on the cut.
 *
 * Types (all the storyboard uses — see brief/storyboard.md "Transition types"):
 *  - cut:           nothing.
 *  - flash:         T7. Overlay #e8edf9 at `peak` (0.35) × E.exit(k): on the in half that is
 *                   "0.35 decaying to 0 over f0–3 (E.push)"; the out half (if any) ramps up.
 *  - whip-left:     T2. Content x = −960·E.exit(k) on the out half (E.exit), +960·E.exit(k) on the
 *                   in half (= +960→0 with E.push); horizontal blur 40·k. (whip-right/up/down work too.)
 *  - blur-dissolve: T6 without overlap: blur 16·g(k), opacity 1 − (1 − floor)·g(k),
 *                   scale 1 + 0.03·g(k), g = E.glide; dips through the canvas behind the layer.
 *  - match-cut:     T5. Scene-specific choreography: the wrapper leaves the picture untouched and
 *                   exposes the progress (render-prop child or useTransition()) so the scene can morph
 *                   its element (UBI → in-app UBI, keycap → chip).
 */
import React, {createContext, useContext} from 'react';
import {AbsoluteFill, useCurrentFrame} from 'remotion';
import {DirectionalBlur} from '../components/Transitions';
import {clamp} from '../design/motion';
import {useSceneOrNull} from './scene';
import {E} from './motion';
import type {TransitionType} from '../storyboard';

export type SplitSide = 'in' | 'out';

/** Storyboard types plus the obvious whip variants. */
export type SplitType = TransitionType | 'whip-right' | 'whip-up' | 'whip-down';

export type SplitOptions = {
	/** whip: travel in px at the cut. Default 960 (half the frame). */
	distance?: number;
	/** whip: horizontal (vertical) blur std-dev at the cut, px. Default 40. */
	blur?: number;
	/** flash: overlay colour. Default #e8edf9 (ink). */
	color?: string;
	/** flash: overlay opacity at the cut. Default 0.35. */
	peak?: number;
	/** blur-dissolve: blur at the cut, px. Default 16. */
	dissolveBlur?: number;
	/** blur-dissolve: scale at the cut. Default 1.03. */
	dissolveScale?: number;
	/** blur-dissolve: opacity at the cut (0 = full dip to what is behind). Default 0.25. */
	dissolveFloor?: number;
	/** Override the curve applied to k (proximity to the cut). */
	shape?: (k: number) => number;
};

export type SplitState = {
	type: SplitType;
	side: SplitSide;
	/** Half progress 0→1 in time (out: 1 on its last frame; in: 0 on its first frame, 1 once settled). */
	u: number;
	/** Proximity to the cut: 1 = the cut state, 0 = untouched. */
	k: number;
	/** True while this half is running (k > 0). */
	active: boolean;
	/** Style for the wrapped layer (transform / opacity / CSS blur). */
	style: React.CSSProperties;
	/** Directional (whip) blur to apply, px, and its angle (0 = horizontal). */
	motionBlur: {amount: number; angle: number};
	/** Flash overlay drawn above the layer. */
	overlay: {color: string; opacity: number} | null;
};

/** Half progress u for `frame` (scene-relative). Returns null outside the half. */
export const halfProgress = (side: SplitSide, frame: number, frames: number, start: number): number | null => {
	if (frames <= 0) return null;
	const i = frame - start;
	if (side === 'out') {
		if (i < 0) return null;
		return clamp((i + 1) / frames);
	}
	if (i < 0) return 0;
	if (i >= frames) return null;
	return clamp(i / frames);
};

const IDLE = (type: SplitType, side: SplitSide): SplitState => ({
	type,
	side,
	u: side === 'out' ? 0 : 1,
	k: 0,
	active: false,
	style: {},
	motionBlur: {amount: 0, angle: 0},
	overlay: null,
});

/**
 * The single source of truth for both halves of every split transition.
 *
 * @param start  scene-relative first frame of the half (in: 0; out: duration − frames)
 */
export const splitTransition = (
	type: SplitType,
	side: SplitSide,
	frame: number,
	frames: number,
	start: number,
	opts: SplitOptions = {},
): SplitState => {
	if (type === 'cut') return IDLE(type, side);
	const u = halfProgress(side, frame, frames, start);
	if (u === null) return IDLE(type, side);
	const k = side === 'out' ? u : 1 - u;
	if (k <= 0) return {...IDLE(type, side), u};
	const base: SplitState = {...IDLE(type, side), u, k, active: true};

	if (type === 'flash') {
		const s = (opts.shape ?? E.exit)(k);
		return {...base, overlay: {color: opts.color ?? '#e8edf9', opacity: (opts.peak ?? 0.35) * s}};
	}
	if (type.startsWith('whip-')) {
		const dir = type.slice(5) as 'left' | 'right' | 'up' | 'down';
		const horizontal = dir === 'left' || dir === 'right';
		const travelSign = dir === 'left' || dir === 'up' ? -1 : 1; // direction the content travels
		// out: content leaves toward travelSign; in: content arrives FROM the opposite side
		const sign = side === 'out' ? travelSign : -travelSign;
		const d = (opts.distance ?? 960) * (opts.shape ?? E.exit)(k) * sign;
		return {
			...base,
			style: {transform: horizontal ? `translateX(${d}px)` : `translateY(${d}px)`},
			motionBlur: {amount: (opts.blur ?? 40) * k, angle: horizontal ? 0 : 90},
		};
	}
	if (type === 'blur-dissolve') {
		const g = (opts.shape ?? E.glide)(k);
		const floor = opts.dissolveFloor ?? 0.25;
		const b = (opts.dissolveBlur ?? 16) * g;
		return {
			...base,
			style: {
				opacity: 1 - (1 - floor) * g,
				filter: b > 0.2 ? `blur(${b}px)` : undefined,
				transform: `scale(${1 + ((opts.dissolveScale ?? 1.03) - 1) * g})`,
			},
		};
	}
	// match-cut: the scene choreographs it; expose progress only
	return base;
};

/* ------------------------------------------------------------------------ */
/* Wrappers                                                                  */
/* ------------------------------------------------------------------------ */

const TransitionCtx = createContext<{in: SplitState | null; out: SplitState | null}>({in: null, out: null});

/**
 * Inside <TransitionIn>/<TransitionOut>: the current state of the enclosing
 * half(s). Use it for match cuts: `const {in: t} = useTransition(); t?.k`.
 */
export const useTransition = () => useContext(TransitionCtx);

export type SplitWrapperProps = {
	/** Transition type. Default: the scene's storyboard transitionIn / transitionOut type. */
	type?: SplitType;
	/** Frames of this half. Default: the scene's storyboard value. */
	frames?: number;
	/** First frame of the half (scene-relative). Default: in 0, out duration − frames. */
	at?: number;
	/** Scene duration, only needed outside a scene context for <TransitionOut>. */
	sceneDuration?: number;
	options?: SplitOptions;
	/** Layer to transform. A function child receives the state (match cuts). */
	children: React.ReactNode | ((state: SplitState) => React.ReactNode);
	style?: React.CSSProperties;
};

const Apply: React.FC<{state: SplitState; children: SplitWrapperProps['children']; style?: React.CSSProperties}> = ({state, children, style}) => {
	const content = typeof children === 'function' ? children(state) : children;
	const ctx = useContext(TransitionCtx);
	const value = state.side === 'in' ? {...ctx, in: state} : {...ctx, out: state};
	const inner = (
		<AbsoluteFill style={{...style, ...state.style}}>
			{state.motionBlur.amount > 0.3 ? (
				<DirectionalBlur amount={state.motionBlur.amount} angle={state.motionBlur.angle}>
					{content}
				</DirectionalBlur>
			) : (
				content
			)}
		</AbsoluteFill>
	);
	return (
		<TransitionCtx.Provider value={value}>
			{inner}
			{state.overlay && state.overlay.opacity > 0.001 ? (
				<AbsoluteFill style={{background: state.overlay.color, opacity: state.overlay.opacity, pointerEvents: 'none'}} />
			) : null}
		</TransitionCtx.Provider>
	);
};

/**
 * Incoming half of the scene's transition. With no props it plays exactly
 * what the storyboard says for this scene's transitionIn.
 *
 * @example
 * <TransitionIn><MyLayers /></TransitionIn>                  // storyboard default
 * <TransitionIn type="blur-dissolve" frames={6}><Bg /></TransitionIn>
 * <TransitionIn>{(t) => <Ubi scale={1 + 0.2 * t.k} />}</TransitionIn>   // match cut
 */
export const TransitionIn: React.FC<SplitWrapperProps> = ({type, frames, at, options, children, style}) => {
	const scene = useSceneOrNull();
	const frame = useCurrentFrame();
	const t = type ?? scene?.transitionIn.type ?? 'cut';
	const n = frames ?? scene?.transitionIn.frames ?? 0;
	const state = splitTransition(t, 'in', frame, n, at ?? 0, options);
	return (
		<Apply state={state} style={style}>
			{children}
		</Apply>
	);
};

/**
 * Outgoing half of the scene's transition (ends on the scene's last frame by
 * default). With no props it plays the storyboard's transitionOut.
 *
 * @example <TransitionOut><MyLayers /></TransitionOut>
 */
export const TransitionOut: React.FC<SplitWrapperProps> = ({type, frames, at, sceneDuration, options, children, style}) => {
	const scene = useSceneOrNull();
	const frame = useCurrentFrame();
	const t = type ?? scene?.transitionOut.type ?? 'cut';
	const n = frames ?? scene?.transitionOut.frames ?? 0;
	const dur = sceneDuration ?? scene?.durationInFrames;
	if (at === undefined && dur === undefined && n > 0) {
		throw new Error('<TransitionOut>: pass `at` or `sceneDuration` when used outside a scene.');
	}
	const start = at ?? (dur ?? 0) - n;
	const state = splitTransition(t, 'out', frame, n, start, options);
	return (
		<Apply state={state} style={style}>
			{children}
		</Apply>
	);
};

/** Both halves around a scene's content: `<SceneTransitions>…</SceneTransitions>`. */
export const SceneTransitions: React.FC<{children: React.ReactNode; inOptions?: SplitOptions; outOptions?: SplitOptions}> = ({
	children,
	inOptions,
	outOptions,
}) => (
	<TransitionOut options={outOptions}>
		<TransitionIn options={inOptions}>{children}</TransitionIn>
	</TransitionOut>
);
