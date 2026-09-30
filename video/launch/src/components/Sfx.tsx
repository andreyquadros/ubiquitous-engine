import React from 'react';
import {getStaticFiles, Html5Audio, interpolate, Sequence, staticFile} from 'remotion';

let cache: Set<string> | null = null;

/** Is `name` (path relative to public/) present in the bundle's static files? */
export const hasStaticFile = (name: string): boolean => {
	if (!cache) {
		try {
			const files = getStaticFiles();
			cache = new Set(files.map((f) => f.name.replace(/\\/g, '/')));
		} catch {
			cache = new Set();
		}
	}
	return cache.has(name.replace(/^\/+/, ''));
};

export type SfxProps = {
	/** File name inside public/audio/sfx/ (e.g. "whoosh-1.wav") or a path relative to public/ containing a "/". */
	src: string;
	/** Frame the sound starts (local to the enclosing Sequence). Default 0. */
	at?: number;
	/** Volume 0–1+ (static). Default 1. */
	volume?: number;
	/** Playback rate (pitch + speed). Default 1. */
	playbackRate?: number;
	/** Trim the start of the file, in frames. */
	trimBefore?: number;
	/** Max length in frames (avoids long tails bleeding into the next scene). */
	duration?: number;
};

/**
 * Places a sound effect at a frame. If the file is missing from public/ the
 * cue silently renders nothing (the SFX library is built in parallel), so it
 * is always safe to leave cues in a scene.
 *
 * @example
 * <Sfx src="whoosh.wav" at={28} volume={0.7} />
 */
export const Sfx: React.FC<SfxProps> = ({src, at = 0, volume = 1, playbackRate = 1, trimBefore, duration}) => {
	const rel = src.includes('/') ? src : `audio/sfx/${src}`;
	if (!hasStaticFile(rel)) return null;
	return (
		<Sequence from={at} durationInFrames={duration} layout="none" name={`sfx ${src}`}>
			<Html5Audio src={staticFile(rel)} volume={volume} playbackRate={playbackRate} trimBefore={trimBefore} />
		</Sequence>
	);
};

export type MusicProps = {
	/** File name inside public/audio/music/ or a path relative to public/ containing a "/". */
	src: string;
	/** Frame the music starts. Default 0. */
	at?: number;
	/** Base volume. Default 0.9. */
	volume?: number;
	/** Fade-in frames. Default 0. */
	fadeIn?: number;
	/** Total length in frames (used for fade-out). Required for fadeOut. */
	duration?: number;
	/** Fade-out frames at the end of `duration`. Default 0. */
	fadeOut?: number;
	/** Skip into the track, frames. */
	trimBefore?: number;
};

/** Soundtrack bed with fades. Missing file → renders nothing. */
export const Music: React.FC<MusicProps> = ({src, at = 0, volume = 0.9, fadeIn = 0, duration, fadeOut = 0, trimBefore}) => {
	const rel = src.includes('/') ? src : `audio/music/${src}`;
	if (!hasStaticFile(rel)) return null;
	return (
		<Sequence from={at} durationInFrames={duration} layout="none" name={`music ${src}`}>
			<Html5Audio
				src={staticFile(rel)}
				trimBefore={trimBefore}
				volume={(f) => {
					let v = volume;
					if (fadeIn > 0) v *= interpolate(f, [0, fadeIn], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
					if (fadeOut > 0 && duration)
						v *= interpolate(f, [duration - fadeOut, duration], [1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
					return v;
				}}
			/>
		</Sequence>
	);
};
