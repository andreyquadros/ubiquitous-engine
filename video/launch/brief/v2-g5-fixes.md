# v2 review fixes: G5 (s12–s15, abs 780–1124)

This pass fixes the three film reviewers' findings on s12–s15 of the rendered v2. It finishes or keeps the
previous engineer's work-in-progress commits. Only these files were edited: `src/scenes/s12-um-clique.tsx`,
`s13-nada-em-duvida.tsx`, `s14-relatorio.tsx`, `s15-foco.tsx` and `src/scenes/_parts/G5/common.tsx`.

The following did not change:
- timing and scene boundaries;
- the cut frames 779/780 and 1124/1125;
- the s15 whip-left out-half;
- the locked copy and the s15 headline "Foco que se defende.";
- every existing beat landing, except the fixes below.

`npx tsc --noEmit` is clean.

**How it was checked**
- **Stills:** `node tools/stills.mjs Launch …`, written under `out/review/g5fix/`, which is not tracked. Every
  still was viewed at 1920×1080 and at 480×270.
- **Metrics:** `python3 tools/look-metrics.py`.
- **Audio:** an audio-only render of abs 760–1130 (`REMOTION_AUDIO_ONLY=1 … --codec=wav --frames=760-1130`).
  The bed was fitted (gain 0.495) and subtracted, which leaves the SFX track. The table gives, for each
  frame, the SFX level and the sub-frame onset of each transient.
- **Text sizes:** measured on the canvas from glyph ink height, using Inter cap height 0.727 and x-height 0.546.
  Camera sizes come from `mapImagePoint` on the scene's own `ScreenConfig`.

## Findings

| # | Sev | Finding | Result |
|---|---|---|---|
| 1 | MAJOR | s15: UBI in the modal is an upscaled bitmap | **Fixed** (WIP kept, verified) |
| 2 | MAJOR | s12: proof below 34 px, no clean headline band | **Fixed** |
| 3 | MAJOR | s13: lower-right quadrant empty | **Fixed** (the committed chip, enlarged and re-placed) |
| 4 | MAJOR | s13: bubble lands about 889 but the pop is at 897 | **Fixed** |
| 5 | MAJOR | s14: "2 de 3 relatórios prontos" legible | **Fixed** (WIP kept, verified) |
| 6 | minor | s14: field text hard-clipped mid-glyph | **Fixed** |
| 7 | minor | s14: token-usage figures in the meta line | **Fixed** (WIP kept, verified) |
| 8 | minor | s14: ghosted "Relatórios" title and subline | **Fixed** (WIP kept, verified) |
| 9 | minor | s12 vs s11 continuity | **No contradiction left**; see the note |
| 10 | minor | click SFX leads the visible press by 1 f | **Fixed** (WIP kept, verified at 810, 990 and 1080) |
| 11 | minor | s15 headline lands about 1062, the tick is at 1065 | **Fixed** |
| 12 | minor | lift whooshes late and buried | **Fixed** |

### 1. s15: crisp 3D UBI (abs 1035–1124)

**What changed.** The WIP's `HiResUbi` is kept, and it now renders on the regenerated frames.
- The capture's 120-app-px bitmap UBI is covered with the panel's own background: `bg-panel` plus the
  `--hero-glow` gradient, measured median error 0.6 levels.
- `UbiClip` is composited at the capture's pose and feet:
  - frame scale 423/637;
  - `no` (two head shakes) plays f4–41 under "Não! Foque na sua produtividade.";
  - after the "Ok, foco!" click, `yes` (a nod) plays from f46, spliced seamlessly because `no` ends on
    `yes`'s first frame;
  - he floats like the in-app mascot.
- The app's rose FloorGlow is re-drawn under his feet.

**Evidence.**
- Frames 1035, 1038, 1042, 1046, 1050, 1055, 1070, 1076, 1085, 1095, 1105 and 1118.
- A full-resolution crop at 1050 shows clean anti-aliased edges, no fringing and no bitmap ghost. He is about
  400 px tall.
- The shake reads over 1038–1055 and the nod over 1085–1105.

### 2. s12: the proof reads at ≥ 34 px, with a clean headline band (abs 810–869)

**What changed.**
- **Hold camera.** Zoom 2.4 → 2.6. The camera pivots on row 1's top edge: focus at image y 432, anchored at
  canvas y 345. That keeps row 1 just under the band through the drift. The drift is now a 1 % push plus a slow
  pan instead of a 3.5 % push.
- **Tilt.** It now settles with the push by f50 (ry −6°, rx 6°), then only creeps to −5° and 5.5°, so the
  framing holds.
- **Row titles** (28 image px) measure 35–36 px on the canvas: the "M" cap height is 26 px at 836, 850 and 869.
  Through the projection they are 35.1 px at f50 and 35.6 px at f89. Before the fix they were 31.7–33.7 px.
- **"você" badges.** Their label is about 23 image px, which was about 28 px on the canvas. As the mint wave
  reaches each row, its badge pops (1 → 1.4) and stays swollen at 1.28× about its centre. The swell uses the
  capture's own @3x pixels via `Crop`. The swollen badge spans x 1815–1955, clear of the patched "100%" and
  of the chevron. The label measures about 38 px (x-height 20.7 px at 850). The card's drop target follows the
  swollen badge.
- **Clean band.** Everything of the list card above row 1 is flattened to the card colour #0c1220 while it
  slides under the navy band (image x 514–2043, y ≤ 427, faded in over f38–50 during the push). That covers the
  pending rows, "Classificados neste dia (19)" and its subline. The band is now clean navy: no ghost rows and no
  half-dimmed subline at its lower edge.

**Evidence.**
- Frames 780, 782, 786, 800, 809–812, 814–828, 832–850, 860 and 869.
- At 480×270 (frame 850) the headline, the app names and every "você" badge read.
- The row icons → badges span canvas x ≈ 55–1880. The chevrons sit at the right edge.

### 3. s13: the lower-right quadrant is filled (abs 885–929)

**What changed.** The committed chip was verified and made stronger. It is the empty state's own heading
"Nada esperando por você" with a mint check, lifted from the racking app at f17.
- The crop is tighter (x 930–1550).
- The scale is 1.42 → 1.5, so the heading is about 57 px.
- It moved from y 752 to 790, centred in the quadrant under the bubble.
- The mint spill went from 0.32 to 0.45.
- It shows no number.

**Evidence.**
- Frames 887–929.
- The chip spans about x 920–1840 and y 730–855. The band below it is now 210 px, 19 % of the frame height
  (under the 25 % rule).
- Readable at 480×270 (frames 897, 915, 929).

### 4. s13: the bubble lands on the pop (abs 886–897)

**What changed.** The in-app bubble holds and drifts f15–20, then grows with an ease-in (`t²(0.6 + 0.4t)`,
end slope 2.4). It hits its 1.05 peak on f27 (abs 897), then settles to 1 by f33. The WIP's ease-in-out arrived
with zero velocity, so it read as landed about 895–896. Now f26 is visibly short (scale about 0.85) and f27 is
the hit. The cue stays at f27.

**Evidence.**
- Frames 889 and 892–898, one per frame.
- Audio: the pop+2 transient onset is at abs 897 + 0.03 f, on the landing frame.

### 5. s14: no "2 of 3 ready" count (abs 960–1034)

**What changed.** The WIP's `PagePatches` are kept, in page colour #060a14, inside the window. They cover:
- the status line "Terça-feira, 29 de setembro: 2 de 3 relatórios prontos." (image x 513–1225, y 260–287);
- the "Relatórios" heading block (see 8).

That line is outside the lifted card's crop (y 376–882), so no copy of it exists inside the card. The card
still re-applies the storyboard patches through `patches`.

**Evidence.** Frames 962, 966, 970, 975 and 977: no count is legible.

### 6. s14: no hard-clipped field text (abs 975–1034)

**What changed.**
- Every activity field clips its text at image x 2047 in the capture.
- The overflow fade now runs over the last ≈ 7 glyphs (x 1950 → solid fill at 2047) in the window and in the
  card. The fields now read "…ementas e c" and "…000042/2" as a soft overflow.
- The fade spans only the glyph band (top+10 … top+56) and stops at x 2068. The WIP's full-height fade had cut a
  notch into both right corners of every field's rounded border; that is gone.

**Evidence.** Zoomed crops of 975, 1010 and 1034: a smooth fade and intact borders.

### 7. s14: no token figures

**What changed.** The WIP is kept. The meta line is re-composed from "Gerado 29/09 18:00" plus "3h22 registradas"
only. The "6.120 tokens de entrada, 1.480 de saída" is left out, and the model id stays patched by the storyboard
patches. The card's header shows only "IFRO / Gerado 29/09 18:00".

**Evidence.** Frames 966, 975 (window meta line) and 1010, 1034 (card).

### 8. s14: no ghosted page title

**What changed.** The WIP is kept. "Relatórios" and its subline (image x 513–1441, y 81–183) are patched to the
page colour.

**Evidence.** Frames 966 and 975: nothing reads behind "O relatório sai pronto.".

### 9. s12 continuity with s11

**Result.** There is no contradiction left.
- In the s12 capture the picker subtitle read "Calendário, 5min", which contradicted s11's "Calendário → IFRO".
  The WIP patches it (#0c1220). In s11 the subtitle ("Finder, 3|") is patched too, so both scenes show
  "Atribuir ao grupo selecionado" with no subtitle.
- The pending rows (Notas, Visual Studio Code) are the same tail as s11's queue, and Calendário is no longer
  pending.
- The s12 picker has no "Da última decisão / Sempre: Calendário → IFRO" block. That is an absence, not a
  contradiction. Rebuilding it inside s12's shorter picker card would not be cheap, so it was left alone.

**Evidence.** Frames 779, 780 and 782.

### 10. Click SFX on the visible press (abs 810, 990, 1080)

**What changed.** The WIP is kept and verified.
- `pressAt` is fully pressed on the cue frame.
- The first click ripple and the shockwave are visible on the event frame.
- s12's card also presses on f30, and its face flips mint on f30.

**Evidence.**
- Visible state change on the cue frame:
  - 810: the card turns mint, with a ripple and a ring;
  - 990: the button scale drops 5 %, with a ring and a ripple;
  - 1080: the face sinks to 0.96, with a ring.
- Audio: the click onsets are at 810.08, 990.08 and 1080.08. Each transient lands on the press frame.

### 11. s15: the headline lands on 1065

**What changed.**
- `HeadUnit` gains an optional `land` frame (`_parts/G5/common.tsx`). It time-stretches the SNAPPY spring so
  that staggered starts converge on one frame. s12's and s14's headlines are unchanged.
- s15 starts its words at 22, 23 and 24, and all three land at f30 (abs 1065) with the tick (`TICK = 30`):
  - at f29 all three are 2.4–3 px short, volt 0.86–0.91;
  - at f30 all are settled (≤ 1.1 px, volt 1).
- The reviewed version landed word by word: Foco at 1061, "que se" at 1063, defende. at 1065.

**Evidence.**
- Frames 1058, 1060 and 1062–1066: the line resolves at 1064–1065.
- Audio: the tick onset is at 1065.08.

### 12. Lift whooshes on the lift, audible (abs 782 and 977)

**What changed.**
- The whoosh_in_3 peak (0.31 s into the file) now sits on each lift's fastest frames:
  - s12: `atFrame` 10 → 2, abs 782. The card's SNAPPY lift from f0 moves fastest over f1–3. The file starts in
    s11's tail.
  - s14: `atFrame` 53 → 47, abs 977. The lift runs from f45.
- The level went from −22 to −14 dB, the top of the requested −16 to −14.

**Evidence.** In the mix the SFX-only peak is −17.6 dBFS at 781–782 and at 976–977. It was −25.6 dBFS before the
fix, and the RMS envelope peaks on those frames.

## Look metrics

These are the final stills, 54 frames across G5 (`out/review/g5fix/final`). Targets: mean ≥ 0.14, frac > 0.3
≥ 0.08, p10 ≤ 0.12.

| scene | n | mean | p10 | p50 | p90 | > 0.3 | verdict |
|---|---|---|---|---|---|---|---|
| s12-um-clique | 13 | 0.187 | 0.066 | 0.175 | 0.301 | 0.101 | PASS |
| s13-nada-em-duvida | 10 | 0.178 | 0.090 | 0.135 | 0.285 | 0.092 | PASS |
| s14-relatorio | 14 | 0.171 | 0.090 | 0.140 | 0.267 | 0.084 | PASS |
| s15-foco | 15 | 0.161 | 0.059 | 0.123 | 0.255 | 0.091 | PASS |

The mean of these stills is 0.173.

**s14 margin.** s14's > 0.3 fraction sat on the 0.08 floor: 0.078–0.080 depending on which frames were sampled.
Part B's window and card grade went from 1.30 to 1.36, the card glow from 0.6 to 0.8 and the key light from 0.16
to 0.18. A dense sample (every 3rd frame, 35 frames) now gives mean 0.171, p10 0.091 and > 0.3 0.084. The
weakest stretch is abs 978–993, between the lift and the click, at 0.070–0.079.

## Guards re-checked after zooming

- **s12:** the widened keys-legend patch and the sidebar badge patch are in place. The confidence
  percentages are patched (reviewed and pending rows), and so is the picker subtitle. The "(19)" header is now
  flattened away in the hold, and the toast is out of frame.
- **s13:** the resolved count, the settled count and the legend stay patched. The chip shows no number.
- **s14:** the model id, the token counts, "2 de 3" and the page title are all gone.
- **s15:** nothing new.
- **All four scenes:** no price, no provider logo, no purchase verb. The only fictional data is Ana, 12345.000042/2026-00
  and IFRO/Incubadora.

## Open issues

- **s14 brightness margin:** the > 0.3 fraction passes, but with a small margin (0.084 dense).
- **s12 framing:** the chevron column sits at the right edge through the hold. That is the price of fitting the
  row icons and the swollen badges at ≥ 34 px.
- **s11/s12 picker:** s12's picker lacks s11's "Da última decisão" rule block. That is an absence, not a
  contradiction.
- **Commit history:** an automatic checkpointer committed parts of this work as
  `wip(video): G5 review fixes in progress` (6ab8bc6, c3bd8d1) before the scene commits.
