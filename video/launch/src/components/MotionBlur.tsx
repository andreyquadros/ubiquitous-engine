import React from 'react';
import {AbsoluteFill} from 'remotion';
import {CameraMotionBlur} from '@remotion/motion-blur';
import {isDraft} from '../design/env';

export type MotionBlurProps = {
	/** Sub-frame samples. Cost scales linearly (6 samples ≈ 6× render time for the subtree). Default 6. */
	samples?: number;
	/** Shutter angle in degrees (180 = film look, 360 = maximum smear). Default 180. */
	shutterAngle?: number;
	/** Turn off without restructuring the tree (e.g. for drafts). Default true. */
	enabled?: boolean;
	children: React.ReactNode;
};

/**
 * True temporal motion blur (renders the subtree N times at sub-frame offsets
 * and averages them). Wrap ONLY short, fast-moving segments — it multiplies
 * render cost. For whips prefer the cheaper <DirectionalBlur>.
 * Automatically disabled in draft renders (`npm run render:draft`).
 *
 * @example
 * <MotionBlur samples={6}><FlyingCard /></MotionBlur>
 */
export const MotionBlur: React.FC<MotionBlurProps> = ({samples = 6, shutterAngle = 180, enabled = true, children}) => {
	if (!enabled || samples <= 1 || isDraft()) return <AbsoluteFill>{children}</AbsoluteFill>;
	return (
		<CameraMotionBlur samples={samples} shutterAngle={shutterAngle}>
			{children}
		</CameraMotionBlur>
	);
};
