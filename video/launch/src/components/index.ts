/**
 * Motion-graphics primitive library for the ubiqX AI launch video.
 * See ./README.md for props and copy-paste examples.
 */
export {Background, type BackgroundProps, type OrbSpec} from './Background';
export {KineticText, type KineticTextProps, type KineticMode, type KineticExit, type UnderlineSpec, type HighlightStyle} from './KineticText';
export {Screen, useScreenGeometry, resolveSrc, SPOTLIGHT_DIM, WINDOW_SHADOW, GLASS_INSET, RIM_PX, rimBackground, type ScreenProps} from './Screen';
export {
	Stage,
	StageBase,
	StageLights,
	StageFinish,
	STAGE,
	navyDim,
	v2Dim,
	vignetteAlpha,
	DEFAULT_KEY,
	DEFAULT_POOLS,
	DEFAULT_KEY_LIGHT,
	type StageProps,
	type StageLook,
	type StagePool,
	type StageKeyLight,
} from './Stage';
export {
	screenGeometry,
	mapImagePoint,
	mapImageRect,
	mapWithGeometry,
	onScreenScale,
	GRADE,
	resolveGrade,
	gradeFilter,
	gradeHex,
	type Grade,
	type ScreenConfig,
	type CameraKey,
	type SpotlightSpec,
	type Rect,
	type Point,
	type ScreenGeometry,
} from './screen-geometry';
export {Cursor, cursorPathPoint, useImageToComp, type CursorProps, type CursorKey} from './Cursor';
export {Callout, type CalloutProps} from './Callout';
export {KeyCap, KeyCombo, type KeyCapProps, type KeyComboProps} from './KeyCap';
export {Counter, type CounterProps, type CounterFormat} from './Counter';
export {FeatureCard, CardGrid, type FeatureCardProps, type CardGridProps} from './FeatureCard';
export {LogoRow, type LogoRowProps, type LogoItem} from './LogoRow';
export {
	whipPan,
	zoomThrough,
	maskWipe,
	flashCut,
	blurDissolve,
	beatTiming,
	TransitionIn,
	TransitionOut,
	DirectionalBlur,
	Flash,
	type WhipPanProps,
	type ZoomThroughProps,
	type MaskWipeProps,
	type FlashCutProps,
	type BlurDissolveProps,
} from './Transitions';
export {LiftCard, liftCardPose, type LiftCardProps, type LiftCardEnter, type LiftCardExit} from './LiftCard';
export {Grain, type GrainProps} from './Grain';
export {Glow, type GlowProps} from './Glow';
export {MotionBlur, type MotionBlurProps} from './MotionBlur';
export {Sfx, Music, hasStaticFile, type SfxProps, type MusicProps} from './Sfx';
export {Center, type CenterProps} from './Layout';
export {Wordmark, type WordmarkProps} from './Wordmark';
