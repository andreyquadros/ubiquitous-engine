import React, {useId} from 'react';
import {AbsoluteFill, useCurrentFrame, useVideoConfig} from 'remotion';
import {alpha, color, ease, font, glow, layer, resolveColor, shadow, tracking, type Accent} from '../design/tokens';
import {clamp, lerp, progress, springIn} from '../design/motion';
import {mapImageRect, type Rect, type ScreenConfig} from './screen-geometry';

export type CalloutProps = {
	/** Rect to point at: image px when `screen` is given, otherwise composition px. */
	target: Rect;
	/** Share the Screen's config so the callout follows tilt + camera. */
	screen?: ScreenConfig;
	/** Main label (short: 2–5 words). */
	label: string;
	/** Optional small uppercase line above the label (e.g. "IA", "NOVO"). */
	kicker?: string;
	/** Optional icon node (e.g. a lucide-react icon) shown before the label. */
	icon?: React.ReactNode;
	/** Side of the target where the pill sits. Default "top". */
	side?: 'top' | 'bottom' | 'left' | 'right';
	/** Leader line length in px. Default 110. */
	distance?: number;
	/** Shift the pill along the side (px), e.g. to avoid other elements. Default 0. */
	shift?: number;
	/** Accent colour. Default volt. */
	accent?: Accent | string;
	/** Frame the callout starts drawing. Default 0. */
	at?: number;
	/** Frame the callout starts leaving. */
	exitAt?: number;
	/** Draw corner brackets around the target. Default true. */
	brackets?: boolean;
	/** Label font size (px). Default 30. */
	size?: number;
};

/**
 * A pill badge with a leader line that draws from the target to the pill.
 * Lives in composition space (crisp at any zoom) but tracks the target.
 *
 * @example
 * <Callout screen={shot} target={{x: 1200, y: 420, w: 600, h: 180}} kicker="IA" label="Classifica sozinho" side="right" at={40} />
 */
export const Callout: React.FC<CalloutProps> = ({
	target,
	screen,
	label,
	kicker,
	icon,
	side = 'top',
	distance = 110,
	shift = 0,
	accent = 'volt',
	at = 0,
	exitAt,
	brackets = true,
	size = 30,
}) => {
	const frame = useCurrentFrame();
	const cleanId = useId().replace(/[^a-zA-Z0-9_-]/g, '');
	const {width, height, fps} = useVideoConfig();
	const r = screen ? mapImageRect(screen, frame, {width, height}, target) : target;
	const c = resolveColor(accent);

	const pBr = progress(frame, at, 12, ease.push);
	const pLine = progress(frame, at + 4, 12, ease.push);
	const pPill = springIn(frame, fps, at + 10, 'snappy');
	const out = exitAt !== undefined ? progress(frame, exitAt, 10, ease.exit) : 0;
	if (frame < at || out >= 1) return null;

	// anchor on the target edge, pill end point
	const dir = {top: [0, -1], bottom: [0, 1], left: [-1, 0], right: [1, 0]}[side] as [number, number];
	const ax = side === 'left' ? r.x : side === 'right' ? r.x + r.w : r.x + r.w / 2 + (side === 'top' || side === 'bottom' ? 0 : 0);
	const ay = side === 'top' ? r.y : side === 'bottom' ? r.y + r.h : r.y + r.h / 2;
	const perp = side === 'top' || side === 'bottom' ? [1, 0] : [0, 1];
	const ex = ax + dir[0] * distance + perp[0] * shift;
	const ey = ay + dir[1] * distance + perp[1] * shift;
	// elbow: go straight out, then shift sideways
	const midX = ax + dir[0] * distance * 0.55;
	const midY = ay + dir[1] * distance * 0.55;
	const d = `M ${ax} ${ay} L ${midX} ${midY} L ${ex} ${ey}`;
	const len = Math.hypot(midX - ax, midY - ay) + Math.hypot(ex - midX, ey - midY);

	const pillTransform = {
		top: 'translate(-50%, -100%)',
		bottom: 'translate(-50%, 0%)',
		left: 'translate(-100%, -50%)',
		right: 'translate(0%, -50%)',
	}[side];
	const pad = 10;
	const arm = Math.min(28, r.w / 3, r.h / 3);
	const brScale = lerp(1.12, 1, pBr);
	const bx = r.x - pad;
	const by = r.y - pad;
	const bw = r.w + pad * 2;
	const bh = r.h + pad * 2;
	const cxr = bx + bw / 2;
	const cyr = by + bh / 2;
	const corner = (x: number, y: number, sx: number, sy: number) => `M ${x + sx * arm} ${y} L ${x} ${y} L ${x} ${y + sy * arm}`;
	// glow filter region: bbox of brackets + leader (+ margin), in user space so
	// perfectly straight lines (zero-width bbox) still render
	const m = 40;
	const fx0 = Math.min(bx, ex, ax) - m;
	const fy0 = Math.min(by, ey, ay) - m;
	const fx1 = Math.max(bx + bw, ex, ax) + m;
	const fy1 = Math.max(by + bh, ey, ay) + m;
	const fid = `callout-glow-${cleanId}`;

	return (
		<AbsoluteFill style={{zIndex: layer.callout, pointerEvents: 'none', opacity: 1 - out}}>
			<svg width={width} height={height} style={{position: 'absolute', inset: 0, overflow: 'visible'}}>
				<defs>
					<filter id={fid} filterUnits="userSpaceOnUse" x={fx0} y={fy0} width={fx1 - fx0} height={fy1 - fy0}>
						<feGaussianBlur stdDeviation="4" result="b" />
						<feMerge>
							<feMergeNode in="b" />
							<feMergeNode in="SourceGraphic" />
						</feMerge>
					</filter>
				</defs>
				{brackets ? (
					<g
						opacity={pBr}
						transform={`translate(${cxr} ${cyr}) scale(${brScale}) translate(${-cxr} ${-cyr})`}
						stroke={c}
						strokeWidth={3}
						fill="none"
						strokeLinecap="round"
						filter={`url(#${fid})`}
					>
						<path d={corner(bx, by, 1, 1)} />
						<path d={corner(bx + bw, by, -1, 1)} />
						<path d={corner(bx, by + bh, 1, -1)} />
						<path d={corner(bx + bw, by + bh, -1, -1)} />
					</g>
				) : null}
				<path
					d={d}
					stroke={c}
					strokeWidth={2.5}
					fill="none"
					strokeLinecap="round"
					strokeLinejoin="round"
					strokeDasharray={len}
					strokeDashoffset={len * (1 - pLine)}
					filter={`url(#${fid})`}
				/>
				<circle cx={ax} cy={ay} r={6 * clamp(pBr * 1.5)} fill={c} filter={`url(#${fid})`} />
				<circle cx={ax} cy={ay} r={6 + 18 * ((frame - at) % 30) / 30} fill="none" stroke={alpha(c, 0.5 * (1 - ((frame - at) % 30) / 30))} strokeWidth={2} />
			</svg>
			<div
				style={{
					position: 'absolute',
					left: ex,
					top: ey,
					transform: `${pillTransform} scale(${lerp(0.7, 1, clamp(pPill))})`,
					transformOrigin: {top: '50% 100%', bottom: '50% 0%', left: '100% 50%', right: '0% 50%'}[side],
					opacity: clamp(pPill * 1.5),
				}}
			>
				<div
					style={{
						display: 'flex',
						alignItems: 'center',
						gap: 14,
						padding: `${size * 0.42}px ${size * 0.8}px`,
						borderRadius: 999,
						background: 'linear-gradient(180deg, rgba(28,38,60,0.94) 0%, rgba(17,23,38,0.94) 100%)',
						boxShadow: `${shadow.contact}, inset 0 0 0 1.5px ${alpha(c, 0.7)}, ${glow.of(c, 0.55)}`,
						whiteSpace: 'nowrap',
					}}
				>
					{icon ? <div style={{display: 'flex', color: c, width: size * 1.05, height: size * 1.05}}>{icon}</div> : null}
					<div style={{display: 'flex', flexDirection: 'column', gap: 2}}>
						{kicker ? (
							<div
								style={{
									fontFamily: font.text,
									fontSize: size * 0.5,
									fontWeight: 700,
									letterSpacing: tracking.caps,
									textTransform: 'uppercase',
									color: c,
								}}
							>
								{kicker}
							</div>
						) : null}
						<div style={{fontFamily: font.display, fontSize: size, fontWeight: 600, letterSpacing: '-0.02em', color: color.ink}}>{label}</div>
					</div>
				</div>
			</div>
		</AbsoluteFill>
	);
};
