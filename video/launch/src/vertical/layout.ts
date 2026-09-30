/**
 * 9:16 cut (Reels / TikTok): per-scene panel layout.
 *
 * The vertical film is rebuilt from the rendered 16:9 picture (public/vertical/film.mp4). Each scene
 * shows one or more PANELS. A panel crops a rect of the 16:9 frame (`src`, 1920x1080 px) and places
 * it on the 1080x1920 canvas at `dst` (x, y, width; the height follows the crop's aspect). Panel
 * edges are feathered into the ambient background (public/vertical/bg.mp4, the same film blurred), so
 * the frame reads as one stage.
 *
 * The idea: restack, don't letterbox. The headline always goes on top, even when it sits at the bottom
 * of the 16:9 frame (s09), and the subject (a UI card, UBI, the key) sits large below it.
 *
 * Safe zones (TikTok / Reels UI): keep what must be read inside y 250–1500 and away from the right
 * 140 px between y 900 and 1600 (the like/comment/share column).
 *
 * `keys` animate the crop and its placement (scene-relative frames, eased in-out), which is how the
 * cut keeps the film's match cuts: s04's caret sits where s05's X pops, s05's UBI hands off to s06's
 * pull-back, and s10's key flies into s11's picker.
 */
import type {SceneId} from '../storyboard';

export type VRect = {x: number; y: number; w: number; h: number};
export type VDst = {x: number; y: number; w: number};
export type VKey = {f: number; src: VRect; dst: VDst};

export type VPanel = {
	id: string;
	/** One pose, or keyframes (scene-relative frames, ascending). */
	keys: VKey[];
	/** First / last scene-relative frame the panel is on (inclusive). Default: the whole scene. */
	from?: number;
	to?: number;
	/** Fade in/out length at `from` / `to`, frames. Default 0 (cut). */
	fade?: number;
	/** Feather width in canvas px: one number, or [top, right, bottom, left]. Default 48. */
	feather?: number | [number, number, number, number];
	/** Stacking order; higher draws on top. Default 1. */
	z?: number;
};

export const VW = 1080;
export const VH = 1920;

const k = (src: VRect, dst: VDst, f = 0): VKey => ({f, src, dst});
const one = (id: string, src: VRect, dst: VDst, extra: Partial<VPanel> = {}): VPanel => ({id, keys: [k(src, dst)], ...extra});

/* Shared poses (continuity across cuts) ---------------------------------------------------------- */

/** s05's lockup crop; s04's caret uses the same mapping so the caret sits exactly where the X pops. */
const LOCKUP = {src: {x: 180, y: 370, w: 1120, h: 360}, dst: {x: 0, y: 905, w: 1080}};
/** s05's UBI crop; s06 opens on the same mapping for the UBI match cut, then pulls back. */
const UBI05 = {src: {x: 1200, y: 10, w: 560, h: 728}, dst: {x: 307, y: 250, w: 466}};
/** s10's key crop; s11 opens on it and flies to the picker. */
const KEY10 = {src: {x: 560, y: 200, w: 800, h: 700}, dst: {x: 100, y: 560, w: 880}};

export const VLAYOUT: Record<SceneId, VPanel[]> = {
	/* Hook: question on top, the empty timesheet large below. */
	's01-hook-terca': [
		one('headline', {x: 110, y: 90, w: 1700, h: 270}, {x: 20, y: 350, w: 1040}, {feather: [40, 24, 30, 24]}),
		one('sheet', {x: 110, y: 380, w: 1160, h: 700}, {x: 0, y: 560, w: 1080}, {feather: [60, 70, 90, 70]}),
	],
	/* The "just a minute" flurry: one big panel, chips around the counter. */
	's02-so-um-minuto': [one('frame', {x: 250, y: 60, w: 1420, h: 960}, {x: 0, y: 470, w: 1080}, {feather: [70, 60, 70, 60]})],
	/* Guessing the hours: headline, then the sheet with the red question marks. */
	's03-planilha-de-memoria': [
		one('headline', {x: 370, y: 0, w: 1180, h: 225}, {x: 40, y: 360, w: 1000}, {feather: [40, 30, 30, 30]}),
		one('sheet', {x: 40, y: 232, w: 1400, h: 848}, {x: 0, y: 570, w: 1080}, {feather: [60, 80, 100, 80]}),
	],
	/* Silence: the caret alone, on the spot where s05's X will pop. */
	's04-silencio': [{id: 'caret', keys: [k(LOCKUP.src, LOCKUP.dst)], feather: 80}],
	/* The drop, restacked: UBI on top, the lockup in the middle, the tagline and two product fragments below. */
	's05-drop-ubiqx': [
		{id: 'ubi', keys: [k(UBI05.src, UBI05.dst)], feather: [90, 110, 70, 110], z: 1},
		{id: 'lockup', keys: [k(LOCKUP.src, LOCKUP.dst)], feather: [90, 90, 70, 110], z: 2},
		one('tagline', {x: 370, y: 715, w: 1180, h: 120}, {x: 40, y: 1276, w: 1000}, {feather: [30, 90, 30, 90], z: 2}),
		one('track', {x: 70, y: 880, w: 720, h: 170}, {x: 60, y: 1390, w: 500}, {from: 28, fade: 8, feather: 50, z: 1}),
		one('ring', {x: 1560, y: 820, w: 280, h: 280}, {x: 640, y: 1375, w: 200}, {from: 28, fade: 8, feather: 50, z: 1}),
	],
	/* It records: UBI hand-off, pull back to the dashboard, the headline on top. */
	's06-ele-registra': [
		one('headline', {x: 170, y: 20, w: 1600, h: 190}, {x: 20, y: 330, w: 1040}, {feather: [40, 30, 40, 30], z: 2}),
		{
			id: 'dash',
			keys: [k(UBI05.src, UBI05.dst, 0), k({x: 40, y: 170, w: 1840, h: 910}, {x: 0, y: 540, w: 1080}, 24)],
			feather: [50, 40, 70, 40],
			z: 1,
		},
	],
	/* Your categories: headline, then the description field being typed, big. */
	's07-as-suas-categorias': [
		one('headline', {x: 60, y: 40, w: 1560, h: 210}, {x: 20, y: 360, w: 1040}, {feather: [40, 24, 40, 24]}),
		one('field', {x: 90, y: 340, w: 1470, h: 440}, {x: 0, y: 580, w: 1080}, {feather: [40, 70, 60, 50]}),
		one('below', {x: 200, y: 780, w: 1640, h: 300}, {x: 60, y: 950, w: 960}, {feather: 50}),
	],
	/* Rules, memory, AI: the three slams and their badges as one composition. */
	's08-regras-memoria-ia': [one('frame', {x: 50, y: 170, w: 1560, h: 760}, {x: 0, y: 560, w: 1080}, {feather: [60, 60, 60, 50]})],
	/* It only asks what it doesn't know: the headline moves to the top, the pending rows below. */
	's09-so-pergunta': [
		one('headline', {x: 70, y: 850, w: 1780, h: 170}, {x: 20, y: 350, w: 1040}, {feather: [40, 24, 40, 24], z: 2}),
		one('rows', {x: 20, y: 20, w: 1560, h: 820}, {x: 0, y: 520, w: 1080}, {feather: [50, 60, 70, 40], z: 1}),
	],
	/* The key, alone and big. */
	's10-tecla-1': [{id: 'key', keys: [k(KEY10.src, KEY10.dst)], feather: 110}],
	/* One key, and it learns: headline, the picker (the key flies in), the rule chip. */
	's11-e-ele-aprende': [
		one('headline', {x: 70, y: 340, w: 920, h: 320}, {x: 80, y: 300, w: 920}, {from: 12, fade: 6, feather: [50, 90, 50, 90], z: 2}),
		{
			id: 'picker',
			keys: [k(KEY10.src, KEY10.dst, 0), k({x: 840, y: 50, w: 1060, h: 680}, {x: 40, y: 640, w: 1000}, 12)],
			feather: [60, 60, 70, 120],
			z: 1,
		},
		one('rule', {x: 950, y: 740, w: 960, h: 180}, {x: 60, y: 1290, w: 960}, {from: 20, fade: 6, feather: [24, 40, 24, 40], z: 2}),
	],
	/* One click becomes memory: the hero button first, then headline + the mint wave down the rows. */
	's12-um-clique': [
		one('headline', {x: 50, y: 50, w: 1560, h: 190}, {x: 20, y: 360, w: 1040}, {from: 14, fade: 6, feather: [40, 24, 40, 24], z: 2}),
		{
			id: 'rows',
			keys: [k({x: 620, y: 330, w: 1040, h: 540}, {x: 20, y: 640, w: 1040}, 0), k({x: 620, y: 330, w: 1040, h: 540}, {x: 20, y: 640, w: 1040}, 10), k({x: 20, y: 300, w: 1880, h: 780}, {x: 0, y: 540, w: 1080}, 26)],
			feather: [50, 30, 70, 30],
			z: 1,
		},
	],
	/* Nothing in doubt: the in-app state, then the bubble on top and UBI big below. */
	's13-nada-em-duvida': [
		one('app', {x: 470, y: 150, w: 980, h: 930}, {x: 50, y: 520, w: 980}, {to: 15, fade: 4, feather: 60}),
		one('bubble', {x: 840, y: 250, w: 1040, h: 300}, {x: 30, y: 340, w: 1020}, {from: 13, fade: 4, feather: [30, 40, 30, 40], z: 2}),
		one('ubi', {x: 330, y: 290, w: 560, h: 790}, {x: 275, y: 630, w: 530}, {from: 13, fade: 4, feather: [90, 110, 90, 110], z: 1}),
		one('chip', {x: 900, y: 730, w: 980, h: 160}, {x: 50, y: 1370, w: 980}, {from: 17, fade: 4, feather: [20, 40, 20, 40], z: 2}),
	],
	/* 18:00: the clock big, then the report card with its headline. */
	's14-relatorio': [
		{
			id: 'frame',
			keys: [
				k({x: 420, y: 110, w: 1080, h: 720}, {x: 0, y: 560, w: 1080}, 0),
				k({x: 420, y: 110, w: 1080, h: 720}, {x: 0, y: 560, w: 1080}, 26),
				k({x: 40, y: 0, w: 1860, h: 1080}, {x: 0, y: 520, w: 1080}, 38),
			],
			feather: [150, 40, 150, 40],
		},
	],
	/* Focus that defends itself: headline on top, the UBI intervention modal below. */
	's15-foco': [
		one('headline', {x: 50, y: 50, w: 1380, h: 190}, {x: 20, y: 360, w: 1040}, {feather: [40, 24, 40, 24], z: 2}),
		one('modal', {x: 30, y: 250, w: 1860, h: 820}, {x: 0, y: 570, w: 1080}, {feather: [40, 30, 60, 30], z: 1}),
	],
	/* Your AI: the two-line headline on top, the settings window and the provider pills below. */
	's16-sua-ia': [
		one('headline', {x: 70, y: 640, w: 1700, h: 360}, {x: 20, y: 330, w: 1040}, {feather: [40, 24, 40, 24], z: 2}),
		one('settings', {x: 220, y: 0, w: 1500, h: 600}, {x: 0, y: 590, w: 1080}, {feather: [40, 40, 60, 40], z: 1}),
	],
	/* Privacy: both lines on top, the masking panel, UBI below. */
	's17-privacidade': [
		one('line1', {x: 90, y: 80, w: 1540, h: 160}, {x: 20, y: 300, w: 1040}, {feather: [40, 24, 40, 24], z: 2}),
		one('line2', {x: 650, y: 240, w: 980, h: 160}, {x: 110, y: 410, w: 860}, {feather: [40, 30, 40, 30], z: 2}),
		one('panel', {x: 680, y: 450, w: 1240, h: 600}, {x: 30, y: 560, w: 1020}, {feather: [30, 40, 40, 40], z: 1}),
		one('ubi', {x: 150, y: 350, w: 520, h: 730}, {x: 370, y: 1050, w: 340}, {feather: [70, 90, 70, 90], z: 1}),
	],
	/* End card, restacked: UBI, the wordmark, then tagline + CTA + platforms. */
	's18-end-card': [
		one('ubi', {x: 1170, y: 0, w: 460, h: 560}, {x: 335, y: 290, w: 410}, {feather: [80, 100, 60, 100], z: 1}),
		one('wordmark', {x: 380, y: 200, w: 880, h: 320}, {x: 100, y: 760, w: 880}, {feather: [80, 130, 50, 130], z: 2}),
		one('cta', {x: 420, y: 535, w: 1080, h: 420}, {x: 0, y: 1080, w: 1080}, {feather: [50, 120, 90, 120], z: 2}),
	],
};
