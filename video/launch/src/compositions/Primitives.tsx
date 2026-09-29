/**
 * "Primitives" — a dev reel that exercises every primitive (~2 s each).
 * Use it to eyeball changes to the library: `npm run studio` → Primitives.
 * The screenshot is the 1440x900 placeholder (public/ui/_placeholder.png).
 */
import React from 'react';
import {AbsoluteFill, Series} from 'remotion';
import {TransitionSeries} from '@remotion/transitions';
import {Bot, Clock3, FileText, Keyboard, ShieldCheck, Sparkles} from 'lucide-react';
import {color, font, tracking, type} from '../design/tokens';
import {
	Background,
	beatTiming,
	blurDissolve,
	Callout,
	CardGrid,
	Center,
	Counter,
	Cursor,
	flashCut,
	Flash,
	Glow,
	Grain,
	KeyCombo,
	KineticText,
	LogoRow,
	maskWipe,
	MotionBlur,
	Screen,
	Sfx,
	TransitionIn,
	whipPan,
	Wordmark,
	zoomThrough,
	type ScreenConfig,
} from '../components';
import {ease} from '../design/tokens';
import {tween} from '../design/motion';
import {useCurrentFrame} from 'remotion';

const PLACEHOLDER = {src: 'ui/_placeholder.png', imageSize: {w: 1440, h: 900}} as const;

const Label: React.FC<{n: number; name: string}> = ({n, name}) => (
	<div
		style={{
			position: 'absolute',
			left: 48,
			top: 40,
			zIndex: 100,
			fontFamily: font.text,
			fontSize: 18,
			fontWeight: 600,
			letterSpacing: tracking.caps,
			textTransform: 'uppercase',
			color: color.ink3,
		}}
	>
		<span style={{color: color.volt}}>{String(n).padStart(2, '0')}</span> · {name}
	</div>
);

/* ---------------------------------------------------------------- scenes */

const SlamDemo = () => (
	<Background variant="grid" seed="slam">
		<Center>
			<KineticText text="Retome o controle." mode="slam" size={type.hero} highlight={{controle: 'volt'}} shake={8} at={4} />
		</Center>
		<Label n={1} name="Background grid · KineticText slam" />
		<Sfx src="impact.wav" at={13} />
	</Background>
);

const WordsDemo = () => (
	<Background variant="orbs" seed="words">
		<Center>
			<KineticText
				text={'Controle de tempo\nautomático com IA.'}
				mode="stagger-words"
				highlight={{IA: 'volt'}}
				underline={{word: 'automático', color: 'ember'}}
				exitAt={48}
				fill="gradient"
			/>
		</Center>
		<Label n={2} name="stagger-words · highlight · underline · exit" />
	</Background>
);

const CharsDemo = () => (
	<Background variant="plain" seed="chars">
		<Center>
			<KineticText text="Sem cronômetro." mode="stagger-chars" size={type.hero} exitAt={46} exit="blur" highlight={{cronômetro: 'ember'}} highlightStyle="color" />
		</Center>
		<Label n={3} name="stagger-chars · exit blur" />
	</Background>
);

const MaskDemo = () => (
	<Background variant="orbs" seed="mask">
		<Center>
			<KineticText
				text={'Seu dia inteiro,\nclassificado sozinho.'}
				mode="mask-up"
				highlight={{sozinho: 'ember'}}
				highlightStyle="gradient"
				exitAt={48}
				exit="mask"
			/>
		</Center>
		<Label n={4} name="mask-up · gradient highlight · mask exit" />
	</Background>
);

const TypeDemo = () => (
	<Background variant="plain" seed="type">
		<Center>
			<KineticText text="Relatório pronto às 18h." mode="typewriter" size={96} at={6} highlight={{'18h': 'mint'}} highlightStyle="marker" />
		</Center>
		<Label n={5} name="typewriter · caret · marker highlight" />
	</Background>
);

const riseShot: ScreenConfig = {
	...PLACEHOLDER,
	width: 1280,
	y: 30,
	enter: 'rise',
	enterDuration: 36,
	rotateX: [
		[0, 16],
		[70, 6, ease.settle],
	],
	rotateY: [
		[0, -12],
		[70, -4, ease.settle],
	],
	float: 8,
	sheenAt: 34,
	title: 'ubiqX — Hoje',
};

const PulseRing: React.FC<{x: number; y: number; r: number}> = ({x, y, r}) => {
	const frame = useCurrentFrame();
	const t = ((frame - 30) % 30) / 30;
	if (frame < 30) return null;
	const rr = r * (1 + t * 0.35);
	return (
		<div
			style={{
				position: 'absolute',
				left: x - rr,
				top: y - rr,
				width: rr * 2,
				height: rr * 2,
				borderRadius: '50%',
				border: '3px solid rgba(46,204,143,0.9)',
				opacity: 1 - t,
				boxShadow: '0 0 24px rgba(46,204,143,0.6)',
			}}
		/>
	);
};

const RiseDemo = () => (
	<Background variant="orbs" seed="rise">
		<Screen {...riseShot}>
			{/* image-space overlay: a pulse ring locked onto the focus gauge (image px 381,305) */}
			<PulseRing x={381} y={305} r={96} />
		</Screen>
		<Label n={6} name="Screen · enter rise · 3D tilt · float · sheen" />
	</Background>
);

const zoomShot: ScreenConfig = {
	...PLACEHOLDER,
	width: 1440,
	camera: [
		{at: 26, rect: {x: 257, y: 119, w: 1158, h: 374}, fit: 0.86, duration: 22},
		{at: 74, rect: {x: 470, y: 270, w: 470, h: 100}, fit: 0.55, duration: 20},
	],
	spotlights: [{rect: {x: 505, y: 299, w: 266, h: 40}, at: 40, radius: 12, pad: 8}],
};

const ZoomDemo = () => (
	<Background variant="orbs" seed="zoom">
		<Screen {...zoomShot} />
		<Callout screen={zoomShot} target={{x: 292, y: 217, w: 180, h: 180}} kicker="Foco" label="88 de foco hoje" side="bottom" distance={90} at={30} exitAt={60} />
		<Cursor
			screen={zoomShot}
			path={[
				{at: 0, x: 1100, y: 720},
				{at: 54, x: 640, y: 318, click: true, hand: true},
			]}
			moveDuration={24}
		/>
		<Label n={7} name="Screen camera · spotlight · Callout · Cursor click" />
	</Background>
);

const tiltShot: ScreenConfig = {
	...PLACEHOLDER,
	width: 1300,
	x: 80,
	rotateY: -18,
	rotateX: 9,
	perspective: 2000,
};

const TiltDemo = () => (
	<Background variant="grid" seed="tilt" gridOpacity={0.16}>
		<Screen {...tiltShot} />
		<Cursor
			screen={tiltShot}
			path={[
				{at: 0, x: 800, y: 620},
				{at: 28, x: 80, y: 197, click: true},
			]}
		/>
		<Callout screen={tiltShot} target={{x: 12, y: 180, w: 207, h: 36}} label="8 blocos para revisar" side="bottom" distance={120} shift={80} accent="ember" at={32} />
		<Label n={8} name="Cursor + Callout mapped through a 3D tilt" />
	</Background>
);

const KEY_PRESSES = [24, 28, 32, 36, 40, 44, 48, 52, 56];

const KeysDemo = () => (
	<Background variant="plain" seed="keys">
		<Center gap={56}>
			<KineticText text="Decida com 1–9." mode="fade" size={type.h2} at={0} highlight={{'1–9': 'volt'}} />
			<KeyCombo keys={['1', '2', '3', '4', '5', '6', '7', '8', '9']} size={112} appearAt={4} stagger={2} pressAt={KEY_PRESSES} />
		</Center>
		{/* SFX positive path: a real file (public/_demo/tick.wav) on every press */}
		{KEY_PRESSES.map((f) => (
			<Sfx key={f} src="_demo/tick.wav" at={f} volume={0.5} />
		))}
		<Label n={9} name="KeyCap / KeyCombo press" />
	</Background>
);

const CountDemo = () => (
	<Background variant="orbs" seed="count">
		<Center direction="row" gap={120}>
			<Counter to={1234} at={4} duration={40} size={132} />
			<Counter to={49} at={8} duration={36} format="currency" size={132} color="ember" />
			<Counter to={8.5} decimals={1} suffix="h" at={10} duration={40} mode="roll" size={132} color="volt" />
			<Counter to={87} format="percent" at={12} duration={40} size={132} color="mint" />
		</Center>
		<Label n={10} name="Counter · pt-BR · currency · odometer roll" />
	</Background>
);

const CardsDemo = () => (
	<Background variant="orbs" seed="cards">
		<Center>
			<CardGrid
				at={4}
				columns={3}
				cardWidth={460}
				cardHeight={270}
				cards={[
					{icon: <Clock3 size="100%" strokeWidth={1.8} />, title: 'Captura sozinho', subline: 'Apps, janelas e sites — sem cronômetro.'},
					{icon: <Sparkles size="100%" strokeWidth={1.8} />, title: 'Classifica com IA', subline: 'Nas categorias que você cria.', accent: 'ember', badge: 'IA'},
					{icon: <FileText size="100%" strokeWidth={1.8} />, title: 'Relatório pronto', subline: 'Diário e mensal, por categoria.', accent: 'mint'},
					{icon: <Keyboard size="100%" strokeWidth={1.8} />, title: 'Revisão em 1–9', subline: 'Cada decisão ensina o classificador.'},
					{icon: <ShieldCheck size="100%" strokeWidth={1.8} />, title: 'Local primeiro', subline: 'Sem teclas, sem conteúdo.', accent: 'mint'},
					{icon: <Bot size="100%" strokeWidth={1.8} />, title: 'UBI de olho', subline: 'Seu mascote cuida do seu foco.', accent: 'ember'},
				]}
			/>
		</Center>
		<Label n={11} name="FeatureCard / CardGrid stagger" />
	</Background>
);

const LogosDemo = () => (
	<Background variant="plain" seed="logos">
		<Center gap={90}>
			<LogoRow
				heading="Escolha sua IA"
				variant="chip"
				monochrome
				at={4}
				logos={[
					{src: '_demo/claude.svg', label: 'Claude'},
					{src: '_demo/googlegemini.svg', label: 'Gemini'},
					{src: '_demo/x.svg', label: 'Grok'},
					{src: '_demo/ollama.svg', label: 'Ollama'},
				]}
			/>
			<LogoRow heading="macOS · Windows · Linux" headingTransform="none" monochrome at={16} height={64} logos={[{src: '_demo/apple.svg'}, {src: '_demo/linux.svg'}]} restOpacity={0.9} />
		</Center>
		<Label n={12} name="LogoRow chip + plain" />
	</Background>
);

const Panel: React.FC<{title: string; tint: string; seed: string}> = ({title, tint, seed}) => (
	<Background variant="orbs" seed={seed} orbs={[{color: tint, x: 0.5, y: 0.5, size: 0.9, opacity: 0.3}]} grain={0}>
		<Center>
			<div style={{fontFamily: font.display, fontSize: type.h1, fontWeight: 700, letterSpacing: tracking.display, color: color.ink}}>{title}</div>
		</Center>
	</Background>
);

const TransitionsDemo = () => (
	<AbsoluteFill>
		<TransitionSeries>
			<TransitionSeries.Sequence durationInFrames={28}>
				<Panel title="whipPan →" tint="volt" seed="t1" />
			</TransitionSeries.Sequence>
			<TransitionSeries.Transition presentation={whipPan({direction: 'left'})} timing={beatTiming(10)} />
			<TransitionSeries.Sequence durationInFrames={30}>
				<Panel title="zoomThrough" tint="ember" seed="t2" />
			</TransitionSeries.Sequence>
			<TransitionSeries.Transition presentation={zoomThrough()} timing={beatTiming(12)} />
			<TransitionSeries.Sequence durationInFrames={30}>
				<Panel title="maskWipe circle" tint="mint" seed="t3" />
			</TransitionSeries.Sequence>
			<TransitionSeries.Transition presentation={maskWipe({shape: 'circle', origin: {x: 70, y: 40}})} timing={beatTiming(14, ease.inOut)} />
			<TransitionSeries.Sequence durationInFrames={30}>
				<Panel title="maskWipe diagonal" tint="volt" seed="t4" />
			</TransitionSeries.Sequence>
			<TransitionSeries.Transition presentation={maskWipe({shape: 'diagonal'})} timing={beatTiming(14, ease.inOut)} />
			<TransitionSeries.Sequence durationInFrames={30}>
				<Panel title="flashCut" tint="rose" seed="t5" />
			</TransitionSeries.Sequence>
			<TransitionSeries.Transition presentation={flashCut()} timing={beatTiming(6, ease.linear)} />
			<TransitionSeries.Sequence durationInFrames={30}>
				<Panel title="blurDissolve" tint="ember" seed="t6" />
			</TransitionSeries.Sequence>
			<TransitionSeries.Transition presentation={blurDissolve()} timing={beatTiming(14, ease.inOut)} />
			<TransitionSeries.Sequence durationInFrames={30}>
				<Panel title="fim" tint="volt" seed="t7" />
			</TransitionSeries.Sequence>
		</TransitionSeries>
		<Grain />
		<Label n={13} name="Transitions (TransitionSeries)" />
	</AbsoluteFill>
);
/** Length of TransitionsDemo: sequences minus transition overlaps. */
const TRANSITIONS_LEN = 28 + 30 * 6 - (10 + 12 + 14 + 14 + 6 + 14);

const Flyer: React.FC = () => {
	const frame = useCurrentFrame();
	const x = tween(frame, 6, 22, -700, 700, ease.whip);
	return (
		<Center>
			<div
				style={{
					transform: `translateX(${x}px)`,
					width: 420,
					height: 220,
					borderRadius: 28,
					background: 'linear-gradient(135deg, #4d8dff, #2a5bd7)',
					boxShadow: '0 30px 80px -20px rgba(77,141,255,0.6)',
					display: 'flex',
					alignItems: 'center',
					justifyContent: 'center',
					fontFamily: font.display,
					fontSize: 56,
					fontWeight: 700,
					color: '#fff',
					letterSpacing: '-0.03em',
				}}
			>
				MotionBlur
			</div>
		</Center>
	);
};

const BlurGlowDemo = () => (
	<Background variant="plain" seed="mb">
		<Glow x={960} y={540} size={1100} color="volt" intensity={0.3} pulse={0.4} pulsePeriod={30} />
		<MotionBlur samples={10} shutterAngle={180}>
			<Flyer />
		</MotionBlur>
		<Label n={14} name="Glow pulse · MotionBlur (CameraMotionBlur)" />
	</Background>
);

const EndDemo = () => (
	<Background variant="grid" seed="end" gridOpacity={0.2}>
		<Glow y={470} size={1000} intensity={0.28} />
		<TransitionIn presentation={zoomThrough({fromScale: 0.7})} duration={14} easing={ease.push}>
			<Center gap={36}>
				<Wordmark size={150} at={4} />
				<KineticText text="Retome o controle do seu dia." mode="stagger-words" role="text" size={40} weight={500} color={color.ink2} at={22} />
			</Center>
		</TransitionIn>
		<Flash at={0} duration={8} peak={0.35} />
		<Label n={15} name="Wordmark · TransitionIn wrapper · Flash · end card" />
	</Background>
);

/* ---------------------------------------------------------------- reel */

export const PRIMITIVE_SEGMENTS: {name: string; frames: number; C: React.FC}[] = [
	{name: 'slam', frames: 60, C: SlamDemo},
	{name: 'words', frames: 60, C: WordsDemo},
	{name: 'chars', frames: 60, C: CharsDemo},
	{name: 'mask', frames: 60, C: MaskDemo},
	{name: 'typewriter', frames: 75, C: TypeDemo},
	{name: 'screen-rise', frames: 75, C: RiseDemo},
	{name: 'screen-zoom', frames: 96, C: ZoomDemo},
	{name: 'screen-tilt', frames: 60, C: TiltDemo},
	{name: 'keys', frames: 72, C: KeysDemo},
	{name: 'counter', frames: 66, C: CountDemo},
	{name: 'cards', frames: 60, C: CardsDemo},
	{name: 'logos', frames: 60, C: LogosDemo},
	{name: 'transitions', frames: TRANSITIONS_LEN, C: TransitionsDemo},
	{name: 'motionblur', frames: 40, C: BlurGlowDemo},
	{name: 'end', frames: 75, C: EndDemo},
];

export const PRIMITIVES_DURATION = PRIMITIVE_SEGMENTS.reduce((a, s) => a + s.frames, 0);

export const Primitives: React.FC = () => (
	<AbsoluteFill style={{backgroundColor: color.canvas}}>
		<Series>
			{PRIMITIVE_SEGMENTS.map(({name, frames, C}) => (
				<Series.Sequence key={name} durationInFrames={frames} name={name}>
					<C />
				</Series.Sequence>
			))}
		</Series>
	</AbsoluteFill>
);
