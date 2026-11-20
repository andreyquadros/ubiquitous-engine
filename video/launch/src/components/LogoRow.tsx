import React from 'react';
import {Img, useCurrentFrame, useVideoConfig} from 'remotion';
import {color, ease, font, tracking} from '../design/tokens';
import {clamp, lerp, progress, springIn, staggerDelay, type StaggerOrder} from '../design/motion';
import {resolveSrc} from './Screen';

export type LogoItem = {
	/** Image path relative to public/ (SVG/PNG) or URL. Use either `src` or `node`. */
	src?: string;
	/** Inline node (e.g. an <svg>) instead of an image. */
	node?: React.ReactNode;
	/** Caption under (plain) or beside (chip) the logo. */
	label?: string;
	/** Per-logo height override px. */
	height?: number;
	/** Force this logo white (overrides the row's `monochrome`). */
	monochrome?: boolean;
};

export type LogoRowProps = {
	/** Logos, left to right. */
	logos: LogoItem[];
	/** Frame the first logo appears. Default 0. */
	at?: number;
	/** Frames between logos. Default 3. */
	stagger?: number;
	/** Stagger order. Default "index". */
	order?: StaggerOrder;
	/** Gap between logos px. Default 72 (plain) / 20 (chip). */
	gap?: number;
	/** Logo height px. Default 56 (plain) / 36 (chip). */
	height?: number;
	/** "plain" floating logos, or each inside a glass "chip" with its label. Default "plain". */
	variant?: 'plain' | 'chip';
	/** Render all image logos pure white (brightness(0) invert(1)). Default false. */
	monochrome?: boolean;
	/** Opacity of logos at rest (0–1) — 0.85 looks classy for "works with". Default 1. */
	restOpacity?: number;
	/** Small caps heading above the row (e.g. "Funciona com"). */
	heading?: string;
	/** Heading case. Use "none" to keep brand casing like "macOS". Default "uppercase". */
	headingTransform?: 'uppercase' | 'none';
	/** Frame the row starts leaving (staggered). */
	exitAt?: number;
};

/**
 * A row of logos (AI providers, operating systems...) that pops in with a
 * stagger. Works with SVG/PNG files from public/ or inline nodes.
 *
 * @example
 * <LogoRow heading="Funciona com" variant="chip" at={10} logos={[
 *   {src: 'brand/openai.svg', label: 'OpenAI'}, {src: 'brand/anthropic.svg', label: 'Claude'},
 * ]} monochrome />
 */
export const LogoRow: React.FC<LogoRowProps> = ({
	logos,
	at = 0,
	stagger = 3,
	order = 'index',
	gap,
	height,
	variant = 'plain',
	monochrome = false,
	restOpacity = 1,
	heading,
	headingTransform = 'uppercase',
	exitAt,
}) => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	const chip = variant === 'chip';
	const h = height ?? (chip ? 36 : 56);
	const g = gap ?? (chip ? 20 : 72);
	const headP = progress(frame, at - 4, 12, ease.push);
	const headOut = exitAt !== undefined ? progress(frame, exitAt, 8, ease.exit) : 0;

	return (
		<div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 28}}>
			{heading ? (
				<div
					style={{
						fontFamily: font.text,
						fontWeight: 600,
						letterSpacing: headingTransform === 'uppercase' ? tracking.caps : '0.02em',
						textTransform: headingTransform,
						fontSize: headingTransform === 'uppercase' ? 20 : 24,
						color: color.ink3,
						opacity: headP * (1 - headOut),
						transform: `translateY(${lerp(10, 0, headP)}px)`,
					}}
				>
					{heading}
				</div>
			) : null}
			<div style={{display: 'flex', alignItems: 'center', gap: g}}>
				{logos.map((l, i) => {
					const d = staggerDelay(i, logos.length, stagger, order);
					const s = springIn(frame, fps, at + d, 'subtleBounce');
					const out = exitAt !== undefined ? progress(frame, exitAt + i * 2, 8, ease.exit) : 0;
					const blur = lerp(8, 0, clamp(s)) + out * 8;
					const mono = l.monochrome ?? monochrome;
					const lh = l.height ?? h;
					const mark = l.node ? (
						<div style={{height: lh, display: 'flex', alignItems: 'center', color: color.ink}}>{l.node}</div>
					) : l.src ? (
						<Img src={resolveSrc(l.src)} style={{height: lh, width: 'auto', display: 'block', filter: mono ? 'brightness(0) invert(1)' : undefined}} />
					) : null;
					return (
						<div
							key={i}
							style={{
								opacity: clamp(s * 1.5) * restOpacity * (1 - out),
								transform: `translateY(${lerp(28, 0, s) - out * 14}px) scale(${lerp(0.8, 1, s)})`,
								filter: blur > 0.2 ? `blur(${blur}px)` : undefined,
							}}
						>
							{chip ? (
								<div
									style={{
										display: 'flex',
										alignItems: 'center',
										gap: 14,
										padding: `${h * 0.42}px ${h * 0.75}px`,
										borderRadius: 999,
										background: 'linear-gradient(180deg, rgba(28,38,60,0.8) 0%, rgba(17,23,38,0.8) 100%)',
										boxShadow: 'inset 0 0 0 1px rgba(255,255,255,0.08), inset 0 1px 0 rgba(255,255,255,0.08), 0 12px 30px -12px rgba(0,0,0,0.7)',
									}}
								>
									{mark}
									{l.label ? (
										<div style={{fontFamily: font.display, fontSize: h * 0.72, fontWeight: 600, letterSpacing: '-0.02em', color: color.ink}}>
											{l.label}
										</div>
									) : null}
								</div>
							) : (
								<div style={{display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 14}}>
									{mark}
									{l.label ? (
										<div style={{fontFamily: font.text, fontSize: 20, fontWeight: 500, color: color.ink2}}>{l.label}</div>
									) : null}
								</div>
							)}
						</div>
					);
				})}
			</div>
		</div>
	);
};
