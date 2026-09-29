# Motion-graphics primitives (ubiqX AI launch video)

Everything here is **frame-driven and deterministic**: no CSS animations, no
`Math.random()`, no timers. All timing props (`at`, `exitAt`, `pressAt`, camera
`at`…) are **frames local to the enclosing `<Sequence>`**. Think in beats:

```ts
import {BEAT, BAR, beats, bars, ease, springs, color, type, font} from '../design/tokens';
// 120 BPM @ 30 fps → BEAT = 15 frames, BAR = 60 frames. beats(2) = 30, bars(1.5) = 90.
```

```ts
import {Background, KineticText, Screen, Cursor, Callout /* … */} from '../components';
```

Watch every primitive live: `npm run studio` → composition **Primitives**
(`src/compositions/Primitives.tsx` is also the best source of working examples).

Shared helpers (`src/design/motion.ts`): `Keyframed` values are either a number
or `[[frame, value], [frame, value, easing?], …]`; `kf(value, frame)` evaluates
one. `progress(frame, start, dur, easing)`, `tween(...)`, `springIn(frame, fps,
delay, 'snappy'|'smooth'|'subtleBounce')`, `staggerDelay(i, n, each, order)`.

---

## Background

Deep canvas + drifting volt/ember light orbs, optional perspective grid floor,
vignette and film grain. Wrap a scene in it (children render above the orbs,
under vignette + grain).

| prop | type | default | notes |
|---|---|---|---|
| `variant` | `'orbs' \| 'grid' \| 'plain'` | `'orbs'` | `grid` adds a floor receding to a glowing horizon |
| `orbs` | `OrbSpec[]` | volt TL, ember BR, faint volt centre | `{color, x, y, size, opacity}` (fractions of canvas) |
| `orbIntensity` | number | 1 | global multiplier |
| `drift` / `speed` | number | 0.06 / 1 | wander distance (fraction of width) / speed |
| `gridColor` / `gridOpacity` / `gridSpeed` | accent / 0–1 / px per frame | volt / 0.3 / 2 | floor lines scroll toward viewer |
| `horizon` | 0–1 | 0.68 | where the floor fades out |
| `vignette` | 0–1 | 0.6 | |
| `grain` | 0–1 | 0.055 | 0 disables (use when you add a global `<Grain/>`) |
| `seed` | string | `'bg'` | vary per scene so orbs don't drift identically |
| `base` | CSS colour | canvas | |

```tsx
<Background variant="grid" seed="hero">
  <Center><KineticText text="Retome o controle." mode="slam" /></Center>
</Background>
```

## KineticText

Kinetic typography. Block element — position it with a parent (`<Center>`).

| prop | type | default | notes |
|---|---|---|---|
| `text` | string | — | `\n` = line break (a unit for `mask-up`) |
| `mode` | `'slam' \| 'stagger-words' \| 'stagger-chars' \| 'mask-up' \| 'typewriter' \| 'fade'` | `'stagger-words'` | |
| `at` | frame | 0 | entry start |
| `stagger` | frames | slam 4 (0 = all at once), words 3, chars 1.2, mask 5/line, typewriter 1.6/char | |
| `duration` | frames | per mode (slam 9, words 18, chars 16, mask 18, fade 14) | per-unit entry length |
| `exitAt` / `exit` / `exitDuration` / `exitStagger` | frame / `'up' \| 'down' \| 'fade' \| 'blur' \| 'mask' \| 'slam' \| 'none'` / 10 / auto | — / `'up'` | exits are staggered too |
| `highlight` | `Record<word, accent \| css>` | — | case/punctuation-insensitive; multi-word keys match phrases |
| `highlightStyle` | `'glow' \| 'color' \| 'gradient' \| 'marker'` | `'glow'` | `marker` sweeps a tinted box behind the word |
| `underline` | `{word?, at?, duration?, color?, thickness?, offset?}` | — | omit `word` to underline the whole block; `at` defaults to when the entry finishes |
| `role` | `'display' \| 'text'` | `'display'` | Sora vs Inter |
| `size` / `weight` / `color` | px / number / css | 120 / 700 (display) / ink | |
| `fill` | `'solid' \| 'gradient'` | `'solid'` | gradient = top-lit ink |
| `align` / `lineHeight` / `letterSpacing` / `maxWidth` | | center / 1.06 / by size / — | |
| `caret` | boolean | true | typewriter caret (blinks on the beat once typed) |
| `shake` | px | 0 | slam landing camera shake |

```tsx
<KineticText
  text={'Controle de tempo\nautomático com IA.'}
  mode="mask-up"
  highlight={{IA: 'volt'}}
  underline={{word: 'automático', color: 'ember'}}
  exitAt={beats(3)} exit="mask"
/>
<KineticText text="Retome o controle." mode="slam" size={type.hero} highlight={{controle: 'volt'}} shake={8} />
<KineticText text="Relatório pronto às 18h." mode="typewriter" size={96} highlight={{'18h': 'mint'}} highlightStyle="marker" />
```

## Screen (+ camera, spotlight)

A screenshot in a macOS window floating in 3D, with a camera that zooms to any
rect **in the screenshot's own pixels**. Default image size is **2880x1800**
(DPR-2 capture of 1440x900) — pass `imageSize` if different (the placeholder
`ui/_placeholder.png` is 1440x900).

Define the shot once as a `ScreenConfig` const and share it with overlays.

| prop | type | default | notes |
|---|---|---|---|
| `src` | string | — | path under `public/` (`'ui/dashboard.png'`) or URL |
| `imageSize` | `{w, h}` | 2880x1800 | **must match the file** |
| `hires` | boolean \| string | auto | draws `public/ui/<name>@3x.png` when shipped (`public/ui/hires.json`); all coordinates stay in `imageSize` space. `false` = never, string = that bitmap |
| `width` | px | 1440 | rendered content width before camera |
| `x`, `y`, `scale` | Keyframed | 0, 0, 1 | window centre offset / extra scale |
| `rotateX`, `rotateY`, `rotateZ` | Keyframed (deg) | 0 | +X = top leans away, +Y = right side leans away |
| `perspective` | px | 2400 | smaller = more dramatic |
| `float` / `floatPeriod` | px / frames | 0 / 120 | gentle bob |
| `camera` | `CameraKey[]` | — | see below |
| `spotlights` | `SpotlightSpec[]` | — | `{rect, at, until?, fade?, dim?=0.62, color?, radius?=20, pad?=14, outline?}` (image px) |
| `chrome` / `title` / `radius` | `'mac' \| 'none'` / string / px | mac / — / 14 | |
| `glow` | colour \| false | volt | ambient light under the window |
| `enter` / `enterAt` / `enterDuration` | `'rise' \| 'zoom' \| 'fade' \| 'tilt' \| 'none'` | none / 0 / 30 | |
| `exit` / `exitAt` / `exitDuration` | `'sink' \| 'zoom' \| 'fade' \| 'none'` | none / — / 18 | |
| `sheenAt` | frame | — | diagonal light sweep across the glass |
| `children` | ReactNode | — | **image-space overlays**: laid out in an `imageSize` box that moves with the UI (`left: 1200, top: 400` = image pixel) |

`CameraKey`: `{at, rect?: Rect | 'full', zoom?, focus?: {x,y}, fit?=0.7, maxZoom?=3.2, duration?=24, easing?=ease.inOut, anchor?: {x,y}}`
— the camera **arrives** at `at` after moving for `duration` frames; zoom is
interpolated geometrically so push-ins feel linear. `anchor` places the target
somewhere other than canvas centre (e.g. leave room for a headline).

```tsx
const shot: ScreenConfig = {
  src: 'ui/dashboard.png',
  enter: 'rise', rotateX: [[0, 16], [60, 4, ease.settle]], float: 6, sheenAt: 30,
  camera: [
    {at: beats(3), rect: {x: 520, y: 240, w: 2300, h: 740}, fit: 0.85},
    {at: beats(5), rect: {x: 1000, y: 590, w: 540, h: 80}, fit: 0.55, duration: 20},
    {at: beats(8), rect: 'full'},
  ],
  spotlights: [{rect: {x: 1010, y: 598, w: 532, h: 80}, at: beats(4.5), until: beats(7.5)}],
};
<Screen {...shot}>
  {/* live overlay glued to the UI, image px */}
  <div style={{position: 'absolute', left: 760, top: 600, width: 20, height: 20, borderRadius: 10, background: color.mint}} />
</Screen>
```

Geometry helpers (for custom overlays): `mapImagePoint(cfg, frame, {width, height}, {x, y})`,
`mapImageRect(...)`, `screenGeometry(...)`, hook `useImageToComp(cfg)` (in Cursor.tsx),
and `useScreenGeometry()` inside Screen children.

## Cursor

macOS pointer with eased, slightly arced travel, press-scale and a click ripple.
Same Sequence as the Screen, rendered after it.

| prop | type | default | notes |
|---|---|---|---|
| `path` | `{at, x, y, click?, hand?, move?, easing?}[]` | — | cursor **arrives** at each key at `at`; image px when `screen` is set |
| `screen` | ScreenConfig | — | follow tilt + camera |
| `clicks` | frame[] | — | extra clicks |
| `moveDuration` | frames | 20 | default travel time into each key |
| `appearAt` / `hideAt` | frame | auto / — | 6-frame fades |
| `size` | px | 40 | |
| `scaleWithCamera` | boolean | true with screen | cursor grows with zoom (feels in-UI) |
| `arc` | 0–0.3 | 0.12 | path curvature |
| `rippleColor` / `variant` | accent / `'arrow' \| 'hand'` | volt / arrow | `hand: true` on a key switches while resting there |

```tsx
<Cursor screen={shot} path={[
  {at: 0, x: 2400, y: 1500},
  {at: beats(4.5), x: 1270, y: 636, click: true, hand: true},
]} />
```

## Callout

Pill badge + leader line (with an elbow) pointing at a rect; brackets frame the
target. Lives in composition space (crisp at any zoom) but tracks the target.

| prop | type | default |
|---|---|---|
| `target` | Rect (image px with `screen`, else comp px) | — |
| `screen` | ScreenConfig | — |
| `label` / `kicker` / `icon` | string / string / ReactNode | — |
| `side` | `'top' \| 'bottom' \| 'left' \| 'right'` | `'top'` |
| `distance` / `shift` | px | 110 / 0 |
| `accent` | accent | volt |
| `at` / `exitAt` | frame | 0 / — |
| `brackets` | boolean | true |
| `size` | label px | 30 |

```tsx
<Callout screen={shot} target={{x: 584, y: 434, w: 360, h: 360}} kicker="Foco" label="88 de foco hoje" side="bottom" at={beats(2)} exitAt={beats(4)} />
```

## KeyCap / KeyCombo

3D keyboard key that physically presses (cap drops, wall shrinks, legend lights).

`KeyCap`: `label`, `sublabel?`, `pressAt?: frame | frame[]`, `hold?=4`, `size?=120`,
`units?=1` (Enter ≈ 2.25), `accent?=volt`, `appearAt?`, `latch?=false`.
`KeyCombo`: `keys`, `pressAt?` (one per key or one for a chord), `separator?=''`
(`'+'` for chords), `size`, `gap`, `appearAt`, `stagger?=3`, `accent`, `latch`.

```tsx
<KeyCombo keys={['1','2','3','4','5','6','7','8','9']} size={112} appearAt={4} stagger={2}
  pressAt={[24, 28, 32, 36, 40, 44, 48, 52, 56]} />
<KeyCombo keys={['⌘', 'K']} separator="+" pressAt={30} />
```

## Counter

Count-up with pt-BR formatting and tabular figures; optional odometer roll.

| prop | type | default | notes |
|---|---|---|---|
| `from` / `to` | number | 0 / — | |
| `at` / `duration` / `easing` | frame / frames / fn | 0 / 45 / ease.push | |
| `format` | `'number' \| 'currency' \| 'percent' \| 'compact' \| (v) => string` | number | `1.234`, `R$ 49`, `87%` (pass 87), `1,2 mil` |
| `decimals` | number | 0 | `8,5` with 1 |
| `prefix` / `suffix` | string | — | not animated, rendered at `affixScale` |
| `currency` / `locale` | string | BRL / pt-BR | |
| `mode` | `'count' \| 'roll'` | count | roll = odometer digits |
| `size` / `weight` / `color` / `affixColor` / `affixScale` | | 168 / 700 / ink / ink2 / 0.5 | |
| `landPulse` | boolean | true | glow + scale pulse when it lands |

```tsx
<Counter to={1234} />
<Counter to={49} format="currency" color="ember" />
<Counter to={8.5} decimals={1} suffix="h" mode="roll" color="volt" />
```

## FeatureCard / CardGrid

Icon + title + subline cards that pop in with a spring (scale, rise, blur).
Icons: pass any node — `lucide-react` is installed (`<Clock3 size="100%" strokeWidth={1.8} />`).

`FeatureCard`: `icon?`, `title`, `subline?`, `accent?=volt`, `badge?`, `at?`,
`exitAt?`, `width?=440`, `height?`, `variant?='glass'|'solid'|'outline'`,
`shine?` (border light run), `align?='left'|'center'`, `scale?=1`.
`CardGrid`: `cards`, `columns?=3`, `gap?=28`, `at?`, `stagger?=4`,
`order?='index'|'reverse'|'center-out'|'random'`, `cardWidth?=440`, `cardHeight?`,
`exitAt?`, `exitStagger?=2`, `variant?`, `align?`.

```tsx
<CardGrid at={6} columns={3} cardWidth={460} cardHeight={270} cards={[
  {icon: <Clock3 size="100%" />, title: 'Captura sozinho', subline: 'Apps, janelas e sites.'},
  {icon: <Sparkles size="100%" />, title: 'Classifica com IA', subline: 'Nas suas categorias.', accent: 'ember', badge: 'IA'},
  {icon: <FileText size="100%" />, title: 'Relatório pronto', subline: 'Diário e mensal.', accent: 'mint'},
]} />
```

## LogoRow

Row of logos with stagger. `logos: {src? | node?, label?, height?, monochrome?}[]`,
`at?`, `stagger?=3`, `order?`, `gap?`, `height?` (56 plain / 36 chip),
`variant?='plain'|'chip'`, `monochrome?` (force white), `restOpacity?=1`,
`heading?`, `headingTransform?='uppercase'|'none'`, `exitAt?`.

```tsx
<LogoRow heading="Escolha sua IA" variant="chip" monochrome at={4} logos={[
  {src: 'brand/claude.svg', label: 'Claude'}, {src: 'brand/openai.svg', label: 'OpenAI'},
]} />
```

(`public/_demo/*.svg` are simple-icons used only by the Primitives reel.)

## Transitions

`@remotion/transitions` presentations — use inside `<TransitionSeries>`:

| factory | props | use |
|---|---|---|
| `whipPan()` | `direction?='left'|'right'|'up'|'down'`, `blur?=42`, `distance?=1` | fast pan with directional motion blur; `beatTiming(8–10)` |
| `zoomThrough()` | `zoom?=2.6`, `blur?=22`, `fromScale?=0.55`, `origin?={x:50,y:50}` (%) | fly through old scene into new |
| `maskWipe()` | `shape?='circle'|'diagonal'`, `origin?` (%), `direction?`, `slant?=360`, `edge?=volt|false` | iris / blade reveal with glowing edge |
| `flashCut()` | `color?='#fff'`, `peak?=0.9`, `punch?=true` | hard cut behind a flash; `beatTiming(4–6, ease.linear)` |
| `blurDissolve()` | `blur?=18`, `fromScale?=1.04` | soft cross-dissolve |

`beatTiming(frames = 8, easing = ease.whip)` → `linearTiming` snapped to the grid.
**TransitionSeries overlaps scenes**: total length = Σ sequences − Σ transitions.

```tsx
<TransitionSeries>
  <TransitionSeries.Sequence durationInFrames={bars(2)}><SceneA /></TransitionSeries.Sequence>
  <TransitionSeries.Transition presentation={whipPan({direction: 'left'})} timing={beatTiming(10)} />
  <TransitionSeries.Sequence durationInFrames={bars(2)}><SceneB /></TransitionSeries.Sequence>
</TransitionSeries>
```

Wrappers for a plain `<Sequence>`: `<TransitionIn presentation={zoomThrough()} at={0} duration={12}>…</TransitionIn>`,
`<TransitionOut presentation={whipPan()} at={50} duration={8}>…</TransitionOut>`.
Also `<DirectionalBlur amount={30} angle={0}>` (any angle) and `<Flash at={f} duration={8} peak={0.8} />` for beat hits.

## Grain / Glow / MotionBlur

- `<Grain opacity={0.06} scale={1} every={1} blend="overlay" seed="grain" />` — animated film
  grain from a pre-baked tile (`public/fx/grain.png`, `npm run grain` regenerates). Put it last.
- `<Glow x y size color intensity aspect pulse pulsePeriod blend />` — soft radial light blob;
  all positional props are Keyframed; `pulse` modulates on the beat.
- `<MotionBlur samples={6} shutterAngle={180}>` — true temporal blur via
  `@remotion/motion-blur` `CameraMotionBlur`. Renders reliably, but costs `samples`× for
  its subtree and looks steppy below ~10 samples on very fast moves; auto-disabled in
  `render:draft`. Prefer `DirectionalBlur`/`whipPan` for whips.

## Sfx / Music

```tsx
<Sfx src="whoosh.wav" at={28} volume={0.7} />          // public/audio/sfx/whoosh.wav
<Sfx src="_demo/tick.wav" at={12} />                     // any path under public/ containing "/"
<Music src="launch.wav" volume={0.9} fadeIn={15} duration={1500} fadeOut={45} />  // public/audio/music/
```

If the file isn't in `public/` (checked with `getStaticFiles()` at bundle time),
the cue renders nothing — cues are always safe to leave in. Also `hasStaticFile(path)`.

## Center / Wordmark

- `<Center x y direction gap safe align justify>` — full-frame flex centring inside title-safe margins.
- `<Wordmark size={140} ai at={frame} glowX />` — "ubiq" + volt "X" + "AI"; with `at` the letters rise and the X lands last.

## Fonts & tokens

Sora + Inter (variable, latin + latin-ext) load from `public/fonts/` through
`@remotion/fonts` (`src/design/fonts.ts`, imported by `Root.tsx`), which blocks
rendering until they're ready. Use `font.display` / `font.text`, the `type`
scale, `color`, `glow`, `shadow`, `radius`, `space` from `src/design/tokens.ts`.
