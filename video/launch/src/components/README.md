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

## Stage (v2 look) + Background

`brief/v2-look.md` §3: every backdrop in the film is built from `src/components/Stage.tsx`
(the group backdrops in `src/scenes/_parts/G1..G6/common.tsx` and `<Background>` all use it).

| layer | what |
|---|---|
| `<StageBase opacity?>` | navy vertical gradient `#0f1730 → #0a0f20` (`STAGE.top/bottom`), never black |
| `<StageLights {...look} seed>` | volt **key pool** behind the subject (0.42, a real pool: w 0.82 × h 0.92 of the canvas, 0 inside the frame edges), **indigo** `#6c5cff` (light only, never UI/type) + ember pools (0.18 / 0.17), a slow **aurora** sweep (two diagonal bands, one pass ≈ 12 s), and the white-blue **key light** `#cfe0ff` (0.15, w 0.5 × h 0.62) right behind the subject |
| `<StageGuards guard level keyPool keyLight>` | soft navy **type guards** (`look.guard`), drawn by every group backdrop above its lights/orbs/floor and under its children; in dev it `console.warn`s once when the look breaks the GUARD RULE (`guardIssues(look)`) |
| `<StageFinish vignette grain seed>` | vignette (v1 strength scale: `vignetteAlpha(s)` = 0.35 × s / 0.6, capped at **0.35**) + grain (default 0.042) |
| `<Stage {...look}>` | all three with children between lights and finish |

`StageLook` (accepted as `look` by every group backdrop and `<Background>`):

| prop | default | notes |
|---|---|---|
| `level` | 1 | multiplier on every light; 0 = lights out (base only). s04 passes `look={{level: 0}}` explicitly |
| `keyPool` | `{x: .5, y: .5, w: .82, h: .92, opacity: .42}` | fractions of the canvas; **move it behind the subject of the shot** (`keyPool: {x: .7, y: .45}`) or `false` |
| `pools` | indigo TR 0.18, ember BL 0.17 | `StagePool[]` `{color, x, y, w, h?, opacity, drift?}` |
| `keyLight` | `{x: .5, y: .5, w: .5, h: .62, opacity: .15}` | put it right behind the window / card / subject — **not** behind volt type |
| `guard` | none | `StageGuard[]` `{x, y, w, h, opacity? = GUARD_OPACITY (0.68)}` (fractions); build one from the glyph box in px with `guardFor({x, y, w, h}, {padX?, padY?, opacity?})` (default pads 0.5 w / 1.2 h per side). Profile: a smooth ramp `guardProfile(r)` = 1 − smoothstep(0.12, 1, r), no flat core. Scaled by `level`. Only ever trims a light's FALLOFF: see the GUARD RULE below |
| `aurora` / `speed` | 1 / 1 | |

All pure CSS gradients, normal alpha compositing (no blend modes, no filters).

**Volt type rule + GUARD RULE (round 2).** Volt `#4d8dff` needs a local background darker than about
rgb(24, 34, 64) for 4.5:1, which a lit part of the key pool is not. The fix is where the LIGHT goes, not a
darker guard:

1. **Light the subject, not the type.** The key-pool centre, the key-light centre and any volt orb of the
   scene sit behind the subject (UBI, the window, the card), away from the volt type. The type sits on the
   light's falloff.
2. **A guard only trims falloff.** The key pool's and key light's ≥ 50 % isolines stay out of every guard's
   core (guard alpha ≥ 50 % of its peak, r ≤ ~0.6 of the drawn ellipse). A guard that sits on a light's hot
   centre carves a navy hole out of it and leaves a bright ring around the type: no real light looks like
   that (round 1's s05 / s18 did it, 2.0–2.3× ring/interior). `guardIssues(look)` checks this and
   `StageGuards` warns in dev.
3. **Keep guards light.** Default peak 0.68 with a long soft feather (pads 0.5 w / 1.2 h). Use a shorter
   `padX` when the subject's light is close beside the type (s05 / s18: `padX: 0.2–0.3 × w`), and a lower
   peak when another light sits right under or over the type (s05: 0.68 → 0.55 as the floor's horizon glow
   rises under the lockup).
4. **A pill is a surface.** A volt label on a translucent volt pill has almost no contrast margin; give the
   pill its own navy backing (s05 / s18 AI pill: `linear-gradient(volt 0.11, volt 0.11), rgba(10,16,36,0.88)`)
   instead of guarding the stage behind it.
5. **Check with stills:** ≥ 4.5:1 for the volt type against a 6–22 px ring around the glyphs, AND a luma
   profile through each guard (columns through the type block, a row through its centre, content masked)
   has no valley: min(peak on either side) / guard interior ≤ 1.25.

Reference setups: **s05 / s18** (key pool, key light and the volt bloom orb all behind UBI; one lockup
guard with a short horizontal feather; no CTA guard), **s14** (footlight under the 18:00 + a default guard;
the pool rises behind the report window after the morph and the guard fades).

Helpers: `navyDim(a)` → `rgba(10, 16, 36, a)` (use it for every dim/scrim instead of black),
`SPOTLIGHT_DIM` (0.38), `v2Dim(v1)` (maps a v1 dim tuned against 0.62 black: 0.62 → 0.38),
`vignetteAlpha(strength)`, `STAGE` palette.

```tsx
<Stage seed="s09" keyPool={{x: 0.62, y: 0.55}} keyLight={{x: 0.62, y: 0.52}}>
  <Screen {...shot} style={{zIndex: 'auto'}} />
</Stage>
// group backdrops keep their v1 props and take `look` on top:
<Backdrop seed="s13" ember={0.07} look={{keyPool: {x: 0.3, y: 0.55}, keyLight: {x: 0.3, y: 0.55}}}>…</Backdrop>
```

### Background

Stage rig + optional extra drifting orbs, optional perspective grid floor, vignette and film grain. Wrap a
scene in it (children render above the lights, under vignette + grain).

| prop | type | default | notes |
|---|---|---|---|
| `variant` | `'orbs' \| 'grid' \| 'plain'` | `'orbs'` | `grid` adds a floor receding to a glowing horizon; `plain` = rig at 70 %, no aurora |
| `orbs` | `OrbSpec[]` | none | extra `{color, x, y, size, opacity}` (fractions of canvas) on top of the rig |
| `orbIntensity` | number | 1 | multiplier on orbs and the rig |
| `look` | `StageLook` | — | see Stage |
| `drift` / `speed` | number | 0.06 / 1 | wander distance (fraction of width) / speed |
| `gridColor` / `gridOpacity` / `gridSpeed` | accent / 0–1 / px per frame | volt / 0.3 / 2 | floor lines scroll toward viewer |
| `horizon` | 0–1 | 0.68 | where the floor fades out |
| `vignette` | 0–1 | 0.6 | v1 strength scale, mapped to ≤ 0.35 alpha |
| `grain` | 0–1 | 0.042 | 0 disables (use when you add a global `<Grain/>`) |
| `seed` | string | `'bg'` | vary per scene so lights don't drift identically |
| `base` | CSS colour | navy gradient | a flat base instead of the gradient |

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
| `spotlights` | `SpotlightSpec[]` | — | `{rect, at, until?, fade?, dim?=0.38, color?, radius?=20, pad?=14, outline?}` (image px). v2: navy tint `rgba(10,16,36,…)`, default dim 0.38 (v1 0.62 black), stronger outline glow; drawn ungraded above the graded content |
| `grade` | boolean \| `{brightness, contrast, saturate, lift}` | on: 1.2 / 1.05 / 1.15 / lift 0.05 | v2 grade of the window CONTENT (the lift raises the black point: out = lift + (1 − lift) · in, emitted as `contrast(0.958) brightness(1.2495) saturate(1.15)`, so the app's panels land at ≈ 0.10 luma instead of 0.066): the bitmap **and** the image-space children are graded together, so patches sampled from the ungraded capture still match. For a colour OUTSIDE the window that must match one inside it use `gradeHex(hex)` (s05's hero-card tint → `HERO_CARD_GRADED`) |
| `rim` | boolean | true | v2 rim light: 1.5 px gradient border (volt ≈ 0.7 top-left → transparent) + a white top highlight; the drop shadow is deeper/larger (`WINDOW_SHADOW`) and the ambient `glow` is 0.35 |
| `chrome` / `title` / `radius` | `'mac' \| 'none'` / string / px | mac / — / 14 | |
| `dots` | `'mac' \| 'neutral'` | mac | title-bar dots: macOS traffic lights, or neutral `#3a4560` dots for platform-agnostic shots (style §S10) |

> **Stacking:** Screen's outer layer has `zIndex: layer.screen` (20), so siblings rendered after it *without* a z-index (headlines, scrims, cursor, a 3D UBI) end up UNDER the window. Pass `style={{zIndex: 'auto'}}` (as every film scene does) or give the siblings a higher z-index. Also note that a camera key with `focus` re-centres on the projected pose, so `x`/`y` offsets and `enter: 'rise'` are cancelled while such a key is active; do rises with a screen-space wrapper.

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

## LiftCard (v2)

Lifts one element of a UI capture out of the screenshot as a floating 3D card (brief §3 "Depth: lift
cards": the 88 ring, a review row, "Confirmar os 19", the provider pills, the redaction chip).
`rect` is in the capture's **2x image px** (hotspots work: `hotspot('ui/dashboard.png', 'focus-dial')`);
the `@3x` twin is drawn when shipped. The card is graded like the window. **Claim-safety patches inside the
crop must be re-applied**: pass `patches={storyboardPatches(scene, file)}` (drawn exactly like `<Patches>`:
same frames, colour, 2 px bleed; patches outside the crop are skipped) and/or image-space `children`
(image px of the full capture, same as `<Screen>` children).

| prop | type | default | notes |
|---|---|---|---|
| `src` / `rect` / `imageSize` / `hires` | | — / — / 2880x1800 / auto | crop source |
| `x`, `y` | Keyframed | — | card centre, comp px |
| `scale` | Keyframed | 1.4 | relative to the element's size in a 1440-wide Screen at zoom 1 (brief: 1.2–1.6) |
| `width` | Keyframed | — | explicit on-canvas width (overrides `scale`) |
| `rotateX` / `rotateY` / `rotateZ` | Keyframed (deg) | 8 / −10 / 0 | brief: 6–14° |
| `at` / `enter` / `spring` | frame / `'lift' \| 'rise' \| 'pop' \| 'fade' \| 'none'` / preset | 0 / rise / smooth (pop: subtleBounce) | `lift` flies from `from` to the target pose, shadow + glow growing with it |
| `from` | Rect \| `(frame) => Rect` | — | the element's comp rect in the window. Static: `mapImageRect(shot, at, COMP, rect)`. **Animated: pass a function** `(f) => mapImageRect(shot, f, COMP, rect)` so the lift origin and the `drop` target track a moving camera / floating window (frames local to the Sequence) |
| `exitAt` / `exit` / `exitDuration` | frame / `'sink' \| 'drop' \| 'fade' \| 'none'` / 12 | — | `drop` flies back into `from` |
| `float` / `floatPeriod` | px / frames | 5 / 96 | sine bob |
| `drift` | `{x, y}` px per frame | — | parallax against the window's camera |
| `glow` / `glowOpacity` | accent \| false / 0–1 | volt / 0.35 | coloured light under the card |
| `rim` / `shadow` / `radius` / `perspective` | | true / 1 / 16 / 1600 | |
| `patches` | StoryboardPatch[] | — | claim-safety patches re-applied inside the card |
| `grade` / `opacity` / `style` | | on / 1 / — | pass `style={{zIndex: 30}}` to sit above a Screen |

`liftCardPose(props, frame, fps)` returns the pose (`cx, cy, w, h, k, rx, ry…`) to glue a cursor or callout.

### LiftHole: the element leaves the window

So a lifted element never appears twice, put `<LiftHole>` **as a child of the `<Screen>`** (image space,
tilted + graded with the window, like patches) with the card's `rect` and timing — spread the same object
into both. It draws a feathered fill over the source rect with a faint "empty socket" (inset shade + hairline):

- On from `at` for `enter="lift"` (the card starts exactly on top, so no pop); for rise/pop/fade it fades in with the card's spring.
- `exit="drop"`: stays until the card has landed back, then vanishes (the element is "back"). `fade`/`sink`: the element fades back in as the card leaves.
- **Fill choice: pass `color` sampled from the capture's surface around the element** (hex, like the storyboard patch colours;
  e.g. `python3 -c "from PIL import Image; print('#%02x%02x%02x' % Image.open('public/ui/dashboard.png').convert('RGB').getpixel((760, 610)))"`).
  It is drawn inside the Screen's grade, so the raw capture colour matches. Without `color` it falls back to a navy
  dim (`rgba(7,11,20,0.78)`) that leaves a faint ghost of the element — acceptable on any surface, but less clean.
- `pad` 6 / `feather` 18 / `radius` 24 (image px; use `rect.w / 2` for round elements like the ring) / `socket` 0.5 (0 = flat fill) / `opacity`.
- End any Screen spotlight on the element (`until`) around the lift, otherwise its outline circles an empty hole.
- `liftHolePresence(props, frame, fps)` is the pure 0–1 presence.

```tsx
const RING = hotspot('ui/dashboard.png', 'focus-dial');
const COMP = {width: 1920, height: 1080};
const RING_LIFT = {rect: RING, at: 14, enter: 'lift' as const, exitAt: 54, exit: 'drop' as const, exitDuration: 18};
<Screen {...shot} style={{zIndex: 'auto'}}>
  <LiftHole {...RING_LIFT} color="#101a2f" radius={RING.w / 2} />
</Screen>
<LiftCard src="ui/dashboard.png" {...RING_LIFT} x={1380} y={500} scale={1.5}
  from={(f) => mapImageRect(shot, f, COMP, RING)} patches={storyboardPatches(scene, 'ui/dashboard.png')}
  rotateX={[[14, 8], [75, 4]]} rotateY={[[14, -12], [75, -6]]} drift={{x: -0.2, y: 0}} glow="mint" style={{zIndex: 30}} />
```

Primitives reel segment `lift` (frames 1057–1131) shows lift → hole → drop back live.

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
