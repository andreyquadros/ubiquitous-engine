/**
 * Pure geometry for <Screen>: where the window is, how it is tilted, where the
 * camera is looking — and the inverse question every overlay needs:
 * "where on the canvas is pixel (x, y) of the screenshot right now?".
 *
 * Coordinate spaces
 *  - image px:        pixels of the source screenshot (e.g. 2880x1800)
 *  - composition px:  the 1920x1080 canvas, after 3D tilt, perspective and camera
 *
 * The same function drives the DOM transforms in Screen.tsx and the mapping
 * used by Cursor / Callout, so overlays stay glued to the UI.
 */
import {BAR, ease} from '../design/tokens';
import {kf, lerp, progress, rad, type EaseFn, type Keyframed} from '../design/motion';

export type Rect = {x: number; y: number; w: number; h: number};
export type Point = {x: number; y: number};

export type CameraKey = {
	/** Frame at which the camera ARRIVES at this shot (move happens before it). */
	at: number;
	/** Rect to frame, in image px. `'full'` (default) = the whole window, no zoom. */
	rect?: Rect | 'full';
	/** Explicit zoom factor (overrides fit). Combine with `focus` or `rect`. */
	zoom?: number;
	/** Point (image px) to centre when using `zoom` without `rect`. */
	focus?: Point;
	/** Fraction of the canvas the rect should fill (by its limiting side). Default 0.7. */
	fit?: number;
	/** Upper bound for fitted zoom. Default 3.2. */
	maxZoom?: number;
	/** Frames the move takes, ending at `at`. Default 24 (≈ 1.6 beats). 0 = hard cut. */
	duration?: number;
	/** Easing of the move. Default ease.inOut. */
	easing?: EaseFn;
	/** Where on the canvas the target is centred (composition px). Default canvas centre. */
	anchor?: Point;
};

export type SpotlightSpec = {
	/** Area to keep lit, image px. */
	rect: Rect;
	/** Frame the spotlight starts fading in. */
	at: number;
	/** Frame it starts fading out. Omit to keep it. */
	until?: number;
	/** Fade duration in frames. Default 10. */
	fade?: number;
	/** Darkness outside the rect, 0–1. Default 0.62. */
	dim?: number;
	/** Outline/glow colour (palette name or CSS). Default volt. */
	color?: string;
	/** Corner radius in image px. Default 20. */
	radius?: number;
	/** Padding around the rect in image px. Default 14. */
	pad?: number;
	/** Draw the glowing outline. Default true. */
	outline?: boolean;
};

export type ScreenEnter = 'rise' | 'zoom' | 'fade' | 'tilt' | 'none';
export type ScreenExit = 'sink' | 'zoom' | 'fade' | 'none';

/**
 * Everything that defines a Screen shot. Define it once per scene as a const,
 * pass it to <Screen {...cfg}/> AND to <Cursor screen={cfg}/> / <Callout screen={cfg}/>.
 */
export type ScreenConfig = {
	/** Screenshot path relative to public/ (e.g. "ui/dashboard.png"), or an absolute/remote URL. */
	src: string;
	/**
	 * Hi-res bitmap. Default (undefined / true): when public/ui/<name>@3x.png is
	 * shipped (listed in public/ui/hires.json), draw it instead of `src`. The
	 * coordinate space (imageSize, camera rects, hotspots, children, cursor,
	 * callouts) stays the 2x capture's. `false` = always draw `src`; a string =
	 * draw that file (path under public/) as the hi-res bitmap.
	 */
	hires?: boolean | string;
	/** Natural pixel size of the screenshot. Default {w: 2880, h: 1800}. MUST match the file. */
	imageSize?: {w: number; h: number};
	/** Rendered width of the window content in composition px (before camera). Default 1440. */
	width?: number;
	/** Window-centre offset from canvas centre, px (keyframeable). */
	x?: Keyframed;
	/** Window-centre offset from canvas centre, px (keyframeable). */
	y?: Keyframed;
	/** Extra window scale around its centre (keyframeable). Default 1. */
	scale?: Keyframed;
	/** Tilt around the horizontal axis, degrees (keyframeable). Positive = top leans away. */
	rotateX?: Keyframed;
	/** Tilt around the vertical axis, degrees (keyframeable). Positive = right side leans away. */
	rotateY?: Keyframed;
	/** In-plane rotation, degrees (keyframeable). */
	rotateZ?: Keyframed;
	/** CSS perspective distance in px. Smaller = more dramatic. Default 2400. */
	perspective?: number;
	/** Float bob amplitude in px (0 = off). Default 0. */
	float?: number;
	/** Float period in frames. Default 2 bars (120). */
	floatPeriod?: number;
	/** Camera keys (see CameraKey). Omit for a static full view. */
	camera?: CameraKey[];
	/** Window chrome: macOS title bar with traffic lights, or bare image. Default "mac". */
	chrome?: 'mac' | 'none';
	/**
	 * Title-bar dots: "mac" traffic lights (default) or "neutral" #3a4560 dots
	 * for platform-agnostic shots (style §S10; the storyboard's UI planes).
	 */
	dots?: 'mac' | 'neutral';
	/** Optional centred title in the title bar. */
	title?: string;
	/** Window corner radius in composition px (at width 1440). Default 14. */
	radius?: number;
	/** Spotlights (dim everything outside a rect + glowing outline). */
	spotlights?: SpotlightSpec[];
	/** Ambient glow colour under the window, or false. Default "volt". */
	glow?: string | false;
	/** Entry animation. Default "none". */
	enter?: ScreenEnter;
	/** Entry start frame. Default 0. */
	enterAt?: number;
	/** Entry duration. Default 30. */
	enterDuration?: number;
	/** Exit animation. Default "none". */
	exit?: ScreenExit;
	/** Exit start frame. */
	exitAt?: number;
	/** Exit duration. Default 18. */
	exitDuration?: number;
	/** Frame at which a diagonal light sheen sweeps across the glass. */
	sheenAt?: number;
};

export const DEFAULT_IMAGE = {w: 2880, h: 1800};

export type ScreenGeometry = {
	/** image px → content px */
	s0: number;
	imgW: number;
	imgH: number;
	contentW: number;
	contentH: number;
	titleH: number;
	winW: number;
	winH: number;
	/** window centre (composition px, pre-camera, pre-bob) */
	cx: number;
	cy: number;
	/** window top-left (composition px, pre-camera) */
	left: number;
	top: number;
	rx: number;
	ry: number;
	rz: number;
	scale: number;
	bob: number;
	opacity: number;
	blur: number;
	perspective: number;
	/** camera: comp = C + k * (p - f) */
	k: number;
	fx: number;
	fy: number;
	/** canvas centre */
	Cx: number;
	Cy: number;
};

type Pose = {rx: number; ry: number; rz: number; scale: number; cx: number; cy: number; bob: number};

/** Project a window-local point (px from window centre, before scale) through the 3D pose. No camera. */
const projectLocal = (lx: number, ly: number, pose: Pose, P: number): Point => {
	// scale
	let x = lx * pose.scale;
	let y = ly * pose.scale;
	let z = 0;
	// rotateZ
	const cz = Math.cos(rad(pose.rz));
	const sz = Math.sin(rad(pose.rz));
	[x, y] = [cz * x - sz * y, sz * x + cz * y];
	// rotateY
	const cyy = Math.cos(rad(pose.ry));
	const syy = Math.sin(rad(pose.ry));
	[x, z] = [cyy * x + syy * z, -syy * x + cyy * z];
	// rotateX
	const cxx = Math.cos(rad(pose.rx));
	const sxx = Math.sin(rad(pose.rx));
	[y, z] = [cxx * y - sxx * z, sxx * y + cxx * z];
	// bob translate
	y += pose.bob;
	// perspective about the window centre
	const f = P / Math.max(1, P - z);
	return {x: pose.cx + x * f, y: pose.cy + y * f};
};

/** Compute the full geometry of a Screen at `frame`. */
export const screenGeometry = (cfg: ScreenConfig, frame: number, comp: {width: number; height: number}): ScreenGeometry => {
	const imgW = cfg.imageSize?.w ?? DEFAULT_IMAGE.w;
	const imgH = cfg.imageSize?.h ?? DEFAULT_IMAGE.h;
	const contentW = cfg.width ?? 1440;
	const s0 = contentW / imgW;
	const contentH = imgH * s0;
	const titleH = cfg.chrome === 'none' ? 0 : Math.round(contentW * 0.026);
	const winW = contentW;
	const winH = contentH + titleH;
	const P = cfg.perspective ?? 2400;
	const Cx = comp.width / 2;
	const Cy = comp.height / 2;

	let ox = kf(cfg.x, frame, 0);
	let oy = kf(cfg.y, frame, 0);
	let rx = kf(cfg.rotateX, frame, 0);
	let ry = kf(cfg.rotateY, frame, 0);
	let rz = kf(cfg.rotateZ, frame, 0);
	let scale = kf(cfg.scale, frame, 1);
	let opacity = 1;
	let blur = 0;

	// entry
	const enter = cfg.enter ?? 'none';
	if (enter !== 'none') {
		const p = progress(frame, cfg.enterAt ?? 0, cfg.enterDuration ?? 30, ease.push);
		const q = 1 - p;
		opacity *= Math.min(1, p * 2.2);
		if (enter === 'rise') {
			oy += q * 260;
			rx += q * 32;
			scale *= lerp(0.9, 1, p);
			blur += q * 10;
		} else if (enter === 'zoom') {
			scale *= lerp(0.6, 1, p);
			blur += q * 16;
		} else if (enter === 'tilt') {
			ry += q * -38;
			rx += q * 18;
			ox += q * 220;
			scale *= lerp(0.88, 1, p);
		}
	}
	// exit
	const exit = cfg.exit ?? 'none';
	if (exit !== 'none' && cfg.exitAt !== undefined) {
		const q = progress(frame, cfg.exitAt, cfg.exitDuration ?? 18, ease.exit);
		opacity *= 1 - q;
		if (exit === 'sink') {
			oy += q * 240;
			rx += q * 24;
			scale *= lerp(1, 0.92, q);
			blur += q * 10;
		} else if (exit === 'zoom') {
			scale *= lerp(1, 1.5, q);
			blur += q * 18;
		}
	}

	const bob = cfg.float ? Math.sin((frame / (cfg.floatPeriod ?? BAR * 2)) * Math.PI * 2) * cfg.float : 0;
	const cx = Cx + ox;
	const cy = Cy + oy;
	const pose: Pose = {rx, ry, rz, scale, cx, cy, bob};

	// camera
	const cam = cameraAt(cfg, frame, (r) => {
		// bbox of projected rect corners (image px → comp, no camera)
		const pts = [
			[r.x, r.y],
			[r.x + r.w, r.y],
			[r.x, r.y + r.h],
			[r.x + r.w, r.y + r.h],
		].map(([ix, iy]) => projectLocal(ix * s0 - winW / 2, titleH + iy * s0 - winH / 2, pose, P));
		const xs = pts.map((p) => p.x);
		const ys = pts.map((p) => p.y);
		const minX = Math.min(...xs);
		const minY = Math.min(...ys);
		return {x: minX, y: minY, w: Math.max(...xs) - minX, h: Math.max(...ys) - minY};
	}, (pt) => projectLocal(pt.x * s0 - winW / 2, titleH + pt.y * s0 - winH / 2, pose, P), comp);

	return {
		s0,
		imgW,
		imgH,
		contentW,
		contentH,
		titleH,
		winW,
		winH,
		cx,
		cy,
		left: cx - winW / 2,
		top: cy - winH / 2,
		rx,
		ry,
		rz,
		scale,
		bob,
		opacity,
		blur,
		perspective: P,
		k: cam.k,
		fx: cam.fx,
		fy: cam.fy,
		Cx,
		Cy,
	};
};

type CamState = {k: number; fx: number; fy: number};

const cameraAt = (
	cfg: ScreenConfig,
	frame: number,
	projectRect: (r: Rect) => Rect,
	projectPoint: (p: Point) => Point,
	comp: {width: number; height: number},
): CamState => {
	const Cx = comp.width / 2;
	const Cy = comp.height / 2;
	const identity: CamState = {k: 1, fx: Cx, fy: Cy};
	const keys = [...(cfg.camera ?? [])].sort((a, b) => a.at - b.at);
	if (keys.length === 0) return identity;

	const target = (key: CameraKey): CamState => {
		const ax = key.anchor?.x ?? Cx;
		const ay = key.anchor?.y ?? Cy;
		let k = 1;
		let fx = Cx;
		let fy = Cy;
		if (key.rect && key.rect !== 'full') {
			const r = projectRect(key.rect);
			const fit = key.fit ?? 0.7;
			k = key.zoom ?? Math.min((comp.width * fit) / r.w, (comp.height * fit) / r.h, key.maxZoom ?? 3.2);
			fx = r.x + r.w / 2;
			fy = r.y + r.h / 2;
		} else if (key.focus) {
			const p = projectPoint(key.focus);
			k = key.zoom ?? 1.6;
			fx = p.x;
			fy = p.y;
		} else if (key.zoom) {
			k = key.zoom;
		}
		// anchor: shift the focus so the target lands on the anchor instead of the centre
		// comp = C + k (p - f)  → want anchor = C + k (target - f)  → f = target - (anchor - C) / k
		fx -= (ax - Cx) / k;
		fy -= (ay - Cy) / k;
		return {k, fx, fy};
	};

	let prev = identity;
	for (let i = 0; i < keys.length; i++) {
		const key = keys[i];
		const dur = key.duration ?? 24;
		const start = key.at - dur;
		const tgt = target(key);
		if (frame < start) return prev;
		if (frame < key.at && dur > 0) {
			const e = progress(frame, start, dur, key.easing ?? ease.inOut);
			return {
				k: Math.exp(lerp(Math.log(prev.k), Math.log(tgt.k), e)),
				fx: lerp(prev.fx, tgt.fx, e),
				fy: lerp(prev.fy, tgt.fy, e),
			};
		}
		prev = tgt;
	}
	return prev;
};

/**
 * Map an image-pixel point to composition px for a Screen at `frame`
 * (includes tilt, bob, perspective and camera).
 */
export const mapImagePoint = (cfg: ScreenConfig, frame: number, comp: {width: number; height: number}, pt: Point): Point => {
	const g = screenGeometry(cfg, frame, comp);
	return mapWithGeometry(g, pt);
};

/** Same as mapImagePoint but reuses a precomputed geometry. */
export const mapWithGeometry = (g: ScreenGeometry, pt: Point): Point => {
	const pose: Pose = {rx: g.rx, ry: g.ry, rz: g.rz, scale: g.scale, cx: g.cx, cy: g.cy, bob: g.bob};
	const p = projectLocal(pt.x * g.s0 - g.winW / 2, g.titleH + pt.y * g.s0 - g.winH / 2, pose, g.perspective);
	return {x: g.Cx + g.k * (p.x - g.fx), y: g.Cy + g.k * (p.y - g.fy)};
};

/** Map an image-pixel rect to its composition-px bounding box. */
export const mapImageRect = (cfg: ScreenConfig, frame: number, comp: {width: number; height: number}, r: Rect): Rect => {
	const g = screenGeometry(cfg, frame, comp);
	const pts = [
		{x: r.x, y: r.y},
		{x: r.x + r.w, y: r.y},
		{x: r.x, y: r.y + r.h},
		{x: r.x + r.w, y: r.y + r.h},
	].map((p) => mapWithGeometry(g, p));
	const xs = pts.map((p) => p.x);
	const ys = pts.map((p) => p.y);
	const x = Math.min(...xs);
	const y = Math.min(...ys);
	return {x, y, w: Math.max(...xs) - x, h: Math.max(...ys) - y};
};

/** Effective on-screen scale of one image pixel (camera × window scale × s0), ignoring tilt. */
export const onScreenScale = (g: ScreenGeometry): number => g.k * g.scale * g.s0;
