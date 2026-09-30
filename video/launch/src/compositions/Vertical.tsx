/**
 * LaunchVertical: the 9:16 cut (1080x1920) for Reels and TikTok.
 *
 * Built from the rendered 16:9 picture, not re-rendered scene code: every scene is restacked from
 * panels cropped out of public/vertical/film.mp4 (see src/vertical/layout.ts), feathered into an
 * ambient background (public/vertical/bg.mp4: the same film, scaled to cover and heavily blurred).
 * Muted: the master takes the film's audio mix (same frames, same length) in tools/master/master.py.
 *
 * Sources (gitignored, regenerable):
 *   cp out/launch-<tag>.mp4 public/vertical/film.mp4 && tools/vertical/make-bg.sh
 */
import React from 'react';
import {AbsoluteFill, Easing, OffthreadVideo, Sequence, interpolate, staticFile, useCurrentFrame} from 'remotion';
import {Grain} from '../components/Grain';
import {SCENES, type SceneSpec} from '../storyboard';
import {VLAYOUT, VW, VH, type VDst, type VKey, type VPanel, type VRect} from '../vertical/layout';

const FILM = 'vertical/film.mp4';
const BG = 'vertical/bg.mp4';
const SRC_W = 1920;
const SRC_H = 1080;
const ease = Easing.bezier(0.65, 0, 0.35, 1);

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

/** The panel's crop and placement at a scene-relative frame. */
const poseAt = (keys: VKey[], f: number): {src: VRect; dst: VDst} => {
	if (keys.length === 1 || f <= keys[0].f) return keys[0];
	const last = keys[keys.length - 1];
	if (f >= last.f) return last;
	const i = keys.findIndex((k) => k.f > f);
	const a = keys[i - 1];
	const b = keys[i];
	const t = ease((f - a.f) / (b.f - a.f));
	return {
		src: {x: lerp(a.src.x, b.src.x, t), y: lerp(a.src.y, b.src.y, t), w: lerp(a.src.w, b.src.w, t), h: lerp(a.src.h, b.src.h, t)},
		dst: {x: lerp(a.dst.x, b.dst.x, t), y: lerp(a.dst.y, b.dst.y, t), w: lerp(a.dst.w, b.dst.w, t)},
	};
};

const featherMask = (feather: VPanel['feather'], w: number, h: number): React.CSSProperties => {
	const [t, r, b, l] = typeof feather === 'number' ? [feather, feather, feather, feather] : feather ?? [80, 80, 80, 80];
	const pct = (px: number, size: number) => `${Math.min(49, (100 * px) / Math.max(1, size)).toFixed(3)}%`;
	const vertical = `linear-gradient(to bottom, transparent 0%, #000 ${pct(t, h)}, #000 calc(100% - ${pct(b, h)}), transparent 100%)`;
	const horizontal = `linear-gradient(to right, transparent 0%, #000 ${pct(l, w)}, #000 calc(100% - ${pct(r, w)}), transparent 100%)`;
	return {
		maskImage: `${vertical}, ${horizontal}`,
		maskComposite: 'intersect',
		WebkitMaskImage: `${vertical}, ${horizontal}`,
		WebkitMaskComposite: 'source-in',
	};
};

const Panel: React.FC<{panel: VPanel; scene: SceneSpec}> = ({panel, scene}) => {
	const f = useCurrentFrame();
	const from = panel.from ?? 0;
	const to = panel.to ?? scene.durationInFrames - 1;
	if (f < from || f > to) return null;
	const fade = panel.fade ?? 0;
	const opacity =
		fade > 0
			? Math.min(
					from > 0 ? interpolate(f, [from, from + fade], [0, 1], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}) : 1,
					panel.to !== undefined ? interpolate(f, [to - fade, to], [1, 0], {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'}) : 1,
				)
			: 1;
	const {src, dst} = poseAt(panel.keys, f);
	const s = dst.w / src.w;
	const h = src.h * s;
	return (
		<div
			style={{
				position: 'absolute',
				left: dst.x,
				top: dst.y,
				width: dst.w,
				height: h,
				overflow: 'hidden',
				opacity,
				zIndex: panel.z ?? 1,
				...featherMask(panel.feather, dst.w, h),
			}}
		>
			<OffthreadVideo
				src={staticFile(FILM)}
				trimBefore={scene.startFrame}
				muted
				style={{
					position: 'absolute',
					left: 0,
					top: 0,
					width: SRC_W,
					height: SRC_H,
					maxWidth: 'none',
					transformOrigin: '0 0',
					transform: `scale(${s}) translate(${-src.x}px, ${-src.y}px)`,
				}}
			/>
		</div>
	);
};

export const Vertical: React.FC = () => (
	<AbsoluteFill style={{background: 'linear-gradient(180deg, #0f1730 0%, #0c1328 55%, #0a0f20 100%)', width: VW, height: VH}}>
		{/* ambient: the same film, blurred, at half strength over the navy stage (panel edges fade into it) */}
		<OffthreadVideo src={staticFile(BG)} muted style={{position: 'absolute', inset: 0, width: VW, height: VH, opacity: 0.5}} />
		{/* a soft key light behind the content column */}
		<AbsoluteFill style={{background: 'radial-gradient(ellipse 70% 38% at 50% 46%, rgba(77,141,255,0.16) 0%, rgba(77,141,255,0.05) 55%, transparent 100%)', mixBlendMode: 'screen'}} />
		{SCENES.map((scene) => (
			<Sequence key={scene.id} from={scene.startFrame} durationInFrames={scene.durationInFrames} name={`v ${scene.id}`}>
				<AbsoluteFill>
					{VLAYOUT[scene.id].map((p) => (
						<Panel key={p.id} panel={p} scene={scene} />
					))}
				</AbsoluteFill>
			</Sequence>
		))}
		<Grain opacity={0.035} seed="vertical-grain" />
	</AbsoluteFill>
);
