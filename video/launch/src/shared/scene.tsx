/**
 * Scene context: every scene component is mounted by the film (Launch, the
 * S01..S18 and G1..G6 compositions) inside <SceneProvider>, so any component
 * inside can ask which scene it is in, how long it lasts and what its
 * transitions are — frames stay SCENE-RELATIVE (useCurrentFrame() = 0 on the
 * scene's first frame).
 */
import React, {createContext, useContext} from 'react';
import {useCurrentFrame} from 'remotion';
import type {SceneSpec} from '../storyboard';

const SceneContext = createContext<SceneSpec | null>(null);

export const SceneProvider: React.FC<{scene: SceneSpec; children: React.ReactNode}> = ({scene, children}) => (
	<SceneContext.Provider value={scene}>{children}</SceneContext.Provider>
);

/** The storyboard spec of the enclosing scene, or null outside one (e.g. in the Primitives reel). */
export const useSceneOrNull = (): SceneSpec | null => useContext(SceneContext);

/** The storyboard spec of the enclosing scene. Throws outside a scene. */
export const useScene = (): SceneSpec => {
	const s = useContext(SceneContext);
	if (!s) throw new Error('useScene(): not inside a scene (mount scenes through src/compositions/Film.tsx).');
	return s;
};

/** Scene-relative frame plus handy derived values. */
export const useSceneFrame = (): {frame: number; abs: number; duration: number; scene: SceneSpec} => {
	const scene = useScene();
	const frame = useCurrentFrame();
	return {frame, abs: scene.startFrame + frame, duration: scene.durationInFrames, scene};
};
