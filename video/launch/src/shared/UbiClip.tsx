/**
 * UBI, the 3D mascot, from the pre-rendered PNG sequences in public/ubi/
 * (900x900 RGBA, 30 fps, one shared camera — see public/ubi/ubi-manifest.json).
 *
 * - <UbiClip clip="idle" …/>: one clip with loop / hold / playbackRate.
 * - <UbiTrack segments={scene.ubiTrack} …/>: the storyboard's ubiTrack
 *   (index = startIndex + (f − from); "hold" freezes; "hidden" draws nothing).
 * - ubiFrameIndex / ubiFrameSrc / UBI_CLIPS: the pure maths.
 *
 * Positioning: the clip frame is scaled to `size` px and placed so that the
 * `anchor` point of the 900-px frame (default the feet: (450, 797)) lands on
 * canvas (x, y). Because every clip shares the camera, keeping size/x/y
 * constant keeps UBI planted across clip changes (compositing_tips).
 *
 * Life (compositing_tips): optional slow float (4.2 s sine), a soft volt
 * elliptical floor glow and a contact shadow under the feet — none are baked
 * into the renders.
 */
import React, {useEffect} from 'react';
import {AbsoluteFill, getRemotionEnvironment, Img, staticFile, useCurrentFrame} from 'remotion';
import manifestJson from '../../public/ubi/ubi-manifest.json';
import {alpha, color} from '../design/tokens';
import {kf, type Keyframed} from '../design/motion';
import type {StoryboardUbiSegment} from '../storyboard';

type ClipEntry = {
	name: string;
	dir: string;
	frames: number;
	loopable: boolean;
	first_frame_equals?: {clip: string; frame: number};
	last_frame_equals?: {clip: string; frame: number};
};

type UbiManifest = {
	fps: number;
	size: [number, number];
	anchors_px: Record<string, [number, number]>;
	feet_bottom_y_px: number;
	clips: ClipEntry[];
	compositing_tips: string[];
};

const MANIFEST = manifestJson as unknown as UbiManifest;

export type UbiClipName =
	| 'idle'
	| 'yes'
	| 'no'
	| 'wave'
	| 'jump'
	| 'excited'
	| 'worried'
	| 'sleep'
	| 'turntable'
	| 'look'
	| 'wave-from-idle'
	| 'yes-from-idle'
	| 'no-from-idle'
	| 'jump-from-idle'
	| 'idle-to-excited'
	| 'idle-to-worried'
	| 'idle-to-sleep';

/** Clip table from the manifest: frames, loopable, splice info. */
export const UBI_CLIPS: Record<UbiClipName, ClipEntry> = Object.fromEntries(MANIFEST.clips.map((c) => [c.name, c])) as Record<
	UbiClipName,
	ClipEntry
>;

/** Size of a clip frame in px (900). */
export const UBI_FRAME = MANIFEST.size[0];

/** Named anchor points in the 900-px frame (idle frame 0; one camera for every clip). */
export const UBI_ANCHORS = {
	/** Centre of the helmet shell (point speech bubbles here). */
	headCenter: {x: 449.3, y: 302.8},
	headTop: {x: 449.3, y: 160.9},
	/** Bottom of the soles. */
	feet: {x: 450, y: MANIFEST.feet_bottom_y_px},
	/** Floor point under the body centre (soles reach ~7 px below it). */
	ground: {x: 450, y: MANIFEST.anchors_px.ground?.[1] ?? 789.7},
	/** Centre of the 900-px frame. */
	center: {x: 450, y: 450},
	topLeft: {x: 0, y: 0},
	/** Idle body ink box ≈ x 262–624, y 157–797. */
	body: {x: 443, y: 477},
} as const;

export type UbiAnchor = keyof typeof UBI_ANCHORS | {x: number; y: number};

const clipName = (clip: string): UbiClipName => {
	const n = clip.replace(/^ubi\//, '') as UbiClipName;
	if (!UBI_CLIPS[n]) throw new Error(`UbiClip: unknown clip "${clip}". Clips: ${Object.keys(UBI_CLIPS).join(', ')}`);
	return n;
};

/** Path (relative to public/) of frame `index` of `clip`. */
export const ubiFrameSrc = (clip: UbiClipName | string, index: number): string => {
	const c = UBI_CLIPS[clipName(clip)];
	const i = Math.max(0, Math.min(c.frames - 1, Math.round(index)));
	return `${c.dir}/${String(i).padStart(4, '0')}.png`;
};

export type UbiPlayOptions = {
	/** Scene frame at which `playFrom` is shown. Default 0. */
	startFrame?: number;
	/** First clip index. Default 0. */
	playFrom?: number;
	/** Loop the clip (default: the clip's own `loopable`). Non-looping clips hold their last frame. */
	loop?: boolean;
	/** Speed multiplier. Default 1. */
	playbackRate?: number;
};

/**
 * Clip index to show at scene frame `frame`. Before `startFrame` the first
 * index holds; a non-looping clip holds its last frame after the end.
 */
export const ubiFrameIndex = (clip: UbiClipName | string, frame: number, opts: UbiPlayOptions = {}): number => {
	const c = UBI_CLIPS[clipName(clip)];
	const t = Math.max(0, frame - (opts.startFrame ?? 0)) * (opts.playbackRate ?? 1);
	const raw = (opts.playFrom ?? 0) + Math.floor(t + 1e-6);
	const loop = opts.loop ?? c.loopable;
	if (loop) return ((raw % c.frames) + c.frames) % c.frames;
	return Math.max(0, Math.min(c.frames - 1, raw));
};

/**
 * Idle start index so that an idle segment of `n` frames ENDS on idle frame
 * 119 (to splice a *-from-idle / idle-to-* clip right after it).
 */
export const idleLeadIn = (n: number): number => (120 - (n % 120)) % 120;

/** Resolve a storyboard ubiTrack at scene frame `frame` → {clip, index} or null (hidden / no segment). */
export const ubiTrackAt = (segments: StoryboardUbiSegment[], frame: number): {clip: UbiClipName; index: number} | null => {
	const seg = segments.find((s) => frame >= s.from && frame <= s.to);
	if (!seg || seg.clip === 'hidden') return null;
	const clip = clipName(seg.clip);
	const c = UBI_CLIPS[clip];
	const start = seg.startIndex ?? 0;
	const index = seg.hold ? start : start + (frame - seg.from);
	return {clip, index: c.loopable ? ((index % c.frames) + c.frames) % c.frames : Math.max(0, Math.min(c.frames - 1, index))};
};

/* ------------------------------------------------------------------------ */
/* Preloading (Studio playback only; rendering waits on <Img> anyway)         */
/* ------------------------------------------------------------------------ */

const preloaded = new Set<string>();
const preload = (paths: string[]) => {
	if (typeof window === 'undefined') return;
	for (const p of paths) {
		if (preloaded.has(p)) continue;
		preloaded.add(p);
		const im = new Image();
		im.decoding = 'async';
		im.src = staticFile(p);
	}
};

/* ------------------------------------------------------------------------ */
/* Components                                                                */
/* ------------------------------------------------------------------------ */

export type UbiPlacement = {
	/** Rendered size of the 900-px frame, px. Default 600. Keyframeable. */
	size?: Keyframed;
	/** Canvas x of the anchor point. Default 960. Keyframeable. */
	x?: Keyframed;
	/** Canvas y of the anchor point. Default 900. Keyframeable. */
	y?: Keyframed;
	/** Which point of the clip frame sits on (x, y). Default "feet". */
	anchor?: UbiAnchor;
	/** Opacity 0–1. Keyframeable. Default 1. */
	opacity?: Keyframed;
	/** Float amplitude in px (0 = off). The floor glow/shadow stay on the floor. Default 0. */
	float?: number;
	/** Float period in frames. Default 126 (4.2 s). */
	floatPeriod?: number;
	/** Soft volt elliptical floor glow under the feet: true, or an opacity 0–1 / colour. Default false. */
	floorGlow?: boolean | number | {color?: string; opacity?: number; width?: number};
	/** Contact shadow under the feet: true or an opacity 0–1. Default false. */
	shadow?: boolean | number;
	/** Volt rim light (CSS drop-shadow; costs a little per frame). Default false. */
	rim?: boolean | string;
	/** Mirror horizontally. */
	flip?: boolean;
	/** Extra style on the outer layer. */
	style?: React.CSSProperties;
	/** How many upcoming frames to preload in the Studio. Default 8. */
	preloadAhead?: number;
};

export type UbiClipProps = UbiPlacement &
	UbiPlayOptions & {
		clip: UbiClipName | `ubi/${UbiClipName}`;
		/** Force a specific index (custom retiming); overrides start/playFrom/loop. */
		index?: number;
	};

type DrawProps = UbiPlacement & {clip: UbiClipName; index: number; frame: number; next: string[]};

const floorGlowSpec = (g: UbiPlacement['floorGlow']) => {
	if (!g) return null;
	if (g === true) return {color: color.volt, opacity: 0.35, width: 1};
	if (typeof g === 'number') return {color: color.volt, opacity: g, width: 1};
	return {color: g.color ?? color.volt, opacity: g.opacity ?? 0.35, width: g.width ?? 1};
};

const UbiDraw: React.FC<DrawProps> = ({clip, index, frame, next, ...p}) => {
	const size = kf(p.size, frame, 600);
	const x = kf(p.x, frame, 960);
	const y = kf(p.y, frame, 900);
	const opacity = kf(p.opacity, frame, 1);
	const k = size / UBI_FRAME;
	const a = typeof p.anchor === 'object' ? p.anchor : UBI_ANCHORS[p.anchor ?? 'feet'];
	const left = x - a.x * k;
	const top = y - a.y * k;
	const bob = p.float ? Math.sin((frame / (p.floatPeriod ?? 126)) * Math.PI * 2) * p.float : 0;
	const feetX = left + UBI_ANCHORS.feet.x * k;
	const feetY = top + UBI_ANCHORS.feet.y * k;
	const glow = floorGlowSpec(p.floorGlow);
	const shadowOpacity = p.shadow === true ? 0.45 : typeof p.shadow === 'number' ? p.shadow : 0;
	// the lift from the float shrinks the shadow a touch
	const lift = Math.max(0, -bob);
	const shadowScale = 1 - Math.min(0.25, lift / (size * 0.4));
	const rim = p.rim ? (typeof p.rim === 'string' ? p.rim : alpha(color.volt, 0.35)) : null;

	const nextKey = next.join('|');
	useEffect(() => {
		if (!getRemotionEnvironment().isRendering && nextKey) preload(nextKey.split('|'));
	}, [nextKey]);

	if (opacity <= 0.001) return null;
	return (
		<AbsoluteFill style={{pointerEvents: 'none', opacity, ...p.style}}>
			{glow ? (
				<div
					style={{
						position: 'absolute',
						left: feetX - size * 0.36 * glow.width,
						top: feetY - size * 0.07,
						width: size * 0.72 * glow.width,
						height: size * 0.14,
						borderRadius: '50%',
						background: `radial-gradient(closest-side, ${alpha(glow.color, glow.opacity)} 0%, ${alpha(glow.color, glow.opacity * 0.4)} 45%, transparent 100%)`,
					}}
				/>
			) : null}
			{shadowOpacity > 0 ? (
				<div
					style={{
						position: 'absolute',
						left: feetX - size * 0.19 * shadowScale,
						top: feetY - size * 0.022,
						width: size * 0.38 * shadowScale,
						height: size * 0.044,
						borderRadius: '50%',
						background: `radial-gradient(closest-side, rgba(0,0,0,${shadowOpacity * shadowScale}) 0%, rgba(0,0,0,${shadowOpacity * 0.5 * shadowScale}) 55%, transparent 100%)`,
					}}
				/>
			) : null}
			<Img
				src={staticFile(ubiFrameSrc(clip, index))}
				style={{
					position: 'absolute',
					left,
					top: top + bob,
					width: size,
					height: size,
					transform: p.flip ? 'scaleX(-1)' : undefined,
					filter: rim ? `drop-shadow(0 0 ${Math.round(30 * k * 1.5)}px ${rim})` : undefined,
				}}
			/>
		</AbsoluteFill>
	);
};

/**
 * One UBI clip on the canvas.
 *
 * @example
 * // idle loop, feet on (1560, 820), 600 px frame, with life
 * <UbiClip clip="idle" x={1560} y={820} size={600} float={8} floorGlow shadow />
 * // jump from its apex (index 15) starting at scene frame 9, then hold the rest pose
 * <UbiClip clip="jump" startFrame={9} playFrom={15} x={1250} y={660} size={600} />
 */
export const UbiClip: React.FC<UbiClipProps> = ({clip, index, startFrame, playFrom, loop, playbackRate, preloadAhead = 8, ...place}) => {
	const frame = useCurrentFrame();
	const name = clipName(clip);
	const opts = {startFrame, playFrom, loop, playbackRate};
	const i = index ?? ubiFrameIndex(name, frame, opts);
	const next: string[] = [];
	for (let d = 1; d <= preloadAhead; d++) next.push(ubiFrameSrc(name, index ?? ubiFrameIndex(name, frame + d, opts)));
	return <UbiDraw clip={name} index={i} frame={frame} next={next} preloadAhead={preloadAhead} {...place} />;
};

export type UbiTrackProps = UbiPlacement & {
	/** The storyboard's ubiTrack for the scene (scene.ubiTrack). */
	segments: StoryboardUbiSegment[];
};

/**
 * Plays a storyboard ubiTrack exactly (index = startIndex + (f − from),
 * hold freezes, "hidden" draws nothing). Placement props are shared by all
 * segments so the clips splice in place; keyframe x/y/size for moves.
 *
 * @example <UbiTrack segments={useScene().ubiTrack} x={1383} y={660} size={600} floorGlow />
 */
export const UbiTrack: React.FC<UbiTrackProps> = ({segments, preloadAhead = 8, ...place}) => {
	const frame = useCurrentFrame();
	const cur = ubiTrackAt(segments, frame);
	const next: string[] = [];
	for (let d = 1; d <= preloadAhead; d++) {
		const n = ubiTrackAt(segments, frame + d);
		if (n) next.push(ubiFrameSrc(n.clip, n.index));
	}
	if (!cur) return null;
	return <UbiDraw clip={cur.clip} index={cur.index} frame={frame} next={next} preloadAhead={preloadAhead} {...place} />;
};
