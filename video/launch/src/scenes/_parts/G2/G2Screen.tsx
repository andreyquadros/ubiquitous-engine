/**
 * G2Screen: the shared <Screen> with neutral #3a4560 title-bar dots
 * (style §S10). This used to be a full local copy of Screen.tsx because the
 * shared component hard-coded macOS traffic lights; the integrator added the
 * `dots` prop to <Screen>, so this is now a thin wrapper (pixel-identical).
 */
import React from 'react';
import {Screen, useScreenGeometry, type ScreenProps} from '../../../components/Screen';

export type {ScreenConfig, CameraKey, SpotlightSpec, Rect, Point} from '../../../components/screen-geometry';
export type {ScreenProps};

/** Inside <G2Screen> children: the geometry of the enclosing screen at the current frame. */
export const useG2ScreenGeometry = useScreenGeometry;

export const G2Screen: React.FC<ScreenProps> = (props) => <Screen dots="neutral" {...props} />;
