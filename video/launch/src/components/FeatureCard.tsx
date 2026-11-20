import React from 'react';
import {useCurrentFrame, useVideoConfig} from 'remotion';
import {alpha, color, ease, font, glow, resolveColor, shadow, tracking, type Accent} from '../design/tokens';
import {clamp, lerp, progress, springIn, staggerDelay, type StaggerOrder} from '../design/motion';

export type FeatureCardProps = {
	/** Icon node — e.g. `<Clock3 strokeWidth={1.75} />` from lucide-react. Sized by the card. */
	icon?: React.ReactNode;
	/** Title (2–4 words). */
	title: string;
	/** One-line benefit under the title. */
	subline?: string;
	/** Accent colour for icon tile, border glow and badge. Default volt. */
	accent?: Accent | string;
	/** Small pill in the top-right corner (e.g. "Novo", "IA"). */
	badge?: string;
	/** Frame the card pops in. Default 0. */
	at?: number;
	/** Frame the card starts leaving. */
	exitAt?: number;
	/** Card width px. Default 440. */
	width?: number;
	/** Card height px. Default auto. */
	height?: number;
	/** Surface style. Default "glass". */
	variant?: 'glass' | 'solid' | 'outline';
	/** Animated light running along the border after landing. Default false. */
	shine?: boolean;
	/** Layout of icon + text. Default "left". */
	align?: 'left' | 'center';
	/** Scale factor for all inner sizes. Default 1. */
	scale?: number;
};

/**
 * Icon + title + subline card that pops in with a spring (scale, rise, blur).
 *
 * @example
 * <FeatureCard icon={<Sparkles />} title="Classifica com IA" subline="Nas categorias que você cria." accent="volt" at={10} />
 */
export const FeatureCard: React.FC<FeatureCardProps> = ({
	icon,
	title,
	subline,
	accent = 'volt',
	badge,
	at = 0,
	exitAt,
	width = 440,
	height,
	variant = 'glass',
	shine = false,
	align = 'left',
	scale = 1,
}) => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	const c = resolveColor(accent);
	const s = springIn(frame, fps, at, 'snappy');
	const out = exitAt !== undefined ? progress(frame, exitAt, 10, ease.exit) : 0;
	if (s <= 0.001 || out >= 1) return <div style={{width, height, flexShrink: 0}} />;

	const blur = lerp(10, 0, clamp(s)) + out * 10;
	const u = scale;
	const shineP = shine ? ((frame - at - 12) % 90) / 90 : -1;

	const surface: React.CSSProperties =
		variant === 'solid'
			? {background: color.panel2}
			: variant === 'outline'
				? {background: 'transparent'}
				: {background: 'linear-gradient(180deg, rgba(26,35,56,0.78) 0%, rgba(15,21,36,0.78) 100%)'};

	return (
		<div
			style={{
				position: 'relative',
				width,
				height,
				flexShrink: 0,
				transform: `translateY(${lerp(40, 0, s) - out * 20}px) scale(${lerp(0.88, 1, s)})`,
				opacity: clamp(s * 1.6) * (1 - out),
				filter: blur > 0.2 ? `blur(${blur}px)` : undefined,
			}}
		>
			<div
				style={{
					position: 'relative',
					height: '100%',
					boxSizing: 'border-box',
					padding: `${34 * u}px ${34 * u}px ${32 * u}px`,
					borderRadius: 26 * u,
					overflow: 'hidden',
					...surface,
					boxShadow: `${shadow.card}, inset 0 0 0 1px rgba(255,255,255,0.07), inset 0 1px 0 rgba(255,255,255,0.08)`,
					display: 'flex',
					flexDirection: 'column',
					alignItems: align === 'center' ? 'center' : 'flex-start',
					textAlign: align,
					gap: 18 * u,
				}}
			>
				{/* accent wash in the corner */}
				<div
					style={{
						position: 'absolute',
						left: -80 * u,
						top: -80 * u,
						width: 260 * u,
						height: 260 * u,
						borderRadius: '50%',
						background: `radial-gradient(closest-side, ${alpha(c, 0.2)}, transparent)`,
					}}
				/>
				{shineP >= 0 && shineP < 0.5 ? (
					<div
						style={{
							position: 'absolute',
							inset: 0,
							borderRadius: 26 * u,
							padding: 1.5,
							background: `conic-gradient(from ${shineP * 720}deg, transparent 0deg, ${alpha(c, 0.9)} 30deg, transparent 70deg)`,
							WebkitMask: 'linear-gradient(#000 0 0) content-box, linear-gradient(#000 0 0)',
							WebkitMaskComposite: 'xor',
							maskComposite: 'exclude',
						}}
					/>
				) : null}
				{badge ? (
					<div
						style={{
							position: 'absolute',
							right: 22 * u,
							top: 22 * u,
							padding: `${5 * u}px ${12 * u}px`,
							borderRadius: 999,
							background: alpha(c, 0.16),
							boxShadow: `inset 0 0 0 1px ${alpha(c, 0.45)}`,
							fontFamily: font.text,
							fontSize: 15 * u,
							fontWeight: 700,
							letterSpacing: tracking.caps,
							textTransform: 'uppercase',
							color: c,
						}}
					>
						{badge}
					</div>
				) : null}
				{icon ? (
					<div
						style={{
							position: 'relative',
							width: 68 * u,
							height: 68 * u,
							borderRadius: 18 * u,
							display: 'flex',
							alignItems: 'center',
							justifyContent: 'center',
							background: `linear-gradient(160deg, ${alpha(c, 0.3)} 0%, ${alpha(c, 0.1)} 100%)`,
							boxShadow: `inset 0 0 0 1px ${alpha(c, 0.45)}, ${glow.of(c, 0.35)}`,
							color: c,
						}}
					>
						<div style={{width: 34 * u, height: 34 * u, display: 'flex'}}>{icon}</div>
					</div>
				) : null}
				<div style={{position: 'relative', display: 'flex', flexDirection: 'column', gap: 10 * u}}>
					<div
						style={{
							fontFamily: font.display,
							fontSize: 34 * u,
							fontWeight: 600,
							letterSpacing: tracking.heading,
							lineHeight: 1.12,
							color: color.ink,
						}}
					>
						{title}
					</div>
					{subline ? (
						<div style={{fontFamily: font.text, fontSize: 23 * u, fontWeight: 450, lineHeight: 1.35, color: color.ink2}}>{subline}</div>
					) : null}
				</div>
			</div>
		</div>
	);
};

export type CardGridProps = {
	/** Cards (their own `at` is ignored; the grid staggers them). */
	cards: FeatureCardProps[];
	/** Columns. Default 3. */
	columns?: number;
	/** Gap between cards px. Default 28. */
	gap?: number;
	/** Frame the first card pops. Default 0. */
	at?: number;
	/** Frames between cards. Default 4. */
	stagger?: number;
	/** Stagger order. Default "index". */
	order?: StaggerOrder;
	/** Card width px. Default 440. */
	cardWidth?: number;
	/** Card height px (uniform rows look best). Default auto. */
	cardHeight?: number;
	/** Frame all cards start leaving (staggered by `exitStagger`). */
	exitAt?: number;
	/** Frames between card exits. Default 2. */
	exitStagger?: number;
	/** Passed to each card. */
	variant?: FeatureCardProps['variant'];
	/** Passed to each card. */
	align?: FeatureCardProps['align'];
};

/**
 * Grid of FeatureCards with staggered pop-in (and optional staggered exit).
 *
 * @example
 * <CardGrid at={6} columns={3} cards={[{icon: <Timer/>, title: 'Captura sozinho', subline: 'Sem cronômetro.'}, ...]} />
 */
export const CardGrid: React.FC<CardGridProps> = ({
	cards,
	columns = 3,
	gap = 28,
	at = 0,
	stagger = 4,
	order = 'index',
	cardWidth = 440,
	cardHeight,
	exitAt,
	exitStagger = 2,
	variant,
	align,
}) => (
	<div style={{display: 'grid', gridTemplateColumns: `repeat(${columns}, ${cardWidth}px)`, gap}}>
		{cards.map((card, i) => (
			<FeatureCard
				key={i}
				variant={variant}
				align={align}
				{...card}
				width={cardWidth}
				height={cardHeight ?? card.height}
				at={at + staggerDelay(i, cards.length, stagger, order)}
				exitAt={exitAt === undefined ? undefined : exitAt + i * exitStagger}
			/>
		))}
	</div>
);
