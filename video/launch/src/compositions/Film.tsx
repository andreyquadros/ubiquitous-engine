/**
 * <Film from to>: renders EXACTLY frames [from, to) of the film — the scene
 * Sequences that intersect the window plus the MASTER AUDIO layer — with
 * frame 0 = film frame `from`. Launch = Film 0→1560; the G1..G6 group
 * compositions are slices of it; the S01..S18 scene compositions use it with
 * `sfxScope` = that scene only.
 *
 * Scenes are absolute <Sequence from durationInFrames premountFor={30}>:
 * boundaries are hard cuts, transitions are split halves INSIDE the scenes
 * (src/shared/transitions.tsx). The master audio lives OUTSIDE the scene
 * Sequences so no sound is cut at a scene boundary:
 *   - music: public/audio/music/music.wav (skipped when absent), volume
 *     musicVolume(absFrame) = BED × ducks (src/shared/audio.ts: MUSIC, DUCKS);
 *   - every scene's `sfx` cue at abs = scene.startFrame + atFrame − round(hit_offset_s × 30),
 *     volume BED × 10^(gainDb/20); cues that start before `from` are trimmed.
 */
import React from 'react';
import {AbsoluteFill, Html5Audio, Sequence, staticFile} from 'remotion';
import {hasStaticFile} from '../components/Sfx';
import {color} from '../design/tokens';
import {SCENE_MODULES} from '../scenes';
import {MUSIC, musicVolume, placeCue, type PlacedCue} from '../shared/audio';
import {SceneProvider} from '../shared/scene';
import {SCENES, type SceneId} from '../storyboard';

export type FilmProps = {
	/** First film frame (inclusive). */
	from: number;
	/** Last film frame (exclusive). */
	to: number;
	/** Which scenes' SFX cues to play: 'all' (default) or a list of scene ids. */
	sfxScope?: 'all' | SceneId[];
	/** Play the music bed. Default true. */
	music?: boolean;
	/** Play SFX. Default true. */
	sfx?: boolean;
};

/** Every SFX cue of the film, placed at absolute frames (for QA tools and the audio layer). */
export const placedCues = (scope: 'all' | SceneId[] = 'all'): (PlacedCue & {scene: SceneId})[] =>
	SCENES.filter((s) => scope === 'all' || scope.includes(s.id)).flatMap((s) =>
		SCENE_MODULES[s.id].sfx.map((cue) => ({...placeCue(s.startFrame, cue), scene: s.id})),
	);

export const MasterAudio: React.FC<Omit<FilmProps, 'music' | 'sfx'> & {music?: boolean; sfx?: boolean}> = ({
	from,
	to,
	sfxScope = 'all',
	music = true,
	sfx = true,
}) => {
	const len = to - from;
	const musicOn = music && hasStaticFile(MUSIC.file);
	const musicStart = MUSIC.startFrame - from; // local frame where music.wav's first sample plays
	return (
		<>
			{musicOn ? (
				<Sequence from={Math.max(0, musicStart)} durationInFrames={len - Math.max(0, musicStart)} layout="none" name="music">
					<Html5Audio
						src={staticFile(MUSIC.file)}
						trimBefore={musicStart < 0 ? -musicStart : undefined}
						// f is relative to this Sequence: film frame = from + max(0, musicStart) + f
						volume={(f) => musicVolume(from + Math.max(0, musicStart) + f)}
					/>
				</Sequence>
			) : null}
			{sfx
				? placedCues(sfxScope).map((c, i) => {
						if (c.absEnd <= from || c.absStart >= to) return null;
						if (!hasStaticFile(c.info.path)) return null;
						const localStart = c.absStart - from;
						const trim = localStart < 0 ? -localStart : 0;
						const start = Math.max(0, localStart);
						const dur = Math.min(c.absEnd - c.absStart - trim, len - start);
						if (dur <= 0) return null;
						return (
							<Sequence key={`${c.scene}-${i}`} from={start} durationInFrames={dur} layout="none" name={`sfx ${c.cue.ref} @${c.absHit}`}>
								<Html5Audio
									src={staticFile(c.info.path)}
									volume={c.volume}
									playbackRate={c.cue.playbackRate ?? 1}
									trimBefore={trim > 0 ? Math.round(trim * (c.cue.playbackRate ?? 1)) : undefined}
								/>
							</Sequence>
						);
					})
				: null}
		</>
	);
};

export const Film: React.FC<FilmProps> = ({from, to, sfxScope = 'all', music = true, sfx = true}) => (
	<AbsoluteFill style={{backgroundColor: color.canvas}}>
		{SCENES.filter((s) => s.startFrame < to && s.startFrame + s.durationInFrames > from).map((s) => {
			const C = SCENE_MODULES[s.id].component;
			const start = s.startFrame - from;
			return (
				<Sequence key={s.id} from={start} durationInFrames={s.durationInFrames} name={s.id} premountFor={30}>
					<SceneProvider scene={s}>
						<C />
					</SceneProvider>
				</Sequence>
			);
		})}
		<MasterAudio from={from} to={to} sfxScope={sfxScope} music={music} sfx={sfx} />
	</AbsoluteFill>
);
