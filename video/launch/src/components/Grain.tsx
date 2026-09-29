import React from 'react';
import {AbsoluteFill, random, staticFile, useCurrentFrame} from 'remotion';
import {layer} from '../design/tokens';

export type GrainProps = {
	/** Overall strength, 0–1. 0.05–0.08 reads as "filmic" without looking dirty. Default 0.06. */
	opacity?: number;
	/** Tile scale; 1 = 512px tile. Larger = coarser grain. Default 1. */
	scale?: number;
	/** Re-roll the grain every N frames (1 = every frame, 2 = "on twos"). Default 1. */
	every?: number;
	/** CSS blend mode. `overlay` keeps blacks black. Default "overlay". */
	blend?: React.CSSProperties['mixBlendMode'];
	/** Seed so two grain layers don't correlate. Default "grain". */
	seed?: string;
};

/**
 * Animated film grain. A pre-baked 512px gaussian noise tile
 * (public/fx/grain.png, see tools/make-grain.py) re-offset every frame with a
 * deterministic seed — much cheaper than SVG feTurbulence. Put it as the LAST
 * child of a composition so it sits over everything.
 */
export const Grain: React.FC<GrainProps> = ({opacity = 0.06, scale = 1, every = 1, blend = 'overlay', seed = 'grain'}) => {
	const frame = useCurrentFrame();
	const step = Math.floor(frame / Math.max(1, every));
	const tile = 512 * scale;
	const ox = Math.floor(random(`${seed}-x-${step}`) * tile);
	const oy = Math.floor(random(`${seed}-y-${step}`) * tile);
	return (
		<AbsoluteFill
			style={{
				zIndex: layer.grain,
				pointerEvents: 'none',
				backgroundImage: `url(${staticFile('fx/grain.png')})`,
				backgroundSize: `${tile}px ${tile}px`,
				backgroundPosition: `${ox}px ${oy}px`,
				backgroundRepeat: 'repeat',
				mixBlendMode: blend,
				opacity,
			}}
		/>
	);
};
