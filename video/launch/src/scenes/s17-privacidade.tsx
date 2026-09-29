/**
 * s17-privacidade — S17 · abs 1230–1379 (150 f) · proof · bar 21.3 → 24.1
 *
 * The breather and the trust proof, no UI. f0–5 blur-dissolve in-half on the
 * background + UBI (the type stays crisp); line 1 masks up from f0 (≥ 0.9 by f8).
 * f0–44 UBI idles (idle 75→119). f45 (beat 2) internal jump-cut to a composition
 * scale 1.08; the window-title chip "ana@example.com" is set below the lines and
 * UBI's look clip starts. f60 line 2 masks up. f76–88 a volt marker sweeps the
 * address. f90 (downbeat) CONTACT: the address collapses into "[email]" (SNAPPY
 * width morph, old glyphs blur/fade 4 f). f92–104 a 2-px volt line draws from the
 * token up to "IA" and underlines it; "mínimo." turns volt. f105–149 the build:
 * the push accelerates (E.exit), grid floor 0 → 30 %, volt orb 0.16 → 0.24.
 * f150: hard cut on the final hit.
 */
import React from 'react';
import {AbsoluteFill} from 'remotion';
import {evolvePath} from '@remotion/paths';
import {alpha, color} from '../design/tokens';
import {E, springAt, TransitionIn, TransitionOut, UbiTrack, useSceneFrame, type SfxCue} from '../shared';
import {clamp01, FilmGrain, lerp, MaskLine, MEASURED, ramp, Stage, TYPE, Vignette} from './_parts/G6/common';

/** SFX cues, scene-relative HIT frames (the master audio layer places them at abs = start + atFrame − hit offset). */
export const sfx: SfxCue[] = [
	{ref: 'ui_tick_1.wav', atFrame: 76, gainDb: -22, note: 'Marker sweep starts.'},
	{ref: 'pop.wav', atFrame: 90, gainDb: -14, note: 'Mask contact.'},
];

/* ------------------------------------------------------------------------ */
/* Layout (canvas px at camera scale 1)                                      */
/* ------------------------------------------------------------------------ */

const COL_X = 740;
const L1 = {capTop: 300, size: 72};
const L2 = {capTop: 396, size: 72};
const L2_BASELINE = L2.capTop - TYPE.sora.capTop * L2.size + TYPE.sora.baseline * L2.size; // 448.6

/** Window-title chip: panel-2, radius 22, 1.5 px border, 96 px tall, 28-px window glyph, Inter 48. */
const CHIP = {x: COL_X, y: 520, h: 96, padL: 28, glyph: 28, gap: 18, padR: 32, radius: 22} as const;
const TEXT_X = CHIP.padL + CHIP.glyph + CHIP.gap; // 74, text start inside the chip
const CHIP_W_ADDRESS = TEXT_X + MEASURED.address + CHIP.padR; // ≈ 537 → x 740–1277
const CHIP_W_TOKEN = TEXT_X + MEASURED.token + CHIP.padR; // ≈ 263 → x 740–1003

/** Connector: token top-centre → under "IA" (line 2), then an underline across "IA". */
const TOKEN_CX = CHIP.x + TEXT_X + MEASURED.token / 2;
const IA = {left: COL_X + MEASURED.line2BeforeIA, w: MEASURED.ia};
const IA_CX = IA.left + IA.w / 2;
const UNDER_Y = Math.round(L2_BASELINE + 0.16 * L2.size); // ≈ 460
const CONNECTOR = `M ${TOKEN_CX.toFixed(1)} ${CHIP.y} C ${TOKEN_CX.toFixed(1)} ${CHIP.y - 30} ${IA_CX.toFixed(1)} ${UNDER_Y + 30} ${IA_CX.toFixed(1)} ${UNDER_Y}`;

/** UBI: 780-px frame (k 0.867), feet anchor on (420, 930). */
const UBI = {x: 420, y: 930, size: 780} as const;

/* ------------------------------------------------------------------------ */
/* Timing                                                                    */
/* ------------------------------------------------------------------------ */

const JUMP = 45;
const LINE2 = 60;
const SWEEP = 76;
const SWEEP_END = 88;
const CONTACT = 90;
const DRAW = 92;
const DRAW_END = 104;
const BUILD = 105;

/**
 * Composition camera (storyboard keys): 1.0 (slight life drift) → hard 1.08 at
 * f45 → 1.10 linear by f119 → 1.16 E.exit by f149. Scale origin (1435, 460): the
 * text column stays inside title-safe at 1.16 (right edge 1729 → 1776) and UBI
 * grows toward the left edge.
 */
const ORIGIN = {x: 1435, y: 460};
const camZoom = (f: number) => {
	if (f < JUMP) return lerp(1, 1.008, f / (JUMP - 1));
	if (f <= 119) return lerp(1.08, 1.1, (f - JUMP) / (119 - JUMP));
	return lerp(1.1, 1.16, E.exit(clamp01((f - 119) / 30)));
};

/* ------------------------------------------------------------------------ */
/* Pieces                                                                    */
/* ------------------------------------------------------------------------ */

const WindowGlyph: React.FC<{tint: string}> = ({tint}) => (
	<svg width={28} height={28} viewBox="0 0 28 28" style={{display: 'block'}}>
		<rect x={2} y={4} width={24} height={20} rx={4.5} fill="none" stroke={tint} strokeWidth={2} />
		<line x1={2} y1={10.5} x2={26} y2={10.5} stroke={tint} strokeWidth={2} />
		<circle cx={6.4} cy={7.3} r={1.1} fill={tint} />
		<circle cx={9.8} cy={7.3} r={1.1} fill={tint} />
	</svg>
);

const Chip: React.FC<{f: number; address: string; token: string}> = ({f, address, token}) => {
	if (f < JUMP) return null;
	// width morph (SNAPPY) on contact
	const morph = f < CONTACT ? 0 : springAt(f, CONTACT, 'SNAPPY');
	const w = lerp(CHIP_W_ADDRESS, CHIP_W_TOKEN, morph);
	// state: panel-2 → volt 14 %
	const st = ramp(f, CONTACT, CONTACT + 4, E.enter);
	const bg = st <= 0 ? color.panel2 : `color-mix(in srgb, ${color.panel2} ${((1 - st) * 100).toFixed(1)}%, rgba(77, 141, 255, 0.14))`;
	const border = st <= 0 ? 'rgba(255, 255, 255, 0.10)' : alpha(color.volt, lerp(0.1, 0.42, st));
	const burst = f >= CONTACT ? 1 - ramp(f, CONTACT, CONTACT + 14, E.enter) : 0;
	// marker sweep f76–88 (E.glide), fades into the chip's volt state on contact
	const sweep = ramp(f, SWEEP, SWEEP_END, E.glide);
	const markerO = f < CONTACT ? 1 : 1 - ramp(f, CONTACT, CONTACT + 2);
	const edge = f >= SWEEP && f <= SWEEP_END + 2 ? 1 - ramp(f, SWEEP_END, SWEEP_END + 2) : 0;
	// the address goes: blur 0 → 6 px, fade, squeeze toward the left over 4 f
	// (f90 1 → f91 .45 → f92 .12 → f93 0: gone before the token is fully in)
	const gone = f < CONTACT ? 0 : [0, 0.55, 0.88, 1][Math.min(3, f - CONTACT)];
	// the token lands f92: blur 6 → 0, scale .94 → 1 (SNAPPY), opacity f90.5 → f92
	const tokIn = f < CONTACT ? 0 : springAt(f, CONTACT, 'SNAPPY');
	const tokO = ramp(f, CONTACT + 0.5, CONTACT + 2);
	const tokSharp = ramp(f, CONTACT, CONTACT + 2, E.enter); // blur 6 → 0 by f92 (landed)
	const markW = MEASURED.address + 16;
	return (
		<div
			style={{
				position: 'absolute',
				left: CHIP.x,
				top: CHIP.y,
				width: w,
				height: CHIP.h,
				boxSizing: 'border-box',
				borderRadius: CHIP.radius,
				background: bg,
				border: `1.5px solid ${border}`,
				overflow: 'hidden',
				boxShadow: [
					'inset 0 1px 0 rgba(255,255,255,0.05)',
					'0 10px 24px -8px rgba(0,0,0,0.55)',
					burst > 0.01 ? `0 0 ${Math.round(44 * burst)}px ${alpha(color.volt, 0.35 * burst)}` : null,
				]
					.filter(Boolean)
					.join(', '),
			}}
		>
			<div style={{position: 'absolute', left: CHIP.padL - 1.5, top: (CHIP.h - CHIP.glyph) / 2 - 1.5}}>
				<WindowGlyph tint={st > 0.5 ? color.volt : color.ink3} />
			</div>
			{/* marker */}
			{f >= SWEEP && markerO > 0.01 ? (
				<div
					style={{
						position: 'absolute',
						left: TEXT_X - 8 - 1.5,
						top: CHIP.h / 2 - 30 - 1.5,
						width: markW * sweep,
						height: 60,
						borderRadius: 10,
						background: alpha(color.volt, 0.26),
						opacity: markerO,
					}}
				>
					{edge > 0.01 ? (
						<div
							style={{
								position: 'absolute',
								right: -1,
								top: -6,
								bottom: -6,
								width: 3,
								borderRadius: 2,
								background: color.volt,
								opacity: edge,
								boxShadow: `0 0 12px ${alpha(color.volt, 0.6)}`,
							}}
						/>
					) : null}
				</div>
			) : null}
			{/* address */}
			{gone < 1 ? (
				<div
					style={{
						position: 'absolute',
						left: TEXT_X - 1.5,
						top: -1.5,
						height: CHIP.h,
						display: 'flex',
						alignItems: 'center',
						fontFamily: '"Inter", sans-serif',
						fontWeight: 500,
						fontSize: 48,
						lineHeight: 1,
						letterSpacing: '-0.01em',
						color: color.ink,
						whiteSpace: 'nowrap',
						opacity: 1 - gone,
						filter: gone > 0 ? `blur(${(6 * gone).toFixed(2)}px)` : undefined,
						transformOrigin: '0% 50%',
						transform: gone > 0 ? `scaleX(${lerp(1, 0.8, gone).toFixed(4)})` : undefined,
					}}
				>
					{address}
				</div>
			) : null}
			{/* token */}
			{tokO > 0 ? (
				<div
					style={{
						position: 'absolute',
						left: TEXT_X - 1.5,
						top: -1.5,
						height: CHIP.h,
						display: 'flex',
						alignItems: 'center',
						fontFamily: '"Inter", sans-serif',
						fontWeight: 600,
						fontSize: 48,
						lineHeight: 1,
						letterSpacing: '-0.01em',
						color: color.volt,
						whiteSpace: 'nowrap',
						opacity: tokO,
						filter: tokSharp < 1 ? `blur(${(6 * (1 - tokSharp)).toFixed(2)}px)` : undefined,
						transformOrigin: '0% 50%',
						transform: tokIn < 0.999 ? `scale(${lerp(0.94, 1, tokIn).toFixed(4)})` : undefined,
					}}
				>
					{token}
				</div>
			) : null}
		</div>
	);
};

const Connector: React.FC<{f: number}> = ({f}) => {
	if (f < DRAW) return null;
	const p = ramp(f, DRAW, DRAW + 9, E.glide);
	const u = ramp(f, DRAW + 8, DRAW_END, E.push);
	const dot = springAt(f, DRAW, 'SNAPPY');
	const ev = evolvePath(p, CONNECTOR);
	const half = (IA.w / 2 + 4) * u;
	return (
		<svg width={1920} height={1080} style={{position: 'absolute', left: 0, top: 0, overflow: 'visible'}}>
			<path d={CONNECTOR} fill="none" stroke={color.volt} strokeWidth={2} strokeLinecap="round" strokeDasharray={ev.strokeDasharray} strokeDashoffset={ev.strokeDashoffset} />
			<circle cx={TOKEN_CX} cy={CHIP.y} r={4 * Math.min(1.1, dot)} fill={color.volt} />
			{u > 0 ? <line x1={IA_CX - half} y1={UNDER_Y} x2={IA_CX + half} y2={UNDER_Y} stroke={color.volt} strokeWidth={3} strokeLinecap="round" /> : null}
		</svg>
	);
};

/* ------------------------------------------------------------------------ */
/* Scene                                                                     */
/* ------------------------------------------------------------------------ */

const S17Privacidade: React.FC = () => {
	const {frame: f, scene} = useSceneFrame();
	const [line1, line2, address, token] = scene.copy.map((c) => c.text);
	const emph = scene.copy[1].emphasis[0] ?? 'mínimo.';
	const cut = line2.lastIndexOf(emph);
	const line2a = cut >= 0 ? line2.slice(0, cut) : line2;
	const line2b = cut >= 0 ? line2.slice(cut) : '';

	const z = camZoom(f);
	const build = ramp(f, BUILD, 149, E.enter);
	const orbVolt = lerp(0.16, 0.24, build);
	const floor = 0.3 * ramp(f, BUILD, 149, E.glide);
	const volt = ramp(f, DRAW + 6, DRAW_END + 2, E.enter); // "mínimo." turns volt as the line lands

	const emphColor = volt <= 0 ? color.ink : volt >= 1 ? color.volt : `color-mix(in srgb, ${color.volt} ${(volt * 100).toFixed(1)}%, ${color.ink})`;

	return (
		<TransitionOut>
			<AbsoluteFill style={{backgroundColor: color.canvas}}>
				{/* composition camera */}
				<AbsoluteFill style={{transform: `scale(${z.toFixed(5)})`, transformOrigin: `${ORIGIN.x}px ${ORIGIN.y}px`}}>
					{/* storyboard in-half (blur-dissolve 6 f) on the background + UBI only */}
					<TransitionIn>
						<Stage
							seed="s17"
							orbs={[
								{c: color.volt, x: 1560, y: 170, d: 1150, opacity: orbVolt},
								{c: color.ember, x: 300, y: 960, d: 820, opacity: 0.07},
							]}
							floor={floor}
							lineAlpha={0.5}
							horizon={0.7}
							gridSpeed={0.6}
						/>
						<UbiTrack segments={scene.ubiTrack} x={UBI.x} y={UBI.y} size={UBI.size} float={8} floatPeriod={126} floorGlow={0.3} shadow={0.4} />
					</TransitionIn>

					{/* type: crisp, outside the dissolve */}
					<MaskLine frame={f} at={0} capTop={L1.capTop} left={COL_X} family="sora" size={L1.size} weight={700} tracking="-0.03em" color={color.ink} ariaLabel={line1}>
						{line1}
					</MaskLine>
					<MaskLine frame={f} at={LINE2} capTop={L2.capTop} left={COL_X} family="sora" size={L2.size} weight={700} tracking="-0.03em" color={color.ink} ariaLabel={line2}>
						{line2a}
						<span style={{color: emphColor}}>{line2b}</span>
					</MaskLine>
					<Chip f={f} address={address} token={token} />
					<Connector f={f} />
				</AbsoluteFill>

				<Vignette strength={0.55} />
				<FilmGrain opacity={0.045} seed="s17" />
			</AbsoluteFill>
		</TransitionOut>
	);
};

export default S17Privacidade;
