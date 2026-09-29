/**
 * Typed access to brief/storyboard.json — the SOURCE OF TRUTH of the film
 * (absolute frames, 30 fps, 1560 frames, 18 scenes, exact PT-BR copy, camera
 * keys, cursor paths, transitions, SFX cues, UBI tracks).
 *
 * Never copy numbers out of the storyboard by hand when you can read them
 * from here: `sceneById('s05-drop-ubiqx').copy[0].text`.
 *
 * Frames inside a scene (copy in/land/out, camera atFrame, sfx atFrame,
 * ubiTrack from/to…) are SCENE-RELATIVE unless a field says "abs".
 */
import raw from '../brief/storyboard.json';

/* ------------------------------------------------------------------------ */
/* Types                                                                     */
/* ------------------------------------------------------------------------ */

export type SceneId =
	| 's01-hook-terca'
	| 's02-so-um-minuto'
	| 's03-planilha-de-memoria'
	| 's04-silencio'
	| 's05-drop-ubiqx'
	| 's06-ele-registra'
	| 's07-as-suas-categorias'
	| 's08-regras-memoria-ia'
	| 's09-so-pergunta'
	| 's10-tecla-1'
	| 's11-e-ele-aprende'
	| 's12-um-clique'
	| 's13-nada-em-duvida'
	| 's14-relatorio'
	| 's15-foco'
	| 's16-sua-ia'
	| 's17-privacidade'
	| 's18-end-card';

/** Every scene id, in film order. */
export const SCENE_IDS: readonly SceneId[] = [
	's01-hook-terca',
	's02-so-um-minuto',
	's03-planilha-de-memoria',
	's04-silencio',
	's05-drop-ubiqx',
	's06-ele-registra',
	's07-as-suas-categorias',
	's08-regras-memoria-ia',
	's09-so-pergunta',
	's10-tecla-1',
	's11-e-ele-aprende',
	's12-um-clique',
	's13-nada-em-duvida',
	's14-relatorio',
	's15-foco',
	's16-sua-ia',
	's17-privacidade',
	's18-end-card',
] as const;

export type Act = 'hook' | 'problem' | 'reveal' | 'features' | 'proof' | 'cta';

/** Transition types used by the storyboard (split halves, see src/shared/transitions.tsx). */
export type TransitionType = 'cut' | 'flash' | 'match-cut' | 'whip-left' | 'blur-dissolve';

export type TransitionHalf = {
	type: TransitionType;
	/** Frames of THIS half inside THIS scene (0 = empty half). */
	frames: number;
	note?: string;
};

export type Rect = {x: number; y: number; w: number; h: number};
export type Point = {x: number; y: number};

export type CopyLine = {
	text: string;
	role: string;
	/** Scene-relative frame the line starts entering. */
	inFrame: number;
	/** Scene-relative frame the line has landed (spring ≥ 0.9). */
	landFrame: number;
	/** Scene-relative frame the line starts leaving (or the scene end). */
	outFrame: number;
	emphasis: string[];
	mode: string;
	group?: string;
	groupMode?: string;
	note?: string;
};

export type StoryboardCameraKey = {
	/** Scene-relative frame the camera ARRIVES. */
	atFrame: number;
	/** A hotspot name of `file`, "canvas" or "full". */
	target: string;
	zoom: number;
	tilt?: {rx: number; ry: number};
	/** E.push | E.glide | E.exit | linear | hard… | hold… | SLAM | SMOOTH | free text. */
	ease: string;
	/** Travel frames before atFrame (when given). */
	duration?: number;
	file?: string;
	screenWidth?: number;
	effectiveScale?: number;
	upsample?: number;
	/** Canvas px where `focus` is placed. */
	anchor?: Point;
	/** Image px placed on `anchor`. */
	focus?: Point;
	readTarget?: {text: string; captureFontPx: number; lift: number; onScreenPx: number; crisp: string; ok: boolean} | null;
	textureReason?: string;
	layer?: string;
	note?: string;
};

export type StoryboardCursorKey = {
	atFrame: number;
	/** A hotspot name of the scene's capture, or "rest:x,y" in canvas px. */
	to: string;
	click: boolean;
	note?: string;
};

export type StoryboardSpotlight = {
	rect?: Rect;
	hotspot?: string;
	file: string;
	from: number;
	to: number;
	dim: number;
};

export type StoryboardPatch = {
	file: string;
	rect: Rect;
	color: string;
	from: number;
	to: number;
	covers?: string;
	note?: string;
};

export type StoryboardScrim = {rect: Rect; solid?: boolean; from: number; to: number; note?: string};

export type StoryboardUbiSegment = {
	from: number;
	to: number;
	/** "ubi/<clip>" or "hidden". */
	clip: string;
	startIndex?: number;
	/** Freeze the last index. */
	hold?: boolean;
	note?: string;
};

export type StoryboardSfxCue = {
	ref: string;
	/** Scene-relative frame where the HIT lands. */
	atFrame: number;
	gainDb: number;
	fileStartFrame: number;
	hitOffsetFrames: number;
	note?: string;
};

export type StoryboardAsset = {kind: string; ref: string; hotspots: string[]; note?: string};

export type SceneSpec = {
	id: SceneId;
	/** Absolute start frame in the film. */
	startFrame: number;
	durationInFrames: number;
	act: Act;
	absRange: string;
	timecode: string;
	barBeat: string;
	purpose: string;
	why: string;
	thumbnail: string;
	shotTypes: string[];
	copy: CopyLine[];
	visual: string;
	assets: StoryboardAsset[];
	camera: StoryboardCameraKey[];
	spotlights: StoryboardSpotlight[];
	patches: StoryboardPatch[];
	scrims: StoryboardScrim[];
	cursor: StoryboardCursorKey[];
	ubiTrack: StoryboardUbiSegment[];
	motion: string;
	transitionIn: TransitionHalf;
	transitionOut: TransitionHalf;
	sfx: StoryboardSfxCue[];
	shots: number[];
	readingCheck: string;
	sfxNote?: string;
};

export type Storyboard = {
	title: string;
	angle: string;
	fps: number;
	width: number;
	height: number;
	bpm: number;
	durationInFrames: number;
	conventions: Record<string, string>;
	guards: string[];
	synthesis: unknown;
	ubiMoments: {scene: string; clip: string; beat: string}[];
	music: {
		key: string;
		bpm: number;
		meter: string;
		lengthFrames: number;
		lengthBars: number;
		progression: string;
		style: string;
		mix: string;
		stems: string;
		sections: {
			name: string;
			startBar: number;
			bars: number;
			energy: number;
			description: string;
			events: {frame: number; type: string; note?: string; durationFrames?: number}[];
		}[];
	};
	qa: string[];
	scenes: SceneSpec[];
};

/* ------------------------------------------------------------------------ */
/* Data                                                                      */
/* ------------------------------------------------------------------------ */

export const STORYBOARD: Storyboard = raw as unknown as Storyboard;

/** All 18 scenes in film order. */
export const SCENES: readonly SceneSpec[] = STORYBOARD.scenes;

/** Scene number 1..18 → "S01".."S18" (the per-scene composition id). */
export const sceneCompId = (id: SceneId): string => `S${String(SCENE_IDS.indexOf(id) + 1).padStart(2, '0')}`;

const byId = new Map<string, SceneSpec>(SCENES.map((s) => [s.id, s]));

/** Look a scene up by id; throws with the list of valid ids when unknown. */
export const sceneById = (id: SceneId | string): SceneSpec => {
	const s = byId.get(id);
	if (!s) throw new Error(`storyboard: unknown scene "${id}". Valid: ${SCENE_IDS.join(', ')}`);
	return s;
};

/** The scene playing at absolute frame `abs` (clamped to the film). */
export const sceneAt = (abs: number): SceneSpec => {
	for (const s of SCENES) if (abs >= s.startFrame && abs < s.startFrame + s.durationInFrames) return s;
	return abs < 0 ? SCENES[0] : SCENES[SCENES.length - 1];
};

/** Absolute end frame (exclusive) of a scene. */
export const sceneEnd = (s: SceneSpec): number => s.startFrame + s.durationInFrames;

/* Sanity: fail loudly at bundle time if the JSON and the id list drift apart. */
(() => {
	const ids = SCENES.map((s) => s.id).join(',');
	if (ids !== SCENE_IDS.join(',')) throw new Error(`storyboard.json scene ids changed: ${ids}`);
	let t = 0;
	for (const s of SCENES) {
		if (s.startFrame !== t) throw new Error(`storyboard: ${s.id} starts at ${s.startFrame}, expected ${t} (scenes must be contiguous)`);
		t += s.durationInFrames;
	}
	if (t !== STORYBOARD.durationInFrames) throw new Error(`storyboard: scenes sum to ${t}, film is ${STORYBOARD.durationInFrames}`);
})();
