import React from 'react';
import {Composition} from 'remotion';
import './design/fonts';
import {FPS, HEIGHT, WIDTH} from './design/tokens';
import {TIMELINE} from './timeline';
import {Launch} from './compositions/Launch';
import {Primitives, PRIMITIVES_DURATION} from './compositions/Primitives';

export const RemotionRoot: React.FC = () => (
	<>
		<Composition id="Launch" component={Launch} durationInFrames={TIMELINE.durationInFrames} fps={FPS} width={WIDTH} height={HEIGHT} />
		<Composition id="Primitives" component={Primitives} durationInFrames={PRIMITIVES_DURATION} fps={FPS} width={WIDTH} height={HEIGHT} />
	</>
);
