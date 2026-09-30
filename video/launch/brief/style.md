# ubiqX AI launch video: style guide

**What this is.** Rules for building the ubiqX AI launch film in Remotion so it looks like the motion-graphics
launch videos SaaS teams post on X/Twitter (Linear, Raycast, Arc, Cursor, Framer, Vercel, Notion, Superhuman,
Cal.com, Supabase launch weeks). Every rule is a number or a recipe an engineer can type in.

- **Canvas:** 1920×1080, 30 fps. **Grid:** 120 BPM, 4/4. **1 beat = 15 frames, 1 bar = 60 frames (2 s).**
- **Target:** 40–55 s (1200–1650 frames). The reference cut is **48 s = 1440 frames = 24 bars**.
- **On-screen language:** PT-BR. No voice-over. It must work **muted**, because X autoplays muted in the feed.
- **Source of truth for copy and claims:** `brief/facts.md`. This file covers only *how* things move, look and
  sound. The PT-BR lines quoted here are style examples taken from the approved phrases in facts.md §6. If this file
  and facts.md disagree about wording or a claim, facts.md wins.
- **Stack in this repo:** Remotion 4.0.529 plus `@remotion/transitions`, `@remotion/motion-blur`, `@remotion/noise`,
  `@remotion/paths`, `@remotion/shapes` and `@remotion/layout-utils`. Fonts are local variable woff2 files in
  `public/fonts/` (Sora wght 100–800, Inter wght 100–900).

Research notes, what was verified and what comes from craft knowledge are listed in [§11 Sources](#11-sources-and-research-notes).

---

## 0. Frame math and global constants

```ts
// src/design/grid.ts (suggested)
export const FPS = 30, BEAT = 15, BAR = 60;
/** bar is 1-based, beat is 0–3, sub is extra frames */
export const at = (bar: number, beat = 0, sub = 0) => (bar - 1) * BAR + beat * BEAT + sub;
/** 8th notes land on 0, 8, 15, 23, 30, 38, 45, 53 … (7.5 frames rounded, alternating 8/7) */
export const eighth = (n: number) => Math.round(n * 7.5);
/** 16th notes land on 0, 4, 8, 11, 15, 19, 23, 26 … */
export const sixteenth = (n: number) => Math.round(n * 3.75);
```

**Where a frame counts as "on the beat": four accent types.** Pick the accent type for each motion. Choosing the
wrong type is the most common reason motion looks slightly off the music.

| Accent type | Examples | Rule |
|---|---|---|
| **Onset** (fast start, then slows) | spring or expo-out entrances, pops, reveals, push-ins | First moving frame = `beat − 1`. The 1-frame pre-roll makes the element clearly visible on the beat frame. |
| **Contact** (the motion ends in a hit) | word slam, key press, click, stamp, counter landing, logo slam | The **contact frame = beat**. Start = `beat − lead`, with `lead` taken from the spring table in §3.1. |
| **Symmetric** (fastest in the middle) | whip pan, zoom-through, blur dissolve | The **midpoint (the cut) = beat**. Half the frames go before the beat and half after. |
| **Span** (camera travel between two holds) | glide from framing A to framing B, split-screen divider | Starts on a beat and ends on a beat. Duration is always 15, 30, 45 or 60 frames. |

**Audio/visual sync:** an SFX transient lands on the visual event frame or up to 1 frame after it. It never lands
before. Audio that arrives early reads as a mistake; audio 1 frame late reads as natural.

**Safe areas (X player):**
- Title-safe box: `x 144…1776`, `y 81…999`. That is a 7.5% inset.
- Keep the **bottom 120 px** clear of critical text, and keep 200×120 px in the bottom-left and bottom-right
  corners clear. X draws the timecode and the mute button there.
- Frame 0 is the poster and the thumbnail on many clients. It must be a **designed, legible frame**: never black,
  and never the first frame of a fade.

**Determinism:** never use `Math.random()`, CSS `transition`/`animation`, or Tailwind animate classes. Use
`random(seed)` from `remotion` and `noise2D/3D` from `@remotion/noise`.

---

## 1. Structure: the shared beat sheet

Launch films in this genre follow the same arc. Craft write-ups consistently give three rules: make a
scroll-stopping first frame, change something every 2–3 s, and sell one clear outcome instead of listing
everything.

### 1.1 Reference cut, 48 s / 24 bars / 1440 frames

| Act | Bars | Frames | Time | Shots | Job | Music |
|---|---|---|---|---|---|---|
| **A. Hook** | 1 | 0–59 | 0–2 s | 1–2 | Stop the scroll. Name the pain as a question or a bold claim. Frame 0 is already readable. | The track starts **on a downbeat at frame 0**, with a hit or a clear kick and no silent lead-in. |
| **B. Tension / problem** | 2–5 | 60–299 | 2–10 s | 4–7, plus an optional chaos montage of 4–8 flash cuts | Make the problem felt: context switching, hours leaking away, the Friday timesheet filled in from memory. | Sparse, low-pass filtered ("under water"). A **riser across bar 5**. |
| ↳ drop-out | last 8–15 f of bar 5 | 285–299 | 0.3–0.5 s | 0 | Cut to near-black and silence. The pause is what makes the drop land. | Silence. |
| **C. Reveal** | 6–7 | 300–419 | 10–14 s | 2–3 | Logo slam **on frame 300** (the drop), then the product hero shot (tilt-to-flat), then the UBI entrance. | **Drop** at 300: full-range groove begins. |
| **D. Feature montage** | 8–18 | 420–1079 | 14–36 s | 5 modules × 3–4 shots (15–20 shots) | One benefit per module: headline plus UI proof. The order follows the product loop: registra → classifica → revisão → relatório → foco/privacidade. | Main groove. Variation every 4 bars. |
| **E. Proof** | 19–21 | 1080–1259 | 36–42 s | 3–4 | Facts only, from facts.md §4: counters, the platform row, a 3D wall of screens. **No invented stats, logos or testimonials.** | Lift or breakdown, then a 1-bar build. |
| **F. CTA end card** | 22–24 | 1260–1439 | 42–48 s | 1–2 | Wordmark, tagline, "Baixe em ubiqx.com.br", macOS · Windows · Linux. Holds ≥ 90 f fully legible. | **Final hit on 1260**, then ring-out. The audio tail finishes by 1439. |

**Scaling to other lengths (keep act proportions):** hook 4%, problem 16–20%, reveal 8%, features 42–48%,
proof 10–12%, CTA 11–13%.
- **40 s (1200 f, 20 bars):** 4 feature modules, a 2-bar proof, and a 2.5-bar CTA.
- **54 s (1620 f, 27 bars):** 6 feature modules.

Always change length in whole bars.

### 1.2 Pacing budget

- **Total shots:** 28–38. **Average shot length:** 1.2–1.7 s. Feature modules run faster (1.0–1.5 s); the reveal
  and end card run slower (3–6 s).
- **Shot length limits:** minimum 8 frames (flash cuts: 4). Maximum 120 frames. The end card may run to 180.
- **Something new every 30–60 frames:** a cut, a camera move or a new element entrance. A 90-frame stretch where
  only the float moves counts as a dead spot.
- **Dynamics:** the problem act is slower and darker, the reveal is the loudest and brightest moment, and the
  features act is fast and regular. Put one short breather (≥ 1 bar of calmer motion) before the proof.
  Monotony is the enemy.

### 1.3 Feature module templates (2 bars = 120 frames each)

**Module A, "Title → proof" (sequential):**

| Frames (relative) | What happens |
|---|---|
| 0–14 (beat 1) | Kicker and headline enter (word stagger, S03). The kicker leads by 3 f. |
| 15–44 (beats 2–3) | Headline holds. The UI plane rises in behind or below it (S09 or S10). |
| 45–104 (beats 4–7) | UI proof: push-in to the hotspot (S11/S12). **Click or key press on frame 60** (bar 2 downbeat, S13/S14). The state change lands at 62–64. |
| 105–119 (beat 8) | Hold, then out-transition. Symmetric transitions have their midpoint at 120. |

**Module B, "Side by side" (text and UI on screen together; the most common layout in these films):**
- Text column on the left: x 144–760, left-aligned, vertically centred.
- UI plane on the right: 1100 px wide at `rotateY(-10deg) rotateX(4deg)`, right edge bleeding off-frame by
  60–120 px.
- The text enters on beat 1. The UI performs its action on beat 5 (frame 60).

Alternate A and B. Never use the same module layout three times in a row.

---

## 2. Shot catalog

Common notation: **f** = frames; **C** = contact/hit frame (always on a beat); **B** = a beat frame. Spring
presets (`SNAPPY`, `SMOOTH`, `SLAM`, …) and easings (`E.push`, `E.glide`, …) are defined in §3.

### S01 · Cold-open hook frame
- **Purpose:** the scroll-stopper and the thumbnail.
- **Composition:** one display headline (S03 styles, 112–128 px) naming the pain. Example from facts.md §6:
  *"Você senta às 8. Levanta às 18."* The emphasis word is volt.
- **Background:** an out-of-focus UI fragment (Timeline blocks) at 40% opacity, blur 8 px, plus a volt glow orb.
- **Choreography:**
  - Frame 0 shows the text **fully set**, with no entrance.
  - 0 → end: camera scale 1.00 → 1.04, `E.glide`. Background parallax x 0 → −24 px.
  - Frame 2: the underline sweep under the emphasis word starts (12 f).
  - A second line may enter at B = 30 (masked reveal, S04).
- **Duration:** 45–60 f. **Beat:** hard cut on 60.
- **SFX:** none extra. The music's first downbeat is the hit.

### S02 · Kinetic word slam
- **Purpose:** punch 1–3 words: a verb, a promise, or the brand.
- **Type:** Sora 800, 180–240 px, tracking −0.045em, centred. ≤ 12 characters per line.
- **Choreography (relative to C):**
  - C−4: mount the word at scale 1.45, opacity 0, blur 12 px, tracking −0.01em.
  - C−4 → C: `spring(SLAM)` drives scale 1.45 → 1. It reaches 0.98 exactly 4 frames later, which is C.
    Opacity 0 → 1 over the first 2 f; blur 12 → 0 over 4 f; tracking → −0.045em.
  - C: camera shake.
    - Translation: `noise2D(seed, t*0.9, 0) * 6px * exp(-t/3)` for 8 f.
    - Rotation: ±0.4° with the same decay.
  - C: optional canvas lift `#0a0d16 → #121a2c → #0a0d16` over 3 f.
  - Next word: **hard cut** to it at C+15 (the next beat). Words replace each other; there is no exit animation.
- **Duration:** 12–30 f per word. A sequence of 2–4 words fills 1 bar. The last word holds ≥ 30 f.
- **Easing:** SLAM spring (4.2% overshoot). The shake decays exponentially.
- **Beat:** contact type, C = beat.
- **SFX:** impact at C, +0 to +2 dB relative to the bed. Music duck −5 dB (see §7.4).
- **Don't:** slam more than 4 words in a row, slam sublines, or push overshoot above 5%.

### S03 · Word-by-word stagger (the workhorse headline)
- **Purpose:** headlines of 3–7 words. Readable, but with rhythm.
- **Choreography:**
  - Split on spaces. Punctuation stays attached to its word. Each word is an `inline-block`.
  - Each word animates `translateY 28 → 0 px`, `opacity 0 → 1` (over the first 6 f) and `blur 8 → 0 px`, with
    `spring(SNAPPY)`.
  - Stagger **3 f** (fluid) or **8th notes via `eighth(n)`** (rhythmic, only for ≤ 5 words).
  - The first word starts at B−1. Arrange the stagger so the **emphasis word lands on a beat**.
  - The emphasis word turns volt as it lands. The underline sweep (S03-U, §4.3) starts 2 f later.
  - Exit: all words together, `translateY 0 → −16`, opacity → 0, 8 f, `E.exit`. A hard cut is preferred.
- **Duration:** entrance = `3·(N−1) + 10` f, plus the hold from §3.6.
- **SFX:** none, or one soft tick on the emphasis word (−18 dB).

### S04 · Masked line reveal
- **Purpose:** calm, premium statements such as the tagline, the privacy line, or the end card.
- **Choreography:**
  - Each line sits in an `overflow: hidden` wrapper with **padding-top 0.16em and padding-bottom 0.12em**,
    compensated by negative margins. Without the padding, PT accents (Ê, Ã, Ó) and Ç get clipped.
  - The inner span animates `translateY 110% → 0` and `rotate 3° → 0` (origin left bottom) over 18 f with `E.enter`.
  - Lines stagger by 5 f.
  - Exit: `translateY 0 → −110%`, 10 f, `E.exit`, lines staggered by 3 f.
- **Duration:** 18 + 5·(lines−1) f to enter, plus the hold.
- **Beat:** onset type (starts at B−1).
- **SFX:** none, or a soft air swell (−20 dB).

### S05 · Chaos montage (flash cuts)
- **Purpose:** show the problem: fragmentation, context switching.
- **Content:**
  - 6–10 micro-shots of **generic, unbranded** UI fragments: tab strips multiplying, window-switcher tiles,
    a generic chat badge counter, an empty timesheet grid, a calendar full of blocks.
  - No real brand UIs (facts.md §5.15). No keylogger, screen-recording or typing-capture imagery (facts.md §5.5).
- **Choreography:**
  - Each micro-shot has its own push: scale 1.00 → 1.06, linear, rotation `±2°` from `random(seed)`.
  - Rhythm: one bar of 8th notes (`eighth(n)` boundaries: 8/7/8/7…), then one beat of 16ths (4/4/4/3).
    The montage ends on a **drop-out** (S-T8).
  - Overlay a ticking kicker such as "Trocas por hora: 12… 17… 23" (Inter 600 28 px, rose). This is demo data
    and must match the demo UI.
- **Look:** desaturated 30%, brightness 0.9. Rose accents only. Keep every fragment on the dark canvas so
  full-frame luminance never flips (photosensitivity: < 3 large luminance flashes per second).
- **Duration:** 45–75 f.
- **SFX:** a glitch tick on each cut (−16 dB). The riser runs underneath.

### S06 · Stacked notification pop-ins
- **Purpose:** UBI nudges and "things happening to you".
- **Toast size and layers:**
  - 560 × 104 px, radius 22, background panel-2 at 94%, 1 px `rgba(255,255,255,0.08)` border, shadow tier 2 (§5.3).
  - Icon: 44 px UBI avatar.
  - Title: Inter 600, 26 px, ink.
  - Body: Inter 400, 24 px, ink-2, one line.
  - Time: "agora", 20 px, ink-3.
- **Anchor:** top-right (right 96 px, top 96 px), or centred when the toast is the subject.
- **Choreography:**
  - Enter: `translateY −32 → 0`, `scale 0.94 → 1`, `opacity 0 → 1`, `spring(SNAPPY)`.
  - A new toast pushes older ones down by 118 px (104 + 14 gap) with the same spring. Animate a `slot` index; do
    not re-layout.
  - Depth `d` (0 = newest) sets `scale 1 − 0.03d`, `opacity 1 − 0.22d` and `blur 1.5d px`. At most 4 are visible;
    the 5th fades out at the bottom (6 f).
  - Stagger: **1 per beat** when the text must be read; **8th notes** when the toasts are only texture.
  - Finish by holding on the toast that matters, with a push 1.0 → 1.25 on it (`E.push`, 20 f).
- **Copy:** real UBI lines only (facts.md §6), for example "Muitas trocas de contexto hoje. Vamos fechar uma coisa
  de cada vez?"
- **SFX:** a soft pop per toast (−14 dB). Each successive toast is **one semitone higher**. Pre-render the pitched
  variants with ffmpeg `asetrate=48000*2^(n/12),aresample=48000`, because `playbackRate` does not pitch-shift in
  renders.

### S07 · Number counter
- **Purpose:** a fact that becomes a moment, for example `0 → 100` "Score de foco" or `1–9` "teclas". Values come
  only from facts.md §4 or visible demo data.
- **Type:**
  - Number: Sora 700, 200–280 px, tracking −0.04em, `fontVariantNumeric: 'tabular-nums'`. Both Sora and Inter
    ship `tnum` (checked in the files); without it, digits jitter.
  - Unit or suffix: 50–60% of the number's size, ink-2. Kicker above.
- **Choreography:**
  - Landing frame **L = a downbeat**.
  - `value = interpolate(frame, [L−30, L], [from, to], {easing: Easing.bezier(0.25, 0.1, 0.25, 1), extrapolate: clamp})`,
    then `Math.round`.
  - Format with `Intl.NumberFormat('pt-BR')`. Durations render as `5h08` or `30 min`.
  - At L: scale pulse 1 → 1.05 → 1 (`BOUNCY_SUBTLE`), the number flashes volt for 1 beat, and an underline sweep
    runs under the unit.
- **Duration:** 30–45 f count plus a ≥ 30 f hold.
- **SFX:** ticks at most 1 per 3 frames (−22 dB); a shimmer or ding at L (−12 dB).

### S08 · Logo reveal slam (brand reveal)
- **Purpose:** the payoff of the tension. Always on the drop.
- **Elements:**
  - Wordmark "ubiq" in ink plus "X" in volt: Sora 700, 200 px, tracking −0.03em.
  - "AI" pill: Sora 600, 56 px, volt text on `rgba(77,141,255,0.14)`, radius 999, padding 0.2em 0.5em.
- **Choreography (C = drop frame, for example 300):**
  - C: the wordmark is on screen at **full opacity from its first frame** at scale 1.10, and springs to 1 with
    `SLAM`. Tracking goes +0.02em → −0.03em over 20 f (`E.push`).
  - C+3: the "X" pops separately: rotate −90° → 0, scale 0 → 1, `BOUNCY_SUBTLE`.
  - C: flash overlay `#e8edf9` at 0.35 opacity, decaying to 0 over 4 f (see T7).
  - Glow orb behind: opacity 0 → 0.30 over 6 f, then settles to 0.18 by C+30.
  - C+8: the "AI" pill pops on (`BOUNCY_SUBTLE`, on the 8th).
  - C+10 → C+28: glint. A 140 px wide diagonal highlight (white, 18% opacity) sweeps across the letters, masked to
    the glyphs with `background-clip: text` on a duplicate layer.
- **Hold:** ≥ 45 f. Exit via zoom-through the "X" (T3) into the product hero.
- **SFX:** sub boom plus impact at C (+2 dB relative). The riser ends at C−1. Shimmer on the glint (−14 dB).
  Duck −6 dB.

### S09 · Hero product reveal (tilt-to-flat)
- **Purpose:** the first full look at the product. It should feel expensive.
- **Setup:**
  - Window-framed UI (S10 chrome), 1640 px wide.
  - Parent `perspective: 2200px`. Plane `transform-origin: 50% 100%`.
- **Choreography (start = B−1):**
  - From `translateY 260, rotateX 32°, scale 0.88, opacity 0` to `translateY 0, rotateX 6°, scale 1, opacity 1`.
  - Opacity takes 6 f. Everything else takes 45 f with `E.push` (97% done by 22 f).
  - Glow orb rises with it (y 200 → 0, same easing). The grid floor fades from 0 to 40%.
  - The window then continues into the S10 float.
- **Duration:** 60–90 f.
- **SFX:** low whoosh starting 6 f before, peaking around +6 (−8 dB).

### S10 · Window frame float (idle state for UI planes)
- **Window chrome:**
  - Radius 18 px (display size). 1 px `rgba(255,255,255,0.08)` border. Top inner highlight
    `inset 0 1px 0 rgba(255,255,255,0.06)`.
  - Title bar 44 px, panel colour. Three 12 px dots `#3a4560` (neutral, platform-agnostic), or real macOS colours
    at 80% saturation only in shots marked macOS.
  - Shadow tier 3 (§5.3).
- **Float (never fully static):**
  - `y = 6·sin(2π·f/120)`
  - `rotateX = base + 0.6·sin(2π·f/180)`
  - `rotateY = base + 0.8·sin(2π·(f+30)/240)`
  - The shadow offset tracks: `shadowY = 48 + 0.5·y`.
- **Specular sheen:** a diagonal linear gradient (white 4% → 0) that drifts across the glass 0.5 px/f.
- **Use:** the base state of every UI plane between actions, and the right-hand plane in module B.

### S11 · UI zoom-to-element with 3D tilt
- **Purpose:** make a detail readable and cinematic. This is the signature move of the genre. Raycast-style
  continuous panning inside one UI reads more premium than cutting between screenshots.
- **Camera rig:** in the snippet below, `(x, y)` is the UI point placed at screen centre. Rotation happens around
  that point.

```tsx
// Parent: <AbsoluteFill style={{perspective: 2000, perspectiveOrigin: '50% 50%'}}>
<div style={{
  position: 'absolute', left: 0, top: 0, width: W, height: H, transformOrigin: '0 0',
  transform: `translate(960px,540px) rotateX(${rx}deg) rotateY(${ry}deg) rotateZ(${rz}deg) scale(${zoom}) translate(${-x}px,${-y}px)`,
}}>{/* <Img> of the UI or live UI */}</div>
```

- **Tilt sign convention (CSS):**
  - `rotateX(+θ)`: the top edge recedes and the bottom edge comes toward the camera.
  - `rotateY(+θ)`: the right edge recedes and the left edge comes toward the camera.
  - For a target at normalised offset `(dx, dy) ∈ [−1, 1]` from the plane centre, use
    **`ry = −10°·dx`, `rx = +6°·dy`**. The larger part of the UI then recedes behind the target.
  - Limits for readable UI: |rx| ≤ 10°, |ry| ≤ 14°, |rz| ≤ 3°.
- **Values:**
  - Wide framing: zoom 0.85–1.0 with a tilt of (rx 6°, ry −10°).
  - Target framing: zoom 1.8–2.6, with the tilt from the formula above.
  - Interpolate zoom in log space: `zoom = exp(lerp(ln z0, ln z1, e))`. Remotion 4.0.529's
    `interpolate(..., {output: 'perceptual-scale'})` does the same.
- **Timing:**
  - Glide between framings: **30 or 45 f**, `E.glide`, span type (starts and ends on beats).
  - Punch-in: 18–24 f, `E.push`, onset type.
  - Hold at the target ≥ the text's read time (§3.6), with micro drift zoom ×1.00 → ×1.02.
- **Readability:**
  - Effective on-screen font size = `uiCssPx × (planeDisplayWidth / uiViewportCssWidth) × zoom`. Anything that
    carries the message must be **≥ 36 px**.
  - Bitmap upsampling = `displayedWidthPx / bitmapWidthPx` must stay **≤ 1.15** on every frame. Capture zoom
    targets at DPR 3, or swap to a high-DPR crop once zoom > 1.4. Prefer live HTML UI when it is available.
- **SFX:** fast punch-ins get a soft whoosh (−14 dB). Slow glides get nothing; the music carries them.

### S12 · Camera push-in on a hotspot with focus dimming
- **Purpose:** point at the one thing that matters, such as "Confirmar os N", a category chip, or "Copiar Markdown".
- **Focus mask:** place it **inside the UI plane** so it moves with the camera. `R` is the hotspot rect plus 16 px
  padding.

```tsx
<div style={{position: 'absolute', left: R.x, top: R.y, width: R.w, height: R.h, borderRadius: 14,
  boxShadow: `0 0 0 4000px rgba(10,13,22,${0.62*d}), 0 0 0 2px rgba(77,141,255,${0.7*d}), 0 0 40px 4px rgba(77,141,255,${0.3*d})`}}/>
```

- **Choreography (start = B−1):**
  - `d` 0 → 1 over 12 f (`E.enter`).
  - At the same time the camera pushes zoom ×1.25–1.6 over 20–30 f (`E.push`).
  - +8 f: the callout pill appears beside the hotspot (Inter 600, 28 px, ink on `rgba(77,141,255,0.16)`, radius 999)
    with a 1.5 px ink-3 connector line drawn on over 10 f (`@remotion/paths` `evolvePath`).
  - Release: `d` 1 → 0 over 8 f as the camera leaves.
- **Duration:** 45–75 f.

### S13 · Cursor move, click ripple and UI state change
- **Cursor:**
  - macOS-style arrow SVG, **30 px tall in screen space**, white fill, 1.5 px `#0a0d16` stroke, shadow
    `0 2px 6px rgba(0,0,0,.5)`. The hotspot is the tip.
  - Render it **outside the camera plane**. If the camera zooms, cursor scale = `clamp(1 + (zoom−1)·0.35, 1, 1.5)`,
    so it never becomes giant.
- **Path:**
  - A quadratic arc from A to B. The control point is offset perpendicular by 12% of the distance, and the arc
    bends the same way throughout the video.
  - Duration = `clamp(10 + dist/60, 12, 26)` f, `E.cursor`.
  - Arrive **3–4 f before** the click. The cursor must never teleport: it enters from off-frame, or fades in at a
    rest point (6 f) before moving.
- **Click at C (on the beat):**
  - Hover state: the target's hover background fades in over 4 f when the cursor crosses its bounds (about C−6).
  - Press: cursor scale 1 → 0.85 over C−1..C+1, then back to 1 by C+6 (`SNAPPY`). The button does the same with
    scale 0.96 and its pressed colour.
  - Ripple (screen space, centred on the tip): circle radius 0 → 44 px, stroke 2 → 0.5 px, opacity 0.6 → 0, 14 f,
    `E.push`. An optional second ring starts 3 f later: 0 → 64 px at opacity 0.3.
- **State change:**
  - C+2: the new state morphs in: `opacity 0 → 1`, `scale 0.98 → 1`, 6 f, `SNAPPY`.
  - Success: a mint check draws on (`evolvePath`, 10 f) and the row flashes mint at 12% for 8 f.
- **Duration:** 30–60 f.
- **SFX:** click at C (−12 dB). Success shimmer at C+3 (−14 dB).

### S14 · Keycap press close-up
- **Purpose:** keyboard speed. The Revisão queue uses keys **1–9**, ↑/↓ and Enter (facts.md §4).
- **Keycap (CSS 3D):**
  - Size: 220 × 220 px (Enter: 420 × 220). Radius 32.
  - Top face: gradient `#1c2740 → #141c2e`, 1 px `rgba(255,255,255,0.10)` border, inner top highlight.
  - Skirt: 18 px of `#0c111d`, plus a floor shadow.
  - Legend: Inter 600, 84 px, ink.
  - Glyphs: **draw ↑ ↓ → ⌘ ⏎ as inline SVG**. Sora has none of these arrows and Inter lacks → and ⌘ (checked in
    `public/fonts`), so a font fallback would appear.
  - Parent `perspective: 1200px`. Key at `rotateX(28deg) rotateZ(-6deg)`.
- **Choreography:**
  - Press (C = beat): C−3 → C, `translateY 0 → +14 px` while the skirt shrinks 18 → 4 px (`E.exit`, 3 f).
  - At C: the legend turns volt, and an underglow `0 0 60px rgba(77,141,255,0.55)` flashes, decaying over 8 f.
  - Release: C+1 → C+8 with `SNAPPY`.
  - Key sequences (for example ↓ ↓ 1) go on 8th notes.
- **Cut:** match-cut (T5) from the key legend "1" to the UI chip for category 1.
- **Duration:** 20–45 f.
- **SFX:** mechanical key-down at C (−10 dB), key-up at C+6 (−18 dB).

### S15 · Typing into a field (typewriter)
- **Purpose:** show user intent. Examples: a category description in Categorias ("O que conta como trabalho
  desta categoria"), or a focus task in Foco ("Que tarefa você precisa fazer agora?").
- **Choreography:**
  - Text reveal: **1 character per frame** (30 cps). For strings > 30 characters, 2 characters per frame.
  - Caret: solid while typing, then blinks 8 f on / 8 f off.
  - Hold 8 f after the last character before anything reacts.
  - Ghost-text suggestions (ink-3), if used, accept with a Tab keycap (S14).
- **Duration:** chars + 20 f.
- **SFX:** soft key ticks every 2–3 characters, chosen at random from 4 variants (−20 dB).

### S16 · Feature card grid pop
- **Purpose:** breadth in one shot, and a hub that jumps into a feature.
- **Grid:** 3 × 2 cards, each 540 × 300, gap 32.
- **Card:** panel background, 1 px line border, radius 24, padding 36. Contents:
  - Icon tile: 64 px, radius 16, `rgba(77,141,255,0.12)` background, volt 2 px line icon.
  - Title: Sora 600, 40 px.
  - One line: Inter 400, 26 px, ink-2, ≤ 6 words.
- **Choreography:**
  - Delay = `(row + col) × 4` f (diagonal wave; total spread 12 f).
  - Each card: `y 48 → 0`, `scale 0.92 → 1`, `blur 10 → 0`, `opacity 0 → 1` (6 f), `SNAPPY`. Its icon pops 4 f
    later (`BOUNCY_SUBTLE`, scale 0.6 → 1).
  - The whole grid sits in `perspective 2400px`: rotateX 8° → 4° and scale 1.00 → 1.04 across the shot.
  - Focus at the end: the other cards go to opacity 0.35 and blur 3 px over 10 f. The chosen card goes to scale
    1.04 with a 1.5 px volt border and glow 0.25. Then zoom-through (T3) into that feature.
- **Duration:** 60–90 f. Starts at B−1.
- **SFX:** **one** grouped sweep or tick-cluster (−14 dB), not six pops.

### S17 · Split screen, before/after
- **Purpose:** contrast the manual chore with ubiqX.
- **Content:**
  - Left, "ANTES": an empty timesheet grid, a note reading "o que eu fiz hoje?", a clock. Desaturated 60%,
    brightness 0.8, rose labels, 1 px jitter.
  - Right, "DEPOIS": a clean Relatórios (Diário) screen at full colour.
  - **Do not uppercase the brand**, so the kickers are "ANTES" / "DEPOIS", not "COM UBIQX".
- **Choreography:**
  - A 2 px ink divider with a 40 px handle (volt ring) sweeps x 1920 → 960 over **30 f** (`E.glide`, span type).
  - The right side is revealed with `clipPath: inset(0 0 0 ${x}px)`.
  - Kickers enter as the divider passes.
- **Duration:** 60–90 f.
- **SFX:** swipe whoosh, −12 dB.

### S18 · Data build (timeline blocks, charts)
- **Purpose:** "Registra sozinho". The day fills itself in.
- **Timeline blocks:**
  - Category-coloured blocks grow left → right with `scaleX 0 → 1` (origin left), each 12 f with `E.enter`,
    staggered 2 f. Cap the total spread at 20 f; if there are too many blocks, grow them in groups.
  - A time kicker counts `08:00 → 18:00` (S07 rules).
- **Line chart:** `evolvePath` over 30 f with `E.glide`.
- **Bars:** `scaleY` with `SNAPPY`, staggered 3 f.
- **Labels:** fade in after the geometry has landed.
- **Duration:** 45–75 f.
- **SFX:** optional subtle tick arpeggio (−20 dB).

### S19 · Mascot entrance (UBI)
- **Asset:** UBI render PNG with alpha, ≥ 1200 px tall. Display 420–640 px tall next to text, 800 px as the hero.
- **Entrance (start B−1):**
  - `y +320 → 0`, `rotate −10° → 0`, `scale 0.7 → 1` with `BOUNCY_SUBTLE` (7% overshoot).
  - Landing squash: scaleY 0.97 / scaleX 1.03 for 3 f. He is a hard-shell robot, so keep squash at 3% or less.
  - Volt rim light: `drop-shadow(0 0 30px rgba(77,141,255,0.35))`.
- **Speech bubble** (at land + 8 f):
  - Scale 0.6 → 1 from the tail origin, `BOUNCY_SUBTLE`.
  - Panel-2 background, ink Inter 500 30 px, ≤ 10 words, words staggered 2 f.
  - Real UBI lines only (facts.md §6), for example "Oi, eu sou o UBI."
- **Idle:**
  - `y = 10·sin(2π·f/120)`, `rotate = 1.5·sin(2π·f/160)`.
  - An ellipse shadow below: width `1 − y/100`, opacity 0.35.
- **Duration:** 45–90 f.
- **SFX:** bloop/pop at land (−12 dB). Bubble tick (−18 dB).

### S20 · Logo row ("funciona em…")
- **Content:**
  - **Platforms only:** macOS · Windows · Linux, as monochrome glyphs or as Inter 600 wordmarks.
  - AI providers may appear **as plain text names** ("Claude, OpenAI ou Grok"). **Never provider logos**, and
    never "powered by" or "em parceria" (facts.md §5.18). No customer logos (§5.1).
- **Style:**
  - Colour ink-2 at 80%. Height 44–52 px, optically balanced rather than equal width. Gap 96 px, centred.
  - Kicker above, for example "FUNCIONA EM".
- **Choreography:**
  - Each item: `y 16 → 0`, `opacity 0 → 1`, `blur 6 → 0`, 14 f with `E.enter`, stagger 4 f, start B−1.
  - Optional glint sweep across the row at +20 f.
- **Hold:** ≥ 45 f.
- **SFX:** one soft sweep (−16 dB).

### S21 · Tabletop wall of screens (3D breadth shot)
- **Purpose:** "it's a whole app". Use it for the proof act or as the bridge into the end card.
- **Setup:**
  - 4–6 real screens (Hoje, Timeline, Revisão, Relatórios, Insights, Foco) on one plane.
  - Plane transform: `perspective 2400px`, `rotateX(55deg) rotateZ(-32deg)`. Screens 900 px wide, gaps 48,
    shadow tier 2.
- **Choreography:**
  - The camera translates diagonally across the plane: 90–120 f, `E.glide`, or linear with 10 f ease ends.
  - To finish, one screen "stands up": it rotates to flat and scales to full frame over 24 f (`E.push`). That
    becomes the next shot through a match cut.
- **No reading required:** this is the only place tilt may exceed 14°.

### S22 · End card (CTA)
- **Layout (centred lockup):**
  - Optional UBI, 260 px tall, left of the wordmark.
  - Wordmark: 170 px.
  - Tagline: **"Retome o controle do seu dia."** in Inter 500, 44 px, ink-2.
  - CTA pill: **"Baixe em ubiqx.com.br"** in Sora 600, 44 px. **`#0a0d16` text on a volt fill** (6.1:1 contrast;
    white on volt is only 3.2:1). Radius 999, padding 22 × 44.
  - Platform row: "macOS · Windows · Linux", Inter 500, 28 px, ink-3.
- **Choreography (C = downbeat, for example 1260):**
  - C: the wordmark lands (scale 1.06 → 1 with `SMOOTH` plus a masked rise).
  - C+8: the tagline enters (masked reveal, S04).
  - C+15: the CTA pill pops (`BOUNCY_SUBTLE`).
  - C+22: the platform row fades in (`E.enter`).
  - Optional C+45: the cursor glides to the pill and clicks on the beat (ripple, S13).
  - Background: the glow orb breathes (scale 1 ↔ 1.05 over 120 f) and the grid fades to 20%.
- **Hold:** ≥ **90 f fully legible** after the last element lands. Total 150–180 f.
- **Ending:** the last frame *is* the end card. **No fade to black** (X can loop; a black tail looks broken).
- **SFX:** final hit at C. CTA click (−12 dB). The music resolves, and its tail ends ≤ the last frame
  (a 15 f audio fade is allowed only if the tail is cut).

---

## 3. Motion system

### 3.1 Spring presets

The timings in this table were **computed at 30 fps and verified against the installed `remotion@4.0.529`**: the
`spring()` values and `measureSpring()` settle frames match exactly.

| Preset | `config` | ζ | 90% at | Peak overshoot | Settles (±0.5%) | Contact lead | Use |
|---|---|---|---|---|---|---|---|
| `SNAPPY` | `{damping: 20, mass: 0.7, stiffness: 220}` | 0.81 | 6 f | 1.4% @ f9 | 13 f | 6 | Text, cards, toasts, UI state, press release |
| `SMOOTH` | `{damping: 200, mass: 1, stiffness: 100}` | ≥1 | 12 f | 0 | 23 f | 12 | Big panels, windows, end-card lockup |
| `SMOOTH_SLOW` | `{damping: 200, mass: 1, stiffness: 60}` | ≥1 | 16 f | 0 | 29 f | 16 | Hero drift, glow orbs, slow floats |
| `BOUNCY_SUBTLE` | `{damping: 15, mass: 0.8, stiffness: 170}` | 0.64 | 6 f | **7.0%** @ f8 | 18 f | 6 | Icons, badges, mascot, CTA pill, bubbles |
| `SLAM` | `{damping: 22, mass: 0.6, stiffness: 400}` | 0.71 | 4 f | 4.2% @ f5 | 9 f | 4 | Word slams, logo contact |
| *(Remotion default)* | `{damping: 10, mass: 1, stiffness: 100}` | 0.50 | 7 f | **16.3%** | 28 f | – | **Never.** It is the "everything bounces" look. |

Notes:
- In Remotion's spring, **any ζ ≥ 1 uses the critically damped solution**. Raising damping above critical does not
  slow the motion; lower stiffness or raise mass instead.
- To make a spring settle exactly on a beat, pass `durationInFrames` (for example 30), which time-stretches it.
- Use `spring({frame: frame - start, fps, config})`, or `Easing.spring(...)` inside `interpolate()` when multiple
  keyframes are needed.

### 3.2 Bézier easings

Values are progress at 10/25/50/75/90% of the duration, computed from the curves.

| Name | `Easing.bezier(...)` | Profile | Use |
|---|---|---|---|
| `E.push` | `0.16, 1, 0.3, 1` | .49 / .83 / .97 / 1 / 1 (expo-out) | Push-ins, reveals, incoming halves of transitions, counters' visual scale |
| `E.enter` | `0.22, 1, 0.36, 1` | .40 / .76 / .96 / 1 / 1 | Masked lines, fades of UI elements, logo row |
| `E.glide` | `0.65, 0, 0.35, 1` | .01 / .07 / .50 / .93 / .99 | Camera travel between two holds, wipes, split divider (span type) |
| `E.whip` | `0.85, 0, 0.15, 1` | .01 / .04 / .50 / .96 / .99, peak velocity 6.7× at mid | Single-curve whip pans. Cut at 0.5. |
| `E.exit` | `0.7, 0, 0.84, 0` | 0 / 0 / .03 / .17 / .51 (expo-in) | Exits, outgoing halves of zoom-through and whip, key press down |
| `E.cursor` | `0.45, 0, 0.15, 1` | .02 / .21 / .79 / .96 / .99 | Cursor travel (quick departure, soft arrival) |
| linear | – | – | **Only** for continuous drifts, grain, orb rotation, typewriter |

`E.push` is 97% complete at half its duration, so the second half is an almost invisible settle. Give it 24–45 f;
the move still reads as quick. **No object motion is ever linear**, except the drifts listed above.

### 3.3 Overshoot limits

| Property | Max overshoot |
|---|---|
| Camera (position, zoom, rotation) | **0%**. Only springs with ζ ≥ 1 or Bézier curves. A camera that bounces makes viewers seasick. |
| Text lines and masked text | 0–2% (`SNAPPY` is 1.4%) |
| UI panels and windows | ≤ 2% scale, ≤ 8 px position |
| Cards | ≤ 3% scale |
| Icons, badges, pills, mascot, speech bubbles | ≤ 8% scale (`BOUNCY_SUBTLE` is 7%), ≤ 3° rotation |
| Slams | ≤ 5% (`SLAM` is 4.2%) |

Anything bouncing at the same time as something else on screen must be the **only** bouncy element: one bouncy
thing per shot.

### 3.4 Stagger intervals

| Group | Interval | Notes |
|---|---|---|
| Letters | 1 f | Rare: wordmark only |
| Words (fluid) | 3 f | Default headline |
| Words (rhythmic) | `eighth(n)` | ≤ 5 words, counted to the music |
| Lines | 5 f | Masked reveals |
| Cards / tiles | 3–4 f (diagonal: row+col) | |
| List rows / timeline blocks | 2 f | |
| Logos | 4 f | |
| Toasts / notifications | 1 beat (readable) or `eighth(n)` (texture) | |
| Keycaps in a combo | `eighth(n)` | |

**Spread cap:** the first-to-last start of any group is **≤ 20 f**, so the group reads as one gesture. If
`N × interval > 20`, shrink the interval.

### 3.5 Motion hierarchy and continuity
- **One hero motion per moment.** At most 2 primary animations may start on the same frame. Secondary elements
  follow 3–8 f later.
- **Always alive:** every shot has at least one element moving: camera drift of at least 1% scale per second, a
  float, or orb drift.
  - Deliberate freezes are allowed only at impacts (2–6 f, via `<Freeze>` or a clamped hold).
  - Background motion is ≤ 0.5 px/f.
- **Direction logic:** forward/progress moves left → right or into depth. Whip direction is consistent across the
  film. Content never enters from the bottom 120 px.
- **Resting crispness:** at rest, text scale is exactly 1 and translations are whole pixels. Fractional transforms
  on static text look soft after X's re-encode.

### 3.6 Hold times: the read rule (PT-BR)

PT-BR copy runs about 20–30% longer than the equivalent English. Netflix caps adult subtitles at 17 characters per
second. Kinetic type also asks the eye to track motion, so this guide uses **15 cps**:

> **`holdFrames ≥ max(24, 2 × characters)`**, where characters include spaces and punctuation. The hold is counted
> **from the frame the last word has landed** (spring ≥ 0.9) to the first frame of its exit. Then **round up so
> the exit lands on a beat**.

| Copy | Chars | Minimum hold | Snapped |
|---|---|---|---|
| "Registra sozinho." | 17 | 34 | 45 |
| "Uma tecla. E ele aprende." | 25 | 50 | 60 |
| "Retome o controle do seu dia." | 29 | 58 | 60 |
| "Os dias não estão menores. O seu tempo é que está vazando." | 58 | 116 | 120. Better: split into 2 cards. |

Additional rules:
- **Word slams:** ≥ 12 f per word. Rate ≤ 1 word per beat. 8th-note rate is allowed only for ≤ 3 words of ≤ 8
  characters each. The final word, or the fully assembled phrase, holds ≥ 30 f.
- **Anything the viewer must remember** (tagline, CTA, domain): ≥ 60 f. The end card: ≥ 90 f.
- **Headline plus subline together:** apply the rule to the **sum** of both.

### 3.7 Motion blur
- **Where:** use it only on whips and zoom-throughs, and on fast pushes above 40 px/f. **Never on text meant to be
  read.**
- **Cheap directional blur** (preferred, since CPU is shared):

```tsx
<svg width={0} height={0} style={{position: 'absolute'}}><filter id="hblur" x="-20%" y="0" width="140%" height="100%">
  <feGaussianBlur stdDeviation={`${b} 0`} /></filter></svg>
// b = clamp(|velocityPxPerFrame| * 0.35, 0, 48); apply filter only when b > 0.5
```

- **Heavy alternative:** `@remotion/motion-blur` `<CameraMotionBlur samples={6} shutterAngle={180}>`, only on
  segments of ≤ 12 f, because cost scales with samples. `<HtmlInCanvasMotionBlur>` exists in 4.0.529 but relies on
  HTML-in-canvas. Test one frame before relying on it with the `swangle` renderer.

### 3.8 Suggested tokens

```ts
// src/design/motion.ts (suggested)
import {Easing} from 'remotion';
export const SPRING = {
  snappy: {damping: 20, mass: 0.7, stiffness: 220},
  smooth: {damping: 200, mass: 1, stiffness: 100},
  smoothSlow: {damping: 200, mass: 1, stiffness: 60},
  bouncySubtle: {damping: 15, mass: 0.8, stiffness: 170},
  slam: {damping: 22, mass: 0.6, stiffness: 400},
} as const;
export const E = {
  push: Easing.bezier(0.16, 1, 0.3, 1),
  enter: Easing.bezier(0.22, 1, 0.36, 1),
  glide: Easing.bezier(0.65, 0, 0.35, 1),
  whip: Easing.bezier(0.85, 0, 0.15, 1),
  exit: Easing.bezier(0.7, 0, 0.84, 0),
  cursor: Easing.bezier(0.45, 0, 0.15, 1),
};
export const clamp = {extrapolateLeft: 'clamp', extrapolateRight: 'clamp'} as const;
```

---

## 4. Typography

### 4.1 Families and loading
- **Sora** (display): weights 600 / 700 / 800. **Inter** (text, UI callouts, numbers in small sizes): 400 / 500 / 600.
- Load the local variable woff2 files in `public/fonts/` (latin + latin-ext) with `@remotion/fonts` `loadFont()` and
  wait on them with `delayRender` before the first frame. The latin subset of each family covers every PT-BR accent
  plus “ ” — … (checked).
- **Only these two families.** Only the weights above. No italics (Sora has none). No third font, not even for code;
  use Inter plus tabular figures.

### 4.2 Type scale at 1080p

| Role | Font | Size (default) | Weight | Tracking | Line-height | Colour | Max |
|---|---|---|---|---|---|---|---|
| Hero slam | Sora | 180–240 (200) | 800 | −0.045em | 0.95 | ink | 1–3 words, ≤ 12 characters/line |
| Display headline | Sora | 104–128 (112) | 700 | −0.035em | 1.02 | ink, emphasis volt | 3–7 words, ≤ 2 lines, ≤ 26 characters/line |
| Headline beside UI | Sora | 72–88 (80) | 700 | −0.03em | 1.06 | ink | ≤ 7 words, ≤ 18 characters/line (column 616 px) |
| Subline | Inter | 36–44 (40) | 400/500 | −0.011em | 1.35 | ink-2 | ≤ 12 words, ≤ 2 lines, max-width 1100 px |
| Kicker / label | Inter | 22–26 (24) | 600 | **+0.14em, UPPERCASE** | 1.2 | volt or ink-3 | 1–3 words |
| UI callout pill | Inter | 26–30 (28) | 600 | 0 | 1.2 | ink | ≤ 4 words |
| Stat number | Sora | 200–280 (240) | 700 | −0.04em | 0.9 | ink / volt | `tabular-nums` |
| Wordmark | Sora | 160–200 | 700 | −0.03em | 1.0 | ink + **X volt** | – |
| Speech bubble | Inter | 28–32 (30) | 500 | −0.005em | 1.3 | ink | ≤ 10 words |
| Fine print | Inter | ≥ 22 | 400 | 0 | 1.3 | ink-3 | – |

**How the characters-per-line budgets were derived:** Sora 700 averages **0.53 em per character** on mixed-case
PT-BR text, before tracking. It averages **0.67 em** on all caps (measured from the Google Fonts TTFs). Budgets
then follow from the measure:
- 112 px with −0.035em tracking ≈ 55 px per character, so a 1440 px measure fits 26 characters.
- 200 px with −0.045em ≈ 97 px per character, so the title-safe width fits about 16 characters (13 in all caps).

Verify each line with `@remotion/layout-utils` `measureText` / `fitText`, and fail the build on overflow.

**Minimum sizes:**
- The message itself (headline or slam) is **≥ 72 px**. A 16:9 video in a portrait phone feed is about 0.2× scale,
  so 72 px shows at roughly 15 pt.
- Anything else meant to be read is ≥ 36 px.
- Fine print is ≥ 22 px, and the story must never depend on it.

**Layout:**
- Centred for slams, S01 and the end card. Left-aligned at x = 144 in module B.
- Headline-to-subline gap 0.35em of the headline size. Kicker-to-headline gap 20 px.

### 4.3 Emphasis (exactly one per card)
1. **Volt word (default).** The emphasis word turns `#4d8dff` as it lands.
2. **Underline sweep.** A bar 0.08em tall, radius 999, volt, top at baseline + 0.12em. It grows `scaleX 0 → 1` from
   the left over 12 f with `E.glide`, starting 2 f after the word lands.
3. **Gradient fill.** `linear-gradient(100deg, #4d8dff, #8fb6ff)` with `background-clip: text`. Hero and wordmark
   only.
4. **Marker highlight.** A `rgba(77,141,255,0.16)` rectangle behind the word, swept from the left (10 f).
5. **Rose strike-through**, for "antes"/problem words only. A 0.07em bar drawn over 10 f.

Never combine two of these, never use bold-versus-regular for emphasis inside a slam, and never use glowing text
(the wordmark is the one exception, §5.4).

### 4.4 PT-BR specifics
- **Accents stay, including on capitals:** VOCÊ, NÃO, RELATÓRIO, AÇÃO. Never strip them in uppercase kickers.
  Masked text needs the padding from S04.
- **No hyphenation:** `hyphens: 'none'`, `wordBreak: 'normal'`, `overflowWrap: 'normal'`. Break lines by hand
  (store copy as `string[]` lines). Use `textWrap: 'balance'` only on sublines.
- **No orphans or widows:** glue short words (a, o, e, de, do, da, em, no, na, um, uma, que, se) to the next word
  with U+00A0. Never leave a single word on the last line.
- **Numbers:**
  - Use `Intl.NumberFormat('pt-BR')`: decimal comma ("3,5"), thousands dot ("1.200").
  - Currency: "R$ 49" (NBSP), and always the full phrase "R$ 197/ano ou 10x de R$ 25".
  - Times: "18h", "18:00", "5h08", "30 min". Never "6pm". Percentages: "80%".
- **Punctuation:** curly quotes “ ”, the em dash — with spaces, and the ellipsis character "…". A final full stop
  on declarative headlines is a deliberate, confident voice; keep it consistent across the film.
- **Voice:** "você"; short verbs; say "IA", not "AI", in running copy. The product name "ubiqX AI" stays as is.
- **Brand casing:**
  - The product is always **ubiqX**: never "UBIQX" and never "Ubiqx". Keep it out of all-caps kickers.
  - The mascot is always **UBI**. The managed service is "IA do Ubi", which is the exact UI spelling.
- **UI words:** use the app's own terms (facts.md §7): Hoje, Timeline, Revisão, Relatórios, Categorias, Score de
  foco, "Confirmar os N", Copiar Markdown.

---

## 5. Colour, depth and texture

### 5.1 Palette roles

| Token | Hex | Role | Budget |
|---|---|---|---|
| canvas | `#0a0d16` | The only base background. **Never pure #000**, which makes gradient banding worse. | – |
| panel / panel-2 | `#111726` / `#172033` | Cards, toasts, window bodies | – |
| line | `#1f2a40` | 1 px borders, grid | – |
| ink / ink-2 / ink-3 | `#e8edf9` / `#a8b4cd` / `#6f7d99` | Text hierarchy | ink-3 is labels ≥ 24 px only |
| volt | `#4d8dff` | Emphasis, actions, focus rings, progress, the X | ≤ 10% of the pixels in a frame |
| ember | `#ff7a1f` | UBI's sash, **one** warm highlight per scene | ≤ 3% |
| rose | `#f2555a` | Problems, "antes", distraction | Problem act and split screen only |
| mint | `#2ecc8f` | Success, checks, "confirmado" | Moments of success only |

At most **two accent hues per shot**; volt counts as one.

**Contrast (WCAG, measured):**

| Text colour | On canvas | On panel | On panel-2 |
|---|---|---|---|
| ink | 16.6 | 15.3 | – |
| ink-2 | 9.3 | 8.6 | – |
| ink-3 | 4.7 | 4.3 | **3.9** (labels only) |
| volt | 6.1 | 5.6 | – |
| rose | 5.8 | – | – |
| mint | 9.4 | – | – |

On a volt fill, use `#0a0d16` text (6.1:1). White (3.2:1) and ink (2.7:1) fail.

### 5.2 Background stack (bottom → top)
1. **Canvas** `#0a0d16`.
2. **Glow orbs.** Pure `radial-gradient`, no CSS `filter: blur` (expensive):
   - Volt orb Ø 1100 px: `radial-gradient(circle, rgba(77,141,255,0.20) 0%, rgba(77,141,255,0.07) 35%, transparent 70%)`.
     Place it behind and above the subject.
   - Ember orb Ø 700 px at 0.06–0.08 alpha: UBI and warm scenes only, in the opposite corner.
   - Orbs drift ≤ 0.3 px/f and breathe ±3% scale on a 120 f sine.
3. **Grid:**
   - 64 px squares, 1 px `line` colour at 35–45% opacity.
   - Mask: `radial-gradient(ellipse at 50% 45%, #000 0–40%, transparent 75%)`.
   - Drifts 0.25 px/f.
   - Floor variant: the grid plane at `rotateX(62deg)` under the hero, fading into the distance.
4. **Content.**
5. **Vignette:** `radial-gradient(ellipse at center, transparent 55%, rgba(0,0,0,0.45) 100%)`.
6. **Film grain:**
   - Pre-render 8 grayscale noise tiles of 256² (numpy: mean 128, σ 40) to `public/brand/grain-0..7.png`.
   - Cycle them every 2 f with a `random(seed)` offset. Normal blend, **opacity 0.035–0.05**.
   - Grain also **dithers the dark gradients**, so X's re-encode does not band them. It is there for that reason,
     not for a "vintage" look.
7. *(Optional)* a light leak (`@remotion/effects/light-leak`, ≥ 4.0.500) over **at most 2** cuts, at ≤ 40% opacity.

### 5.3 Floating UI: shadows, depth, glass

| Tier | Use | `box-shadow` |
|---|---|---|
| 1 low | Chips, pills, keycaps at rest | `0 1px 0 rgba(255,255,255,0.05) inset, 0 8px 20px -6px rgba(0,0,0,0.55)` |
| 2 mid | Cards, toasts, tabletop screens | `0 1px 0 rgba(255,255,255,0.06) inset, 0 0 0 1px rgba(255,255,255,0.06), 0 24px 48px -12px rgba(0,0,0,0.6)` |
| 3 high | Hero window, focused plane | tier 2 + `0 60px 120px -20px rgba(0,0,0,0.55), 0 0 120px rgba(77,141,255,0.12)` |

- The shadow y-offset grows with an element's height above the canvas. A rising element gets a larger, softer
  shadow.
- **Glass:** fake it with a semi-opaque fill (`rgba(23,32,51,0.86)`), a 1 px light border and an inset highlight.
  **Avoid large `backdrop-filter`**: it is slow in headless renders and the machine is shared.
- **Depth of field:**
  - Layers behind the subject get 4–10 px blur.
  - Rack focus is 8–16 px → 0 over 12–18 f with `SMOOTH`.
  - Never blur text the viewer should read in that moment.
- **Radii:** 18 windows, 24 cards, 22 toasts, 14 hotspots, 999 pills. The same element always uses the same radius.

### 5.4 Glow: how much is tasteful
- **One glowing element per frame.**
- Volt glow alpha ≤ 0.35 on UI. Orbs ≤ 0.22 at rest (the logo moment peaks at 0.30). Glow radius ≤ 60 px on UI
  elements, 120 px for the hero bloom.
- Text never glows, except the wordmark: `text-shadow 0 0 40px rgba(77,141,255,0.30)` for at most 1 bar.
- No lens flares, no neon outlines, no rainbow gradients, no chromatic aberration outside a 2 f glitch cut.

### 5.5 Encoding-aware choices (X re-encodes everything)
- Avoid 1 px lines at < 20% contrast on scaled planes: they shimmer. Use ≥ 1.5 px on planes scaled below 1.
- Avoid full-frame fine patterns during fast moves; they turn into macroblock mush.
- Render the master at CRF ≤ 18 with yuv420p (the repo config uses CRF 18), 30 fps CFR, and
  `-movflags +faststart` on the final mux. Remux, don't re-encode.

---

## 6. Transitions

Transitions **straddle** the cut: half before the beat, half after. With `<TransitionSeries>`, remember that
transitions shorten the total length (A + B − D). The simplest approach that keeps cuts beat-exact is:
- one `timeline.ts` with absolute cut frames, all on the grid from §0;
- scenes mounted in `<Sequence from durationInFrames premountFor={30}>`;
- transitions implemented as in/out animations inside each scene.

| # | Transition | Frames | Choreography | Use it for | Budget (48 s) |
|---|---|---|---|---|---|
| **T1** | **Hard cut on the beat** | 0 | Cut on a beat frame. Cut **while the outgoing shot is still moving**; it hides the edit. Keep brightness and direction consistent across the cut. | The default | ≥ 50% of cuts (~16) |
| **T2** | **Whip pan** | 8 (4 + 4) or 10 | **Out:** x 0 → −960 with `E.exit`; blur 0 → 40 px (horizontal, §3.7). **In:** x +960 → 0 with `E.push`; blur 40 → 0. The cut sits at peak velocity. | Sideways moves between sibling features | ≤ 3 |
| **T3** | **Zoom-through** | 12–16 | **Out:** push into a target element: zoom 1 → 6 in log space, `E.exit`, 8 f, until the target's colour fills the frame. **In:** the next scene enters at scale 1.2 → 1 and blur 12 → 0 over 8 f, `E.push`. | Going deeper: logo "X" → product, card → feature | 2 |
| **T4** | **Mask/shape wipe** | 12–18 | A rounded-rect or circle `clipPath` grows from a UI element (usually the one just clicked) to full frame, `E.glide`. The new scene is inside the mask. | "This button opens that": after a click, chapter changes | 2 |
| **T5** | **Match cut on a UI element** | 0 cut + 10–15 morph | Element E has the same rect, radius and colour in A's last frame and B's first frame. After the cut, interpolate E's rect, radius and colour to its B layout with `SNAPPY`. | Keycap "1" → category chip; card → full screen; toast → dashboard widget; tabletop screen → full UI | 3–4 (the most "pro" transition) |
| **T6** | **Blur dissolve** | 10–15 | **A:** blur 0 → 16, opacity 1 → 0, scale 1 → 1.03. **B:** blur 16 → 0, opacity 0 → 1, scale 1.03 → 1. Overlapped. | Tonal shifts: problem → turn, proof → end card | ≤ 2 |
| **T7** | **Flash cut on a hit** | 1 + 3–4 decay | On the hit frame, an overlay of `#e8edf9` at 0.35–0.6 decays to 0 over 3–4 f (`E.push`). Never full white, never 100%. | The drop (logo reveal); optionally one other peak | ≤ 2, never > 3 flashes per second |
| **T8** | **Drop-out (cut to silence)** | 6–15 | Canvas plus grain only, with the music silent. The next frame is the downbeat reveal. | Once, right before the product reveal | 1 |
| T9 | Push slide (card stack) | 12 | A slides out (`E.exit`) as B slides in (`SNAPPY`), 24 px gap visible. `@remotion/transitions/slide` with `springTiming` is acceptable here. | Rapid sequential UI states in the same window | 0–2 |

**Choosing a transition:**
- **Same space, different moment** → T1 or T5.
- **Sibling topic** → T2 or T1.
- **Deeper into something** → T3.
- **Caused by an action** → T4.
- **Change of mood** → T6.
- **The single biggest moment** → T8 then T7.

Never use more than 6 distinct transition types in one film, and never use cube spins, page curls, star or clock
wipes, or glitch as a default.

---

## 7. Sound design

### 7.1 Music bed
- 120 BPM, 4/4. Modern electronic / tech-pop with a clear kick, and a structure that can be cut to: intro, build,
  riser, drop, groove, breakdown, final hit, tail.
- Align the bed with the grid: find the file's first strong downbeat `t0` (onset detection) and set
  `trimBefore = round(t0·30) − targetFrame`, so a downbeat falls on frame 0 and another on frame 300.
- **Problem act "under water":**
  - Pre-render a low-passed copy of the bed with `ffmpeg -af lowpass=f=900` and play it through frames 0–284.
  - Silence at 285–299.
  - The full-range bed from 300 (the drop). A 30 f crossfade into the riser is optional.
- The end: the track's final hit on 1260, and the reverb/decay tail finishes by frame 1439.

### 7.2 Event → SFX map

Levels are in **dB relative to the bed's nominal level**. Alignment follows §0.

| Visual event | SFX | Level | Alignment |
|---|---|---|---|
| Word slam / title slam | Short impact (transient plus low body, 150–400 ms) | +0…+2 | Transient on the contact frame. Duck the bed. |
| Logo reveal on the drop | Sub boom + impact (+ optional reverse swell into it) | +2 | Boom on the drop frame. The reverse swell **ends** there. |
| Riser | Noise/synth riser, 1–2 bars | −24 → −4 crescendo | **Last sample at drop − 1.** It never crosses the downbeat. |
| Whip / zoom-through | Air whoosh, 250–500 ms | −8 | **Loudest point on the cut frame.** Offset the file start by its peak time. |
| Fast push-in | Soft whoosh | −14 | Starts with the move |
| Cursor click | UI click, 30–60 ms | −12 | Click frame |
| UI success / state change | Soft shimmer or chime | −14 | C + 3 |
| Keycap press | Mechanical key down / up | −10 / −18 | Down on contact, up on release |
| Typing | Soft key ticks (4 variants) | −20 | Every 2–3 characters |
| Notification pop | Soft pop / bubble, +1 semitone each | −14 | First frame of the toast |
| Card grid / logo row | **One** sweep or tick cluster for the whole group | −16 | Group onset |
| Counter ticking | Micro tick | −22 | Integer changes, ≥ 3 f apart |
| Counter lands | Ding / shimmer | −12 | Landing frame |
| UBI lands | Bloop / pop (+ a tiny servo whirr) | −12 | Land frame |
| Flash-cut montage | Glitch tick | −16 | Each cut |
| Drop-out | *(silence)* | −∞ | 8–15 f before the drop |
| End card CTA click | Click; music resolves | −12 | On the beat |

**Budget:**
- About ≤ 1 SFX per beat on average. At most 3 SFX overlapping.
- **Not every cut gets a whoosh.** Whooshes on every cut make it feel templated.
- Leave at least 30% of the features act with music only.

### 7.3 Levels and loudness
- **File preparation:** peak-normalise every SFX file to −1 dBFS, 48 kHz, stereo:
  `ffmpeg -i in.wav -af "volume=…" -ar 48000 -ac 2 out.wav`, or measure the peak with `-af astats`.
- **Remotion volumes:**
  - Bed nominal `BED = 0.5` (−6 dB).
  - SFX gain = `BED × 10^(rel/20)`. Examples: click −12 → 0.125; impact +2 → 0.63; whoosh −8 → 0.2.
  - Reference: `db(d) = 10 ** (d/20)`. 0 → 1, −3 → 0.71, −6 → 0.5, −10 → 0.32, −12 → 0.25, −14 → 0.2,
    −18 → 0.126, −20 → 0.1, −24 → 0.063.
- **Master:**
  - Target integrated **−14 LUFS (±1)** and **≤ −1 dBTP**. X publishes no official target; −14 LUFS / −1 dBTP is
    the streaming norm.
  - Short-term loudness: problem act ≈ −18 LUFS, drop and features ≈ −12 LUFS. The dynamic range is what sells
    the reveal.
  - Measure with `ffmpeg -hide_banner -i out/launch.mp4 -af ebur128=peak=true -f null - 2>&1 | tail -14`.
  - Correct with two-pass `loudnorm=I=-14:TP=-1:LRA=11` (`linear=true` on the second pass) while copying video.
  - Encode AAC-LC 48 kHz at 320 kbps.

### 7.4 Ducking (no voice-over, so ducking only makes room for hits)

```ts
const db = (d: number) => Math.pow(10, d / 20);
/** −5 dB duck: 1 f attack, 2 f hold, 10 f release, at each hit frame */
export const duck = (f: number, hits: number[], depth = -5, atk = 1, hold = 2, rel = 10) => {
  let g = 0;
  for (const h of hits) {
    const t = f - h;
    if (t < -atk || t > hold + rel) continue;
    const k = t < 0 ? (t + atk) / atk : t <= hold ? 1 : 1 - (t - hold) / rel;
    g = Math.min(g, depth * k);
  }
  return db(g);
};
// <Audio src={staticFile('audio/music/bed.wav')} volume={(f) => BED * duck(f + audioFrom, HITS)} />
// NB: the volume callback's frame is relative to the <Audio>'s own start; add its `from` offset.
```

- Duck depths: slams −5 dB, the logo drop −6 dB, the end-card final hit −4 dB.
- Never duck for clicks, ticks or pops.

### 7.5 Suggested SFX file names
Map these in `public/audio/sfx/` if the SFX agent chose other names:

`impact.wav`, `sub-boom.wav`, `reverse-swell.wav`, `riser-2bar.wav`, `whoosh-fast.wav`, `whoosh-soft.wav`,
`click.wav`, `key-down.wav`, `key-up.wav`, `type-1..4.wav`, `pop.wav` (plus `pop+1..+5.wav` pitched), `tick.wav`,
`glitch.wav`, `shimmer.wav`, `ding.wav`, `bloop.wav`, `sweep.wav`.

---

## 8. Amateur tells to avoid (with the fix)

| # | Tell | Fix / threshold |
|---|---|---|
| 1 | **Text held too briefly.** Viewers pause to read, or give up. | §3.6: `hold ≥ max(24, 2 × chars)` from landing, snapped to a beat. |
| 2 | **Slow fades.** Opacity-only transitions longer than 12 f, fade-from-black at frame 0, fade-to-black at the end. | Frame 0 is designed. Use T1 or T5 cuts. The end card is the last frame. |
| 3 | **Everything bounces.** Default spring (16% overshoot), bouncing cameras and text. | Use the §3.1 presets and §3.3 limits. One bouncy thing per shot. |
| 4 | **Linear or robotic motion.** Constant-speed slides, a cursor on a ruler-straight line. | Use Bézier curves or springs. Cursor arcs with `E.cursor`. Linear only for drifts. |
| 5 | **Too many fonts or weights.** | Sora + Inter only, at the weights in §4.1. |
| 6 | **Cuts and hits off the beat.** Off by ≥ 2 f; motion **starts** on the beat when it should **contact** on it; audio ahead of video. | Use the accent types in §0. Check every cut frame is on the grid (§9). |
| 7 | **UI too small to read.** A full-app screenshot with 13 px text as the "proof", or upscaled blurry bitmaps. | Effective message text ≥ 36 px. Upsampling ≤ 1.15. High-DPR captures or live UI (S11). |
| 8 | **Cursor teleports, is giant, or clicks without feedback.** | S13: enter from off-frame, arc path, arrive 3–4 f early, hover state, press, ripple, then state change. |
| 9 | **Over-glow.** Bloom on everything, neon outlines, lens flares. | §5.4: one glowing element per frame, alpha ≤ 0.35. |
| 10 | **Unreadable contrast.** ink-3 for key lines, text over busy UI with no scrim. | Message text is ink or ink-2 only. Use a 60% canvas scrim or focus dim (S12) under text over UI. |
| 11 | **Generic stock look.** Stock people typing, abstract 3D loops, clip-art icons, template transitions, library music with no hit points. | Real UI and the real mascot, one icon style (2 px line), music cut to the grid. |
| 12 | **SFX overkill.** A whoosh on every cut, SFX louder than the music, clipping, no loudness pass. | §7.2 budget, §7.3 levels, −14 LUFS / −1 dBTP. |
| 13 | **Slideshow.** A still screenshot held > 1 s with no motion. | §3.5: always alive (drift, float, push). |
| 14 | **Uniform rhythm.** Every shot the same length, every group staggered the same way, no quiet before the drop. | Vary shot lengths (8–120 f), alternate module A/B, use a drop-out (T8). |
| 15 | **Feature laundry list.** 8+ features at 1 s each, or 10-word headlines. | ≤ 6 modules, ≤ 7 words per headline, one benefit per module. |
| 16 | **A zoo of transitions.** Every transition different; cube spins, star wipes. | §6: ≤ 6 types, T1/T5 dominant. |
| 17 | **Jittery or fake numbers.** Proportional digits dancing, counters landing off-beat, invented stats. | `tabular-nums`, landing on a downbeat, values only from facts.md §4. |
| 18 | **Inconsistent UI craft.** Different radii and shadows for the same element, misaligned margins, data that changes between shots of the same screen. | §5.3 radii and tiers; one demo dataset across all captures. |
| 19 | **Blurry text at rest.** Motion blur or blur filters left on readable text; fractional scale while it rests. | §3.5 resting crispness; no blur on text meant to be read. |
| 20 | **Safe-area violations.** Text under X's player controls, logos hugging the edges. | §0 safe areas. |
| 21 | **Flashing.** More than 3 flashes per second, pure-white frames. | T7 limits; chaos montage keeps a dark base. |
| 22 | **Language slips.** Missing accents ("Voce", "Relatorio"), anglicisms where the UI has a PT term, "5.5h", "$49", "UBIQX". | §4.4 plus the facts.md §7 glossary. |
| 23 | **Fake product.** UI in the video that doesn't exist in the app, or forbidden claims (grátis, 100% offline, provider logos). | Capture the real app; check facts.md §5 before locking copy. |
| 24 | **Weak ending.** CTA on screen < 2 s, no URL, fade-out over the CTA, music that ends before the picture or trails after it. | S22: ≥ 90 f legible hold; URL in the pill; the audio tail ends ≤ the last frame. |

---

## 9. Pre-render QA checklist (automate what you can)

**Timing**
- [ ] Every cut frame is on the grid: `f % 15 === 0`, or `f` is in `eighth()` / `sixteenth()` inside a montage.
- [ ] Every contact frame (slam, click, key, counter land) is on a beat.
- [ ] Every text card passes the §3.6 hold rule. Write a script over the copy table.
- [ ] No group stagger spread exceeds 20 f. No shot shorter than 8 f outside the montage, or longer than 120 f
      apart from the end card.

**Picture**
- [ ] Frame 0 renders a legible, designed frame (`npx remotion still … --frame=0`).
- [ ] Contact sheet: render stills every 30 f and check the safe areas, the one-glow rule and the accent budget.
- [ ] Zoomed UI shots: upsampling ≤ 1.15, message text ≥ 36 px effective.
- [ ] Fonts load; there are no fallback glyphs (arrows and ⌘ are SVG).
- [ ] No `Math.random`, no CSS animation, no `backdrop-filter` on planes larger than 600 px.

**Copy**
- [ ] Every on-screen string appears in, or is derived from, facts.md, and none hits a §5 forbidden claim.
- [ ] Accents, NBSPs, PT number formats and brand casing are correct.

**Sound and delivery**
- [ ] Loudness −14 ±1 LUFS integrated, ≤ −1 dBTP (ebur128). No SFX lands before its visual event.
- [ ] Master: H.264 High, yuv420p, 1920×1080, 30 fps CFR, AAC 48 kHz 320 kbps, `+faststart`, < 512 MB.
- [ ] Watch it once **muted**: the story must still be complete.

---

## 10. Genre reference notes (what the well-known films do)

These characterisations come from craft knowledge of the published launch films; they were **not re-watched frame
by frame for this brief** (x.com and the video hosts are blocked from this environment). Use them as flavour, not
as specs.
- **Linear:**
  - Near-black canvas with a soft coloured glow.
  - UI shown on a slight 3D tilt that settles flat.
  - Restrained easing with no bounce, crisp Inter-style display type.
  - Long, slow push-ins and minimal SFX.
- **Raycast:**
  - Keyboard-first: keycap close-ups and the command bar appearing instantly.
  - Continuous panning that never breaks frame, from input to result.
  - Jump-cuts that skip loading states.
  - Concentrated ripple click feedback.
  - Warm red/orange gradient accents on dark chrome.
- **Arc (The Browser Company):**
  - More playful, human and colourful.
  - Speed-ramped task footage (tab sorting shown at around 3× speed).
  - Bold typography.
- **Cursor:** dark editor, ghost-text completions appearing, tight zooms on code, minimal chrome and music.
- **Framer:** bold kinetic type slams cut hard to music, big 3D canvas rotations, fast cuts.
- **Vercel:** black and white, geometric grids and glowing lines, abstract geometry standing in for invisible
  back-end work.
- **Notion:** illustration-led; hard cuts between distinct UI windows.
- **Superhuman:** speed via keyboard shortcuts, keycaps, and before/after inbox counts.
- **Supabase launch weeks:** dark canvas with a single brand accent (green), terminal/code textures, 3D ticket and
  typographic motifs, one video per day.
- **Cal.com:** monochrome, clean UI, cursor-driven demos.

**What they share, and what this guide encodes:**
- Dark canvas, one accent.
- Real UI as the hero, framed in 3D and zoomed until readable.
- Cursor and keyboard realism.
- Cuts on the beat, with a silence/drop at the reveal.
- 40–90 s long, ending on a clean lockup with a URL.

---

## 11. Sources and research notes

**Network reality in this environment:**
- **Worked:** WebSearch (result summaries), GitHub clones, the npm registry, Google Fonts.
- **Blocked by the egress proxy when fetching:** x.com, motion.so, advids.co, designer-daily.com, remotion.dev,
  partnerhelp.netflixstudios.com, launchlibrary.xyz.
- Claims from blocked sites come from search-result summaries only.

**Read or cloned directly:**
- `github.com/remotion-dev/skills` (Remotion 4.0.529 rules: `timing.md` for `Easing.bezier(0.16,1,0.3,1)`, spring
  easing and perceptual-scale; `transitions.md` for `TransitionSeries` overlap maths; `motion-blur.md`, `sfx.md`,
  `light-leaks.md`, `effects.md`, `multi-scene-video.md`).
- `github.com/broomva/skills` → `skills/video/launch-video/SKILL.md`: the "liquid glass" launch-video recipe, with
  `perspective(1200px) rotateY(-8deg) rotateX(5deg)` panels, ≤ 8 words per title card, and spring
  `{damping 15, stiffness 80, mass 0.8}`.
- `github.com/ThamJiaHe/claude-code-handbook` → `docs/motion-graphics-claude-remotion-guide.md`: word stagger
  8 f, card stagger 10 f, spring `{damping 12, stiffness 200, mass 0.5}`, titles 72–90 px.
- `github.com/naveen-annam/creativly.ai-brand-video-remotion` README: 17 scenes, 13 transitions, presets
  "smooth" `{damping 200}` and "snappy" `{damping 14, stiffness 120, mass 0.4}`.

**Via search summaries:**
- advids.co, "SaaS Feature Launch Video Prototypes": Linear's colour-inversion click versus Raycast's centre
  ripple; Raycast's continuous panning versus Notion's hard cuts; Arc's 300% speed-up; Vercel and Stripe's
  abstract geometry for back-end work; 4 of 7 films using rapid ease-in micro-zooms.
- SmoothCapture blog: slow zoom to introduce, punch-in to emphasise a click, magnifier for tiny controls. Cut
  loading pauses and cursor wandering; don't show software from one fixed angle; sell one outcome.
- Roy Lee on X, cinematic launch-video tips: a scroll-stopping first frame; 45 s–1:30; very dynamic shots with
  something new every 2–3 s; a clear "introducing" caption; not corporate.
- Krotos, BOOM Library, add.app and Derek Lieu on trailer mixing: layered impacts, sidechain ducking under hits,
  and not putting a whoosh on every cut.
- Loudness references (Dan Murtagh, ClickyApps, OpusClip): −14 LUFS / −1 dBTP streaming norm; X has no
  published target.
- Netflix Timed Text Style Guide via summaries: 17 cps adult reading speed, 42 characters per line, minimum event
  5/6 s. This informed the stricter 15 cps rule in §3.6.
- Raylight (raylight.app): product-video tool built around "lean a flat UI screenshot back into the frame" plus
  shallow depth of field and bloom. It confirms the 3D-tilt look.

**Computed here (reproducible):**
- The spring table: a Python replica of Remotion's spring integrator at 30 fps, cross-checked with `spring()` and
  `measureSpring()` from the project's own `node_modules/remotion` (4.0.529). Identical values.
- The Bézier progress profiles.
- WCAG contrast of the brand palette.
- Sora and Inter metrics from Google Fonts TTFs: Sora 700 averages 0.53 em per character on PT text and 0.67 em
  in caps; both families have `tnum`; PT accents are present.
- The local `public/fonts` files: the Sora latin subset has no ↑ ↓ → ⌘; Inter lacks → ⌘.

**Product constraints:** `brief/facts.md` (claims, approved copy, UI glossary, forbidden claims).
