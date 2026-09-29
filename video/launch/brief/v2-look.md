# v2 pass: brighter, bigger, fuller, deeper

The v1 cut (`out/launch-v1-master.mp4`) has the right structure, copy, timing and music. v2 keeps all of that:
the same 18 scenes on the same frames, the same copy and the same soundtrack. What changes is **how the film
looks on a phone in the X feed**. Every rule below is a number that can be measured.

`brief/facts.md` still decides what may be claimed. `brief/storyboard.json` still decides timing and copy. If
this file conflicts with `style.md` on **looks** (sizes, brightness, dimming), this file wins.

## 1. What is wrong with v1 (measured)

| Problem | Measurement on v1 |
|---|---|
| Too dark | Film mean luma **0.087** (0–1, from a 192×108 grey downscale). Scene means range from 0.05 to 0.10. Fewer than 4 % of pixels are brighter than 0.3. On a phone it reads as near-black mud, and X's re-encode bands the dark gradients. |
| Too small | Headlines are set at about 60–90 px, the CTA label at about 48 px and the OS row at about 36 px. UI text is 16–24 px on the canvas. In the feed a 1920-wide frame is shown about 390 px wide (×0.2), so a 64 px headline becomes 13 px and the OS row becomes 7 px. |
| Empty | s06 has a dead band from y 170 to 450 between the headline and the window. s05 holds the lockup for 4 s with little change. The right halves of s17 and s01's timesheet are dark and empty. |
| Flat and dim | Spotlights dim the whole frame to 62 % black, so the UI we are selling is mostly shown darkened. Windows are flat, with no rim light and weak separation from the stage. |

## 2. Targets for v2

Measure them with `python3 tools/look-metrics.py <video.mp4 | stills dir>`.

**Brightness (per scene, luma 0–1 on a 192×108 grey downscale):**
- Scene mean **≥ 0.14**. The film mean must be **≥ 0.16**. s04 (the designed silence) is exempt; it only needs ≥ 0.05.
- Fraction of pixels above 0.3 must be **≥ 0.08** in every scene except s04.
- Keep real darks so the image does not go milky: the 10th percentile must stay **≤ 0.12**.

**Type minimums (CSS font-size in composition px at 1920×1080):**

| Element | Minimum size |
|---|---|
| One- or two-word slam | ≥ 170 |
| Primary headline, one line | ≥ 112 |
| Primary headline, two lines | ≥ 100 per line |
| Secondary line or tagline | ≥ 64 |
| Kicker, label, caption, chip text | ≥ 40 |
| End-card CTA pill label ("Baixe em ubiqx.com.br") | ≥ 72 |
| End-card OS row | ≥ 52 |
| UI text the viewer is meant to read (a row title, a button, a pill) | ≥ 34 px on the canvas; zoom the camera until it is |

UI text that is only texture may stay small, but it must not be the subject of the shot.

**Phone test:** downscale every hold frame to 480×270. The headline and the thing the scene is about must be
readable there. `tools/look-metrics.py --phone` writes these downscales.

**Fill:** no empty band larger than about 25 % of the frame height during a hold, unless the emptiness is the
point of the shot (s04, and the s10 key alone in the dark).

## 3. The look

**Stage (every backdrop).**
- Navy, not black. The base is a vertical gradient from #0f1730 at the top to #0a0f20 at the bottom.
- A large volt key-light pool sits behind the subject of the shot at alpha 0.30–0.45. Secondary pools are
  indigo #6c5cff (a light colour only, never used for UI or type) and ember, at 0.10–0.20.
- A slow aurora or light sweep keeps the stage alive.
- Vignette ≤ 0.35. Grain stays at 0.035–0.05, because it is what prevents banding.

**Key light.** A soft white-blue radial (#cfe0ff at 0.10–0.18, `screen`) sits behind the main subject, whether a
window, a card or the headline, so it separates from the stage.

**UI windows (`<Screen>`).**
- Grade the whole window content layer, bitmap **and** image-space overlays together so patches still match:
  brightness 1.12–1.2, contrast 1.05, saturate 1.15. This is a `grade` prop, on by default.
- Rim light: a 1.5 px gradient border, volt at about 0.7 on the top-left fading to transparent, plus a white top
  highlight.
- Stronger ambient glow under the window (about 0.35) and a deeper, larger drop shadow.

**Spotlights.** The default dim goes from 0.62 to **0.38**, with a navy tint (rgba(10, 16, 36, …)) instead of
black. The outline glow gets stronger. The goal is "the rest steps back", not "the lights go out".

**Depth: lift cards.** When a shot is about one UI element (the 88 score ring, a review row, the "Confirmar os 19"
button, the provider pills, the redaction chip), lift it out of the screenshot as a floating card:
- A crop of the @3x twin when one exists, at 1.2–1.6×.
- A drop shadow, a rim light, 6–14° of tilt, and parallax against the window behind it.
- Use `<LiftCard>` (`src/components/LiftCard.tsx`).

**Headlines.**
- Sora 700–800, tracking −0.04em, white with a slight top-to-bottom gradient. The highlighted word is a volt
  gradient with a soft glow.
- When a headline sits over UI, give it a band with a scrim so it reads. Headline and subject share the frame; do
  not leave a gap between them.

**Energy.** Any hold longer than 45 frames needs secondary motion: parallax drift, a light sweep, a floating card,
particles or a counter. Nothing sits still for 1.5 s.

## 4. Unchanged guards

- Frames, scene order, copy and music stay as in `storyboard.json`.
- SFX cues may be added from `public/audio/sfx/` for new visual events. Follow the style.md §7 sync rule: the
  transient lands on the event frame or 1 frame after it, never before.
- No real personal data. Only the fictional data from the captures (Ana, ana@example.com, example/ubiqx,
  12345.000042/2026-00).
- No AI-provider logos; provider names appear as text only.
- Never show og.png.
- The CTA is "Baixe em ubiqx.com.br". No purchase verb anywhere ("Assine", "Compre").
- No legible "1–9" / "1 a 9" key legends, no legible model ids, and no demo numbers presented as results.
- **New in v2: no legible price anywhere outside the end card**, and no legible count that contradicts the scene's
  number.
- Frame 0 stays a designed, legible poster.
