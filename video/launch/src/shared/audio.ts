/**
 * Audio maths shared by the master audio layer and the scenes.
 *
 * Levels follow brief/style.md §7.3: every SFX gain is in dB RELATIVE TO THE
 * BED, and the bed's nominal volume is BED = 0.5 (−6 dB). So an SFX cue's
 * Remotion volume is `BED × 10^(gainDb/20)` (click −12 → 0.125, impact +2 → 0.63).
 *
 * Cue timing follows the storyboard convention: `atFrame` is where the HIT
 * lands (transient / loudest point / a riser's END); the file starts
 * `round(hit_offset_s × 30)` frames earlier (hit offsets from
 * public/audio/sfx/sfx-manifest.json, aliases resolved).
 */
import manifestJson from '../../public/audio/sfx/sfx-manifest.json';
import {FPS} from '../design/tokens';

/* ------------------------------------------------------------------------ */
/* dB helpers                                                                */
/* ------------------------------------------------------------------------ */

/** dB → linear gain. `db(-6)` ≈ 0.5, `db(0)` = 1. */
export const db = (d: number): number => Math.pow(10, d / 20);
/** Alias of {@link db}. */
export const dbToGain = db;
/** Linear gain → dB (−Infinity for 0). */
export const gainToDb = (g: number): number => (g <= 0 ? -Infinity : 20 * Math.log10(g));

/** Nominal bed (music) volume: 0.5 = −6 dB. Every SFX gainDb is relative to it. */
export const BED = 0.5;

/** Remotion volume of an SFX cue whose level is `gainDb` relative to the bed. */
export const sfxVolume = (gainDb: number): number => BED * db(gainDb);

/* ------------------------------------------------------------------------ */
/* SFX manifest                                                              */
/* ------------------------------------------------------------------------ */

type ManifestEntry = {
	name: string;
	file: string;
	duration_s: number;
	hit_offset_s: number;
	end_is_downbeat: boolean;
	alias_of?: string;
	pitch?: string;
	suggested_level_db?: number;
	character?: string;
};

const MANIFEST = manifestJson as unknown as ManifestEntry[];
const byFile = new Map<string, ManifestEntry>(MANIFEST.map((e) => [e.file, e]));

export type SfxInfo = {
	/** The name as written in the cue (may be an alias like "click.wav"). */
	ref: string;
	/** The real file in public/audio/sfx/ after resolving aliases. */
	file: string;
	/** Path relative to public/ (for staticFile). */
	path: string;
	durationS: number;
	/** Length in frames (ceil). */
	durationFrames: number;
	hitOffsetS: number;
	/** round(hit_offset_s × 30): frames between file start and the hit. */
	hitOffsetFrames: number;
	/** The file's end IS the hit (risers, reverse swells). */
	endIsDownbeat: boolean;
};

/** Normalise "click", "click.wav", "audio/sfx/click.wav" → "click.wav". */
const normalise = (ref: string): string => {
	const base = ref.replace(/^.*\//, '');
	return /\.wav$/i.test(base) ? base : `${base}.wav`;
};

/**
 * Resolve an SFX reference (file name or alias from sfx-manifest.json) to
 * its real file, duration and hit offset. Throws if the name is unknown.
 */
export const sfxInfo = (ref: string): SfxInfo => {
	const name = normalise(ref);
	const entry = byFile.get(name);
	if (!entry) {
		throw new Error(`sfx: "${ref}" is not in public/audio/sfx/sfx-manifest.json. Known: ${[...byFile.keys()].sort().join(', ')}`);
	}
	// Aliases carry their own (copied) hit offset, but resolve to the real file anyway.
	let real = entry;
	const seen = new Set<string>();
	while (real.alias_of && !seen.has(real.file)) {
		seen.add(real.file);
		const next = byFile.get(real.alias_of);
		if (!next) break;
		real = next;
	}
	return {
		ref,
		file: real.file,
		path: `audio/sfx/${real.file}`,
		durationS: real.duration_s,
		durationFrames: Math.ceil(real.duration_s * FPS - 1e-6),
		hitOffsetS: entry.hit_offset_s,
		hitOffsetFrames: Math.round(entry.hit_offset_s * FPS),
		endIsDownbeat: entry.end_is_downbeat,
	};
};

/** Every file name (and alias) listed in the SFX manifest. */
export const SFX_NAMES: readonly string[] = MANIFEST.map((e) => e.file);

/* ------------------------------------------------------------------------ */
/* Cues                                                                      */
/* ------------------------------------------------------------------------ */

/**
 * One SFX cue of a scene. Each scene module exports `sfx: SfxCue[]`; the
 * master audio layer (src/compositions/Film.tsx) places them all at absolute
 * frames so no sound is cut at a scene boundary.
 */
export type SfxCue = {
	/** File name or alias from public/audio/sfx (e.g. "click.wav", "pop+2.wav", "riser-2bar.wav"). */
	ref: string;
	/** SCENE-RELATIVE frame where the HIT / peak / riser end must land. May be < 0 or ≥ the scene length. */
	atFrame: number;
	/** Level in dB relative to the bed (BED = 0.5). */
	gainDb: number;
	/** Optional playback rate (pitch + speed). Default 1. */
	playbackRate?: number;
	/** Optional max length in frames (cuts a long tail). */
	maxFrames?: number;
	/** Human note (ignored). */
	note?: string;
};

/** Where a cue's FILE starts, relative to the same origin as `atFrame`. */
export const cueFileStart = (cue: SfxCue): number => cue.atFrame - sfxInfo(cue.ref).hitOffsetFrames;

export type PlacedCue = {
	cue: SfxCue;
	info: SfxInfo;
	/** Absolute film frame where the file starts (can be before the scene). */
	absStart: number;
	/** Absolute film frame (exclusive) where the file ends. */
	absEnd: number;
	/** Absolute frame of the hit. */
	absHit: number;
	volume: number;
};

/** Place a scene cue on the film timeline. */
export const placeCue = (sceneStart: number, cue: SfxCue): PlacedCue => {
	const info = sfxInfo(cue.ref);
	const absHit = sceneStart + cue.atFrame;
	const absStart = absHit - info.hitOffsetFrames;
	const len = cue.maxFrames !== undefined ? Math.min(cue.maxFrames, info.durationFrames) : Math.ceil(info.durationFrames / (cue.playbackRate ?? 1));
	return {cue, info, absStart, absEnd: absStart + len, absHit, volume: sfxVolume(cue.gainDb)};
};

/* ------------------------------------------------------------------------ */
/* Music + ducking                                                           */
/* ------------------------------------------------------------------------ */

export type Duck = {
	/** Absolute film frame of the hit. */
	at: number;
	/** Depth in dB (negative). */
	depthDb: number;
};

/**
 * Bed ducks from the storyboard's music mix: −5 dB on the slams, −6 dB on
 * the drop, −4 dB on the final hit. Never duck for clicks, ticks or pops.
 * The integrator tunes these (or disables them if music.wav already has
 * the ducks baked in) via MUSIC below.
 */
export const DUCKS: Duck[] = [
	{at: 105, depthDb: -5},
	{at: 180, depthDb: -5},
	{at: 240, depthDb: -6},
	{at: 510, depthDb: -5},
	{at: 525, depthDb: -5},
	{at: 540, depthDb: -5},
	{at: 1035, depthDb: -5},
	{at: 1380, depthDb: -4},
];

/** Duck envelope shape (frames). style.md §7.4: 1 f attack, 2 f hold, 10 f release. */
export const DUCK_ENVELOPE = {attack: 1, hold: 2, release: 10};

/**
 * Gain (linear) of the duck envelope at absolute frame `f`: 1 when no duck
 * is active; overlapping ducks take the deepest value.
 */
export const duckGain = (f: number, ducks: Duck[] = DUCKS, env = DUCK_ENVELOPE): number => {
	let g = 0;
	for (const d of ducks) {
		const t = f - d.at;
		if (t < -env.attack || t > env.hold + env.release) continue;
		const k = t < 0 ? (t + env.attack) / env.attack : t <= env.hold ? 1 : 1 - (t - env.hold) / env.release;
		g = Math.min(g, d.depthDb * k);
	}
	return db(g);
};

/**
 * Music settings for the master audio layer. Tolerates the file being
 * absent (the composer writes public/audio/music/music.wav).
 */
export const MUSIC = {
	/** Path relative to public/. */
	file: 'audio/music/music.wav',
	/** Base volume (the bed). */
	volume: BED,
	/** Apply DUCKS on top of the base volume. Set false if the mix already ducks. */
	ducking: true,
	ducks: DUCKS,
	/** Frame of the film where music.wav's first sample plays. */
	startFrame: 0,
};

/** Music volume at absolute film frame `f` (base × ducks). */
export const musicVolume = (f: number): number => MUSIC.volume * (MUSIC.ducking ? duckGain(f, MUSIC.ducks) : 1);
