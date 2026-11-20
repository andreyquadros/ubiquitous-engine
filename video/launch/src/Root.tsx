import React from 'react';
import {Composition} from 'remotion';
import './design/fonts';
import {FPS, HEIGHT, WIDTH} from './design/tokens';
import {GROUPS, TIMELINE} from './timeline';
import {SCENES, sceneCompId} from './storyboard';
import {Film} from './compositions/Film';
import {Launch} from './compositions/Launch';
import {Primitives, PRIMITIVES_DURATION} from './compositions/Primitives';
import {Vertical} from './compositions/Vertical';
import {VH, VW} from './vertical/layout';

/*
 * Compositions
 *  - Launch      the film (1560 f)
 *  - Primitives  the primitives reel
 *  - LaunchVertical  the 9:16 cut (1080x1920) for Reels / TikTok, restacked from the rendered film
 *  - S01..S18    one scene each (its own frames 0..duration−1, its own SFX + the music under it)
 *  - G1..G6      act groups: their scenes back to back with the master audio — an exact slice of Launch
 *                (frames relative to the group's first scene)
 */

const sceneComps = SCENES.map((s) => {
	const C: React.FC = () => <Film from={s.startFrame} to={s.startFrame + s.durationInFrames} sfxScope={[s.id]} />;
	C.displayName = `Scene_${s.id}`;
	return {id: sceneCompId(s.id), component: C, durationInFrames: s.durationInFrames};
});

const groupComps = GROUPS.map((g) => {
	const C: React.FC = () => <Film from={g.startFrame} to={g.startFrame + g.durationInFrames} />;
	C.displayName = `Group_${g.id}`;
	return {id: g.id, component: C, durationInFrames: g.durationInFrames};
});

export const RemotionRoot: React.FC = () => (
	<>
		<Composition id="Launch" component={Launch} durationInFrames={TIMELINE.durationInFrames} fps={FPS} width={WIDTH} height={HEIGHT} />
		<Composition id="LaunchVertical" component={Vertical} durationInFrames={TIMELINE.durationInFrames} fps={FPS} width={VW} height={VH} />
		<Composition id="Primitives" component={Primitives} durationInFrames={PRIMITIVES_DURATION} fps={FPS} width={WIDTH} height={HEIGHT} />
		{sceneComps.map((c) => (
			<Composition key={c.id} id={c.id} component={c.component} durationInFrames={c.durationInFrames} fps={FPS} width={WIDTH} height={HEIGHT} />
		))}
		{groupComps.map((c) => (
			<Composition key={c.id} id={c.id} component={c.component} durationInFrames={c.durationInFrames} fps={FPS} width={WIDTH} height={HEIGHT} />
		))}
	</>
);
