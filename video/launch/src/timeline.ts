/**
 * Master timeline of the film, DERIVED from brief/storyboard.json (via
 * src/storyboard.ts). Do not edit frames here — edit the storyboard.
 *
 * 120 BPM at 30 fps: BEAT = 15, BAR = 60. TOTAL = 1560 frames (52 s, 26 bars).
 */
import {FPS} from './design/tokens';
import {SCENES, sceneById, type SceneId, type SceneSpec} from './storyboard';

export type SceneSlot = {
	/** Stable id (= storyboard scene id) used by Launch.tsx to pick the scene component. */
	id: SceneId;
	/** Start frame in the master timeline (= storyboard startFrame). */
	from: number;
	/** Length in frames. */
	durationInFrames: number;
	/** What happens (for humans): the storyboard's purpose line. */
	note?: string;
};

const scenes: SceneSlot[] = SCENES.map((s) => ({id: s.id, from: s.startFrame, durationInFrames: s.durationInFrames, note: s.purpose}));

/** Film length in frames (1560). */
export const TOTAL = Math.max(...scenes.map((s) => s.from + s.durationInFrames));

export const TIMELINE = {
	fps: FPS,
	scenes,
	durationInFrames: TOTAL,
} as const;

/* ------------------------------------------------------------------------ */
/* Act groups (review compositions G1..G6)                                   */
/* ------------------------------------------------------------------------ */

export type GroupId = 'G1' | 'G2' | 'G3' | 'G4' | 'G5' | 'G6';

export type Group = {
	id: GroupId;
	/** Scene ids, back to back. */
	scenes: SceneId[];
	/** Absolute start frame of the group's first scene. */
	startFrame: number;
	durationInFrames: number;
};

const GROUP_SCENES: Record<GroupId, SceneId[]> = {
	G1: ['s01-hook-terca', 's02-so-um-minuto', 's03-planilha-de-memoria', 's04-silencio'],
	G2: ['s05-drop-ubiqx', 's06-ele-registra'],
	G3: ['s07-as-suas-categorias', 's08-regras-memoria-ia', 's09-so-pergunta'],
	G4: ['s10-tecla-1', 's11-e-ele-aprende', 's12-um-clique', 's13-nada-em-duvida'],
	G5: ['s14-relatorio', 's15-foco', 's16-sua-ia'],
	G6: ['s17-privacidade', 's18-end-card'],
};

export const GROUPS: readonly Group[] = (Object.keys(GROUP_SCENES) as GroupId[]).map((id) => {
	const list = GROUP_SCENES[id];
	const specs: SceneSpec[] = list.map((s) => sceneById(s));
	const startFrame = specs[0].startFrame;
	const last = specs[specs.length - 1];
	return {id, scenes: list, startFrame, durationInFrames: last.startFrame + last.durationInFrames - startFrame};
});

export const groupById = (id: GroupId): Group => {
	const g = GROUPS.find((x) => x.id === id);
	if (!g) throw new Error(`timeline: unknown group ${id}`);
	return g;
};

/** The group a scene belongs to. */
export const groupOf = (scene: SceneId): Group => {
	const g = GROUPS.find((x) => x.scenes.includes(scene));
	if (!g) throw new Error(`timeline: scene ${scene} is in no group`);
	return g;
};

/* Groups must tile the film exactly. */
(() => {
	let t = 0;
	for (const g of GROUPS) {
		if (g.startFrame !== t) throw new Error(`timeline: group ${g.id} starts at ${g.startFrame}, expected ${t}`);
		t += g.durationInFrames;
	}
	if (t !== TOTAL) throw new Error(`timeline: groups cover ${t} frames, film is ${TOTAL}`);
})();
