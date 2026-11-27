import React from 'react';
import {useCurrentFrame, useVideoConfig} from 'remotion';
import {color, font, glow} from '../design/tokens';
import {clamp, lerp, springIn} from '../design/motion';

export type WordmarkProps = {
	/** Cap height driver: font size in px. Default 140. */
	size?: number;
	/** Append " AI" after the mark. Default true. */
	ai?: boolean;
	/** Frame to animate in (letters rise + the X lands last with a glow). Omit for static. */
	at?: number;
	/** Glow on the X. Default true. */
	glowX?: boolean;
};

/**
 * The "ubiqX AI" wordmark: Sora bold, "ubiq" in ink, "X" in volt.
 * With `at`, letters rise in and the X slams last.
 */
export const Wordmark: React.FC<WordmarkProps> = ({size = 140, ai = true, at, glowX = true}) => {
	const frame = useCurrentFrame();
	const {fps} = useVideoConfig();
	const letters = ['u', 'b', 'i', 'q'];
	const anim = (i: number) => (at === undefined ? 1 : springIn(frame, fps, at + i * 2, 'snappy'));
	const xS = at === undefined ? 1 : springIn(frame, fps, at + 10, 'subtleBounce');
	const aiS = at === undefined ? 1 : springIn(frame, fps, at + 16, 'smooth');
	return (
		<div
			style={{
				display: 'inline-flex',
				alignItems: 'baseline',
				fontFamily: font.display,
				fontSize: size,
				fontWeight: 700,
				letterSpacing: '-0.05em',
				lineHeight: 1,
				color: color.ink,
			}}
		>
			{letters.map((l, i) => {
				const s = anim(i);
				return (
					<span key={i} style={{display: 'inline-block', opacity: clamp(s * 1.5), transform: `translateY(${lerp(0.4, 0, s)}em)`}}>
						{l}
					</span>
				);
			})}
			<span
				style={{
					display: 'inline-block',
					color: color.volt,
					opacity: clamp(xS * 2),
					transform: `scale(${lerp(1.8, 1, clamp(xS))}) rotate(${lerp(-25, 0, clamp(xS))}deg)`,
					textShadow: glowX ? glow.text(color.volt, 0.9 * clamp(xS)) : undefined,
				}}
			>
				X
			</span>
			{ai ? (
				<span
					style={{
						display: 'inline-block',
						marginLeft: '0.22em',
						fontWeight: 600,
						letterSpacing: '-0.03em',
						color: color.ink2,
						opacity: clamp(aiS),
						transform: `translateX(${lerp(-0.2, 0, aiS)}em)`,
					}}
				>
					AI
				</span>
			) : null}
		</div>
	);
};
