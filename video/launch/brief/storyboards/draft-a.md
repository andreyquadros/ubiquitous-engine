# ubiqX AI — “Onde foi parar o seu dia?” (launch film, draft A)

> Machine-readable twin: `draft-a.json` (same data; this file is generated from it). Angle: A — problem → reveal: a one-second visceral hook (the workday drawn as a bar that drains), two quick pains (context switching, Friday timesheet from memory), a caret alone in silence, the logo on the drop, then the core loop on the real UI (registra → classifica → revisa → entrega), proof (your AI or IA do Ubi, privacy, 3 systems) and the end card.

**1500 frames = 50.0 s = 25 bars** at 1920×1080, 30 fps, 120 BPM (beat 15 f, bar 60 f). 16 scenes. On-screen language PT-BR, no voice-over, built to work muted.

## The idea in one breath

The film opens on the viewer's own workday drawn as a full volt bar with a clock at **08:00**. In one second the clock races to **18:00** and the bar drains: the day leaks out as gray fragments while the question is already on screen: *“Onde foi parar o seu dia?”* Two quick pains follow (window chaos where *“Só um minuto.”* becomes **40 min**, then *“Na sexta: planilha de memória.”*). Then a single caret blinks in total silence. The **logo slams on the drop**, and the camera flies through its X into the app's own X and pulls back to the real Hoje screen. From there the film walks the real loop on the real UI: the **same 08:00→18:00 clock now fills the timeline** with categories (the rhyme with the hook), three words land on three rows (*Regras, memória, IA.*), the review queue holds only the doubts, **one key “1”** flies into the app's badge, **one click** turns into a column of mint “você” badges while UBI nods, and at **18:00 the report is already written**. Proof: Claude, OpenAI or Grok, or the IA do Ubi; an address masked into the exact `[email]` token; macOS · Windows · Linux. End card with the tagline and *Baixe em ubiqx.com.br*.

## Beat sheet

| # | Scene | Frames (abs) | Time | Starts (bar.beat) | Act | On-screen copy | Out |
|---|---|---|---|---|---|---|---|
| 1 | `s01-hook-o-dia-vaza` | 0–59 (60 f) | 00:00.00–00:02.00 | 1.1 | hook | Onde foi parar o seu dia? / 18:00 | cut |
| 2 | `s02-so-um-minuto` | 60–149 (90 f) | 00:02.00–00:05.00 | 2.1 | problem | “Só um minuto.” / 40 min | cut |
| 3 | `s03-planilha-de-memoria` | 150–224 (75 f) | 00:05.00–00:07.50 | 3.3 | problem | Na sexta: planilha de memória. | cut |
| 4 | `s04-silencio` | 225–239 (15 f) | 00:07.50–00:08.00 | 4.4 | problem | — | flash |
| 5 | `s05-logo-slam` | 240–299 (60 f) | 00:08.00–00:10.00 | 5.1 | reveal | ubiqX AI | zoom-through |
| 6 | `s06-hero-do-x-ao-app` | 300–419 (120 f) | 00:10.00–00:14.00 | 6.1 | reveal | Controle de tempo automático com IA. | cut |
| 7 | `s07-ele-registra` | 420–539 (120 f) | 00:14.00–00:18.00 | 8.1 | features | Ele registra. Você trabalha. / 18:00 | whip-left |
| 8 | `s08-regras-memoria-ia` | 540–629 (90 f) | 00:18.00–00:21.00 | 10.1 | features | Regras, memória, IA. | cut |
| 9 | `s09-so-as-duvidas` | 630–719 (90 f) | 00:21.00–00:24.00 | 11.3 | features | Só pergunta o que não sabe. | cut |
| 10 | `s10-uma-tecla` | 720–809 (90 f) | 00:24.00–00:27.00 | 13.1 | features | Uma tecla. E ele aprende. / 1 | cut |
| 11 | `s11-um-clique-vira-memoria` | 810–929 (120 f) | 00:27.00–00:31.00 | 14.3 | features | Um clique vira memória. | cut |
| 12 | `s12-18h-relatorio-pronto` | 930–1049 (120 f) | 00:31.00–00:35.00 | 16.3 | features | 18:00 / O relatório sai pronto. | whip-left |
| 13 | `s13-voce-escolhe-a-ia` | 1050–1169 (120 f) | 00:35.00–00:39.00 | 18.3 | proof | Claude, OpenAI ou Grok. / Ou deixe com o Ubi. | cut |
| 14 | `s14-privado-por-padrao` | 1170–1259 (90 f) | 00:39.00–00:42.00 | 20.3 | proof | Privado por padrão. / ana@example.com / [email] / IA | cut |
| 15 | `s15-tres-sistemas` | 1260–1319 (60 f) | 00:42.00–00:44.00 | 22.1 | proof | macOS · Windows · Linux | cut |
| 16 | `s16-end-card` | 1320–1499 (180 f) | 00:44.00–00:50.00 | 23.1 | cta | ubiqX AI / Retome o controle do seu dia. / Baixe em ubiqx.com.br / macOS · Windows · Linux | cut |

## Music map

- **Key:** A minor (relative C major) — the SFX library's pitched sounds are C-major pentatonic / A4 riser ends / C6–E6 dings, all diatonic here
- **Tempo:** 120 BPM, 4/4, beat = 15 f, bar = 60 f; downbeat of bar 1 on frame 0
- **Style:** Modern electronic / tech-pop: tight four-on-the-floor kick, round sub, plucked arp motif A–C–E–G, airy pads; no vocals. Mix so it works muted-first: the picture tells the story, the music sells the hits.
- **Loudness:** Master −14 LUFS integrated (±1), ≤ −1 dBTP. Short-term: problem act ≈ −18 LUFS (filtered), drop and features ≈ −12 LUFS — the dynamic range sells the reveal.
- **Stems:** Deliver: full mix, a low-passed copy of bars 1–4 (or the filter automated in the mix), and the music without the drop crash (in case the SFX stack is enough).

| Section | Bars | Frames (abs) | Energy | Feel |
|---|---|---|---|---|
| Vazamento (intro under water) | 1–4 | 0–239 | 2 | Whole section through a ~900 Hz low-pass (render a filtered stem). Filtered kick on 1 and 3, a dry clock-tick hi-hat motif in 16ths (the time leaking), Am9 pad, sub on A1. Bars 3–4: low-pass opens 900 → 2.5 kHz, snare roll 8ths from abs 180, 16ths from abs 210. Stops dead at abs 225 — no reverb tail, no pickup. |
| Drop — revelação | 5–7 | 240–419 | 5 | Full range from abs 240: kick, sub, clap on 2 & 4, open hats, wide supersaw Am chord stab; the loudest bar of the film, then settling into energy 4 by bar 7 as the lead motif enters. |
| Groove A — captura e classifica | 8–11 | 420–659 | 4 | Main groove: bass 8ths, plucked arp, claps. Leave room (no melodic notes) on the frames where SFX hit. |
| Groove B — revisão | 12–15 | 660–899 | 4 | Groove variation: counter-melody + percussion layer; builds to the biggest features-act hit on abs 840. |
| Respiro — 18:00 | 16–18 | 900–1079 | 3 | The breather before the proof: drums thin to kick-on-1 + shaker, pad opens, bell motif. Calmer motion on screen (UBI nods, the report reveal). |
| Prova — build | 19–22 | 1080–1319 | 4 | Groove returns lighter, builds over bars 21–22 into the final hit. |
| Final — end card | 23–25 | 1320–1499 | 5 | Final hit, then no more drums: one sustained C(add9) chord with a slow pluck echo, decaying; the tail closes before the last frame. |

**Events the edit needs (abs frames):**

| Frame | Time | Bar.beat | Type | Note |
|---|---|---|---|---|
| 0 | 00:00.00 | 1.1 | downbeat-hit | First downbeat ON frame 0 (poster frame): filtered kick + sub A1 + muted Am9 stab. No silent lead-in. |
| 30 | 00:01.00 | 1.3 | stinger | Clock lands 18:00 in s01: tick motif stops for one beat; the downlifter SFX takes over. |
| 60 | 00:02.00 | 2.1 | downbeat-hit | Cut to s02 (chaos chips). |
| 75 | 00:02.50 | 2.2 | stinger | Slam '“Só um minuto.”' on beat 2: muted bass stab; bed ducked −5 dB (impact). |
| 120 | 00:04.00 | 3.1 | downbeat-hit | Counter lands on '40 min': low piano A1+E2. |
| 120 | 00:04.00 | 3.1 | riser-start | SFX riser-2bar enters (natural end would be abs 240); it is hard-muted at abs 225 with the bed. |
| 150 | 00:05.00 | 3.3 | filter-sweep | Low-pass opens over bars 3–4 while the timesheet scene plays; snare build. |
| 225 | 00:07.50 | 4.4 | stop | Hard stop of bed AND riser (2-f fade max) — the cut to the lone caret. |
| 225 | 00:07.50 | 4.4 | silence | Pre-reveal silence abs 225–239 (15 f, 0.5 s). Nothing plays. |
| 240 | 00:08.00 | 5.1 | drop | Logo slam: the hardest downbeat (crash + sub). SFX impact_deep_2 + sub-boom stack on it; bed duck −6 dB. |
| 300 | 00:10.00 | 6.1 | downbeat-hit | Zoom-through into the product: crash + chord change to F; the whoosh peaks on this frame. |
| 360 | 00:12.00 | 7.1 | downbeat-hit | Lead pluck motif A4–C5–E5 enters as the hero window floats. |
| 420 | 00:14.00 | 8.1 | downbeat-hit | Headline 'Ele registra. Você trabalha.' (cut hidden by the same framing). |
| 450 | 00:15.00 | 8.3 | filter-sweep | A 1-bar rising hi-pass sweep follows the timeline wipe (450→510). |
| 510 | 00:17.00 | 9.3 | stinger | Pluck C6 with the 18:00 landing (ding_2 = C6). |
| 540 | 00:18.00 | 10.1 | downbeat-hit | Whip cut into 'Regras, memória, IA.' |
| 555 | 00:18.50 | 10.2 | stinger | Pluck A4 — 'Regras,' |
| 570 | 00:19.00 | 10.3 | stinger | Pluck C5 — 'memória,' |
| 585 | 00:19.50 | 10.4 | stinger | Pluck E5 — 'IA.' |
| 660 | 00:22.00 | 12.1 | downbeat-hit | Variation starts (the review queue). |
| 720 | 00:24.00 | 13.1 | downbeat-hit | Cut to the keycap. |
| 735 | 00:24.50 | 13.2 | stop | 1-beat drum drop abs 735–749 (bass + pad only) so the key-down SFX sounds alone. |
| 750 | 00:25.00 | 13.3 | stinger | Drums re-enter on the match cut (short reverse cymbal ending exactly on 750). |
| 780 | 00:26.00 | 14.1 | downbeat-hit | Rule chips appear. |
| 840 | 00:28.00 | 15.1 | downbeat-hit | 'Confirmar os 19' click: crash + chord lift Am→C — peak of the features act. |
| 900 | 00:30.00 | 16.1 | filter-sweep | Drums thin out; pad swells (UBI nodding). |
| 945 | 00:31.50 | 16.4 | stinger | Bell E6 with the 18:00 landing (ding_1 = E6). |
| 960 | 00:32.00 | 17.1 | downbeat-hit | Soft hit: the report window rises. |
| 1020 | 00:34.00 | 18.1 | downbeat-hit | Pluck with the 'Copiar Markdown' click. |
| 1050 | 00:35.00 | 18.3 | stinger | Whip into the proof act (beat 3); a 1-beat snare pickup 1035–1049 is welcome. |
| 1080 | 00:36.00 | 19.1 | downbeat-hit | Hover beats on the provider pills (1065/1080/1095). |
| 1140 | 00:38.00 | 20.1 | downbeat-hit | 'IA do Ubi' click: full drums back. |
| 1200 | 00:40.00 | 21.1 | downbeat-hit | Mask contact ana@example.com → [email]. |
| 1260 | 00:42.00 | 22.1 | riser-start | SFX riser_1bar_1 (ends on A4 at 1320) + snare 16ths build under the platforms. |
| 1320 | 00:44.00 | 23.1 | downbeat-hit | Final hit (kick + sub + crash), resolving the A-minor film to C major; bed duck −4 dB. |
| 1380 | 00:46.00 | 24.1 | stinger | Soft pluck with the CTA click. |
| 1440 | 00:48.00 | 25.1 | filter-sweep | Tail: low-pass closes, reverb decays. |
| 1499 | 00:49.97 | 25.4+14 | stop | Last audible sample ≤ frame 1499 (no fade needed); the picture holds the end card to the end. |

**Ducking:** abs 75 -5 dB (slam), abs 240 -6 dB (logo drop), abs 1320 -4 dB (final hit) — 1 f attack, 2 f hold, 10 f release (style §7.4).

## Conventions

- **frames:** Scene-relative frames unless marked 'abs'. Every scene start is a multiple of 15 (a beat); the drop (240), the features-act clicks (660, 780, 840, 1020, 1140, 1200) and the final hit (1320) are downbeats (multiples of 60).
- **transitions:** Not overlapping: a transition is split into an outgoing half inside the end of scene N and an incoming half inside the start of scene N+1, with the same type in N.transitionOut and N+1.transitionIn; 'frames' = the frames of that half inside that scene (0 = the half is empty). The cut frame is the boundary.
- **camera:** Screen primitive with width 1440 for 2880-px captures; 'zoom' is the Screen camera zoom, effective bitmap scale s = zoom × 0.5; on-screen UI text = capture text px × s (capture text px measured from the PNGs: cap/descender height ÷ 0.73–0.95). 'atFrame' = the frame the camera ARRIVES; 'duration' = travel frames before it (default 24). 'anchor' = where the target lands on the canvas (default centre). 'focus' = explicit image point when the hotspot's own centre is not the aim.
- **cursor:** 'atFrame' = arrival frame; a key with click:true clicks on that frame (arrival 3–4 f earlier on the previous key). 'to' is a hotspot of the scene's capture, or 'rest:x,y' in composition px for fade-in/rest points.
- **sfx:** 'atFrame' = the frame the HIT (transient / loudest point / hit_offset) must land, relative to the scene; anchor 'start' marks risers placed by their start (their end_is_downbeat lands on the next downbeat). Cues that straddle a cut (whooshes, whips, risers) are mixed on the master audio track, not inside the scene's <Sequence> (which would truncate them). gainDb is relative to the bed's nominal level (BED = 0.5).
- **reading:** hold ≥ max(24, 2 × chars) from the landing frame (last word at spring ≥ 0.9) to the exit frame, exits snapped to beats. Lines shown together are also checked on their sum from the later landing; sequential cards (one replaces the other) are checked individually. UI bitmap text is not 'copy' — its legibility is covered by the ≥ 36 px zoom rule in camera.uiTextOnScreen.
- **typography:** Sora 600/700/800 + Inter 400/500/600 only; PT-BR accents kept; NBSP glue shown as U+00A0 in the copy strings; brand casing ubiqX / UBI / IA do Ubi.

## Scenes

### 1. `s01-hook-o-dia-vaza` — abs 0–59 (60 f, 00:00.00–00:02.00) · hook

**Purpose.** Stop the scroll: the viewer's workday, drawn as a full volt bar, drains to gray in one second while the question is already on screen.

**Shot types.** S01 cold-open hook frame, S18 data build (inverted: the bar drains), S07 number counter (clock readout)

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Onde foi parar o⍽seu dia? | headline | static | 0 → 0 → 60 | dia | Set on frame 0 (poster, S01). Sora 700 112 px, ink; 'dia' volt from frame 0. NBSP in 'o seu'. |
| 18:00 | label | static | 0 → 30 → 60 | — | Clock readout riding the playhead: reads 08:00 on frame 0, counts in 10-min steps, lands 18:00 on f30. Inter 600 44 px tabular-nums in a panel-2 pill. |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Background variant 'plain' (canvas #0a0d16, one faint volt orb top-centre at 0.10, vignette 0.6, grain 0.045). Headline centred, baseline y≈420. Day bar centred at y=640: capsule 1440×40 px (x 240–1680), radius 20, 1.5 px #1f2a40 border, FULL volt fill on frame 0 (a whole day ahead). Playhead = 3 px volt line 72 px tall + 10 px dot at the bar's left end; clock pill above it reads '08:00'. No UI, no brand, no rose yet. Frame 0 (poster/thumbnail) = headline + full volt bar + '08:00' pill, fully set and legible.

**Assets.**

- `none` `vector`. Bar, playhead, clock pill and fragments are drawn in the composition (tokens: volt, line, panel2).

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, E.glide. Whole-frame camera scale 1.00→1.04 over 60 f (S01); background parallax x 0→−24 px.
- f59 → `full` zoom 1.04, tilt rx 0° ry 0°, E.glide

**Motion.** f0: everything set (poster). f0→30 (span, E.glide): playhead travels x 240→1680 while the clock counts 08:00→18:00 (minutes 480→1080 via interpolate, rounded to 10, tabular) and lands '18:00' on f30 (beat 3) with a 1→1.05→1 pulse on the pill (BOUNCY_SUBTLE, the only bouncy element). Behind the playhead the volt fill does not stay: it splits into 14 segments (random('leak-i') widths 40–140 px) that desaturate volt→#3a4560 over 6 f, detach and drift up 60–180 px with ±8° rotation, fading to 0 over 24 f (E.exit), 2 f stagger following the playhead — the day leaks out of the bar. By f36 the bar is an empty outline. f36–59: last fragments drift ≤0.5 px/f, headline holds crisp. f60: hard cut while fragments still move.

**Transitions.** In: cut (0 f) — Frame 0 is the designed poster; no fade-in.. Out: cut (0 f) — T1 hard cut on the downbeat (abs 60)..

**SFX** (hit frame rel / abs, gain vs bed).

- f3 (abs 3) `tick.wav` -22 dB
- f6 (abs 6) `tick.wav` -22 dB
- f9 (abs 9) `tick.wav` -22 dB
- f12 (abs 12) `tick.wav` -22 dB
- f15 (abs 15) `tick.wav` -22 dB
- f18 (abs 18) `tick.wav` -22 dB
- f21 (abs 21) `tick.wav` -22 dB
- f24 (abs 24) `tick.wav` -22 dB
- f27 (abs 27) `tick.wav` -22 dB
- f30 (abs 30) `downlifter_1.wav` -16 dB — Air falling away as the bar empties (the leak).

**Reading check.**

- "Onde foi parar o seu dia?": 25 chars → max(24, 2×25) = 50 f; held f0→f60 = 60 f → yes
- "18:00": 5 chars → max(24, 2×5) = 24 f; held f30→f60 = 30 f → yes
- Headline and clock are read in parallel: 25+5 = 30 chars → 60 f; both visible f0–60 = 60 f → yes.

### 2. `s02-so-um-minuto` — abs 60–149 (90 f, 00:02.00–00:05.00) · problem

**Purpose.** Name the leak: fragmented windows fly past and every 'só um minuto' turns into forty.

**Shot types.** S05 chaos montage (flash cuts), S02 kinetic word slam, S07 number counter

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| “Só um minuto.” | headline | slam | 11 → 15 → 90 | minuto | Sora 800 128 px one line (≈1020 px, 15 chars > 12 so display size, not 200 px). 'minuto' rose. Contact f15 = beat (abs 75). |
| 40 min | hero | static | 29 → 60 → 90 | — | Counter 1→40 + ' min' suffix (50%, ink-2), Sora 700 240 px tabular-nums; lands f60 = downbeat (abs 120), turns rose. |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Background 'plain', rose orb bottom-left at 0.08 (problem palette), grain. Layer A — TEXTURE, NOT COPY: 12 generic, unbranded window-title chips (panel-2 pills, 1 px border, 18×18 gray app square + Inter 500 30 px ink-2 at 70%, desaturated 30%, 2 px blur so they are never read) flying right→left (time running backwards) in three depth rows (y 180 / 300 / 880, scale 0.8 / 1 / 0.9). Titles drawn from: 'Nova aba', 'Re: Re: Fwd: reunião', 'Planilha de horas — set', '(12) Caixa de entrada', 'Proposta_v3_final.pdf', 'Nova aba'. No real brand names, no e-mail addresses, no typing/keylogger imagery. Layer B centred at y≈420: the slam line. Layer C at y≈700: the counter.

**Assets.**

- `none` `vector`. Chips, slam and Counter primitives only.

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, linear. Each flash-cut layout of chips has its own push 1.00→1.06 (linear) and ±2° rotation from random(seed); the text layers are outside that push.
- f89 → `full` zoom 1.02, tilt rx 0° ry 0°, linear

**Motion.** f0 (abs 60) hard cut: chips already in flight at 38 px/f; flash-cut re-layouts on 8ths at f0 and f8 (new random chip set each; the canvas stays dark — no luminance flash). f11: '“Só um minuto.”' mounts at scale 1.45, blur 12, opacity 0→1 in 2 f; SLAM spring to contact on f15 (beat, abs 75); shake 6 px decaying over 8 f; chips slow to 8 px/f and dim to 40% so the phrase owns the frame. f29: counter mounts under it at '1 min' (SNAPPY, y 24→0). f30→60: counts 1→40 (Easing.bezier(0.25,0.1,0.25,1)), lands on f60 (downbeat abs 120): scale pulse 1→1.05→1, ink→rose, rose underline sweep under 'min' (12 f). f60–89: hold; chips drift at 4 px/f (alive). f90 hard cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed).

- f0 (abs 60) `glitch_1.wav` -16 dB — Flash-cut 1.
- f8 (abs 68) `glitch_3.wav` -16 dB — Flash-cut 2 (8th).
- f15 (abs 75) `impact.wav` +0 dB — Slam contact; bed duck −5 dB.
- f33 (abs 93) `ui_tick_2.wav` -22 dB
- f36 (abs 96) `ui_tick_2.wav` -22 dB
- f39 (abs 99) `ui_tick_2.wav` -22 dB
- f42 (abs 102) `ui_tick_2.wav` -22 dB
- f45 (abs 105) `ui_tick_2.wav` -22 dB
- f48 (abs 108) `ui_tick_2.wav` -22 dB
- f51 (abs 111) `ui_tick_2.wav` -22 dB
- f54 (abs 114) `ui_tick_2.wav` -22 dB
- f57 (abs 117) `ui_tick_2.wav` -22 dB
- f60 (abs 120) `impact_soft_3.wav` -2 dB — Counter lands on 40 min.
- f60 (abs 120) `riser-2bar.wav` -6 dB [start] — anchor=start: starts abs 120 so its natural end would be the drop (abs 240); it is HARD-MUTED at abs 225 (2-f fade) for the drop-out. Mix on the master track, not inside this scene's Sequence.

**Reading check.**

- "“Só um minuto.”": 15 chars → max(24, 2×15) = 30 f; held f15→f90 = 75 f → yes
- "40 min": 6 chars → max(24, 2×6) = 24 f; held f60→f90 = 30 f → yes
- Sequential reading: the slam line is alone f15→60 (45 f ≥ 30), the counter adds 6 chars at f60; total 21 chars → 42 f vs f15→90 = 75 f → yes.

### 3. `s03-planilha-de-memoria` — abs 150–224 (75 f, 00:05.00–00:07.50) · problem

**Purpose.** The second pain: accounting for your time becomes a second job — Friday's timesheet, filled from memory.

**Shot types.** S03 word-by-word stagger, S17 'ANTES' look (empty timesheet), used alone

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Na sexta: planilha de⍽memória. | headline | stagger-words | 0 → 12 → 75 | memória | Sora 700 96 px, one line ≈1530 px, centred y≈230. Fluid stagger 2 f over 4 units ('de memória.' glued) → last unit starts f6, lands f12. Emphasis = rose underline sweep under 'memória' (f14, 12 f, E.glide). |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Background 'plain', rose orb 0.06. Below the headline: a generic, unbranded empty timesheet grid 1280×520 px (x 320–1600, y 360–880): header labels SEG TER QUA QUI SEX (Inter 600 32 px ink-2, +0.14em), hour column 09h…17h (Inter 500 28 px ink-3), 1 px #1f2a40 cell lines; S17 'ANTES' treatment (desaturate 60%, brightness 0.8, 1 px jitter). A VOLT text caret (4×56 px) blinks in cell TER/10h — the only volt pixel in the problem act. Rose '?' glyphs (Inter 600 44 px) pop into empty cells.

**Assets.**

- `none` `vector`. Grid, caret and glyphs drawn in the composition.

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, linear. Slow push 1.00→1.03 on the grid layer (headline layer static, crisp).
- f74 → `full` zoom 1.03, tilt rx 0° ry 0°, linear

**Motion.** f0 hard cut: grid already set. f0–12 headline stagger (2 f) lands f12; f14 rose underline sweep. Caret blinks 8 on / 8 off from f0. f30, f45, f60 (beats): '?' pops into QUA/11h, SEG/15h, QUI/09h (opacity 0→1, scale 0.9→1, SNAPPY — no bounce in the problem act). Caret solid f60–74. f75: hard cut to the drop-out — the caret survives at the same screen position.

**Transitions.** In: cut (0 f). Out: cut (0 f) — T8 drop-out: hard cut to canvas + caret; the music and the riser stop dead on abs 225..

**SFX** (hit frame rel / abs, gain vs bed).

- f30 (abs 180) `ui_tick_1.wav` -22 dB — '?' appears.
- f45 (abs 195) `ui_tick_1.wav` -22 dB — '?' appears.
- f60 (abs 210) `ui_tick_1.wav` -22 dB — '?' appears.

**Reading check.**

- "Na sexta: planilha de memória.": 30 chars → max(24, 2×30) = 60 f; held f12→f75 = 63 f → yes

### 4. `s04-silencio` — abs 225–239 (15 f, 00:07.50–00:08.00) · problem

**Purpose.** The drop-out: everything stops; one blinking caret in total silence makes the reveal land.

**Shot types.** T8 drop-out (cut to silence)

**Visual.** Canvas #0a0d16 + grain (0.045) + vignette only. The volt caret from s03 stays at its exact screen position (4×56 px).

**Assets.**

- `none` `vector`. Caret only.

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, linear. The film's only deliberate freeze (grain keeps it alive).

**Motion.** f0–7 caret on, f8–14 off. Nothing else moves. Music −∞ dB (pre-reveal silence).

**Transitions.** In: cut (0 f) — T8 drop-out.. Out: flash (0 f) — The outgoing half of T7 is empty: the flash lives entirely in s05 f0–3..

**SFX.** None (music only).

**Reading check.**

- No copy in this scene.

### 5. `s05-logo-slam` — abs 240–299 (60 f, 00:08.00–00:10.00) · reveal

**Purpose.** The answer, on the loudest frame of the film: ubiqX AI.

**Shot types.** S08 logo reveal slam, T7 flash cut on the hit

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| ubiqX AI | hero | slam | 0 → 0 → 52 | X | brand/ubiqx-wordmark-bold.svg at height 194 px (= Sora 700 200 px) + 'AI' pill (Sora 600 56 px, volt on rgba(77,141,255,0.14)). Full opacity from its first frame (S08). Emphasis = the volt X (brand). |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Background 'orbs' (volt orb Ø1100 behind the lockup, 0→0.30 then 0.18), no grid yet. Lockup centred on x=960, y=540: wordmark (≈584×194 px) + 24 px gap + 'AI' pill, baseline-aligned. Wordmark text-shadow 0 0 40px rgba(77,141,255,0.30) for this bar only (the one allowed text glow).

**Assets.**

- `brand` `brand/ubiqx-wordmark-bold.svg`. Per-letter paths wm-u, wm-b, wm-i, wm-q, wm-X drive tracking and the separate X pop.

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, SLAM. Wordmark scale 1.10→1 (SLAM).
- f52 → `full` zoom 1.0, tilt rx 0° ry 0°, E.exit. Out-half of T3 starts: zoom 1→6 in log space centred on the X.
- f59 → `full` zoom 6.0, tilt rx 0° ry 0°, E.exit

**Motion.** f0 (abs 240, the drop): wordmark on screen at full opacity, scale 1.10→1 (SLAM); per-letter tracking +0.02em→−0.03em over 20 f (E.push); flash overlay #e8edf9 0.35→0 over 4 f (T7). f3: the X (wm-X) pops separately: rotate −90°→0, scale 0→1 (BOUNCY_SUBTLE — the shot's one bouncy element). f8: 'AI' pill pops (SNAPPY). f0–6 glow orb 0→0.30, settles 0.18 by f30. f10–28: glint (140 px diagonal white 18%, masked to the glyphs). f52–59: out-half of the zoom-through — push into the X (zoom 1→6, E.exit, radial blur 0→12 px) until volt fills ~70% of frame.

**Transitions.** In: flash (4 f) — T7 on the drop: #e8edf9 at 0.35 decaying to 0 over f0–3.. Out: zoom-through (8 f) — Out-half f52–59; the cut is abs 300 (downbeat). Matched in s06 by the app's own sidebar X..

**SFX** (hit frame rel / abs, gain vs bed).

- f0 (abs 240) `impact_deep_2.wav` +2 dB — Logo drop; bed duck −6 dB.
- f0 (abs 240) `sub-boom.wav` +0 dB — Stacked with the impact.
- f10 (abs 250) `shimmer_1.wav` -14 dB — Glint.

**Reading check.**

- "ubiqX AI": 8 chars → max(24, 2×8) = 24 f; held f0→f52 = 52 f → yes

### 6. `s06-hero-do-x-ao-app` — abs 300–419 (120 f, 00:10.00–00:14.00) · reveal

**Purpose.** First look at the real product: out of the logo's X straight into the app's own X, pulling back to the whole Hoje screen with UBI inside.

**Shot types.** T3 zoom-through + T5 match on the X, S09 hero product reveal (tilt-to-flat), S10 window float, S03 word stagger

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Controle de tempo automático com⍽IA. | headline | stagger-words | 14 → 32 → 105 | automático | Sora 700 80 px, one line ≈1480 px, centred y≈150 over a canvas scrim (solid 0–230 px, fade to 280). 5 units ('com IA.' glued), 3 f stagger → last starts f26, lands f32. Exit up 8 f from f105 (E.exit). |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Screen width 1440 px for a 2880-px capture (0.5 display px per capture px); effective scale s = zoom × 0.5; on-screen text = capture text px × s. Background 'grid' (floor fades 0→40%) + volt orb rising behind the window. Screen: ui/dashboard.png in 'mac' chrome (neutral #3a4560 dots), float 6 px/120 f, sheen at f60. Wide framing zoom 1.1 (s=0.55): window ≈1584 px wide, content top y≈294, bleeding off the bottom — hero (focus dial, the app's headline, stats, the 3D UBI with its speech bubble) and the day-track card visible. Vector overlay for the match: the 'X' path of brand/ubiqx-wordmark.svg (semibold, as drawn in the sidebar) pinned over the sidebar wordmark's X (image ≈ x 202–228, y 98–130) while the bitmap is too magnified to be crisp.

**Assets.**

- `ui` `ui/dashboard.png` — hotspots: `brand-wordmark`, `hero`, `day-track-card`, `ubi-robot`. Hero reveal; brand-wordmark is the match target for the X.
- `brand` `brand/ubiqx-wordmark.svg`. wm-X path used as a vector stand-in for the sidebar X during f0–13.

**Camera.**

- f0 → `brand-wordmark` zoom 54, tilt rx 10° ry -4°, E.push (file `ui/dashboard.png`; focus image (215, 114); s = 27). Frame 0 matches s05's last frame: the sidebar X ≈870 px tall. Override Screen maxZoom (default 3.2). Vector X overlay covers the bitmap until zoom < 3 (≈f10), then crossfades over 4 f.
- f45 → `full` zoom 1.1, tilt rx 4° ry -2°, E.push (file `ui/dashboard.png`; travel 45 f; s = 0.55). Pull-back 54→1.1, interpolated in log space ('perceptual-scale'); 97% done by f22. **Text:** Not a read: the app's own headline (≈55 capture px) shows at ≈30 px as texture; the kinetic headline carries the message.
- f90 → `full` zoom 1.13, tilt rx 4° ry -2°, linear (file `ui/dashboard.png`; s = 0.565). Float + micro drift.
- f119 → `day-track-card` zoom 1.52, tilt rx 4° ry -4°, E.glide (file `ui/dashboard.png`; travel 29 f; anchor (960, 700); s = 0.76). Glide f90→119 lands exactly on s07's opening framing, so the cut at abs 420 is invisible except for the text change.

**Motion.** f0–7: in-half of the zoom-through (blur 12→0, scale handled by the camera). The vector X (same size/position as s05's final frame) sits over the sidebar X and the pull-back starts at once (E.push, log zoom 54→1.1 over 45 f): the X shrinks into 'ubiqX' in the sidebar, the Hoje page appears, then the window chrome; rotateX 10°→4° over the same 45 f; grid floor 0→40% f0–30. f14–32 headline stagger. f45–89 window float, drift 1.10→1.13, sheen at f60. f90–119 glide (span, E.glide) toward the day-track card; headline exits up at f105 (8 f); f95–119 the canvas scrim that s07 opens with (solid 0–420, fade to 480) fades in, so the hero section recedes before the cut. f120: cut to s07 on the same framing.

**Transitions.** In: zoom-through (8 f) — In-half f0–7 (blur 12→0) + match on the X (T5): logo X → app sidebar X.. Out: cut (0 f) — T1: same framing on both sides; only the headline changes..

**SFX** (hit frame rel / abs, gain vs bed).

- f0 (abs 300) `whoosh-fast.wav` -8 dB — Zoom-through whoosh: loudest point (hit_offset 0.27 s ≈ 8 f) on the cut abs 300 → file starts abs 292. Master-track cue.

**Reading check.**

- "Controle de tempo automático com IA.": 36 chars → max(24, 2×36) = 72 f; held f32→f105 = 73 f → yes

### 7. `s07-ele-registra` — abs 420–539 (120 f, 00:14.00–00:18.00) · features

**Purpose.** Captura — the rhyme with the hook: the same 08:00→18:00 clock, but now the real Hoje timeline fills with your categories instead of draining.

**Shot types.** S18 data build (timeline wipe), S11 UI zoom-to-element with 3D tilt, S12 focus dimming, S03 word stagger

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Ele registra. Você trabalha. | headline | stagger-words | 0 → 15 → 120 | registra | Sora 700 88 px one line ≈1290 px, centred y≈300, 'registra.' volt; over a 60% canvas scrim band y 220–380. 4 units, 3 f stagger → lands f15. |
| 18:00 | label | static | 30 → 90 → 120 | — | Clock pill identical to s01 (Inter 600 44 px tabular, panel-2), riding the playhead; reads 08:00 at f30, lands 18:00 on f90 (beat). |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Screen width 1440 px for a 2880-px capture (0.5 display px per capture px); effective scale s = zoom × 0.5; on-screen text = capture text px × s. Screen ui/dashboard.png framed on day-track-card at zoom 1.52 (s=0.76 → card ≈1763×273 px) centred at (960, 700), tilt rx 4° ry −4°, float 4 px. Spotlight on day-track (pad 14, dim 0.62): the card title, hour axis and legend fall back (never read). Canvas scrim solid y 0–420, fade to 480 (the hero section above the card — the app's own headline, stats, dial — stays hidden; it was faded in by s06's glide). Image-space overlay glued to the plane: a mask hiding every block right of the playhead; playhead = 3 px volt line + 10 px dot, identical to s01. Image x of the hours: 00h = 554, 1 h = 93.1 px → 08:00 = 1299, 18:00 = 2230.

**Assets.**

- `ui` `ui/dashboard.png` — hotspots: `day-track-card`, `day-track`, `day-track-legend`. Real category blocks of the demo day; the legend is dimmed, not read.

**Camera.**

- f0 → `day-track-card` zoom 1.52, tilt rx 4° ry -4°, E.glide (file `ui/dashboard.png`; anchor (960, 700); s = 0.76). Matches s06's last frame. **Text:** No UI text is read here: the target is the coloured bar (graphic). Card labels (≈22–32 capture px) render at 17–24 px under a 0.62 dim; the clock pill (ours, 44 px) carries the time.
- f115 → `day-track-card` zoom 1.55, tilt rx 4° ry -4°, linear (file `ui/dashboard.png`; anchor (960, 700); s = 0.775). Drift +2%.

**Spotlights.** `day-track` f0–f119 dim 0.62

**Motion.** f0–15 headline stagger lands f15. f0–29: track masked from 08:00 (image x 1299) → empty; clock pill reads 08:00. f30→90 (span 60 f, E.glide): playhead sweeps image x 1299→2230; the mask edge follows, so the app's real category blocks (IFRO blue, Incubadora orange, Cidades Inteligentes green…) appear left→right behind it; clock counts 08:00→18:00 (10-min steps) and lands 18:00 on f90 with a pulse. Blocks after 18:00 and the app's 'now' marker stay masked — the day shown ends at 18:00. f90–115 hold + drift. f116–119: whip-left out-half (x 0→−960, horizontal blur 0→40, E.exit).

**Transitions.** In: cut (0 f) — Same framing as s06's last frame.. Out: whip-left (4 f) — T2 out-half f116–119; cut abs 540 at peak velocity..

**SFX** (hit frame rel / abs, gain vs bed).

- f30 (abs 450) `sweep_1.wav` -16 dB — Wipe onset (C-major pentatonic tick cluster L→R).
- f90 (abs 510) `ding_2.wav` -12 dB — 18:00 lands (C6).

**Reading check.**

- "Ele registra. Você trabalha.": 28 chars → max(24, 2×28) = 56 f; held f15→f120 = 105 f → yes
- "18:00": 5 chars → max(24, 2×5) = 24 f; held f90→f120 = 30 f → yes
- Headline + clock together: 28+5 = 33 chars → 66 f; from f15 to f120 = 105 f → yes.

### 8. `s08-regras-memoria-ia` — abs 540–629 (90 f, 00:18.00–00:21.00) · features

**Purpose.** Classifica — three words on three beats, each lighting a real row classified by rule, by memory and by AI.

**Shot types.** S03 word stagger (rhythmic, on beats), S12 camera push-in with focus dimming, S11 UI zoom

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Regras, memória, IA. | headline | stagger-words | 9 → 45 → 90 | IA | Sora 700 88 px one line, centred y≈150. Rhythmic stagger: 'Regras,' lands f15, 'memória,' f30, 'IA.' f45 (each on a beat, 6-f SNAPPY entries starting f9/f24/f39). 'IA.' volt. |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Screen width 1440 px for a 2880-px capture (0.5 display px per capture px); effective scale s = zoom × 0.5; on-screen text = capture text px × s. Screen ui/review-settled-expanded.png, zoom 2.4 (s=1.2), framed on reviewed-list rows 1–5 (image y 528–1098 → screen y 300–984; rows span screen x 42–1877), tilt rx 3° ry −3°. Canvas scrim solid y 0–300, fade to 320 (hides settled-header and the confirm bar — kept for s11). Row n image rect = {514, 528+114·(n−1), 1529, 114}. Row 1 = sei.ifro.edu.br · IFRO · 100% · 'regra'; row 3 = docs.google.com · IFRO · 74% · 'memória'; row 5 = Terminal · Incubadora · 85% · 'IA'. A 2 px volt ring draws around each row's origin badge (row1 ≈{1826,566,118,38}, row3 ≈{1787,794,156,38}, row5 ≈{1858,1022,86,38}).

**Assets.**

- `ui` `ui/review-settled-expanded.png` — hotspots: `reviewed-list`, `reviewed-row-1`, `settled-header`, `confirm-bar`. Rows 3 and 5 are derived from reviewed-row-1 (+228 / +456 image px). settled-header and confirm-bar stay under the scrim.

**Camera.**

- f0 → `reviewed-list` zoom 2.4, tilt rx 3° ry -3°, E.push (file `ui/review-settled-expanded.png`; focus image (1278, 728); s = 1.2). Whip in-half f0–3 slides the plane in from +960 px. **Text:** Row titles ('Google Chrome', 'Microsoft Teams', 'Terminal') ≈30 capture px × 1.2 = 36 px ✓. Category chips and origin badges ≈26 × 1.2 = 31 px — secondary; each badge word is stated by the 80-px headline word landing on the same frame. Bitmap upsampling 1.2 (> style's 1.15; see issues).
- f89 → `reviewed-list` zoom 2.5, tilt rx 3° ry -3°, linear (file `ui/review-settled-expanded.png`; focus image (1278, 776); s = 1.25). Slow push + 48 image-px downward travel following the spotlight.

**Spotlights.** `reviewed-row-1` f14–f29 dim 0.62; `reviewed-row-1` +228 px y f29–f44 dim 0.62 (row 3); `reviewed-row-1` +456 px y f44–f89 dim 0.62 (row 5)

**Motion.** f0–3 whip in-half (x +960→0, blur 40→0, E.push). f9/24/39 the three headline words start; each lands on a beat (f15, f30, f45). On each landing the matching row's spotlight arrives (d 0→1 over 6 f, starting 1 f early) and the previous one releases (8 f): row 1 'regra' @f15 → row 3 'memória' @f30 → row 5 'IA' @f45; the badge ring draws on (evolvePath 8 f) and stays at 60% after release, so by f45 the three origins are all marked. f45–89 hold on row 5, camera drift. f90 cut.

**Transitions.** In: whip-left (4 f) — T2 in-half f0–3 (x +960→0, blur 40→0, E.push).. Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed).

- f0 (abs 540) `whip_1.wav` -8 dB — Whip pass-by, hit_offset ≈2.6 f on the cut abs 540 → file starts abs 537. Master-track cue.
- f15 (abs 555) `ui_tick_2.wav` -20 dB — Word + row 'regra'.
- f30 (abs 570) `ui_tick_2.wav` -20 dB — 'memória'.
- f45 (abs 585) `ui_tick_2.wav` -20 dB — 'IA'.

**Reading check.**

- "Regras, memória, IA.": 20 chars → max(24, 2×20) = 40 f; held f45→f90 = 45 f → yes

### 9. `s09-so-as-duvidas` — abs 630–719 (90 f, 00:21.00–00:24.00) · features

**Purpose.** Revisão — the queue only holds what the chain could not settle.

**Shot types.** S11 UI zoom-to-element with 3D tilt, S12 focus dimming, S03 word stagger

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Só pergunta o⍽que não sabe. | headline | stagger-words | 0 → 18 → 90 | não sabe | Sora 700 88 px one line ≈1240 px, centred y≈230. 5 units ('o que' glued), 3 f → lands f18. Emphasis = marker highlight (volt 16%) behind 'não sabe' f20–30. |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Screen width 1440 px for a 2880-px capture (0.5 display px per capture px); effective scale s = zoom × 0.5; on-screen text = capture text px × s. Screen ui/review-queue-selected.png, zoom 2.2→2.4 (s 1.1→1.2), anchor = centre of queue-row-2 at screen (960, 547): queue rows 2–5 visible (screen y ≈480–1017): Finder/Downloads, Finder/cidades-inteligentes, Finder/api-incubadora ('Sem categoria · 0% · pendente'), WhatsApp ('IFRO · 40%'). Canvas scrim SOLID y 0–465, fade to 490: it hides queue row 1 including its hint line 'Pressione 1 a 9…' (queue-row-active-hint) and seats the headline. The assign card (and its '1 – 9' legend) is outside the frame.

**Assets.**

- `ui` `ui/review-queue-selected.png` — hotspots: `queue-card`, `queue-row-1`, `queue-row-2`, `queue-row-3`, `queue-row-active-hint`, `assign-card`. queue-row-1 / queue-row-active-hint are listed only because the scrim must cover them; the camera pans toward assign-card at f75 without reaching its '1 – 9' legend.

**Camera.**

- f0 → `queue-row-2` zoom 2.2, tilt rx 3° ry -4°, E.glide (file `ui/review-queue-selected.png`; anchor (960, 547); s = 1.1). Hint of row 1 lands at screen y 415–459 → under the solid scrim.
- f60 → `queue-row-2` zoom 2.4, tilt rx 3° ry -4°, E.glide (file `ui/review-queue-selected.png`; travel 60 f; anchor (960, 547); s = 1.2). Span push f0→60. **Text:** Row title 'Finder' ≈30 capture px × 1.2 = 36 px ✓; chips 'Sem categoria' / 'pendente' ≈26 × 1.2 = 31 px (secondary). Upsampling 1.2 (see Open issues).
- f89 → `queue-row-2` zoom 2.4, tilt rx 3° ry -6°, E.glide (file `ui/review-queue-selected.png`; travel 14 f; focus image (1678, 518); s = 1.2). Pan right +400 image px toward the assign card (span f75→89); cut at abs 720 while still moving. Frame x ends at image 2478, so the keys legend's '1 – 9' part (image x ≥ 2557) stays off-frame.

**Spotlights.** `queue-row-2` f59–f74 dim 0.62; `queue-row-3` f74–f89 dim 0.62

**Motion.** f0–18 headline stagger; f20–30 marker sweep under 'não sabe'. f0→60 camera push 2.2→2.4 (span, E.glide). f60 spotlight on row 2; the row's 'pendente' pill gets a 1-beat ember ring pulse. f75 spotlight moves to row 3 (same pulse). f75→89 camera pans right +400 image px toward the assign card (span, E.glide) — cut while moving.

**Transitions.** In: cut (0 f). Out: cut (0 f) — T1 on the downbeat abs 720, outgoing shot still moving..

**SFX.** None (music only).

**Reading check.**

- "Só pergunta o que não sabe.": 27 chars → max(24, 2×27) = 54 f; held f18→f90 = 72 f → yes
- Music-only scene (style §7.2: ≥30% of the features act without SFX).

### 10. `s10-uma-tecla` — abs 720–809 (90 f, 00:24.00–00:27.00) · features

**Purpose.** One key decides: '1' presses on the beat and flies into the app's own key badge; the app answers with a rule suggestion.

**Shot types.** S14 keycap press close-up, T5 match cut on a UI element, S13 UI state change, S03 word stagger

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Uma tecla. ⏎ E⍽ele aprende. | headline | stagger-words | 0 → 15 → 90 | aprende | Two lines, left-aligned at x=144, Sora 700 88 px, y≈170–370; 4 units ('E ele' glued), 3 f → lands f15. 'aprende.' volt. Persists across the internal match cut. |
| 1 | label | static | 0 → 6 → 30 | — | KeyCap legend, Inter 600 84 px (after the match it becomes the UI badge; the UI badge is the app's own '1'). |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Part A f0–29: Background 'grid' (floor 30%); KeyCap '1' 220×220 px, radius 32, at (1180, 640), rotateX 28° rotateZ −6°, perspective 1200. Part B f30–89: Screen width 1440 px for a 2880-px capture (0.5 display px per capture px); effective scale s = zoom × 0.5; on-screen text = capture text px × s. Screen ui/review-after-assign.png, zoom 2.5 (s=1.25), camera focus image (2133, 573) at screen centre → frame shows image x 1365–2901, y 141–1005: the assign card sits right (screen x ≈900–1834), 'Atribuir ao grupo selecionado' at y≈171, the five options (IFRO 1 · Incubadora 2 · Cidades Inteligentes 3 · Distração 4 · Pausa 5) at y 306–756, the rule suggestions at y≈848–1024. The keys legend with '1 – 9' sits at image y ≥ 1015 → below the frame edge (plus a 40-px canvas fade on the bottom edge). Spotlight on assign-card (dim 0.62) dims the queue on the left. Image-space patch #0c1220 (the card colour) hides last-suggestions until f60.

**Assets.**

- `ui` `ui/review-after-assign.png` — hotspots: `assign-card`, `assign-card-title`, `assign-picker`, `assign-option-1`, `assign-key-1`, `last-suggestions`, `keys-legend`. keys-legend must stay out of frame (it contains '1 – 9').

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, linear. Part A: keycap scene, drift 1.00→1.02.
- f30 → `assign-picker` zoom 2.5, tilt rx 2° ry -6°, E.push (file `ui/review-after-assign.png`; focus image (2133, 573); s = 1.25). Part B opens on this framing (hard match cut). **Text:** 'Atribuir ao grupo selecionado' ≈30 capture px × 1.25 = 37.5 px ✓; option 'IFRO' ≈29 × 1.25 = 36.3 px ✓; rule chips ≈25 × 1.25 = 31 px (secondary — the headline says 'E ele aprende.'). Upsampling 1.25 (see Open issues).
- f89 → `assign-picker` zoom 2.55, tilt rx 2° ry -6°, linear (file `ui/review-after-assign.png`; focus image (2133, 573); s = 1.275). Drift +2%.

**Spotlights.** `assign-card` f30–f89 dim 0.62

**Motion.** f0–15 headline stagger. f0–6 key appears (SNAPPY, y 40→0). f12→15 press: cap drops 14 px, skirt 18→4 px (E.exit 3 f); CONTACT f15 (beat, abs 735): legend turns volt + underglow 0 0 60px rgba(77,141,255,0.55) decaying 8 f; f16–23 release (SNAPPY). f30 (abs 750, beat 3) T5 match cut: the key's top face (legend '1') is drawn at its part-A rect over part B and flies/flattens into assign-key-1 (image {2746,402,40,40} → screen ≈{1726,326,50,50}) over 12 f (SNAPPY: rect, radius 32→8, fill panel→volt 20%). f45 (beat): assign-option-1 row ('IFRO … 1') flashes mint 12% for 8 f and a mint check draws at its right (evolvePath 10 f). f60 (abs 780, downbeat): the patch lifts — the app's real 'Da última decisão · Criar regra: Sempre: Calendário → IFRO' chips morph in (opacity 0→1, scale 0.98→1, SNAPPY). f60–89 hold + drift. f90 cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed).

- f15 (abs 735) `key-down.wav` -10 dB — Contact.
- f21 (abs 741) `key-up.wav` -18 dB — Release.
- f48 (abs 768) `success_chime_1.wav` -14 dB — Row confirmed (C+3 of f45).
- f60 (abs 780) `pop.wav` -16 dB — Rule chips appear.

**Reading check.**

- "Uma tecla. / E ele aprende.": 25 chars → max(24, 2×25) = 50 f; held f15→f90 = 75 f → yes
- "1": 1 chars → max(24, 2×1) = 24 f; held f6→f30 = 24 f → yes
- Headline + key legend: 25+1 = 26 chars → 52 f; f15→90 = 75 f → yes.

### 11. `s11-um-clique-vira-memoria` — abs 810–929 (120 f, 00:27.00–00:31.00) · features

**Purpose.** One click turns everything the Ubi decided into memory: the button becomes a column of mint 'você' badges and UBI nods.

**Shot types.** S12 camera push-in with focus dimming, S13 cursor click + state change, S19 mascot entrance, S03 word stagger

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Um⍽clique vira memória. | headline | stagger-words | 0 → 12 → 120 | memória | Sora 700 88 px one line, left-aligned x=144, y≈150, over a 60% scrim band y 60–250. 3 units ('Um clique' glued), 3 f → lands f12. 'memória.' volt. |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Screen width 1440 px for a 2880-px capture (0.5 display px per capture px); effective scale s = zoom × 0.5; on-screen text = capture text px × s. Screen ui/review-settled-expanded.png, hard-cut at f32 to ui/review-confirmed.png with the identical camera. Opening framing zoom 2.0 (s=1.0): the confirm bar ('19 grupos foram o Ubi que decidiu.' + 'Confirmar os 19') at screen y 360–456, rows with IA / memória / regra badges below. Live vector replica of confirm-all-button (same rect, radius, label, Inter 600) drawn over the bitmap for the hover/press states. UBI (ubi/yes-from-idle) from f74 at screen right: frame 480 px (robot ≈340 px), centre x 1640, feet y 930, over the dimmed assign-card area; volt rim light drop-shadow(0 0 30px rgba(77,141,255,0.35)) + soft elliptical floor glow.

**Assets.**

- `ui` `ui/review-settled-expanded.png` — hotspots: `confirm-bar`, `confirm-all-button`, `reviewed-list`. Before the click.
- `ui` `ui/review-confirmed.png` — hotspots: `reviewed-list`, `reviewed-row-1`, `assign-card`. After the click: every origin badge reads 'você'. The toast (bottom-right) stays out of frame.
- `ubi` `ubi/idle`. Idle frames 114–119 during the rise f74–79 (so idle ends on 119).
- `ubi` `ubi/yes-from-idle`. Frames 0–39 over f80–119 (two nods); the cut at f120 lands mid-clip on purpose.

**Camera.**

- f0 → `confirm-bar` zoom 2.0, tilt rx 3° ry -4°, E.push (file `ui/review-settled-expanded.png`; anchor (960, 408); s = 1.0). Wide on the bar and the rows under it.
- f26 → `confirm-all-button` zoom 2.8, tilt rx 3° ry -4°, E.push (file `ui/review-settled-expanded.png`; travel 15 f; anchor (1300, 560); s = 1.4). Punch-in f11→26 (onset B−1 of the beat at f15). **Text:** 'Confirmar os 19' ≈26 capture px × 1.4 = 36.4 px ✓ (drawn by the vector replica, so the label is crisp; the 1.4× bitmap around it sits under the 0.62 dim — see issues).
- f32 → `reviewed-row-1` zoom 2.8, tilt rx 3° ry -4°, linear (file `ui/review-confirmed.png`; anchor (1300, 560); s = 1.4). Same camera across the hard cut to the after-state. **Text:** Row 1's mint 'você' badge (≈26 × 1.4 = 36.4 px) now sits where the button was (image ≈{1830,470,110,36}).
- f60 → `reviewed-list` zoom 1.6, tilt rx 3° ry -4°, E.glide (file `ui/review-confirmed.png`; travel 15 f; focus image (1200, 900); s = 0.8). Span f45→60. Focus x 1200 keeps the assign card's '1 – 9' legend (image x ≥ 2557 → screen ≥ 2046) off the right edge. **Text:** Pull-back (not a read): the column of 10+ mint 'você' badges is a graphic; rows render ≈24 px.
- f119 → `reviewed-list` zoom 1.63, tilt rx 3° ry -4°, linear (file `ui/review-confirmed.png`; focus image (1200, 900); s = 0.815)

**Spotlights.** `confirm-all-button` f11–f31 dim 0.62; `reviewed-row-1` f32–f45 dim 0.62; `reviewed-list` f45–f119 dim 0.5 (keeps the assign card behind UBI dimmed)

**Cursor.** f14 → `rest:1760,940` (Fades in at a rest point (6 f), never teleports.); f26 → `confirm-all-button` (Quadratic arc, E.cursor; hover bg from f24.); f30 → `confirm-all-button` **click** (Click on the downbeat (abs 840): cursor 1→0.85→1, replica button scale 0.96 + pressed colour, ripple 0→44 px (+ second ring 0→64 px at 0.3).); f44 → `rest:1760,940` (Cursor fades out f38–44 as the camera pulls back.)

**Motion.** f0–12 headline stagger. f0–10 wide hold on the confirm bar. f11→26 punch-in (E.push) to the button, spotlight d 0→1 over 12 f. f14 cursor fades in, arcs to the button (arrives f26). f30 click (downbeat). f32 (C+2) hard cut to review-confirmed, same camera: the button is gone and row 1's mint 'você' badge sits almost exactly in its place — every badge in the list is now 'você'. f33 mint flash on row 1 (12%, 8 f). f45→60 pull back (span, E.glide) to zoom 1.6: the whole column of mint badges. f74 UBI rises (S19: y +320→0, rotate −10°→0, scale 0.7→1, BOUNCY_SUBTLE — the shot's only bouncy element), lands f80; landing squash 3% for 3 f; nods (yes-from-idle) f80–119. f120 cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed).

- f30 (abs 840) `click.wav` -12 dB — Confirmar os 19.
- f33 (abs 843) `shimmer_1.wav` -14 dB — All badges flip to 'você' (C+3).
- f80 (abs 890) `bloop.wav` -12 dB — UBI lands.

**Reading check.**

- "Um clique vira memória.": 23 chars → max(24, 2×23) = 46 f; held f12→f120 = 108 f → yes

### 12. `s12-18h-relatorio-pronto` — abs 930–1049 (120 f, 00:31.00–00:35.00) · features

**Purpose.** Entrega — the bookend: the hook's 18:00 comes back, and this time the day ends with the report already written.

**Shot types.** S07 number counter (clock), T5 match move (clock → kicker), S09 tilt-to-flat, S12 push-in + S13 click, S03 word stagger

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| 18:00 | kicker | static | 0 → 15 → 120 | — | f0–29: Sora 700 240 px tabular, centred (960, 470), reads '17:59', odometer-rolls to '18:00' landing f15 (beat). f30–41: morphs (SNAPPY) into a kicker pill at x 144, y 90–140 (Inter 600 44 px, volt on volt 14%) and stays. |
| O⍽relatório sai pronto. | headline | stagger-words | 31 → 43 → 120 | pronto | Sora 700 88 px, left-aligned x=144, y≈190; 3 units ('O relatório' glued), 3 f → lands f43. 'pronto.' volt. |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Part A f0–29: Background 'orbs'; the big clock. Part B f30–119: Screen width 1440 px for a 2880-px capture (0.5 display px per capture px); effective scale s = zoom × 0.5; on-screen text = capture text px × s. Screen ui/reports.png rising (tilt-to-flat). Wide framing zoom 1.7 (s=0.85), camera focus image (1672, 752): the whole IFRO daily report card — 'Resumo do dia — IFRO', Destaques chips, the Atividade / Minutos / Tipo / Horário table. Canvas scrim SOLID y 0–310, fade to 380 during the wide framing: it hides report-1-meta (which names the model version) and seats kicker + headline. At the push, the frame spans image x 1497–2868, y −76–695: report-1-meta's model name (image x ≈1037–1140) is out of frame; only its '…1.480 de saída' tail is in frame, dimmed. Live vector replica of report-1-copy for hover/press.

**Assets.**

- `ui` `ui/reports.png` — hotspots: `report-card-1`, `report-1-summary`, `report-1-items`, `report-1-copy`, `report-1-meta`. report-1-meta listed only because it must stay under the scrim / out of frame (model version name).

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, linear. Part A: clock card, drift 1.00→1.02.
- f60 → `report-card-1` zoom 1.7, tilt rx 6° ry -3°, E.push (file `ui/reports.png`; travel 30 f; focus image (1672, 752); s = 0.85). Enter 'rise' f30→60: window y +220→0, rotateX 20°→6° (E.push, 30 f). **Text:** Establishing shot, not a read (report text ≈26–29 px); kicker + headline carry the message.
- f75 → `report-1-copy` zoom 2.8, tilt rx 4° ry -3°, E.push (file `ui/reports.png`; travel 15 f; anchor (1300, 700); s = 1.4). Punch-in f60→75; the scrim fades to a 60% band behind kicker + headline only (f60–70). **Text:** 'Copiar Markdown' ≈27 capture px × 1.4 = 37.8 px ✓ (vector replica). Surroundings under 0.62 dim; bitmap upsampling 1.4 — see issues.
- f119 → `report-1-copy` zoom 2.85, tilt rx 4° ry -3°, linear (file `ui/reports.png`; anchor (1300, 700); s = 1.425)

**Spotlights.** `report-1-copy` f60–f115 dim 0.62

**Cursor.** f66 → `rest:1750,940` (Fade in (6 f) at a rest point.); f86 → `report-1-copy` (Arc, E.cursor; hover bg from f84.); f90 → `report-1-copy` **click** (Click on the downbeat (abs 1020); ripple; f92 a mint check draws beside the button (video-layer success glyph — no invented UI text).)

**Motion.** f0: '17:59' set (hard cut). f10–15: last digits roll to '18:00', contact f15 (beat) + 1→1.05→1 pulse + digits volt for 1 beat. f30 (abs 960, downbeat): '18:00' shrinks and flies to the kicker position (SNAPPY 12 f) while the report window rises (E.push 30 f). f31–43 headline stagger. f30–59 wide on the whole daily report. f60→75 punch-in to 'Copiar Markdown' + spotlight. f66 cursor fades in, arcs, arrives f86. f90 click (downbeat). f92 mint check (evolvePath 10 f). f93–115 hold + drift. f116–119 whip-left out-half.

**Transitions.** In: cut (0 f). Out: whip-left (4 f) — T2 out-half f116–119; cut abs 1050 (beat)..

**SFX** (hit frame rel / abs, gain vs bed).

- f15 (abs 945) `ding_1.wav` -12 dB — 18:00 lands (E6) — rhymes with s07's ding.
- f32 (abs 962) `whoosh-soft.wav` -14 dB — Report window rises; hit ≈ fastest point of the rise.
- f90 (abs 1020) `click.wav` -12 dB — Copiar Markdown.
- f93 (abs 1023) `success_chime_2.wav` -14 dB — Mint check.

**Reading check.**

- "18:00": 5 chars → max(24, 2×5) = 24 f; held f15→f120 = 105 f → yes
- "O relatório sai pronto.": 23 chars → max(24, 2×23) = 46 f; held f43→f120 = 77 f → yes
- Kicker + headline together: 5+23 = 28 chars → 56 f; strict from the headline's landing f43 → f120 = 77 f → yes.

### 13. `s13-voce-escolhe-a-ia` — abs 1050–1169 (120 f, 00:35.00–00:39.00) · proof

**Purpose.** Proof 1 — your AI or ours: the cursor passes over Claude, OpenAI and Grok and clicks 'IA do Ubi' in the real settings.

**Shot types.** S11 UI zoom, S13 cursor hover + click + state change, S03 word stagger

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Claude, OpenAI ou⍽Grok. | headline | stagger-words | 0 → 12 → 60 | — | Sora 700 96 px one line, centred y≈190. Fluid 2 f stagger over 3 units ('ou Grok.' glued) → lands f12. Deliberately NO emphasis: providers are named neutrally in plain text (no logos, no endorsement). Exit up 8 f from f60. |
| Ou deixe com o⍽Ubi. | headline | stagger-words | 60 → 72 → 120 | Ubi | Same slot; 4 units ('o Ubi.' glued), 2 f → lands f72. 'Ubi' volt. |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Screen width 1440 px for a 2880-px capture (0.5 display px per capture px); effective scale s = zoom × 0.5; on-screen text = capture text px × s. Screen ui/settings-ai.png, hard-cut at f92 to ui/settings-ai-ubi.png with the identical camera. zoom 2.5 (s=1.25), provider-picker centred at screen (960, 560): 'Provedor de IA' label above it; the provider hint and the key box below. model-fields (with model version ids) sit at image y ≥ 961 → screen ≥ 1195, out of frame. Canvas scrim SOLID y 0–330, fade to 380 (hides the section description). Hover state = rgba(232,237,249,0.06) rounded rect over the hovered pill (no text replica, no selection change).

**Assets.**

- `ui` `ui/settings-ai.png` — hotspots: `provider-picker`, `provider-anthropic`, `provider-openai`, `provider-xai`, `provider-ia-do-ubi`, `provider-active`, `model-fields`. Before the click (Anthropic Claude active). model-fields must stay out of frame.
- `ui` `ui/settings-ai-ubi.png` — hotspots: `provider-picker`, `provider-ia-do-ubi`, `provider-active`. After the click: 'IA do Ubi' active; hint = managed-plan line.

**Camera.**

- f0 → `provider-picker` zoom 2.5, tilt rx 2° ry -3°, E.push (file `ui/settings-ai.png`; anchor (960, 560); s = 1.25). Whip in-half f0–3. **Text:** Pill labels 'Anthropic Claude', 'OpenAI', 'xAI Grok', 'IA do Ubi' ≈30 capture px × 1.25 = 37.5 px ✓. Providers appear as text only. Upsampling 1.25 (see Open issues).
- f92 → `provider-picker` zoom 2.53, tilt rx 2° ry -3°, linear (file `ui/settings-ai-ubi.png`; anchor (960, 560); s = 1.265). Same camera across the state cut.
- f119 → `provider-picker` zoom 2.55, tilt rx 2° ry -3°, linear (file `ui/settings-ai-ubi.png`; anchor (960, 560); s = 1.275)

**Cursor.** f4 → `rest:1560,900` (Fade in at a rest point.); f15 → `provider-anthropic` (Hover (already the active one).); f30 → `provider-openai` (Hover on the beat.); f45 → `provider-xai` (Hover on the beat.); f86 → `provider-ia-do-ubi` (Arrives 4 f before the click; hover from f84.); f90 → `provider-ia-do-ubi` **click** (Click on the downbeat (abs 1140); ripple; f92 cut to settings-ai-ubi.)

**Motion.** f0–3 whip in-half. f0–12 headline 1. Cursor hovers each pill on the following beats (f15 Claude, f30 OpenAI, f45 xAI Grok): hover bg fades in 4 f, out 4 f. f60 headline 1 exits up (8 f); f60–72 headline 2. f86 cursor arrives on 'IA do Ubi'; f90 click (downbeat abs 1140); f92 (C+2) hard cut to settings-ai-ubi: 'IA do Ubi' is the active pill — the app's own state. f93 shimmer. f93–119 hold, drift.

**Transitions.** In: whip-left (4 f) — T2 in-half f0–3.. Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed).

- f0 (abs 1050) `whip_3.wav` -8 dB — Whip on the cut abs 1050 (hit_offset ≈3.4 f → file starts abs 1047). Master-track cue.
- f90 (abs 1140) `click.wav` -12 dB — IA do Ubi.
- f93 (abs 1143) `shimmer_2.wav` -14 dB — State change.

**Reading check.**

- "Claude, OpenAI ou Grok.": 23 chars → max(24, 2×23) = 46 f; held f12→f60 = 48 f → yes
- "Ou deixe com o Ubi.": 19 chars → max(24, 2×19) = 38 f; held f72→f120 = 48 f → yes
- Sequential cards: card 1 f12→60 = 48 f ≥ 46; card 2 f72→120 = 48 f ≥ 38. Each card is alone on screen (card 1 is gone by f68).

### 14. `s14-privado-por-padrao` — abs 1170–1259 (90 f, 00:39.00–00:42.00) · proof

**Purpose.** Proof 2 — privacy you can see: a personal address is masked into the exact token the app sends, before anything reaches the AI.

**Shot types.** S03 word stagger, S13 state change (morph), S04-style calm statement

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Privado por padrão. | headline | stagger-words | 2 → 14 → 90 | Privado | Sora 700 112 px, centred y≈330, 'Privado' volt. 3 units, 3 f → lands f14. |
| ana@example.com | label | static | 0 → 0 → 30 | — | Fictional, RFC-2606-reserved domain. Inside a window-title chip (panel-2, radius 22, 1.5 px border, height 96, generic 28-px window glyph) Inter 500 48 px ink, centred y≈600. Set on f0. |
| [email] | label | static | 30 → 32 → 90 | — | The exact mask token written by crates/ubiqx-core/src/redact.rs. Inter 600 48 px, volt text on volt 14%. |
| IA | label | static | 45 → 51 → 90 | — | Outline pill, Inter 600 40 px ink-2, at x≈1500; a 2 px volt line connects the masked chip to it. |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Background 'orbs' (volt, low 0.14). Headline top-centre; the title chip centre-screen; the 'IA' pill to the right. No UI capture (the privacy settings are not captured); the token is the app's real output, not a mock-up of a screen. Two accent hues max: volt only.

**Assets.**

- `none` `vector`. Kinetic diagram; the token string is taken from the product's redaction code.

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, linear. Drift 1.00→1.02; orb breathe.
- f89 → `full` zoom 1.02, tilt rx 0° ry 0°, linear

**Motion.** f0: chip set (hard cut) reading 'ana@example.com'. f2–14 headline stagger. f20→30 a volt marker sweeps across the address (E.glide 10 f). f30 (abs 1200, downbeat) CONTACT: the address collapses into '[email]' (SNAPPY width morph; old glyphs blur 0→6 and fade in 4 f). f45–51 'IA' pill fades in (E.enter); f45→60 the masked chip slides 120 px right while a 2 px volt line draws from it to the 'IA' pill (evolvePath 12 f). f60–89 hold.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed).

- f20 (abs 1190) `ui_tick_1.wav` -22 dB — Marker sweep starts.
- f30 (abs 1200) `pop.wav` -14 dB — Mask contact.

**Reading check.**

- "Privado por padrão.": 19 chars → max(24, 2×19) = 38 f; held f14→f90 = 76 f → yes
- "ana@example.com": 15 chars → max(24, 2×15) = 30 f; held f0→f30 = 30 f → yes
- "[email]": 7 chars → max(24, 2×7) = 24 f; held f32→f90 = 58 f → yes
- "IA": 2 chars → max(24, 2×2) = 24 f; held f51→f90 = 39 f → yes
- Headline + chip (longest state, 15 chars) together: 19+15 = 34 chars → 68 f; from the headline's landing f14 → f90 = 76 f → yes. 'ana@example.com' → '[email]' is one line that morphs in place.

### 15. `s15-tres-sistemas` — abs 1260–1319 (60 f, 00:42.00–00:44.00) · proof

**Purpose.** Proof 3 — it runs where you work: macOS, Windows and Linux.

**Shot types.** S20 logo row (platforms only), S03 word stagger

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| macOS · Windows · Linux | headline | stagger-words | 0 → 12 → 60 | — | Sora 700 88 px, centred y≈640, middots ink-3; 3 units ('macOS ·', 'Windows ·', 'Linux'), 3 f → lands f12. No emphasis (the glyph row is the accent). |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Background 'grid' (floor 25%) + orbs. OS glyph row centred y≈440: brand/os/apple.svg, brand/os/windows11.svg, brand/os/linux.svg — monochrome ink-2 at 90%, 112 px tall, optically balanced, gap 160 px. No AI-provider logos anywhere.

**Assets.**

- `brand` `brand/os/apple.svg`. macOS
- `brand` `brand/os/windows11.svg`. Windows
- `brand` `brand/os/linux.svg`. Linux

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, linear. Drift 1.00→1.02.
- f59 → `full` zoom 1.02, tilt rx 0° ry 0°, linear

**Motion.** f0: glyphs enter (y 16→0, opacity 0→1, blur 6→0, 14 f E.enter, stagger 4 f). f0–12 text stagger. f20–38 glint sweep across the glyph row. f12–59 hold. f60: hard cut on the final hit.

**Transitions.** In: cut (0 f). Out: cut (0 f) — Cut on the final hit (abs 1320)..

**SFX** (hit frame rel / abs, gain vs bed).

- f0 (abs 1260) `sweep_1.wav` -16 dB — One sound for the whole row.
- f0 (abs 1260) `riser_1bar_1.wav` -6 dB [start] — anchor=start: 1-bar riser (glide ending on A4); end_is_downbeat → last sample = abs 1320. Master-track cue.

**Reading check.**

- "macOS · Windows · Linux": 23 chars → max(24, 2×23) = 46 f; held f12→f60 = 48 f → yes

### 16. `s16-end-card` — abs 1320–1499 (180 f, 00:44.00–00:50.00) · cta

**Purpose.** Close: the brand, the tagline, one action — Baixe em ubiqx.com.br — and UBI waving goodbye.

**Shot types.** S22 end card (CTA), S19 mascot entrance, S13 cursor click

**Copy.**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| ubiqX AI | hero | mask-up | 0 → 12 → 180 | X | brand/ubiqx-wordmark-bold.svg at height 165 px (Sora 170 px) + 'AI' pill; lands with SMOOTH scale 1.06→1 + masked rise. |
| Retome o⍽controle do⍽seu dia. | sub | mask-up | 8 → 26 → 180 | — | Tagline, Inter 500 44 px ink-2, centred y≈565 (S04 masked line, 18 f). NBSPs: 'o controle', 'do seu'. |
| Baixe em ubiqx.com.br | headline | static | 15 → 21 → 180 | — | CTA pill: Sora 600 44 px #0a0d16 on volt (6.1:1), radius 999, padding 22×44, y 640–732; enters scale 0.9→1 SNAPPY. |
| macOS · Windows · Linux | label | static | 22 → 36 → 180 | — | Inter 500 36 px ink-2 with 28-px OS glyphs before each name, centred y≈820 (above the bottom 120 px). Fades in (E.enter 14 f, y 16→0). |

(⍽ = NBSP, ⏎ = line break)

**Visual.** Background 'orbs' + grid at 20%, glow orb breathing 1↔1.05 over 120 f. Lockup row centred y≈400, x≈540–1380: UBI (frame 365 px → robot ≈260 px) · 36 px · wordmark · 24 px · 'AI' pill. Tagline, CTA pill and platform row stacked below, all centred and inside title-safe; nothing in the bottom 120 px or the corner 200×120 boxes.

**Assets.**

- `brand` `brand/ubiqx-wordmark-bold.svg`. Wordmark.
- `brand` `brand/os/apple.svg`. 28-px glyph in the platform row.
- `brand` `brand/os/windows11.svg`. 28-px glyph.
- `brand` `brand/os/linux.svg`. 28-px glyph.
- `ubi` `ubi/idle`. Frames 74–119 over f29–74, then 11–44 over f146–179.
- `ubi` `ubi/wave-from-idle`. Frames 0–70 over f75–145 (splices after idle 119, rejoins idle at 11).

**Camera.**

- f0 → `full` zoom 1.0, tilt rx 0° ry 0°, linear. Static lockup; only the orb breathes and UBI idles (always alive).
- f179 → `full` zoom 1.0, tilt rx 0° ry 0°, linear

**Cursor.** f45 → `rest:1500,900` (Fade in at a rest point (6 f).); f56 → `rest:960,686` (CTA pill centre (composition px); arc, E.cursor; hover from f54.); f60 → `rest:960,686` **click** (Click on the downbeat (abs 1380): pill scale 0.96, ripple. Cursor fades out f66–72.)

**Motion.** f0 (abs 1320, final hit): wordmark lands (SMOOTH scale 1.06→1 + masked rise 18 f), glow orb 0→0.22. f8 tagline mask-up (lands f26). f15 CTA pill (SNAPPY, lands f21). f22 platform row fades in (E.enter 14 f, lands f36). f29 UBI rises (S19, BOUNCY_SUBTLE — the only bouncy element), lands f37 with bloop; idles. f45 cursor fades in, arrives f56, clicks f60 (downbeat). f75 UBI waves (wave-from-idle). f36–179: fully legible hold (144 f ≥ 90). The last frame IS the end card — no fade to black.

**Transitions.** In: cut (0 f) — Hard cut on the final hit.. Out: cut (0 f) — End of film; the last frame is the end card..

**SFX** (hit frame rel / abs, gain vs bed).

- f0 (abs 1320) `impact_deep_1.wav` +2 dB — Final hit; bed duck −4 dB.
- f37 (abs 1357) `bloop.wav` -12 dB — UBI lands.
- f60 (abs 1380) `click.wav` -12 dB — CTA click.

**Reading check.**

- "ubiqX AI": 8 chars → max(24, 2×8) = 24 f; held f12→f180 = 168 f → yes
- "Retome o controle do seu dia.": 29 chars → max(24, 2×29) = 58 f; held f26→f180 = 154 f → yes
- "Baixe em ubiqx.com.br": 21 chars → max(24, 2×21) = 42 f; held f21→f180 = 159 f → yes
- "macOS · Windows · Linux": 23 chars → max(24, 2×23) = 46 f; held f36→f180 = 144 f → yes
- End card: all four lines together 8+29+21+23 = 81 chars → 162 f; from the first landing (f12) to the end (f180) = 168 f → yes; every line individually ≥ 60 f (memorable items) → yes; ≥ 90 f fully legible after the last landing (f36→180 = 144) → yes.

## Open issues

1. Bitmap upsampling above style §S11's 1.15 cap: s08 and s09 at 1.2, s10 and s13 at 1.25, the s11 and s12 button push-ins at 1.4. The app's UI text is CSS 13–15 px (26–30 capture px at DPR 2), so ≥ 36 px on screen needs s = 1.2–1.4. Mitigations already in the boards: the pressed buttons are live vector replicas and everything around them sits under a 0.62 spotlight dim. Clean fix: recapture review-settled-expanded, review-confirmed, review-queue-selected, review-after-assign, reports, settings-ai and settings-ai-ubi at DPR 3 (tools/capture-ui.mjs, newPage({dpr: 3})); hotspot rects scale ×1.5 and every zoom here drops to ≤ 0.94 bitmap scale (1.4 / 1.5).
2. Secondary UI text inside zoomed frames (category chips, origin badges, 'pendente', the rule chips) renders at 29–31 px; in each case the same word is carried by the kinetic headline landing on the same frame.
3. The captures themselves contain '1 – 9' (keys-legend, queue-row-active-hint) and model version ids (report-1-meta 'claude-sonnet-5', settings model-fields). The framings and solid scrims in s09, s10, s11, s12 and s13 keep them out of view; builders must not widen those framings. A cleaner fix is recapturing with a 1–5 legend if the product agrees.
4. Cues that straddle cuts (zoom-through whoosh at abs 300, whips at 540 and 1050, riser-2bar 120→hard-muted 225, riser_1bar_1 1260→1320) must live on a master audio track; inside a scene <Sequence> they would be truncated.
5. s06 needs Screen camera zoom up to 54 (override maxZoom 3.2) and a vector X (brand/ubiqx-wordmark.svg, path wm-X) aligned over the sidebar X; the image rect used (≈ x 202–228, y 98–130) is an estimate from the capture — measure it on a still before locking.
6. s11's match relies on 'Confirmar os 19' (review-settled-expanded, image ≈ 1785–2011 × 446–510) and row 1's 'você' badge (review-confirmed, ≈ 1830–1940 × 470–504) being ~50 px apart; verify on a still and add a 6-f position morph if the jump reads.
7. The hook's 08:00→18:00 clock is abstract and differently worded, but it echoes the existing trailer's '8 → 18' idea. If zero overlap is wanted, s01 can run 09:00→18:00; the 18:00 end must stay because it pays off in s12 (the report's default time).
8. No capture exists of the privacy card or the 'Dados enviados à IA' dialog, so s14 is a kinetic diagram built on the real redaction token '[email]'. A capture of that dialog would make a stronger proof shot.
9. Music does not exist yet: frame 0 must be a downbeat with a hit, the stop at abs 225 must be dry (no tail), and the final tail must end by frame 1499.
10. Several reading holds pass with small margins (s06 73/70 f, s08 45/40, s13 card 1 48/46, s15 48/46, s02 counter 30/24). Re-run the validator after any timing change.

## QA notes

- **privacy:** timeline.png, timeline-selected.png, timeline-reclassify.png and reports-generated.png (real Gmail address) are NOT used. No shot shows an e-mail address except the fictional ana@example.com in s14 (kinetic, not a capture).
- **claims:** No '1–9' on screen (queue-row-active-hint and keys-legend are kept under a scrim / out of frame in s09 and s10); no provider logos (names as UI text only, s13); no model version names (report-1-meta and model-fields framed out in s12/s13); CTA is 'Baixe em ubiqx.com.br' (no Assine/Compre); no stats presented as results (19 grupos, 18:00 and the timeline are demo UI; '40 min' is the landing's 'só um minuto que virou quarenta').
- **transitionsUsed:** T1 cut, T2 whip-left ×2, T3 zoom-through ×1 (+T5 match on the X), T5 match cut (keycap → badge), T7 flash ×1, T8 drop-out ×1 = 6 distinct types.
- **trailerDifference:** No live action, no desk, no hourglass; no trailer line reused verbatim except the tagline and CTA. The 08:00→18:00 clock is an abstract bar and pays off in the product's own 18:00 report time.
