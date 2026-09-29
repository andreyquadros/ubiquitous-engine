# ubiqX AI — launch video (Remotion)

Motion-graphics product launch video for **ubiqX AI** (on-screen copy in PT-BR),
1920x1080 @ 30 fps, cut to a 120 BPM grid (1 beat = 15 frames, 1 bar = 60).

Standalone npm project — not part of any workspace. Remotion **4.0.529**, React 19.

## Setup

```bash
cd video/launch
npm ci            # or npm install
```

The CLI config (`remotion.config.ts`) picks the browser automatically:
`$REMOTION_BROWSER` → Playwright headless shell
(`/opt/pw-browsers/chromium_headless_shell-1194/chrome-linux/headless_shell`) →
Playwright full Chromium → Remotion's own download. GL is `swiftshader`
(override with `REMOTION_GL=swangle|angle|…`), concurrency 3
(`REMOTION_CONCURRENCY=N` or `--concurrency=N`), JPEG q95 intermediates,
H.264 / yuv420p / BT.709 / CRF 18.

## Commands

| command | what |
|---|---|
| `npm run studio` | Remotion Studio on http://localhost:1460 |
| `npm run compositions` | list compositions (`Launch`, `Primitives`, scenes `S01`..`S18`, act groups `G1`..`G6`) |
| `npm run still -- Launch out/launch-0120.png --frame=120` | one still via the CLI |
| `npm run stills -- Launch 0,60,120-600:60` | many stills, one bundle + one browser (→ `out/stills/`), `--scale=0.5`, `--jpeg`, `--out=dir` |
| `npm run bench -- Launch 30,300,900` | per-frame render cost probe (best of `--reps=3`) |
| `npm run render` | final: `Launch` → `out/launch.mp4` |
| `npm run render:draft` | review cut: half scale, CRF 26, veryfast, `draft` prop (disables MotionBlur) → `out/launch-draft.mp4` |
| `npm run render:primitives` | the primitives reel → `out/primitives.mp4` |
| `npm run typecheck` | `tsc --noEmit` |
| `npm run grain` | regenerate `public/fx/grain.png` |

Extra CLI flags pass through after `--`, e.g. `npm run render -- --frames=0-179`.
Inspect outputs with `/usr/local/bin/ffmpeg -i out/launch.mp4` (there is no ffprobe).

Measured on this box (4 shared cores, concurrency 3, swiftshader): the heavy
Screen/Callout/Cursor section of the Primitives reel renders at **~5 frames/s**
(180 frames in 40 s wall incl. ~6 s bundling). A 50 s Launch ≈ 5 min.

## Layout

```
brief/            product truth sheet (facts.md) — every on-screen claim comes from here
public/
  ui/             app captures (2880x1800, DPR 2). _placeholder.png is 1440x900
  ubi/  brand/    mascot renders, logos
  audio/sfx/      sound effects (<Sfx src="name.wav"/>)
  audio/music/    soundtrack (<Music src="name.wav"/>)
  fonts/          Sora + Inter variable woff2 (latin, latin-ext)
  fx/grain.png    film grain tile
  _demo/          assets used only by the Primitives reel
src/
  index.ts        registerRoot
  Root.tsx        compositions: Launch, Primitives, S01..S18, G1..G6
  storyboard.ts   typed brief/storyboard.json (SCENES, sceneById…)
  timeline.ts     TOTAL / TIMELINE / GROUPS derived from the storyboard
  scenes/         one module per storyboard scene (+ index.ts registry)
  shared/         scene API: audio, beats, hotspots, hi-res, UbiClip, split transitions — see shared/README.md
  design/         tokens.ts (palette, type, grid, springs, easings), motion.ts, fonts.ts, env.ts
  components/     primitive library — see src/components/README.md
  compositions/   Film.tsx (scenes + master audio), Launch.tsx, Primitives.tsx (demo reel)
tools/            stills.mjs, bench.mjs, make-grain.py
out/              renders (gitignored)
```

## Scenes (the spine)

The film is specified in `brief/storyboard.json` (source of truth; `python3 tools/validate-storyboard.py`).
`src/storyboard.ts` types it, `src/timeline.ts` derives `TOTAL`/`GROUPS` from it, and
`src/compositions/Film.tsx` mounts every scene as an absolute `<Sequence>` plus one master audio layer
(music + every scene's SFX cues at absolute frames). See **`src/shared/README.md`** for the scene API
(storyboard access, hotspots, hi-res captures, UBI clips, split transitions, audio/beat helpers).

1. Replace the stub in `src/scenes/<scene-id>.tsx` (default export = the scene, frames scene-relative;
   `export const sfx: SfxCue[]` = its cues, hit frames scene-relative).
2. Check it alone: `node tools/stills.mjs S07 "0,30,59" --jpeg`, then with its neighbours: the act group
   (`G1`..`G6`) and `npm run render:draft`.
