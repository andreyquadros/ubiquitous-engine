import React from 'react';
import {useCurrentFrame} from 'remotion';
import {color as palette, ease, font, glow, resolveColor, type, type Accent} from '../design/tokens';
import {lerp, progress, type EaseFn} from '../design/motion';

export type CounterFormat = 'number' | 'currency' | 'percent' | 'compact' | ((value: number) => string);

export type CounterProps = {
	/** Start value. Default 0. */
	from?: number;
	/** End value. */
	to: number;
	/** Frame the count starts. Default 0. */
	at?: number;
	/** Frames to reach `to`. Default 45 (1.5 s = 3 beats). */
	duration?: number;
	/** Easing of the count. Default ease.push (fast then gently lands). */
	easing?: EaseFn;
	/**
	 * - `number`   → "1.234" (pt-BR grouping)
	 * - `currency` → "R$ 49" (set `decimals` for cents: "R$ 49,90")
	 * - `percent`  → "87%" (pass 87, not 0.87)
	 * - `compact`  → "1,2 mil"
	 * - or a custom `(v) => string`
	 * Default "number".
	 */
	format?: CounterFormat;
	/** Fraction digits. Default 0. */
	decimals?: number;
	/** Text before the number (not animated), e.g. "+". */
	prefix?: string;
	/** Text after the number (not animated), e.g. "h" or " blocos". */
	suffix?: string;
	/** ISO currency for `currency`. Default "BRL". */
	currency?: string;
	/** BCP-47 locale. Default "pt-BR". */
	locale?: string;
	/** "count" = plain text count-up; "roll" = odometer digits rolling. Default "count". */
	mode?: 'count' | 'roll';
	/** Font size px. Default type.hero (168). */
	size?: number;
	/** Font weight. Default 700. */
	weight?: number;
	/** Number colour (palette name or CSS). Default ink. */
	color?: Accent | string;
	/** Prefix/suffix colour. Default ink2. */
	affixColor?: string;
	/** Prefix/suffix size relative to the number. Default 0.5. */
	affixScale?: number;
	/** Glow + scale pulse when the count lands. Default true. */
	landPulse?: boolean;
	/** Extra style. */
	style?: React.CSSProperties;
};

const makeFormatter = (format: CounterFormat, decimals: number, locale: string, currency: string) => {
	if (typeof format === 'function') return format;
	const base: Intl.NumberFormatOptions = {minimumFractionDigits: decimals, maximumFractionDigits: decimals};
	let nf: Intl.NumberFormat;
	switch (format) {
		case 'currency':
			nf = new Intl.NumberFormat(locale, {...base, style: 'currency', currency});
			break;
		case 'percent':
			nf = new Intl.NumberFormat(locale, {...base, style: 'percent'});
			return (v: number) => nf.format(v / 100).replace(/ %/, '%');
		case 'compact':
			nf = new Intl.NumberFormat(locale, {notation: 'compact', maximumFractionDigits: Math.max(1, decimals)});
			break;
		default:
			nf = new Intl.NumberFormat(locale, base);
	}
	return (v: number) => nf.format(v);
};

/**
 * Animated number with pt-BR formatting. Tabular figures so digits don't
 * jitter; optional odometer roll.
 *
 * @example
 * <Counter to={1234} />                                  // 1.234
 * <Counter to={49} format="currency" prefix="" />         // R$ 49
 * <Counter to={8.5} decimals={1} suffix="h" mode="roll" /> // 8,5h
 */
export const Counter: React.FC<CounterProps> = ({
	from = 0,
	to,
	at = 0,
	duration = 45,
	easing = ease.push,
	format = 'number',
	decimals = 0,
	prefix,
	suffix,
	currency = 'BRL',
	locale = 'pt-BR',
	mode = 'count',
	size = type.hero,
	weight = 700,
	color = 'ink',
	affixColor = palette.ink2,
	affixScale = 0.5,
	landPulse = true,
	style,
}) => {
	const frame = useCurrentFrame();
	const fmt = makeFormatter(format, decimals, locale, currency);
	const p = progress(frame, at, duration, easing);
	const v = lerp(from, to, p);
	const c = resolveColor(color, palette.ink);

	const land = landPulse ? frame - (at + duration) : -1;
	const pulse = land >= 0 && land < 18 ? Math.sin((land / 18) * Math.PI) * (1 - land / 18) : 0;

	const scaleUnit = Math.pow(10, decimals);
	const quantised = Math.round(v * scaleUnit) / scaleUnit;
	const text = fmt(mode === 'roll' ? Math.floor(v * scaleUnit) / scaleUnit : quantised);

	const affix = (s: string) => (
		<span style={{fontSize: size * affixScale, color: affixColor, fontWeight: 600, letterSpacing: '-0.02em', marginLeft: 0}}>{s}</span>
	);

	return (
		<div
			style={{
				display: 'inline-flex',
				alignItems: 'baseline',
				fontFamily: font.display,
				fontSize: size,
				fontWeight: weight,
				letterSpacing: '-0.04em',
				lineHeight: 1,
				fontVariantNumeric: 'tabular-nums',
				color: c,
				transform: `scale(${1 + pulse * 0.05})`,
				textShadow: pulse > 0 ? glow.text(c === palette.ink ? palette.volt : c, pulse) : undefined,
				whiteSpace: 'nowrap',
				...style,
			}}
		>
			{prefix ? affix(prefix) : null}
			{mode === 'roll' ? <Odometer text={text} value={v * scaleUnit} /> : <span>{text}</span>}
			{suffix ? affix(suffix) : null}
		</div>
	);
};

const CELL = '1.2em';

/** Renders `text`, replacing each digit with a rolling column driven by `value` (integer domain). */
const Odometer: React.FC<{text: string; value: number}> = ({text, value}) => {
	const chars = Array.from(text);
	// place value per digit, counted from the right
	let place = 0;
	const places: (number | null)[] = new Array(chars.length).fill(null);
	for (let i = chars.length - 1; i >= 0; i--) {
		if (/\d/.test(chars[i])) {
			places[i] = place;
			place++;
		}
	}
	const v = Math.max(0, value);
	return (
		<span style={{display: 'inline-block', lineHeight: CELL}}>
			{chars.map((ch, i) => {
				const pl = places[i];
				if (pl === null) return <span key={i}>{ch}</span>;
				const unit = Math.pow(10, pl);
				const digit = Math.floor(v / unit) % 10;
				const lower = (v % unit) / unit; // 0..1 progress of the lower places
				const pos = pl === 0 ? v % 10 : digit + (lower > 0.9 ? (lower - 0.9) * 10 : 0);
				return (
					<span
						key={i}
						style={{
							display: 'inline-block',
							position: 'relative',
							lineHeight: CELL,
							// clip-path (not overflow) keeps the baseline of the in-flow "0"
							clipPath: 'inset(0 -0.1em)',
							WebkitMaskImage: 'linear-gradient(180deg, transparent 0%, #000 12%, #000 88%, transparent 100%)',
							maskImage: 'linear-gradient(180deg, transparent 0%, #000 12%, #000 88%, transparent 100%)',
						}}
					>
						<span style={{visibility: 'hidden'}}>0</span>
						<span style={{position: 'absolute', left: 0, top: 0, display: 'flex', flexDirection: 'column', transform: `translateY(calc(${-pos} * ${CELL}))`}}>
							{[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0].map((d, j) => (
								<span key={j} style={{height: CELL, lineHeight: CELL, display: 'block'}}>
									{d}
								</span>
							))}
						</span>
					</span>
				);
			})}
		</span>
	);
};

