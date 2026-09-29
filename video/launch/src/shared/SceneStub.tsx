/**
 * Placeholder picture for a scene that has not been built yet: background,
 * scene id, its storyboard copy lines (lit while on screen) and a frame
 * counter, wrapped in the scene's split transitions so the plumbing is
 * visible. Act builders replace it with the real scene.
 */
import React from 'react';
import {AbsoluteFill, useCurrentFrame} from 'remotion';
import {Background} from '../components/Background';
import {alpha, color, font} from '../design/tokens';
import {sceneById, sceneCompId, type SceneId} from '../storyboard';
import {barBeat} from './beats';
import {SceneTransitions} from './transitions';

export const SceneStub: React.FC<{id: SceneId}> = ({id}) => {
	const scene = sceneById(id);
	const frame = useCurrentFrame();
	const abs = scene.startFrame + frame;
	const dur = scene.durationInFrames;
	const tIn = scene.transitionIn;
	const tOut = scene.transitionOut;
	const shotIdx = scene.shots.filter((s) => s <= frame).length;
	return (
		<SceneTransitions>
			<Background variant="plain" seed={id} grain={0.03}>
				<AbsoluteFill style={{fontFamily: font.text, color: color.ink}}>
					{/* header */}
					<div
						style={{
							position: 'absolute',
							left: 120,
							top: 72,
							right: 120,
							display: 'flex',
							justifyContent: 'space-between',
							fontSize: 26,
							fontWeight: 600,
							letterSpacing: '0.08em',
							textTransform: 'uppercase',
							color: color.ink3,
						}}
					>
						<span>
							<span style={{color: color.volt}}>{sceneCompId(id)}</span> · {id} · {scene.act} · stub
						</span>
						<span style={{fontVariantNumeric: 'tabular-nums'}}>
							f {String(frame).padStart(3, '0')} / {dur} · abs {String(abs).padStart(4, '0')} · {barBeat(abs)}
						</span>
					</div>
					{/* copy lines */}
					<div
						style={{
							position: 'absolute',
							left: 160,
							right: 160,
							top: 200,
							bottom: 220,
							display: 'flex',
							flexDirection: 'column',
							justifyContent: 'center',
							alignItems: 'center',
							gap: 22,
							textAlign: 'center',
						}}
					>
						{scene.copy.length === 0 ? <div style={{fontSize: 40, color: color.ink3}}>(no copy)</div> : null}
						{scene.copy.map((c, i) => {
							const on = frame >= c.inFrame && frame < c.outFrame;
							const landed = frame >= c.landFrame && frame < c.outFrame;
							const big = /headline|slam|display|hero|wordmark/i.test(c.role);
							return (
								<div key={i} style={{opacity: on ? 1 : 0.22}}>
									<div
										style={{
											fontFamily: big ? font.display : font.text,
											fontWeight: big ? 700 : 500,
											fontSize: big ? 72 : 40,
											letterSpacing: big ? '-0.03em' : '-0.005em',
											lineHeight: 1.1,
											color: landed ? color.ink : color.ink2,
										}}
									>
										{c.text}
									</div>
									<div style={{marginTop: 6, fontSize: 18, color: color.ink3, fontVariantNumeric: 'tabular-nums'}}>
										{c.role} · in {c.inFrame} · land {c.landFrame} · out {c.outFrame}
									</div>
								</div>
							);
						})}
					</div>
					{/* footer: transitions + shot/frame bar */}
					<div style={{position: 'absolute', left: 120, right: 120, bottom: 96, fontSize: 22, color: color.ink3, display: 'flex', justifyContent: 'space-between'}}>
						<span>
							in: {tIn.type} {tIn.frames} f · out: {tOut.type} {tOut.frames} f
						</span>
						<span>
							shot {shotIdx}/{scene.shots.length} · {scene.sfx.length} sfx · {scene.timecode}
						</span>
					</div>
					<div style={{position: 'absolute', left: 120, right: 120, bottom: 72, height: 6, borderRadius: 3, background: alpha(color.ink3, 0.25)}}>
						<div style={{position: 'absolute', left: 0, top: 0, bottom: 0, width: `${((frame + 1) / dur) * 100}%`, borderRadius: 3, background: color.volt}} />
						{scene.shots.map((s) => (
							<div key={s} style={{position: 'absolute', left: `${(s / dur) * 100}%`, top: -6, width: 2, height: 18, background: color.ink2}} />
						))}
					</div>
				</AbsoluteFill>
			</Background>
		</SceneTransitions>
	);
};
