/**
 * Shared helpers for scene builders. See ./README.md for the API and examples.
 * Scenes import from here: `import {useScene, hotspot, UbiClip, TransitionIn} from '../shared';`
 */
export {
	STORYBOARD,
	SCENES,
	SCENE_IDS,
	sceneById,
	sceneAt,
	sceneEnd,
	sceneCompId,
	type SceneId,
	type SceneSpec,
	type CopyLine,
	type TransitionType,
	type TransitionHalf,
	type StoryboardCameraKey,
	type StoryboardCursorKey,
	type StoryboardSpotlight,
	type StoryboardPatch,
	type StoryboardScrim,
	type StoryboardUbiSegment,
	type StoryboardSfxCue,
	type Rect,
	type Point,
} from '../storyboard';
export {TIMELINE, TOTAL, GROUPS, groupById, groupOf, type Group, type GroupId, type SceneSlot} from '../timeline';
export {SceneProvider, useScene, useSceneOrNull, useSceneFrame} from './scene';
export {
	db,
	dbToGain,
	gainToDb,
	BED,
	sfxVolume,
	sfxInfo,
	SFX_NAMES,
	cueFileStart,
	placeCue,
	DUCKS,
	DUCK_ENVELOPE,
	duckGain,
	MUSIC,
	musicVolume,
	type SfxCue,
	type SfxInfo,
	type PlacedCue,
	type Duck,
} from './audio';
export {BAR, BEAT, BPM, FPS, bars, beats, sec, snapToBeat, eighth, sixteenth, isOnBeat, isDownbeat, barBeat, timecode} from './beats';
export {E, SPRING, springAt, springEasing, easeByName, type EName, type SpringName} from './motion';
export {
	hotspot,
	hasHotspot,
	hotspots,
	hotspotCenter,
	uiImage,
	uiFileName,
	UI_FILES,
	hiresOf,
	useHires,
	loadHiresTable,
	isUiCapture,
	bitmapUpsample,
	type UiImage,
	type HiresEntry,
} from './ui';
export {
	UbiClip,
	UbiTrack,
	UBI_CLIPS,
	UBI_ANCHORS,
	UBI_FRAME,
	ubiFrameIndex,
	ubiFrameSrc,
	ubiTrackAt,
	idleLeadIn,
	type UbiClipName,
	type UbiClipProps,
	type UbiTrackProps,
	type UbiPlacement,
	type UbiAnchor,
	type UbiPlayOptions,
} from './UbiClip';
export {
	TransitionIn,
	TransitionOut,
	SceneTransitions,
	splitTransition,
	halfProgress,
	useTransition,
	type SplitType,
	type SplitSide,
	type SplitState,
	type SplitOptions,
	type SplitWrapperProps,
} from './transitions';
export {storyboardCamera, storyboardSpotlights, storyboardPatches, uiCameraKeys, Patches, type StoryboardCameraOptions} from './storyboard-screen';
export {SceneStub} from './SceneStub';
