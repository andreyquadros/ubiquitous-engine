/**
 * G2-local copy of src/components/Screen.tsx (scene-local workaround): the
 * storyboard asks for the "mac" chrome with NEUTRAL #3a4560 dots (style §S10,
 * platform-agnostic), but the shared <Screen> hard-codes the macOS traffic-light
 * colours. Identical geometry (same screenGeometry), hires twin support, spotlights,
 * sheen and image-space children — only the three dots differ.
 */
import React, {createContext, useContext} from 'react';
import {AbsoluteFill, Img, staticFile, useCurrentFrame, useVideoConfig} from 'remotion';
import {alpha, color, ease, layer, resolveColor, shadow} from '../../../design/tokens';
import {progress} from '../../../design/motion';
import {screenGeometry, type ScreenConfig, type ScreenGeometry, type SpotlightSpec} from '../../../components/screen-geometry';
import {useHires} from '../../../shared/ui';

export type {ScreenConfig, CameraKey, SpotlightSpec, Rect, Point} from '../../../components/screen-geometry';

export type ScreenProps = ScreenConfig & {
	/**
	 * Overlays drawn IN IMAGE SPACE: children are laid out in a box of
	 * imageSize (e.g. 2880x1800) that is scaled/tilted/zoomed with the
	 * screenshot, so `left: 1200, top: 400` means image pixel (1200, 400).
	 * Use it to fake live UI (a number ticking, a row appearing, a toast).
	 */
	children?: React.ReactNode;
	/** Extra style on the outer (full-frame) container. */
	style?: React.CSSProperties;
};

const GeometryContext = createContext<ScreenGeometry | null>(null);

/** Inside <G2Screen> children: the geometry of the enclosing screen at the current frame. */
export const useG2ScreenGeometry = (): ScreenGeometry | null => useContext(GeometryContext);

const resolveSrc = (src: string): string =>
	/^(https?:|data:|blob:|\/)/.test(src) ? src : staticFile(src);

/**
 * A product screenshot in a macOS-style window, floating in 3D, with a
 * camera that can zoom to any rect given in the screenshot's own pixels.
 *
 * Define the shot once and share it with overlays so they stay glued:
 *
 * @example
 * const shot: ScreenConfig = {
 *   src: 'ui/dashboard.png',                // 2880x1800
 *   enter: 'rise', rotateX: [[0, 14], [60, 0]], float: 6,
 *   camera: [{at: 45, rect: {x: 520, y: 240, w: 1400, h: 700}}],
 *   spotlights: [{at: 50, rect: {x: 560, y: 260, w: 1300, h: 640}}],
 * };
 * <Screen {...shot} />
 * <Cursor screen={shot} path={[{at: 20, x: 2000, y: 1500}, {at: 50, x: 1100, y: 600, click: true}]} />
 */
export const G2Screen: React.FC<ScreenProps> = ({children, style, ...cfg}) => {
	const frame = useCurrentFrame();
	const {width, height} = useVideoConfig();
	const g = screenGeometry(cfg, frame, {width, height});
	const r = (cfg.radius ?? 14) * (g.winW / 1440);
	const glowColor = cfg.glow === false ? null : resolveColor(cfg.glow ?? 'volt');
	const camK = g.k * g.scale;
	// hi-res twin (same layout, drawn into the same box; coordinates stay in imageSize space)
	const hires = useHires(cfg.hires === false || typeof cfg.hires === 'string' ? null : cfg.src);
	const drawSrc = typeof cfg.hires === 'string' ? cfg.hires : hires ? `ui/${hires.hires}` : cfg.src;

	return (
		<AbsoluteFill style={{zIndex: layer.screen, pointerEvents: 'none', ...style}}>
			{/* camera: comp = C + k (p - f) */}
			<AbsoluteFill
				style={{
					transformOrigin: '0 0',
					transform: `translate(${g.Cx - g.k * g.fx}px, ${g.Cy - g.k * g.fy}px) scale(${g.k})`,
				}}
			>
				<AbsoluteFill style={{perspective: g.perspective, perspectiveOrigin: `${g.cx}px ${g.cy}px`}}>
					<div
						style={{
							position: 'absolute',
							left: g.left,
							top: g.top,
							width: g.winW,
							height: g.winH,
							transformOrigin: '50% 50%',
							transform: `translateY(${g.bob}px) rotateX(${g.rx}deg) rotateY(${g.ry}deg) rotateZ(${g.rz}deg) scale(${g.scale})`,
							opacity: g.opacity,
							filter: g.blur > 0.2 ? `blur(${g.blur}px)` : undefined,
						}}
					>
						{/* ambient glow under the window */}
						{glowColor ? (
							<div
								style={{
									position: 'absolute',
									left: '-12%',
									right: '-12%',
									top: '-10%',
									bottom: '-18%',
									background: `radial-gradient(closest-side, ${alpha(glowColor, 0.22)} 0%, ${alpha(glowColor, 0.08)} 55%, transparent 100%)`,
								}}
							/>
						) : null}
						<div
							style={{
								position: 'absolute',
								inset: 0,
								borderRadius: r,
								overflow: 'hidden',
								background: color.panel,
								boxShadow: shadow.window,
							}}
						>
							{cfg.chrome === 'none' ? null : <TitleBar height={g.titleH} title={cfg.title} />}
							<div style={{position: 'absolute', left: 0, top: g.titleH, width: g.contentW, height: g.contentH, overflow: 'hidden'}}>
								<Img
									src={resolveSrc(drawSrc)}
									style={{position: 'absolute', left: 0, top: 0, width: g.contentW, height: g.contentH, display: 'block'}}
								/>
								{/* image-space overlay layer */}
								<div
									style={{
										position: 'absolute',
										left: 0,
										top: 0,
										width: g.imgW,
										height: g.imgH,
										transformOrigin: '0 0',
										transform: `scale(${g.s0})`,
									}}
								>
									<GeometryContext.Provider value={g}>
										{children}
										{(cfg.spotlights ?? []).map((s, i) => (
											<Spotlight key={i} spec={s} frame={frame} onScreen={camK * g.s0} />
										))}
									</GeometryContext.Provider>
								</div>
							</div>
							{/* glass: hairline + top highlight */}
							<div
								style={{
									position: 'absolute',
									inset: 0,
									borderRadius: r,
									boxShadow: `inset 0 0 0 1px rgba(255,255,255,0.09), inset 0 1px 0 rgba(255,255,255,0.12)`,
									pointerEvents: 'none',
								}}
							/>
							{cfg.sheenAt !== undefined ? <Sheen at={cfg.sheenAt} frame={frame} /> : null}
						</div>
					</div>
				</AbsoluteFill>
			</AbsoluteFill>
		</AbsoluteFill>
	);
};

const TitleBar: React.FC<{height: number; title?: string}> = ({height, title}) => {
	const d = Math.round(height * 0.32);
	const gap = Math.round(d * 0.66);
	const lights = ['#3a4560', '#3a4560', '#3a4560'];
	return (
		<div
			style={{
				position: 'absolute',
				left: 0,
				right: 0,
				top: 0,
				height,
				background: 'linear-gradient(180deg, #1a2236 0%, #131a2a 100%)',
				borderBottom: '1px solid rgba(255,255,255,0.06)',
				display: 'flex',
				alignItems: 'center',
				paddingLeft: Math.round(height * 0.42),
				gap,
			}}
		>
			{lights.map((c) => (
				<div
					key={c}
					style={{
						width: d,
						height: d,
						borderRadius: '50%',
						background: c,
						boxShadow: `inset 0 0 0 0.5px rgba(0,0,0,0.25), inset 0 1px 1px rgba(255,255,255,0.25)`,
					}}
				/>
			))}
			{title ? (
				<div
					style={{
						position: 'absolute',
						left: 0,
						right: 0,
						textAlign: 'center',
						fontFamily: '"Inter", sans-serif',
						fontSize: Math.round(height * 0.36),
						fontWeight: 500,
						color: color.ink3,
						letterSpacing: '-0.005em',
					}}
				>
					{title}
				</div>
			) : null}
		</div>
	);
};

const Spotlight: React.FC<{spec: SpotlightSpec; frame: number; onScreen: number}> = ({spec, frame, onScreen}) => {
	const fade = spec.fade ?? 10;
	const pin = progress(frame, spec.at, fade, ease.settle);
	const pout = spec.until !== undefined ? progress(frame, spec.until, fade, ease.exit) : 0;
	const v = pin * (1 - pout);
	if (v <= 0) return null;
	const pad = spec.pad ?? 14;
	const c = resolveColor(spec.color ?? 'volt');
	// keep the outline ~2.5 composition px regardless of zoom
	const bw = 2.5 / Math.max(0.05, onScreen);
	const glowPx = 26 / Math.max(0.05, onScreen);
	const grow = 1 + (1 - pin) * 0.06;
	return (
		<div
			style={{
				position: 'absolute',
				left: spec.rect.x - pad,
				top: spec.rect.y - pad,
				width: spec.rect.w + pad * 2,
				height: spec.rect.h + pad * 2,
				borderRadius: spec.radius ?? 20,
				transform: `scale(${grow})`,
				boxShadow: [
					spec.outline === false ? null : `0 0 0 ${bw}px ${alpha(c, 0.95 * v)}`,
					spec.outline === false ? null : `0 0 ${glowPx}px ${glowPx * 0.25}px ${alpha(c, 0.55 * v)}`,
					`0 0 0 6000px rgba(6, 9, 16, ${(spec.dim ?? 0.62) * v})`,
				]
					.filter(Boolean)
					.join(', '),
			}}
		/>
	);
};

const Sheen: React.FC<{at: number; frame: number}> = ({at, frame}) => {
	const p = progress(frame, at, 26, ease.inOut);
	if (p <= 0 || p >= 1) return null;
	return (
		<div style={{position: 'absolute', inset: 0, overflow: 'hidden', pointerEvents: 'none', mixBlendMode: 'screen'}}>
			<div
				style={{
					position: 'absolute',
					top: '-50%',
					bottom: '-50%',
					width: '28%',
					left: `${-40 + p * 150}%`,
					transform: 'rotate(18deg)',
					background: 'linear-gradient(90deg, transparent 0%, rgba(160,195,255,0.10) 40%, rgba(220,235,255,0.20) 50%, rgba(160,195,255,0.10) 60%, transparent 100%)',
				}}
			/>
		</div>
	);
};
