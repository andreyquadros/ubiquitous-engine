import React from 'react';
import {useCurrentFrame, useVideoConfig} from 'remotion';
import {alpha, color, ease, font, resolveColor, type Accent} from '../design/tokens';
import {clamp, lerp, springIn} from '../design/motion';

export type KeyCapProps = {
	/** Legend on the key ("1", "⌘", "Enter"...). */
	label: React.ReactNode;
	/** Small secondary legend in the top-left corner (e.g. "shift" symbol). */
	sublabel?: string;
	/** Frame(s) the key goes down. */
	pressAt?: number | number[];
	/** Frames the key stays fully down. Default 4. */
	hold?: number;
	/** Key height in px (the square key is this wide). Default 120. */
	size?: number;
	/** Width in key units (1 = square, 1.5 = Tab, 2.25 = Enter...). Default 1. */
	units?: number;
	/** Glow colour when pressed. Default volt. */
	accent?: Accent | string;
	/** Frame the key pops in. Omit to show immediately. */
	appearAt?: number;
	/** Keep the accent lit after the last press (e.g. "selected"). Default false. */
	latch?: boolean;
};

/**
 * A 3D-looking keyboard key that physically presses down: the cap drops into
 * its well, the side wall shrinks, the legend lights up in the accent colour.
 *
 * @example
 * <KeyCombo keys={['1', '2', '3']} pressAt={[20, 35, 50]} />
 * <KeyCap label="Enter" units={2} pressAt={40} accent="ember" />
 */
export const KeyCap: React.FC<KeyCapProps> = ({
	label,
	sublabel,
	pressAt,
	hold = 4,
	size = 120,
	units = 1,
	accent = 'volt',
	appearAt,
	latch = false,
}) => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	const c = resolveColor(accent);
	const presses = pressAt === undefined ? [] : Array.isArray(pressAt) ? pressAt : [pressAt];

	// depth 0 = up, 1 = fully down
	let down = 0;
	let lit = 0;
	for (const p of presses) {
		const d = frame - p;
		if (d >= -2 && d < 0) down = Math.max(down, (d + 2) / 2);
		else if (d >= 0 && d < hold) down = 1;
		else if (d >= hold && d < hold + 6) down = Math.max(down, 1 - ease.settle((d - hold) / 6));
		if (d >= 0) lit = Math.max(lit, latch ? 1 : 1 - clamp((d - hold) / 16));
	}

	const pop = appearAt === undefined ? 1 : springIn(frame, fps, appearAt, 'subtleBounce');
	if (pop <= 0.001) return null;

	const w = size * units + (units - 1) * size * 0.12;
	const wall = size * 0.1;
	const travel = wall * 0.75;
	const r = size * 0.2;
	const legendSize = typeof label === 'string' && label.length > 2 ? size * 0.24 : size * 0.42;

	return (
		<div
			style={{
				position: 'relative',
				width: w,
				height: size + wall,
				transform: `scale(${pop}) translateY(${lerp(30, 0, clamp(pop))}px)`,
				opacity: clamp(pop * 1.5),
			}}
		>
			{/* key well / floor shadow */}
			<div
				style={{
					position: 'absolute',
					left: -size * 0.06,
					right: -size * 0.06,
					top: size * 0.1,
					bottom: -size * 0.1,
					borderRadius: r * 1.2,
					background: alpha(c, 0.18 * lit),
					filter: `blur(${size * 0.16}px)`,
				}}
			/>
			{/* side wall */}
			<div
				style={{
					position: 'absolute',
					left: 0,
					right: 0,
					top: travel * down + wall * 0.4,
					bottom: 0,
					borderRadius: r,
					background: 'linear-gradient(180deg, #0f1524 0%, #070a12 100%)',
					boxShadow: `0 ${size * 0.12 * (1 - down * 0.6)}px ${size * 0.25}px -${size * 0.06}px rgba(0,0,0,0.8), inset 0 -1px 0 rgba(255,255,255,0.05)`,
				}}
			/>
			{/* cap */}
			<div
				style={{
					position: 'absolute',
					left: 0,
					top: travel * down,
					width: w,
					height: size,
					borderRadius: r,
					background: `linear-gradient(180deg, ${lit > 0 ? mix('#2a3653', c, 0.22 * lit) : '#2a3653'} 0%, #1a2238 100%)`,
					boxShadow: [
						'inset 0 1.5px 0 rgba(255,255,255,0.14)',
						'inset 0 -2px 6px rgba(0,0,0,0.35)',
						`inset 0 0 0 1px ${alpha(lit > 0 ? c : '#ffffff', lit > 0 ? 0.35 + 0.45 * lit : 0.07)}`,
						lit > 0 ? `0 0 ${size * 0.35}px ${alpha(c, 0.45 * lit)}` : '',
					]
						.filter(Boolean)
						.join(', '),
					display: 'flex',
					alignItems: 'center',
					justifyContent: 'center',
					overflow: 'hidden',
				}}
			>
				{/* concave dish */}
				<div
					style={{
						position: 'absolute',
						inset: size * 0.09,
						borderRadius: r * 0.7,
						background: 'radial-gradient(ellipse 80% 70% at 50% 35%, rgba(255,255,255,0.06), rgba(0,0,0,0.12))',
					}}
				/>
				{sublabel ? (
					<div
						style={{
							position: 'absolute',
							left: size * 0.16,
							top: size * 0.12,
							fontFamily: font.text,
							fontSize: size * 0.16,
							fontWeight: 600,
							color: color.ink3,
						}}
					>
						{sublabel}
					</div>
				) : null}
				<div
					style={{
						position: 'relative',
						fontFamily: font.display,
						fontSize: legendSize,
						fontWeight: 600,
						letterSpacing: '-0.02em',
						color: lit > 0 ? mix(color.ink, c, lit) : color.ink,
						textShadow: lit > 0 ? `0 0 ${size * 0.15}px ${alpha(c, 0.8 * lit)}` : '0 1px 0 rgba(0,0,0,0.5)',
					}}
				>
					{label}
				</div>
			</div>
		</div>
	);
};

export type KeyComboProps = {
	/** Legends, left to right. */
	keys: React.ReactNode[];
	/** One press frame per key (same index), or one frame for all (chord). */
	pressAt?: number | number[];
	/** Separator between keys: "+" for chords, "" for a sequence. Default "". */
	separator?: string;
	/** Key size in px. Default 120. */
	size?: number;
	/** Gap between keys. Default size * 0.22. */
	gap?: number;
	/** First key pop-in frame; keys stagger by `stagger`. Omit to show immediately. */
	appearAt?: number;
	/** Frames between key pop-ins. Default 3. */
	stagger?: number;
	/** Accent colour. Default volt. */
	accent?: Accent | string;
	/** Latch keys lit after pressing. */
	latch?: boolean;
};

/** A row of KeyCaps (a chord like ⌘ + K, or a run like 1 2 3). */
export const KeyCombo: React.FC<KeyComboProps> = ({keys, pressAt, separator = '', size = 120, gap, appearAt, stagger = 3, accent, latch}) => {
	const g = gap ?? size * 0.22;
	return (
		<div style={{display: 'flex', alignItems: 'center', gap: g}}>
			{keys.map((k, i) => (
				<React.Fragment key={i}>
					{i > 0 && separator ? (
						<div style={{fontFamily: font.display, fontSize: size * 0.34, color: color.ink3, fontWeight: 500}}>{separator}</div>
					) : null}
					<KeyCap
						label={k}
						size={size}
						accent={accent}
						latch={latch}
						appearAt={appearAt === undefined ? undefined : appearAt + i * stagger}
						pressAt={pressAt === undefined ? undefined : Array.isArray(pressAt) ? pressAt[i] : pressAt}
					/>
				</React.Fragment>
			))}
		</div>
	);
};

/** Mix two #rrggbb colours. */
const mix = (a: string, b: string, t: number): string => {
	const pa = parseInt(a.slice(1), 16);
	const pb = parseInt(b.slice(1), 16);
	const ch = (s: number) => Math.round(lerp((pa >> s) & 255, (pb >> s) & 255, clamp(t)));
	return `rgb(${ch(16)}, ${ch(8)}, ${ch(0)})`;
};

