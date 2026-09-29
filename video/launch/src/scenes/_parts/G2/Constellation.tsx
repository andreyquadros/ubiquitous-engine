/**
 * s05 product constellation (brief v2-scenes G2): once the lockup has settled,
 * real fragments of the captures float in 3D depth around it — the 88 ring,
 * the day-track strip, a review row with its category pill, the "Suas
 * categorias" list. Each is a <LiftCard> (graded like the app, rim, shadow,
 * tilt facing the centre) wrapped in a depth layer: far fragments are smaller,
 * softer and slower; near ones parallax more with the canvas push. They enter
 * on 8th-note beats (f30, 37, 45, 52) with a rack-focus pop and recede
 * (shrink toward the centre, defocus, fade) over f90–106, so the frame is back
 * to the match-cut state (stage + lockup + UBI only) well before f110.
 *
 * Claim safety: no price, no model id, no key legend in any crop (checked);
 * the numbers visible (88, "25min", "68%", hour labels) are texture, not results.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {LiftCard} from '../../../components/LiftCard';
import type {Rect} from '../../../components/screen-geometry';
import {E} from '../../../shared';
import {clamp01, lerp, ramp, W, H} from './common';

export type Fragment = {
	id: string;
	src: string;
	rect: Rect;
	/** Card centre at rest (comp px) and on-canvas width. */
	x: number;
	y: number;
	width: number;
	/** 0 = far (small, soft, slow), 1 = near. */
	depth: number;
	rx: number;
	ry: number;
	rz?: number;
	at: number;
	radius?: number;
	glow?: string | false;
	/** parallax drift px/frame */
	drift: {x: number; y: number};
};

export const FRAGMENTS: Fragment[] = [
	{
		// "Suas categorias" list (IFRO, Incubadora, Cidades Inteligentes) — top left
		id: 'cats',
		src: 'ui/categories.png',
		rect: {x: 524, y: 250, w: 724, h: 350},
		x: 345,
		y: 190,
		width: 470,
		depth: 0.75,
		rx: -8,
		ry: 16,
		rz: -2,
		at: 30,
		drift: {x: -0.12, y: -0.05},
	},
	{
		// the 88 focus ring — bottom right, near
		id: 'ring',
		src: 'ui/dashboard.png',
		rect: {x: 566, y: 416, w: 392, h: 392},
		x: 1735,
		y: 870,
		width: 250,
		depth: 1,
		rx: 12,
		ry: -18,
		at: 37,
		radius: 125,
		glow: 'mint',
		drift: {x: 0.1, y: 0.06},
	},
	{
		// review row with its IFRO pill — top, right of centre, far
		id: 'row',
		src: 'ui/review-queue-selected.png',
		rect: {x: 516, y: 1362, w: 1528, h: 108},
		x: 1085,
		y: 118,
		width: 720,
		depth: 0.3,
		rx: -10,
		ry: -10,
		at: 45,
		drift: {x: 0.1, y: -0.04},
	},
	{
		// the day-track strip 08h → 18h — bottom left
		id: 'track',
		src: 'ui/dashboard.png',
		rect: {x: 1270, y: 1150, w: 1030, h: 150},
		x: 400,
		y: 915,
		width: 640,
		depth: 0.55,
		rx: 12,
		ry: 14,
		rz: 1.5,
		at: 52,
		drift: {x: -0.1, y: 0.05},
	},
];

/** Fragment recede (exit) window. */
export const RECEDE = {start: 90, end: 106};

export const Constellation: React.FC<{f: number; zoom: number}> = ({f, zoom}) => (
	<AbsoluteFill style={{pointerEvents: 'none'}}>
		{FRAGMENTS.map((g, i) => {
			if (f < g.at) return null;
			const out = ramp(f, RECEDE.start + i * 2, RECEDE.end - 6 + i * 2, E.exit);
			if (out >= 0.999) return null;
			// rack focus in (blur 10 → depth blur over 10 f), defocus out
			const inP = ramp(f, g.at, g.at + 10, E.enter);
			const depthBlur = lerp(1.6, 0, g.depth);
			const blur = lerp(10, depthBlur, inP) + 8 * out;
			// parallax with the canvas push: near fragments move out faster
			const zf = 1 + (zoom - 1) * (1 + 1.5 * g.depth);
			const t = f - g.at;
			const bx = W / 2 + (g.x + g.drift.x * t - W / 2) * zf;
			const by = H / 2 + (g.y + g.drift.y * t - H / 2) * zf;
			// recede: pull toward the centre and shrink
			const x = lerp(bx, W / 2 + (bx - W / 2) * 0.82, out);
			const y = lerp(by, H / 2 + (by - H / 2) * 0.82, out);
			const width = g.width * zf * lerp(1, 0.7, out);
			const o = lerp(0.72, 1, g.depth) * (1 - out);
			return (
				<AbsoluteFill key={g.id} style={{filter: blur > 0.2 ? `blur(${blur.toFixed(2)}px)` : undefined}}>
					<LiftCard
						src={g.src}
						rect={g.rect}
						x={x}
						y={y}
						width={width}
						rotateX={g.rx}
						rotateY={g.ry + 4 * Math.sin((f + i * 20) / 30)}
						rotateZ={g.rz ?? 0}
						at={g.at}
						enter="pop"
						spring="smooth"
						float={lerp(3, 7, g.depth)}
						floatPeriod={90 + i * 12}
						radius={g.radius ?? 14}
						glow={g.glow ?? 'volt'}
						glowOpacity={0.28}
						shadow={lerp(0.6, 1, g.depth)}
						opacity={clamp01(o)}
					/>
				</AbsoluteFill>
			);
		})}
	</AbsoluteFill>
);
