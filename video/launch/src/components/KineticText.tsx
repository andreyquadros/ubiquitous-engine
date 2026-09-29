import React from 'react';
import {useCurrentFrame, useVideoConfig} from 'remotion';
import {BEAT, ease, font, glow, gradient, resolveColor, tracking, type, weight as W, alpha, type Accent} from '../design/tokens';
import {clamp, lerp, progress, springIn} from '../design/motion';

export type KineticMode = 'slam' | 'stagger-words' | 'stagger-chars' | 'mask-up' | 'typewriter' | 'fade';
export type KineticExit = 'fade' | 'blur' | 'up' | 'down' | 'mask' | 'slam' | 'none';
export type HighlightStyle = 'color' | 'glow' | 'gradient' | 'marker';

export type UnderlineSpec = {
	/** Word (or phrase) to underline; omit to underline the whole text block. Case/punctuation-insensitive. */
	word?: string;
	/** Frame the sweep starts. Default: when the entry animation finishes. */
	at?: number;
	/** Sweep duration in frames. Default 12. */
	duration?: number;
	/** Palette accent or CSS colour. Default: the word's highlight colour, else volt. */
	color?: Accent | string;
	/** Thickness in em. Default 0.07. */
	thickness?: number;
	/** Distance below the baseline box in em (negative = lower). Default -0.02. */
	offset?: number;
};

export type KineticTextProps = {
	/** Copy. `\n` forces a line break (each line is a unit for `mask-up`). */
	text: string;
	/** Entry animation. Default "stagger-words". */
	mode?: KineticMode;
	/** Frame the entry starts (local to the enclosing Sequence). Default 0. */
	at?: number;
	/**
	 * Frames between units (word / char / line depending on mode).
	 * Defaults: slam 4 (0 = whole block at once), stagger-words 3, stagger-chars 1.2,
	 * mask-up 5 (per line), typewriter 1.6 (per char), fade 0.
	 */
	stagger?: number;
	/** Frames each unit takes to animate in. Defaults per mode (slam 9, words 18, chars 16, mask 18, fade 14). */
	duration?: number;
	/** Frame the exit starts. Omit for no exit. */
	exitAt?: number;
	/** Exit animation. Default "up". */
	exit?: KineticExit;
	/** Frames each unit takes to exit. Default 10. */
	exitDuration?: number;
	/** Frames between units on exit. Default: 1 (words/chars), 3 (lines). */
	exitStagger?: number;
	/**
	 * Words/phrases to colour: `{ "IA": "volt", "sozinho": "ember" }`. Keys are
	 * matched case-insensitively, ignoring surrounding punctuation; multi-word
	 * keys match consecutive words.
	 */
	highlight?: Record<string, Accent | string>;
	/** How highlighted words are drawn. Default "glow". */
	highlightStyle?: HighlightStyle;
	/** Underline sweep under a word (or the whole block). */
	underline?: UnderlineSpec;
	/** Font role. Default "display" (Sora). */
	role?: 'display' | 'text';
	/** Font size in px. Default type.display (120). */
	size?: number;
	/** Font weight. Default 700 for display, 500 for text. */
	weight?: number;
	/** Base text colour. Default ink. Ignored when `fill="gradient"`. */
	color?: string;
	/** "solid" or a subtle top-lit ink gradient. Default "solid". */
	fill?: 'solid' | 'gradient';
	/** Text alignment. Default "center". */
	align?: 'left' | 'center' | 'right';
	/** Line height (unitless). Default 1.06 display / 1.3 text. */
	lineHeight?: number;
	/** Letter-spacing. Default tracking.display for display, tracking.body for text. */
	letterSpacing?: string;
	/** Max width in px before wrapping. Default none. */
	maxWidth?: number;
	/** Show a blinking caret (typewriter only). Default true. */
	caret?: boolean;
	/** Camera shake on landing (px amplitude, slam only). Default 0. */
	shake?: number;
	/** Extra style on the outer block. */
	style?: React.CSSProperties;
};

type Word = {lead: string; core: string; trail: string; norm: string; line: number; index: number; charStart: number};

const PUNCT = /^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu;
const norm = (s: string) => s.replace(PUNCT, '').toLocaleLowerCase('pt-BR');

const splitWord = (w: string) => {
	const lead = w.match(/^[^\p{L}\p{N}]+/u)?.[0] ?? '';
	const trail = w.slice(lead.length).match(/[^\p{L}\p{N}]+$/u)?.[0] ?? '';
	return {lead, core: w.slice(lead.length, w.length - trail.length), trail};
};

const DEFAULTS: Record<KineticMode, {stagger: number; duration: number}> = {
	slam: {stagger: 4, duration: 9},
	'stagger-words': {stagger: 3, duration: 18},
	'stagger-chars': {stagger: 1.2, duration: 16},
	'mask-up': {stagger: 5, duration: 18},
	typewriter: {stagger: 1.6, duration: 1},
	fade: {stagger: 0, duration: 14},
};

/**
 * Kinetic typography — the workhorse of every text beat.
 *
 * Renders as a block element; position it with a parent (e.g. `<Center>`).
 *
 * @example
 * <KineticText
 *   text={'Controle de tempo\nautomático com IA.'}
 *   mode="mask-up"
 *   highlight={{IA: 'volt'}}
 *   underline={{word: 'automático', color: 'ember'}}
 *   exitAt={50}
 * />
 */
export const KineticText: React.FC<KineticTextProps> = ({
	text,
	mode = 'stagger-words',
	at = 0,
	stagger,
	duration,
	exitAt,
	exit = 'up',
	exitDuration = 10,
	exitStagger,
	highlight,
	highlightStyle = 'glow',
	underline,
	role = 'display',
	size = type.display,
	weight,
	color: baseColor,
	fill = 'solid',
	align = 'center',
	lineHeight,
	letterSpacing,
	maxWidth,
	caret = true,
	shake = 0,
	style,
}) => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	const st = stagger ?? DEFAULTS[mode].stagger;
	const dur = duration ?? DEFAULTS[mode].duration;
	const isDisplay = role === 'display';
	const ink = baseColor ?? '#e8edf9';

	/* ---------- tokenise ---------- */
	const lines = text.split('\n');
	const words: Word[] = [];
	let charCount = 0;
	lines.forEach((ln, li) => {
		ln.split(/\s+/)
			.filter(Boolean)
			.forEach((w) => {
				const parts = splitWord(w);
				words.push({...parts, norm: norm(w), line: li, index: words.length, charStart: charCount});
				charCount += w.length + 1;
			});
	});
	const totalChars = charCount;

	/* ---------- highlight map (supports phrases) ---------- */
	const hl: (string | undefined)[] = new Array(words.length).fill(undefined);
	if (highlight) {
		for (const [key, c] of Object.entries(highlight)) {
			const kw = key.split(/\s+/).map(norm).filter(Boolean);
			for (let i = 0; i + kw.length <= words.length; i++) {
				if (kw.every((k, j) => words[i + j].norm === k)) {
					for (let j = 0; j < kw.length; j++) hl[i + j] = resolveColor(c);
				}
			}
		}
	}
	let ulWords: Set<number> | null = null;
	if (underline?.word) {
		const kw = underline.word.split(/\s+/).map(norm).filter(Boolean);
		ulWords = new Set();
		for (let i = 0; i + kw.length <= words.length; i++) {
			if (kw.every((k, j) => words[i + j].norm === k)) {
				for (let j = 0; j < kw.length; j++) ulWords.add(i + j);
				break;
			}
		}
	}

	/* ---------- timing ---------- */
	const unitCount = mode === 'mask-up' ? lines.length : mode === 'stagger-chars' || mode === 'typewriter' ? totalChars : words.length;
	const entryEnd = at + Math.max(0, unitCount - 1) * st + dur;
	const exStagger = exitStagger ?? (mode === 'mask-up' ? 3 : mode === 'stagger-chars' ? 0.6 : 1);

	const exitQ = (unit: number): number =>
		exitAt === undefined || exit === 'none' ? 0 : progress(frame, exitAt + unit * exStagger, exitDuration, ease.exit);

	/** Entry transform for one unit (word or char) given its start delay. */
	const entryStyle = (delay: number): React.CSSProperties => {
		const start = at + delay;
		switch (mode) {
			case 'slam': {
				const p = progress(frame, start, dur, ease.slam);
				const blur = lerp(22, 0, p);
				return {
					opacity: clamp(p * 2.5),
					transform: `scale(${lerp(2.1, 1, p)})`,
					filter: blur > 0.1 ? `blur(${blur}px)` : undefined,
				};
			}
			case 'stagger-words': {
				const s = springIn(frame, fps, start, 'smooth', dur);
				const blur = lerp(12, 0, s);
				return {
					opacity: clamp(s * 1.4),
					transform: `translateY(${lerp(0.42, 0, s)}em)`,
					filter: blur > 0.1 ? `blur(${blur}px)` : undefined,
				};
			}
			case 'stagger-chars': {
				const s = springIn(frame, fps, start, 'snappy', dur);
				const blur = lerp(8, 0, clamp(s));
				return {
					opacity: clamp(s * 1.6),
					transform: `translateY(${lerp(0.55, 0, s)}em) rotate(${lerp(6, 0, s)}deg)`,
					filter: blur > 0.1 ? `blur(${blur}px)` : undefined,
				};
			}
			case 'fade': {
				const p = progress(frame, start, dur, ease.settle);
				const blur = lerp(14, 0, p);
				return {opacity: p, filter: blur > 0.1 ? `blur(${blur}px)` : undefined};
			}
			default:
				return {};
		}
	};

	const exitStyle = (unit: number): React.CSSProperties => {
		const q = exitQ(unit);
		if (q <= 0) return {};
		switch (exit) {
			case 'fade':
				return {opacity: 1 - q};
			case 'blur':
				return {opacity: 1 - q, filter: `blur(${q * 18}px)`};
			case 'up':
				return {opacity: 1 - q, transform: `translateY(${-0.35 * q}em)`, filter: `blur(${q * 8}px)`};
			case 'down':
				return {opacity: 1 - q, transform: `translateY(${0.35 * q}em)`, filter: `blur(${q * 8}px)`};
			case 'slam':
				return {opacity: 1 - q, transform: `scale(${lerp(1, 0.55, q)})`, filter: `blur(${q * 14}px)`};
			default:
				return {};
		}
	};

	/** Merge entry + exit (transforms are concatenated). */
	const merge = (a: React.CSSProperties, b: React.CSSProperties): React.CSSProperties => {
		const out: React.CSSProperties = {...a, ...b};
		if (a.transform && b.transform) out.transform = `${a.transform} ${b.transform}`;
		if (a.opacity !== undefined && b.opacity !== undefined) out.opacity = (a.opacity as number) * (b.opacity as number);
		if (a.filter && b.filter) out.filter = `${a.filter} ${b.filter}`;
		return out;
	};

	/* ---------- typewriter state ---------- */
	const typed = mode === 'typewriter' ? Math.floor(Math.max(0, frame - at) / Math.max(0.01, st)) : Infinity;
	const typingDone = typed >= totalChars;
	// global index of the last RENDERED char already typed (spaces are not rendered chars)
	let caretIdx = -1;
	if (mode === 'typewriter') {
		for (const w of words) {
			const n = Array.from(w.lead + w.core + w.trail).length;
			for (let ci = 0; ci < n; ci++) if (w.charStart + ci < typed) caretIdx = w.charStart + ci;
		}
	}
	/** Frame a word's marker/highlight sweep starts, per mode. */
	const wordLandsAt = (w: Word): number => {
		switch (mode) {
			case 'typewriter':
				return at + (w.charStart + Array.from(w.lead + w.core + w.trail).length) * st;
			case 'stagger-chars':
				return at + w.charStart * st + dur * 0.4;
			case 'mask-up':
				return at + w.line * st + dur * 0.4;
			case 'slam':
				return at + (st === 0 ? 0 : w.index * st) + dur;
			default:
				return at + w.index * st + dur * 0.4;
		}
	};
	const caretOn = !typingDone && frame >= at ? true : Math.floor((frame - at) / BEAT) % 2 === 0;

	/* ---------- fill ---------- */
	const unitFill = (hlColor?: string): React.CSSProperties => {
		if (hlColor) {
			if (highlightStyle === 'gradient') {
				const g = hlColor.toLowerCase() === '#ff7a1f' ? gradient.emberText : gradient.voltText;
				return {backgroundImage: g, WebkitBackgroundClip: 'text', backgroundClip: 'text', color: 'transparent'};
			}
			if (highlightStyle === 'glow') return {color: hlColor, textShadow: glow.text(hlColor, 0.9)};
			return {color: hlColor};
		}
		if (fill === 'gradient') {
			return {backgroundImage: gradient.inkText, WebkitBackgroundClip: 'text', backgroundClip: 'text', color: 'transparent'};
		}
		return {color: ink};
	};

	/* ---------- underline + marker ---------- */
	const ulAt = underline?.at ?? entryEnd;
	const ulP = underline ? progress(frame, ulAt, underline.duration ?? 12, ease.push) : 0;
	const ulColor = (wi?: number) => resolveColor(underline?.color ?? (wi !== undefined ? hl[wi] : undefined) ?? 'volt');
	const ulBar = (c: string) => (
		<span
			style={{
				position: 'absolute',
				left: 0,
				right: 0,
				bottom: `${underline?.offset ?? -0.02}em`,
				height: `${underline?.thickness ?? 0.07}em`,
				borderRadius: 999,
				background: c,
				boxShadow: `0 0 18px ${alpha(c, 0.6)}`,
				transform: `scaleX(${ulP})`,
				transformOrigin: '0% 50%',
				opacity: ulP > 0 ? 1 : 0,
			}}
		/>
	);

	/* ---------- slam shake ---------- */
	let shakeX = 0;
	let shakeY = 0;
	if (mode === 'slam' && shake > 0) {
		const landed = frame - (at + dur);
		if (landed >= 0 && landed < 10) {
			const decay = 1 - landed / 10;
			shakeX = Math.sin(landed * 2.9) * shake * decay;
			shakeY = Math.cos(landed * 3.7) * shake * decay * 0.6;
		}
	}

	/* ---------- render ---------- */
	const renderWord = (w: Word) => {
		const hlc = hl[w.index];
		const fillStyle = unitFill(hlc);
		const underlined = ulWords?.has(w.index);
		const marker =
			hlc && highlightStyle === 'marker' ? (
				<span
					style={{
						position: 'absolute',
						left: '-0.08em',
						right: '-0.08em',
						top: '0.08em',
						bottom: '0.02em',
						borderRadius: '0.12em',
						background: alpha(hlc, 0.22),
						boxShadow: `inset 0 0 0 2px ${alpha(hlc, 0.35)}`,
						transform: `scaleX(${progress(frame, wordLandsAt(w), 12, ease.push)})`,
						transformOrigin: '0% 50%',
						zIndex: -1,
					}}
				/>
			) : null;

		// Char-level modes
		if (mode === 'stagger-chars' || mode === 'typewriter') {
			const chars = Array.from(w.lead + w.core + w.trail);
			const coreStart = Array.from(w.lead).length;
			const coreEnd = coreStart + Array.from(w.core).length;
			const nodes: React.ReactNode[] = [];
			chars.forEach((ch, ci) => {
				const gi = w.charStart + ci;
				const isCore = ci >= coreStart && ci < coreEnd;
				const f = isCore ? fillStyle : unitFill(undefined);
				if (mode === 'typewriter') {
					const visible = gi < typed;
					nodes.push(
						<span key={ci} style={{...f, visibility: visible ? 'visible' : 'hidden'}}>
							{ch}
						</span>,
					);
					if (caret && gi === caretIdx && exitQ(0) < 1) nodes.push(<Caret key="caret" on={caretOn} size={size} />);
				} else {
					nodes.push(
						<span
							key={ci}
							style={{display: 'inline-block', ...f, ...merge(entryStyle(gi * st), exitStyle(gi))}}
						>
							{ch}
						</span>,
					);
				}
			});
			const wordExit = mode === 'typewriter' ? exitStyle(w.index) : {};
			return (
				<span key={w.index} style={{display: 'inline-block', position: 'relative', whiteSpace: 'nowrap', ...wordExit}}>
					{marker}
					{nodes}
					{underlined ? ulBar(ulColor(w.index)) : null}
				</span>
			);
		}

		// Word-level modes
		const wordAnim =
			mode === 'mask-up'
				? {}
				: merge(entryStyle(mode === 'slam' && st === 0 ? 0 : w.index * st), exitStyle(w.index));
		return (
			<span key={w.index} style={{display: 'inline-block', position: 'relative', whiteSpace: 'nowrap', ...wordAnim}}>
				{w.lead ? <span style={unitFill(undefined)}>{w.lead}</span> : null}
				<span style={{position: 'relative', display: 'inline-block', isolation: 'isolate'}}>
					{marker}
					<span style={fillStyle}>{w.core}</span>
				</span>
				{w.trail ? <span style={unitFill(undefined)}>{w.trail}</span> : null}
				{underlined ? ulBar(ulColor(w.index)) : null}
			</span>
		);
	};

	const renderLine = (li: number) => {
		const lw = words.filter((w) => w.line === li);
		const content: React.ReactNode[] = [];
		lw.forEach((w, i) => {
			if (i > 0) content.push(' ');
			content.push(renderWord(w));
		});
		// typewriter caret at very start (nothing typed yet)
		if (mode === 'typewriter' && caret && li === 0 && typed <= 0 && frame >= at - 6) content.unshift(<Caret key="c0" on={caretOn} size={size} />);

		const masked = mode === 'mask-up' || exit === 'mask';
		if (!masked) return <div key={li}>{content}</div>;

		const pIn = mode === 'mask-up' ? progress(frame, at + li * st, dur, ease.push) : 1;
		const qOut = exit === 'mask' ? exitQ(li) : 0;
		const ty = (1 - pIn) * 108 - qOut * 108;
		const lineExit = exit !== 'mask' && mode === 'mask-up' ? exitStyle(li) : {};
		return (
			<div key={li} style={{overflow: 'hidden', paddingBottom: '0.14em', marginBottom: '-0.14em', paddingTop: '0.04em', marginTop: '-0.04em'}}>
				<div style={{transform: `translateY(${ty}%)`, ...lineExit}}>{content}</div>
			</div>
		);
	};

	const blockUnderline = underline && !underline.word;

	return (
		<div
			style={{
				fontFamily: isDisplay ? font.display : font.text,
				fontSize: size,
				fontWeight: weight ?? (isDisplay ? W.bold : W.medium),
				lineHeight: lineHeight ?? (isDisplay ? 1.06 : 1.3),
				letterSpacing: letterSpacing ?? (isDisplay ? (size >= type.hero ? tracking.hero : tracking.display) : tracking.body),
				textAlign: align,
				maxWidth,
				color: ink,
				fontFeatureSettings: '"ss01", "cv11"',
				transform: shakeX || shakeY ? `translate(${shakeX}px, ${shakeY}px)` : undefined,
				...style,
			}}
		>
			<div style={{position: 'relative', display: 'inline-block'}}>
				{lines.map((_, li) => renderLine(li))}
				{blockUnderline ? ulBar(ulColor()) : null}
			</div>
		</div>
	);
};

const Caret: React.FC<{on: boolean; size: number}> = ({on, size}) => (
	<span style={{position: 'relative', display: 'inline-block', width: 0}}>
		<span
			style={{
				position: 'absolute',
				left: size * 0.04,
				bottom: '-0.14em',
				width: Math.max(3, size * 0.055),
				height: '0.98em',
				borderRadius: 2,
				background: '#4d8dff',
				boxShadow: glow.of('#4d8dff', 0.8),
				opacity: on ? 1 : 0,
			}}
		/>
	</span>
);
