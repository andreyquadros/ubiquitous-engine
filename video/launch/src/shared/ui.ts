/**
 * UI capture helpers: named hotspots from public/ui/ui-manifest.json (imported
 * at build time) and hi-res twins (public/ui/<name>@3x.png listed in
 * public/ui/hires.json by the recapture agent — the file may be absent).
 *
 * ALL coordinates (hotspots, camera rects, cursor, callouts, patches) are in
 * the 2x capture's pixel space (2880x1800 for the standard captures), even
 * when the hi-res bitmap is drawn.
 */
import {useEffect, useRef, useState} from 'react';
import {continueRender, delayRender, staticFile} from 'remotion';
import manifestJson from '../../public/ui/ui-manifest.json';
import {hasStaticFile} from '../components/Sfx';
import type {Rect} from '../storyboard';

type ManifestHotspot = {name: string; x: number; y: number; w: number; h: number};
type ManifestEntry = {
	file: string;
	route?: string;
	theme?: string;
	width: number;
	height: number;
	dpr: number;
	description_pt?: string;
	description_en?: string;
	hotspots: ManifestHotspot[];
};

const MANIFEST = manifestJson as unknown as ManifestEntry[];
const byFile = new Map<string, ManifestEntry>(MANIFEST.map((e) => [e.file, e]));

/** "ui/dashboard.png" | "dashboard.png" | "dashboard" | "ui/dashboard@3x.png" → "dashboard.png". */
export const uiFileName = (file: string): string => {
	let base = file.replace(/^.*\//, '');
	base = base.replace(/@\d+x(\.png)?$/i, '$1');
	return /\.png$/i.test(base) ? base : `${base}.png`;
};

const entryOf = (file: string): ManifestEntry => {
	const name = uiFileName(file);
	const e = byFile.get(name);
	if (!e) throw new Error(`ui: "${file}" is not in public/ui/ui-manifest.json. Captures: ${[...byFile.keys()].join(', ')}`);
	return e;
};

/** All capture file names in the manifest. */
export const UI_FILES: readonly string[] = MANIFEST.map((e) => e.file);

/**
 * Named hotspot rect {x, y, w, h} in the capture's 2x pixel space.
 * Throws a clear error listing the valid names when missing.
 *
 * @example hotspot('ui/categories.png', 'category-editor')
 */
export const hotspot = (file: string, name: string): Rect => {
	const e = entryOf(file);
	const h = e.hotspots.find((x) => x.name === name);
	if (!h) {
		throw new Error(`ui: hotspot "${name}" not found in ${e.file}. Available: ${e.hotspots.map((x) => x.name).join(', ')}`);
	}
	return {x: h.x, y: h.y, w: h.w, h: h.h};
};

/** Does `file` have a hotspot called `name`? */
export const hasHotspot = (file: string, name: string): boolean => {
	const e = byFile.get(uiFileName(file));
	return Boolean(e && e.hotspots.some((x) => x.name === name));
};

/** Every hotspot of a capture, by name. */
export const hotspots = (file: string): Record<string, Rect> =>
	Object.fromEntries(entryOf(file).hotspots.map((h) => [h.name, {x: h.x, y: h.y, w: h.w, h: h.h}]));

/** Centre point of a hotspot. */
export const hotspotCenter = (file: string, name: string): {x: number; y: number} => {
	const r = hotspot(file, name);
	return {x: r.x + r.w / 2, y: r.y + r.h / 2};
};

/* ------------------------------------------------------------------------ */
/* Hi-res twins                                                              */
/* ------------------------------------------------------------------------ */

export type HiresEntry = {
	/** Base capture file name, e.g. "review-done.png". */
	file: string;
	/** Hi-res file name in public/ui/, e.g. "review-done@3x.png". */
	hires: string;
	/** Hi-res pixels per 2x-capture pixel (1.5 for a DPR-3 twin of a DPR-2 capture). */
	scale: number;
};

type HiresTable = Map<string, HiresEntry>;

let loaded: HiresTable | null = null;
let loading: Promise<HiresTable> | null = null;

const HIRES_JSON = 'ui/hires.json';

const parseTable = (json: unknown): HiresTable => {
	const t: HiresTable = new Map();
	const list = Array.isArray(json) ? json : Array.isArray((json as {files?: unknown})?.files) ? (json as {files: unknown[]}).files : [];
	for (const item of list as Partial<HiresEntry>[]) {
		if (!item || typeof item.file !== 'string' || typeof item.hires !== 'string') continue;
		const file = uiFileName(item.file);
		const hires = item.hires.replace(/^.*\//, '');
		if (!hasStaticFile(`ui/${hires}`)) continue; // listed but not shipped: ignore
		t.set(file, {file, hires, scale: typeof item.scale === 'number' && item.scale > 0 ? item.scale : 1.5});
	}
	return t;
};

/* Absent hires.json → known empty table right away (no delayRender ever). */
if (!hasStaticFile(HIRES_JSON)) loaded = new Map();

/** Load public/ui/hires.json once (resolves to an empty table when the file is absent). */
export const loadHiresTable = (): Promise<HiresTable> => {
	if (loaded) return Promise.resolve(loaded);
	if (loading) return loading;
	if (!hasStaticFile(HIRES_JSON)) {
		loaded = new Map();
		return Promise.resolve(loaded);
	}
	loading = fetch(staticFile(HIRES_JSON))
		.then((r) => (r.ok ? r.json() : []))
		.catch(() => [])
		.then((json) => {
			loaded = parseTable(json);
			return loaded;
		});
	return loading;
};

/**
 * Hi-res twin of a capture, if any. Uses hires.json once it has loaded
 * (see useHires / loadHiresTable) and otherwise falls back to the naming
 * convention `<name>@3x.png` present in public/ui/ (scale 1.5).
 */
export const hiresOf = (file: string): HiresEntry | null => {
	const name = uiFileName(file);
	const fromJson = loaded?.get(name);
	if (fromJson) return fromJson;
	const conv = name.replace(/\.png$/i, '@3x.png');
	if (hasStaticFile(`ui/${conv}`)) return {file: name, hires: conv, scale: 1.5};
	return null;
};

/**
 * React hook: the hi-res twin of `file` (or null), waiting (delayRender) for
 * public/ui/hires.json on first use so the rendered frame uses the right
 * bitmap. Returns null when `file` is not a ui/ capture.
 */
export const useHires = (file: string | null | undefined): HiresEntry | null => {
	const [table, setTable] = useState<HiresTable | null>(() => (loaded ? loaded : (loadHiresTable(), loaded)));
	const handle = useRef<number | null>(null);
	if (!table && handle.current === null) handle.current = delayRender('ui/hires.json');
	useEffect(() => {
		let alive = true;
		if (!table) {
			loadHiresTable().then((t) => {
				if (alive) setTable(t);
			});
		}
		return () => {
			alive = false;
		};
	}, [table]);
	useEffect(() => {
		if (table && handle.current !== null) {
			continueRender(handle.current);
			handle.current = null;
		}
	}, [table]);
	useEffect(
		() => () => {
			// unmounted before the table arrived: never leave a render blocked
			if (handle.current !== null) {
				continueRender(handle.current);
				handle.current = null;
			}
		},
		[],
	);
	if (!file || !isUiCapture(file)) return null;
	return hiresOf(file);
};

/** Is `src` one of the manifest's captures (path under ui/ or a bare file name)? */
export const isUiCapture = (src: string): boolean => {
	if (/^(https?:|data:|blob:)/.test(src)) return false;
	if (src.includes('/') && !/(^|\/)ui\//.test(src)) return false;
	return byFile.has(uiFileName(src));
};

/* ------------------------------------------------------------------------ */
/* uiImage                                                                   */
/* ------------------------------------------------------------------------ */

export type UiImage = {
	/** Path relative to public/ of the 2x capture ("ui/dashboard.png"). Pass it to <Screen src>. */
	src: string;
	/** Manifest file name ("dashboard.png"). */
	file: string;
	/** Pixel size of the 2x capture — the coordinate space of every hotspot. */
	width: number;
	height: number;
	dpr: number;
	/** The hi-res twin, when shipped: drawn by <Screen> automatically. */
	hires?: {src: string; scale: number; width: number; height: number};
	/** <Screen imageSize> for this capture. */
	imageSize: {w: number; h: number};
};

/**
 * Everything about a capture: `{src, width, height, hires?}`.
 *
 * @example
 * const img = uiImage('review-done.png');
 * <Screen src={img.src} imageSize={img.imageSize} … />
 */
export const uiImage = (file: string): UiImage => {
	const e = entryOf(file);
	const h = hiresOf(e.file);
	return {
		src: `ui/${e.file}`,
		file: e.file,
		width: e.width,
		height: e.height,
		dpr: e.dpr,
		imageSize: {w: e.width, h: e.height},
		...(h ? {hires: {src: `ui/${h.hires}`, scale: h.scale, width: Math.round(e.width * h.scale), height: Math.round(e.height * h.scale)}} : {}),
	};
};

/**
 * Bitmap upsample factor of a capture shown at Screen `width` (default 1440)
 * and camera `zoom`: 0.5 × zoom for a 2880-px capture, divided by the hi-res
 * scale when the @3x twin is drawn. Style cap: ≤ 1.40 hard, prefer ≤ 1.15.
 */
export const bitmapUpsample = (file: string, zoom: number, width = 1440): number => {
	const img = uiImage(file);
	const s = (zoom * width) / img.width;
	return img.hires ? s / img.hires.scale : s;
};
