/**
 * Registers the brand faces (Sora + Inter, variable, latin + latin-ext) from
 * public/fonts/. `@remotion/fonts` wraps every face in delayRender /
 * continueRender, so no frame is captured before the fonts are ready.
 *
 * Import this module once (src/Root.tsx does it) — calling `ensureFonts()`
 * repeatedly is a no-op.
 */
import {loadFont} from '@remotion/fonts';
import {staticFile} from 'remotion';

const LATIN =
	'U+0000-00FF,U+0131,U+0152-0153,U+02BB-02BC,U+02C6,U+02DA,U+02DC,U+0304,U+0308,U+0329,U+2000-206F,U+20AC,U+2122,U+2191,U+2193,U+2212,U+2215,U+FEFF,U+FFFD';
const LATIN_EXT =
	'U+0100-02BA,U+02BD-02C5,U+02C7-02CC,U+02CE-02D7,U+02DD-02FF,U+0304,U+0308,U+0329,U+1D00-1DBF,U+1E00-1E9F,U+1EF2-1EFF,U+2020,U+20A0-20AB,U+20AD-20C0,U+2113,U+2C60-2C7F,U+A720-A7FF';

const FACES = [
	{family: 'Sora', file: 'fonts/sora-latin-wght-normal.woff2', unicodeRange: LATIN},
	{family: 'Sora', file: 'fonts/sora-latin-ext-wght-normal.woff2', unicodeRange: LATIN_EXT},
	{family: 'Inter', file: 'fonts/inter-latin-wght-normal.woff2', unicodeRange: LATIN},
	{family: 'Inter', file: 'fonts/inter-latin-ext-wght-normal.woff2', unicodeRange: LATIN_EXT},
] as const;

let started: Promise<void> | null = null;

/** Load all brand faces (idempotent). Returns a promise for convenience. */
export const ensureFonts = (): Promise<void> => {
	if (started) return started;
	if (typeof document === 'undefined') return Promise.resolve();
	started = Promise.all(
		FACES.map((f) =>
			loadFont({
				family: f.family,
				url: staticFile(f.file),
				weight: '100 900',
				style: 'normal',
				display: 'block',
				format: 'woff2',
				unicodeRange: f.unicodeRange,
			}),
		),
	).then(() => undefined);
	return started;
};

ensureFonts();
