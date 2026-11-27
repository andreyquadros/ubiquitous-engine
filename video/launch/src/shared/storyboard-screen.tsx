/**
 * Adapters from the storyboard's UI-plane fields to the <Screen> primitive:
 * camera keys (+ tilt), spotlights and claim-safety patches.
 *
 * Storyboard camera keys give `zoom` (Screen zoom: bitmap scale = 0.5 × zoom
 * for a 2880-px capture at width 1440), `focus` (image px) placed on `anchor`
 * (canvas px), the ARRIVAL frame `atFrame`, an easing name and sometimes a
 * travel `duration`. Rules used here:
 *   - "hard…", "hold…", "arrives with…" or a first key → duration 0 (cut).
 *   - explicit `duration` → that many frames before atFrame.
 *   - otherwise (linear drifts, E.glide spans without duration) → travel from
 *     the previous key's atFrame.
 */
import React from 'react';
import {useCurrentFrame} from 'remotion';
import type {CameraKey, SpotlightSpec} from '../components/screen-geometry';
import {v2Dim} from '../components/Stage';
import type {Keyframe} from '../design/motion';
import type {SceneSpec, StoryboardCameraKey, StoryboardPatch} from '../storyboard';
import {easeByName, E} from './motion';
import {hotspot, uiFileName} from './ui';

export type StoryboardCameraOptions = {
	/** Keep only keys for this capture (ui/<file>.png). Omit to keep every UI key (same-camera capture swaps). */
	file?: string;
	/** Keep only keys for this layer ("backdrop" / "subject" in s15). */
	layer?: string;
};

const isCut = (k: StoryboardCameraKey) => /^(hard|hold|arrives)/i.test(k.ease.trim());

/** The scene's UI camera keys (target ≠ "canvas"), filtered. */
export const uiCameraKeys = (scene: SceneSpec, opts: StoryboardCameraOptions = {}): StoryboardCameraKey[] =>
	scene.camera
		.filter((k) => k.target !== 'canvas')
		.filter((k) => !opts.file || (k.file && uiFileName(k.file) === uiFileName(opts.file)))
		.filter((k) => !opts.layer || k.layer === opts.layer)
		.sort((a, b) => a.atFrame - b.atFrame);

/**
 * Screen `camera` + `rotateX` / `rotateY` keyframes for a scene, straight
 * from the storyboard. Spread into a ScreenConfig:
 *
 * @example
 * const scene = useScene();
 * const shot: ScreenConfig = {src: 'ui/dashboard.png', ...storyboardCamera(scene, {file: 'ui/dashboard.png'})};
 */
export const storyboardCamera = (
	scene: SceneSpec,
	opts: StoryboardCameraOptions = {},
): {camera: CameraKey[]; rotateX: Keyframe[]; rotateY: Keyframe[]} => {
	const keys = uiCameraKeys(scene, opts);
	const camera: CameraKey[] = [];
	const rotateX: Keyframe[] = [];
	const rotateY: Keyframe[] = [];
	keys.forEach((k, i) => {
		const prev = keys[i - 1];
		const duration = k.duration !== undefined ? k.duration : !prev || isCut(k) ? 0 : Math.max(0, k.atFrame - prev.atFrame);
		const easing = duration > 0 ? easeByName(k.ease, duration) : E.linear;
		const cam: CameraKey = {at: k.atFrame, zoom: k.zoom, duration, easing};
		if (k.focus) cam.focus = {x: k.focus.x, y: k.focus.y};
		else if (k.file && k.target !== 'full') cam.rect = hotspot(k.file, k.target);
		if (k.anchor) cam.anchor = {x: k.anchor.x, y: k.anchor.y};
		camera.push(cam);
		const rx = k.tilt?.rx ?? 0;
		const ry = k.tilt?.ry ?? 0;
		if (prev) {
			// hold the previous tilt until this move starts (or until the frame before a cut)
			const holdAt = Math.max(prev.atFrame, duration > 0 ? k.atFrame - duration : k.atFrame - 1);
			rotateX.push([holdAt, prev.tilt?.rx ?? 0]);
			rotateY.push([holdAt, prev.tilt?.ry ?? 0]);
		}
		rotateX.push([k.atFrame, rx, easing]);
		rotateY.push([k.atFrame, ry, easing]);
	});
	return {camera, rotateX, rotateY};
};

/**
 * Screen `spotlights` for a scene (optionally only those of one capture).
 * Hotspot names are resolved from ui-manifest.json. v2: the board's dims were
 * tuned for v1's black 0.62 spotlight; they are mapped with `v2Dim` (0.62 → 0.38).
 */
export const storyboardSpotlights = (scene: SceneSpec, file?: string): SpotlightSpec[] =>
	scene.spotlights
		.filter((s) => !file || uiFileName(s.file) === uiFileName(file))
		.map((s) => ({
			rect: s.rect ?? hotspot(s.file, s.hotspot ?? ''),
			at: s.from,
			until: s.to,
			dim: s.dim === undefined ? undefined : v2Dim(s.dim),
		}));

/** The scene's claim-safety patches for one capture. */
export const storyboardPatches = (scene: SceneSpec, file: string): StoryboardPatch[] =>
	scene.patches.filter((p) => uiFileName(p.file) === uiFileName(file));

/**
 * Solid image-space patches (claim safety: key legends, model lines, counts).
 * Render as a CHILD of <Screen> so they move with the plane. Each patch is
 * drawn on frames from ≤ f ≤ to, in the sampled card colour.
 *
 * @example <Screen {...shot}><Patches patches={storyboardPatches(scene, shot.src)} /></Screen>
 */
export const Patches: React.FC<{patches: StoryboardPatch[]; bleed?: number}> = ({patches, bleed = 2}) => {
	const frame = useCurrentFrame();
	return (
		<>
			{patches
				.filter((p) => frame >= p.from && frame <= p.to)
				.map((p, i) => (
					<div
						key={i}
						style={{
							position: 'absolute',
							left: p.rect.x - bleed,
							top: p.rect.y - bleed,
							width: p.rect.w + bleed * 2,
							height: p.rect.h + bleed * 2,
							background: p.color,
						}}
					/>
				))}
		</>
	);
};
