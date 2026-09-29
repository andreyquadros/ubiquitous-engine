/**
 * "Launch" — the master composition. Scenes are placed from TIMELINE
 * (src/timeline.ts). Until real scenes land, the "placeholder" slot renders a
 * title card built from the primitives so the composition always renders.
 */
import React from 'react';
import {AbsoluteFill, Sequence} from 'remotion';
import {color, type} from '../design/tokens';
import {TIMELINE} from '../timeline';
import {Background, Center, KineticText, Wordmark} from '../components';

const Placeholder: React.FC = () => (
	<Background variant="grid" seed="launch-placeholder">
		<Center gap={40}>
			<Wordmark size={150} at={6} />
			<KineticText text="Retome o controle do seu dia." mode="stagger-words" size={type.h3} role="display" weight={600} color={color.ink2} at={24} />
		</Center>
	</Background>
);

/** Map timeline slot ids → scene components. Scene agents register here. */
export const SCENES: Record<string, React.FC> = {
	placeholder: Placeholder,
};

export const Launch: React.FC = () => (
	<AbsoluteFill style={{backgroundColor: color.canvas}}>
		{TIMELINE.scenes.map((s) => {
			const C = SCENES[s.id];
			if (!C) return null;
			return (
				<Sequence key={s.id} from={s.from} durationInFrames={s.durationInFrames} name={s.id}>
					<C />
				</Sequence>
			);
		})}
	</AbsoluteFill>
);
