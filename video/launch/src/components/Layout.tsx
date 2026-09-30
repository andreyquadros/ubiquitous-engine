import React from 'react';
import {AbsoluteFill} from 'remotion';
import {space} from '../design/tokens';

export type CenterProps = {
	children: React.ReactNode;
	/** Vertical offset from centre, px. Default 0. */
	y?: number;
	/** Horizontal offset from centre, px. Default 0. */
	x?: number;
	/** Stack direction for multiple children. Default "column". */
	direction?: 'row' | 'column';
	/** Gap between children px. Default 24. */
	gap?: number;
	/** Keep content inside title-safe margins. Default true. */
	safe?: boolean;
	/** Cross-axis alignment. Default "center". */
	align?: 'flex-start' | 'center' | 'flex-end';
	/** Main-axis alignment. Default "center". */
	justify?: 'flex-start' | 'center' | 'flex-end' | 'space-between';
	style?: React.CSSProperties;
};

/** Full-frame flex container that centres its children (title-safe by default). */
export const Center: React.FC<CenterProps> = ({children, x = 0, y = 0, direction = 'column', gap = 24, safe = true, align = 'center', justify = 'center', style}) => (
	<AbsoluteFill
		style={{
			display: 'flex',
			flexDirection: direction,
			alignItems: align,
			justifyContent: justify,
			gap,
			padding: safe ? `${space.safeY}px ${space.safeX}px` : 0,
			transform: x || y ? `translate(${x}px, ${y}px)` : undefined,
			...style,
		}}
	>
		{children}
	</AbsoluteFill>
);
