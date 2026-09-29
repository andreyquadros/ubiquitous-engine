# Shared spine (scene builders read this first)

Everything a scene needs besides the primitives in `src/components/`.
**Shared code is frozen for act builders**: you may only add/replace files under
`src/scenes/`. If you need a change here, ask the integrator.

```ts
import {useScene, hotspot, uiImage, UbiClip, UbiTrack, TransitionIn, TransitionOut, E, SPRING, type SfxCue} from '../shared';
```

## How the film is assembled

| piece | where | what |
|---|---|---|
| storyboard | `src/storyboard.ts` | typed `brief/storyboard.json` (SOURCE OF TRUTH): `SCENES`, `sceneById(id)`, `SCENE_IDS`, `sceneCompId(id)` → `"S05"` |
| timeline | `src/timeline.ts` | `TOTAL` (1560), `TIMELINE` (legacy shape), `GROUPS` G1..G6, `groupById`, `groupOf` |
| scenes | `src/scenes/<scene-id>.tsx` | `export default` the scene component; `export const sfx: SfxCue[]` |
| registry | `src/scenes/index.ts` | `SCENE_MODULES[id] = {component, sfx}` |
| film | `src/compositions/Film.tsx` | `<Film from to sfxScope?>`: scene Sequences + MASTER AUDIO |
| compositions | `src/Root.tsx` | `Launch`, `Primitives`, `S01`..`S18`, `G1`..`G6` |

- Each scene is mounted as `<Sequence from={startFrame} durationInFrames name={id} premountFor={30}>`
  inside `<SceneProvider>`. **Frames are scene-relative** (`useCurrentFrame()` = 0 on the scene's first frame).
- Boundaries are hard cuts. Transitions are split halves you render **inside** your scene
  (`<TransitionIn>` / `<TransitionOut>`, below). Never use `TransitionSeries`.
- The master audio layer is outside the scenes: music + every scene's `sfx` cues at absolute frames,
  so a whip that starts 2 f before your scene, or a riser that ends on your first frame, is never cut.
- `S01..S18`: one scene, its own frames, **its own** SFX + the music under it (local checks).
  `G1..G6`: exact slices of `Launch` (all cues audible in the window), frames relative to the group start:
  G1 = s01–s04 (0–239), G2 = s05–s06 (240–449), G3 = s07–s09 (450–659), G4 = s10–s13 (660–929),
  G5 = s14–s16 (930–1229), G6 = s17–s18 (1230–1559).

```bash
node tools/stills.mjs S07 "0,3,6,30,59" --scale=0.5 --jpeg     # stills of one scene
node tools/stills.mjs G3 "0,59,60,134,135" --jpeg                # across your cuts
npx remotion render src/index.ts G3 out/g3.mp4 --scale=0.5       # with audio
```

## A scene module

```tsx
// src/scenes/s07-as-suas-categorias.tsx
import React from 'react';
import {Background, Screen, type ScreenConfig} from '../components';
import {useScene, SceneTransitions, storyboardCamera, storyboardSpotlights, type SfxCue} from '../shared';

export const sfx: SfxCue[] = [
	{ref: 'whip_2.wav', atFrame: 0, gainDb: -8},   // HIT on the cut; the file starts 2 f earlier (handled)
	{ref: 'type-1.wav', atFrame: 8, gainDb: -22},
];

const Scene: React.FC = () => {
	const scene = useScene();                         // storyboard spec of s07
	const shot: ScreenConfig = {
		src: 'ui/categories.png',
		...storyboardCamera(scene, {file: 'ui/categories.png'}),   // camera + rotateX/rotateY from the board
		spotlights: storyboardSpotlights(scene, 'ui/categories.png'),
	};
	return (
		<SceneTransitions>                              {/* storyboard in/out halves (whip-left in, cut out) */}
			<Background variant="plain" seed={scene.id}>
				<Screen {...shot} />
			</Background>
		</SceneTransitions>
	);
};
export default Scene;
```

## Storyboard access — `src/storyboard.ts`, `./scene.tsx`

- `useScene()` → `SceneSpec` (id, startFrame, durationInFrames, act, copy[], camera[], cursor[], spotlights[],
  patches[], scrims[], ubiTrack[], transitionIn/Out, sfx[], shots[]…). `useSceneOrNull()` outside scenes.
- `useSceneFrame()` → `{frame, abs, duration, scene}`.
- `sceneById('s12-um-clique').copy[0].text` — use the storyboard's exact PT-BR strings, never retype them.

## Audio — `./audio.ts`

| export | what |
|---|---|
| `type SfxCue = {ref, atFrame, gainDb, playbackRate?, maxFrames?, note?}` | `atFrame` = scene-relative frame where the **hit** lands (may be < 0 or past the scene end) |
| `db(d)` / `dbToGain(d)` / `gainToDb(g)` | `db(-6)` ≈ 0.5 |
| `BED` (0.5), `sfxVolume(gainDb)` | cue volume = `BED × 10^(gainDb/20)` (style §7.3) |
| `sfxInfo(ref)` | `{file, path, durationFrames, hitOffsetFrames, endIsDownbeat}`; resolves aliases (`click.wav` → `ui_click_1.wav`); throws on unknown names |
| `placeCue(sceneStart, cue)` | absolute `{absStart, absEnd, absHit, volume}`; file starts at `abs hit − round(hit_offset_s × 30)` |
| `MUSIC` | `{file: 'audio/music/music.wav', volume: BED, ducking: true, ducks: DUCKS, startFrame: 0}` — integrator-tunable |
| `DUCKS`, `DUCK_ENVELOPE`, `duckGain(f)`, `musicVolume(f)` | storyboard ducks (−5 dB slams, −6 drop, −4 final hit; 1 f attack, 2 hold, 10 release) plus the integrator's −2.5 dB dips under the clicks at 810, 990, 1080, 1200 |
| `MIX_TRIMS`, `mixTrimDb(file)` | master-bus trims added to every cue's `gainDb` in `placeCue` (UI clicks +8, typing +10, ticks +9, pops/bloops +4, chimes +3, whips/whooshes +5, impacts −2…). The storyboard's levels assume a bed with headroom; music.wav is a finished −14 LUFS master. Scene modules keep the storyboard's gainDb. |

Risers are placed by their END: `{ref: 'riser-2bar.wav', atFrame: 0, gainDb: -6}` in s04 plays abs 105–224.
Music is skipped when `music.wav` is absent; an SFX whose file is missing is skipped too.

## Beats — `./beats.ts`

`BEAT` (15), `BAR` (60), `beats(n)`, `bars(n)`, `sec(s)`, `snapToBeat(f)`, `eighth(i, start)` (0, 8, 15, 23…),
`sixteenth(i)`, `isOnBeat(abs, sub?)`, `isDownbeat(abs)`, `barBeat(abs)` → `"5.2"`, `timecode(abs)` → `"00:08.50"`.

## Motion names from the board — `./motion.ts`

`E.push | E.enter | E.glide | E.whip | E.exit | E.cursor | E.linear` and
`SPRING.SNAPPY | SMOOTH | SMOOTH_SLOW | BOUNCY_SUBTLE | SLAM` (style §3.1–3.2 numbers — these differ slightly from
`design/tokens` `springs`; when the storyboard names a preset, use these).

```ts
const s = springAt(frame, 15, 'SLAM');            // 0→1 spring from f15
const e = easeByName('E.glide', 30);              // easing from a storyboard string ('SMOOTH' works too)
interpolate(frame, [0, 24], [0, 1], {easing: E.push, extrapolateLeft: 'clamp', extrapolateRight: 'clamp'});
```

## UI captures — `./ui.ts`

```ts
hotspot('ui/categories.png', 'category-editor')   // {x, y, w, h} in 2x capture px; throws listing valid names
hotspots('review-done.png')                       // {name: rect}
hotspotCenter('dashboard.png', 'ubi-robot')       // {x, y}
uiImage('review-done.png')                        // {src: 'ui/review-done.png', width, height, dpr, imageSize, hires?}
bitmapUpsample('categories.png', 2.8)             // 1.4 (or /1.5 when the @3x twin exists) — cap 1.40, prefer ≤ 1.15
```

`intervention.png` is 1840x752 (DPR 4) and `dashboard-full.png` is 2880x2340: pass `imageSize={uiImage(f).imageSize}`.

### Hi-res twins (`@3x`)

`<Screen>` now draws `public/ui/<name>@3x.png` automatically when it is shipped (listed in `public/ui/hires.json`
`[{file, hires, scale: 1.5}]`, or present by that name). **Nothing else changes**: `imageSize`, camera rects,
hotspots, `Screen` children, `Cursor`, `Callout` and patches all stay in the 2880x1800 space.
`hires={false}` forces the 2x file; `hires="ui/other.png"` draws a specific bitmap. `useHires(file)` /
`hiresOf(file)` expose the table.

## Storyboard → Screen adapters — `./storyboard-screen.tsx`

- `storyboardCamera(scene, {file?, layer?})` → `{camera, rotateX, rotateY}` for `<Screen>`: storyboard `zoom`,
  `focus` → `anchor`, arrival `atFrame`; "hard/hold/arrives…" = cut, explicit `duration` = travel before
  `atFrame`, otherwise travel from the previous key (linear drifts, spans). Omit `file` to keep one camera
  across a capture swap (s12). `layer` selects s15's "backdrop" / "subject".
- `storyboardSpotlights(scene, file?)` → Screen `spotlights` (hotspot names resolved).
- `storyboardPatches(scene, file)` + `<Patches patches={…}/>` as a **Screen child**: the solid claim-safety
  rects (key legends, model lines, counts) drawn on frames `from ≤ f ≤ to` with a 2 px bleed.
  The validator only passes if these are on screen.

## UBI — `./UbiClip.tsx`

```tsx
// one clip; feet (450, 797 of the 900-px frame) on canvas (1560, 820); 600-px frame
<UbiClip clip="idle" playFrom={11} x={1560} y={820} size={600} float={8} floorGlow shadow />
// jump from its apex at scene frame 9, holds the rest pose after the end
<UbiClip clip="jump" startFrame={9} playFrom={15} x={1250} y={660} size={600} />
// the storyboard's track verbatim (index = startIndex + f − from; hold; hidden)
<UbiTrack segments={useScene().ubiTrack} x={1383} y={660} size={600} floorGlow />
// keyframed move (x/y/size/opacity accept Keyframed)
<UbiClip clip="idle" size={[[105, 600], [119, 475, E.exit]]} x={[[105, 1383], [119, 1560, E.exit]]} y={660} />
```

Props: `clip` (manifest name, `ubi/` prefix ok), `startFrame` (scene frame where `playFrom` shows; before it
`playFrom` holds), `playFrom`, `loop` (default: the clip's `loopable`; non-loops hold the last frame),
`playbackRate`, `index` (force), `size`, `x`, `y`, `anchor` (`'feet' | 'ground' | 'headCenter' | 'headTop' |
'center' | 'body' | 'topLeft' | {x, y}` in 900-px frame px), `opacity`, `float` (px; 4.2 s sine), `floatPeriod`,
`floorGlow` (true | opacity | `{color, opacity, width}`), `shadow` (true | opacity), `rim` (volt drop-shadow),
`flip`, `preloadAhead` (Studio preloading, default 8 frames). Rendering waits for each PNG (`<Img>`), so no flashes.

Pure helpers: `ubiFrameIndex(clip, frame, opts)`, `ubiFrameSrc(clip, i)`, `ubiTrackAt(segments, f)`,
`idleLeadIn(n)` (idle start so an n-frame idle ends on frame 119 for a splice), `UBI_CLIPS`, `UBI_ANCHORS`.
Splices: `*-from-idle` start on idle 0 and end on idle 10 (continue idle from 11); `idle-to-*` end on the mood
loop's frame 10 (continue that loop from 11). One camera for every clip: keep size/x/y and UBI stays planted.

## Split transitions — `./transitions.tsx`

Types used by the board: `cut`, `flash` (T7), `whip-left` (T2), `blur-dissolve` (T6), `match-cut` (T5).

```tsx
<SceneTransitions>…</SceneTransitions>                 // both storyboard halves around the scene
<TransitionIn>…</TransitionIn>                          // storyboard transitionIn (type + frames)
<TransitionOut>…</TransitionOut>                        // storyboard transitionOut, ends on the last frame
<TransitionIn type="blur-dissolve" frames={6}><Bg/></TransitionIn>   // only some layers (s17: bg + UBI)
<TransitionOut>{(t) => <Ubi size={lerp(600, 475, t.k)} />}</TransitionOut>   // match cut: drive it yourself
const {in: tin, out: tout} = useTransition();           // inside a wrapper
```

Both halves come from one function, `splitTransition(type, side, frame, frames, start, opts)`, of the
proximity to the cut `k` (out: `k = (i+1)/N`, so its last frame is the cut state; in: `k = 1 − j/M`, so its first
frame is the cut state):

| type | out half | in half | options |
|---|---|---|---|
| `flash` | overlay ramps up | `#e8edf9` × 0.35 × E.exit(k) = 0.35 → 0 over f0–3 (E.push), drawn as a radial burst (peak at the centre, 0 at the corners, `screen` blend) so it adds light instead of a grey veil | `color` (6-digit hex), `peak` |
| `whip-left` | x 0 → −960 (E.exit), blur 0 → 40 | x +960 → 0 (E.push), blur 40 → 0 | `distance`, `blur` |
| `blur-dissolve` | blur → 16, opacity → 0.25, scale → 1.03 (E.glide) | mirror | `dissolveBlur`, `dissolveFloor`, `dissolveScale` |
| `match-cut` | untouched; `k`/`u` exposed | same | — |
| `cut` | nothing | nothing | — |

Wrap the layers that should move (a whip moves the plane and the type; keep the background still if you
prefer). Flash overlays are drawn above the wrapped layer.

## Stubs

`<SceneStub id="s05-drop-ubiqx" />` (background, id, copy lines lit while on screen, frame counter,
transitions applied) — what every scene file renders until its act builder replaces it.
