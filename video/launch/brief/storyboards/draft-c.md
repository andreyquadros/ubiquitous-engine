# ubiqX AI — “O UBI apresenta” (launch film, draft C)

**Angle.** C — UBI hosts the launch. A cold open on a pain every professional who reports hours knows (Friday, 17:00: what did you do on Tuesday?), a chaos beat, then UBI's first cameo shaking his head at the 'planilha de memória'. Silence. On the drop UBI leaps into the ubiqX AI lockup, says 'Oi, eu sou o UBI.' and shrinks straight into the real app, where every feature line is spoken in his first person (Você trabalha. Eu anoto. / Só pergunto o que não sei. / Uma tecla. E eu aprendo.). He steps back out of the app when the queue empties, nods to take over the AI (Ou deixe comigo.), and lands on the end card. Five short UBI moments + the end card; the real UI carries every proof.

**Specs.** 1920×1080, 30 fps, 120 BPM (beat 15 f, bar 60 f) · **1500 frames = 50.0 s = 25 bars** · 16 scenes · PT-BR on screen, no voice-over, works muted.

Twin of `draft-c.json` (the JSON is the source of truth; this file is generated from it). Frames are scene-relative unless marked *abs*.

## At a glance

- **Hook (frame 0, fully set):** kicker “SEXTA-FEIRA · 17:00” over **“O que você fez na terça?”** on an empty, generic timesheet — the Friday reckoning everyone who reports hours knows.
- **Story:** Tuesday shatters into tabs and pings (09:00 → 17:00) → “Planilha de memória?” and UBI's first cameo shaking his head → 0.5 s of silence → **drop**: UBI leaps into the ubiqX AI lockup → “Oi, eu sou o UBI.” → he shrinks straight into the real app and every feature is said in his voice: *Você trabalha. Eu anoto. · Nas suas categorias. · Só pergunto o que não sei. · Uma tecla. E eu aprendo. · Um clique vira memória. · Nada em dúvida! · O relatório sai pronto. · Não! Foque na sua produtividade. · Claude, OpenAI ou Grok. Ou deixe comigo.* → end card “Retome o controle do seu dia.” / “Baixe em ubiqx.com.br” / macOS · Windows · Linux.
- **Three strongest moments:** (1) the drop at abs 240 — silence, flash, UBI leaps into frame as the wordmark slams; (2) the two UBI match-cuts — he shrinks *into* the real “Hoje” screen and the camera pulls back to reveal the product (abs 408–456), then steps back *out* of the app when the review queue empties (abs 930); (3) the keyboard/click pair — a keycap “1” pressed on the beat lights IFRO and the rule chip appears (abs 735), then one click on “Confirmar os 19” turns every chip mint “você” (abs 840).

## UBI moments (5 + the end card)

| # | Scene | Clip(s) | Beat |
|---|---|---|---|
| 1 | `s03-planilha-de-memoria` | `ubi/no` | Shakes his head at 'Planilha de memória?' (first cameo, pre-brand). |
| 2 | `s05-drop-ubiqx + s06-oi-eu-sou-o-ubi` | `ubi/jump-from-idle → ubi/wave` | Leaps into the lockup on the drop, then 'Oi, eu sou o UBI.' and shrinks into the app (match-cut). |
| 3 | `s09-so-pergunto-o-que-nao-sei` | `ubi/look` | Reads the queue, then looks up at his bubble 'Só pergunto o que não sei.' |
| 4 | `s12-nada-em-duvida` | `ubi/idle-to-excited → ubi/excited` | Steps OUT of the app (match-cut from the in-app render) when the queue is empty: 'Nada em dúvida!' |
| 5 | `s15-ou-deixe-comigo` | `ubi/yes-from-idle` | Nods yes: 'Ou deixe comigo.' (IA do Ubi). |
| 6 | `s16-end-card` | `ubi/jump → ubi/idle → ubi/wave-from-idle` | Drops onto the lockup on the beat after the final hit and waves goodbye. |

UBI never covers the key UI: when he shares the frame with the product he stands in his own column (body x ≥ 1315 in s03/s09/s15) while the UI sits in a separate viewport, or — in s12 only — he steps in front of an app that has already racked out of focus after its message was read. `sleep`, `worried` and `turntable` are deliberately unused — a sleeping or worried UBI next to the product would read as the product failing.

## Beat sheet

| # | Scene | abs frames | time (s.ff) | bar.beat → cut | act | on-screen copy | UBI | out |
|---|---|---|---|---|---|---|---|---|
| 1 | `s01-hook-terca` | 0–89 | 00.00–03.00 | 1.1 → 2.3 | hook | SEXTA-FEIRA · 17:00 / O que você fez na terça? | — | cut |
| 2 | `s02-terca-em-pedacos` | 90–164 | 03.00–05.15 | 2.3 → 3.4 | problem | TERÇA-FEIRA / 17:00 | — | cut |
| 3 | `s03-planilha-de-memoria` | 165–224 | 05.15–07.15 | 3.4 → 4.4 | problem | Planilha de memória? | no | cut |
| 4 | `s04-silencio` | 225–239 | 07.15–08.00 | 4.4 → 5.1 | problem | — | — | flash |
| 5 | `s05-drop-ubiqx` | 240–359 | 08.00–12.00 | 5.1 → 7.1 | reveal | ubiqX AI / Controle de tempo automático com IA. | idle, jump-from-idle | cut |
| 6 | `s06-oi-eu-sou-o-ubi` | 360–419 | 12.00–14.00 | 7.1 → 8.1 | reveal | Oi, eu sou o UBI. | wave | match-cut |
| 7 | `s07-voce-trabalha-eu-anoto` | 420–539 | 14.00–18.00 | 8.1 → 10.1 | features | REGISTRA SOZINHO / Você trabalha. Eu anoto. | — | whip-left |
| 8 | `s08-nas-suas-categorias` | 540–629 | 18.00–21.00 | 10.1 → 11.3 | features | CLASSIFICA COM IA / Nas suas categorias. | — | cut |
| 9 | `s09-so-pergunto-o-que-nao-sei` | 630–719 | 21.00–24.00 | 11.3 → 13.1 | features | Só pergunto o que não sei. | look | cut |
| 10 | `s10-uma-tecla` | 720–809 | 24.00–27.00 | 13.1 → 14.3 | features | Uma tecla. / E eu aprendo. | — | whip-left |
| 11 | `s11-um-clique-vira-memoria` | 810–899 | 27.00–30.00 | 14.3 → 16.1 | features | Um clique vira memória. | — | cut |
| 12 | `s12-nada-em-duvida` | 900–989 | 30.00–33.00 | 16.1 → 17.3 | features | Nada em dúvida! | excited, idle-to-excited | blur-dissolve |
| 13 | `s13-relatorio-sai-pronto` | 990–1109 | 33.00–37.00 | 17.3 → 19.3 | features | 18:00 / O relatório sai pronto. | — | cut |
| 14 | `s14-nao-foque` | 1110–1199 | 37.00–40.00 | 19.3 → 21.1 | features | BLOQUEIOS / Não! Foque na sua produtividade. | — | cut |
| 15 | `s15-ou-deixe-comigo` | 1200–1319 | 40.00–44.00 | 21.1 → 23.1 | proof | Claude, OpenAI ou Grok. / Ou deixe comigo. | idle, yes-from-idle | cut |
| 16 | `s16-end-card` | 1320–1499 | 44.00–50.00 | 23.1 → 26.1 | cta | ubiqX AI / Retome o controle do seu dia. / Baixe em ubiqx.com.br / macOS · Windows · Linux | idle, jump, wave-from-idle | cut |

Act lengths: hook 90 f (6%) · problem 150 f (10%) · reveal 180 f (12%) · features 780 f (52%) · proof 120 f (8%) · cta 180 f (12%).

Transitions used (5 types): cut (T1, the default), flash (T7, once, on the drop), match-cut (T5, UBI into the app), whip-left (T2, ×2: abs 540, 810), blur-dissolve (T6, once, into the 18:00 breather). Scene starts on downbeats: 0, 240, 360, 420, 540, 720, 900, 1200, 1320; every other start is on a beat.

## Music map

**Key:** A minor (relative of C major — matches the riser glides ending on A4, the C-major-pentatonic shimmers/sweeps and the C6/E6 dings). **Tempo:** 120 BPM, 4/4, 25 bars. **Harmony:** Am – F – C – G (one chord per bar) in the grooves; Am(add9) for intro and final hit; breakdown Fmaj7 → Am.

**Mix:** Bed nominal 0.5 (−6 dB). Problem act ≈ −18 LUFS short-term (low-passed at 900 Hz), drop and features ≈ −12 LUFS. Duck the bed (1 f attack, 2 f hold, 10 f release) at 240 (−6 dB), 1110 (−5 dB), 1320 (−4 dB). Master −14 LUFS integrated, ≤ −1 dBTP. The composer should leave space (no busy fills) at 660, 945 and 1305 where speech-bubble lines land.

| Section | bars | abs frames | energy | feel / instrumentation |
|---|---|---|---|---|
| Intro — under water | 1–3 | 0–179 | 2 | Felt kick on beats 1 and 3, clock-like closed-hat 8ths, plucked Am9 arpeggio and a warm sub pad, all through a 900 Hz low-pass. Frame 0 is hit by a clear (filtered) downbeat — no silent lead-in. Sparse; leaves room for the question. |
| Build → drop-out | 4–4 | 180–239 | 3 | Kick doubles to 8ths, riser swells, filter keeps opening while UBI shakes his head. Everything CHOKES at 225 (hard stop, no reverb tail): 15 frames of digital silence. |
| Drop — reveal | 5–7 | 240–419 | 5 | Full range: four-on-the-floor kick, sidechained sub bass on A1, wide supersaw chords Am (bar 5) – F (bar 6) – C (bar 7), claps on 2 and 4, open hats; the main motif (A-minor pentatonic pluck A–C–E–G) states the hook in bar 6. Bar 7 (UBI says hello) drops the claps for a light call-and-response pluck. |
| Groove A — registra / classifica / pergunta | 8–12 | 420–719 | 4 | Main groove: kick, clap 2/4, 16th hats, bass following Am–F–C–G, pluck motif; variation after 4 bars (an arp joins at bar 12). Mostly music-only under the typing and the look — let the picture breathe. |
| Groove B — revisão (peak) | 13–17 | 720–1019 | 5 | Arp layer, tighter bass, key and click accents written into the groove; open hats from 900 for the empty-queue payoff. At 990 (beat 3 of bar 17) the drums drop out under the blur dissolve — pad and clock-tick percussion carry into the breakdown. |
| Breakdown — 18:00 | 18–19 | 1020–1139 | 2 | Kick-less: Fmaj7 → Am pad, clock-tick percussion, a bell on the clock landing; half-bar snare roll and noise lift 1080–1109; a full-band stab on beat 3 of bar 19 (1110) for the focus-window slam, then one beat of air before the groove returns. |
| Groove C — proof | 20–22 | 1140–1319 | 4 | Groove returns with a brighter lead; bar 22 thins out after the IA-do-Ubi click so the bubble and the nod read, then a 2-beat reverse cymbal into the final hit. |
| Final hit & ring-out | 23–25 | 1320–1499 | 2 | Final hit on 1320 (Am(add9) stab + impact + sub), then pad and one last statement of the pluck motif ring out; a soft accent on the CTA click and a two-note goodbye (C–A) on the wave. Tail fully decayed by 1499 (a 15 f fade only if the tail must be cut). |

| abs frame | time | type | event |
|---|---|---|---|
| 0 | 00.00 | downbeat-hit | Kick + filtered Am stab on frame 0 (the poster frame). |
| 90 | 03.00 | stinger | Reversed-hat pickup into the montage cut (beat 3). |
| 120 | 04.00 | downbeat-hit | Filtered stab + low tom: the clock lands on '17:00'. |
| 150 | 05.00 | filter-sweep | Low-pass starts opening 900 Hz → 3 kHz by 224. |
| 165 | 05.15 | riser-start | 1-bar noise/supersaw riser (the SFX riser_1bar_1 sits on top), choked at 225. |
| 180 | 06.00 | downbeat-hit | Muted kick accent as 'memória?' lands and UBI rises. |
| 225 | 07.15 | stop | Hard stop of every stem. |
| 225 | 07.15 | silence | 225–239 true silence (−∞). Nothing — not even a reverse swell — may play here. |
| 240 | 08.00 | drop | THE drop: full band + impact_deep_2 + flash; bed duck −6 dB. |
| 255 | 08.15 | stinger | Snare accent on beat 2 as UBI lands. |
| 300 | 10.00 | downbeat-hit | Crash; motif enters. |
| 360 | 12.00 | downbeat-hit | Cut to UBI's close-up: pads filter down, pluck 'call' (two notes, like 'oi'). |
| 405 | 13.15 | filter-sweep | Quick high-pass lift 405–419 into the match-cut. |
| 420 | 14.00 | downbeat-hit | Match-cut into the real app: crash. |
| 540 | 18.00 | downbeat-hit | Whip to Categorias (whip_2 on the cut). |
| 660 | 22.00 | stinger | Short pluck stab as 'Só pergunto o que não sei.' lands. |
| 705 | 23.15 | filter-sweep | Snare fill 705–719 into the keycap downbeat. |
| 720 | 24.00 | downbeat-hit | Keycap cut. |
| 735 | 24.15 | stinger | Short synth stab on the '1' key press (beat 2). |
| 810 | 27.00 | stinger | Whip to 'Um clique vira memória.' (beat 3). |
| 840 | 28.00 | downbeat-hit | Click on 'Confirmar os 19': stab + crash. |
| 900 | 30.00 | downbeat-hit | Empty queue: lift, open hats. |
| 930 | 31.00 | stinger | UBI steps out of the app: sparkle arp in C-major pentatonic (sits with shimmer_2). |
| 990 | 33.00 | stop | Drums stop on beat 3 of bar 17 (the breather begins). |
| 1020 | 34.00 | downbeat-hit | Bell (C6/E6) + sub as the clock lands on '18:00'. |
| 1080 | 36.00 | riser-start | Half-bar snare roll + noise lift. |
| 1110 | 37.00 | stinger | Full-band stab for the window slam (beat 3); bed duck −5 dB. |
| 1140 | 38.00 | downbeat-hit | Groove back in. |
| 1200 | 40.00 | downbeat-hit | Cut to the AI choice. |
| 1260 | 42.00 | downbeat-hit | Click on 'IA do Ubi'. |
| 1290 | 43.00 | riser-start | Reverse cymbal 1290→1319 (ends on 1319, never crosses 1320). |
| 1320 | 44.00 | downbeat-hit | Final hit; bed duck −4 dB. |
| 1380 | 46.00 | stinger | Soft pluck accent on the CTA click. |
| 1410 | 47.00 | stinger | Two-note goodbye (C5→A4) as UBI waves. |
| 1485 | 49.15 | stop | Last audible sample by 1499; the film ends on the end card. |

Cut/hit list the composer must honour: 0, 120, 180, 225, 240, 255, 300, 360, 420, 540, 660, 720, 735, 810, 840, 900, 930, 990, 1020, 1110, 1140, 1200, 1260, 1320, 1380, 1410. The 225–239 silence is absolute; the riser never crosses 225 and the final tail is gone by 1499.

## Conventions

- **frames:** Scene-relative frames unless marked abs. Every scene starts on a beat (multiple of 15); starts on downbeats (multiples of 60): 0, 240, 360, 420, 540, 720, 900, 1200, 1320.
- **transitions:** Not overlapping. A transition is split into an outgoing half at the end of scene N and an incoming half at the start of scene N+1, named identically; 'frames' = frames of that half inside that scene (0 = empty half). The boundary frame is the cut.
- **camera:** Screen primitive, width 1440 for every 2880-px capture: effective bitmap scale s = zoom × 1440/2880 = 0.5 × zoom = upsample factor. intervention.png (1840 px, DPR 4) is shown at width 1520: s = 0.826 × zoom. On-screen UI text = captureFontPx × s. captureFontPx was measured from the PNGs (cap height ÷ 0.727 for Inter, ÷ 0.73 for Sora). 'atFrame' = frame the camera ARRIVES; 'duration' = travel frames before it; 'anchor' = canvas point where 'focus' (image px, inside the named hotspot) lands; 'hard' = no travel (first frame of a cut). 'canvas' = a scene without UI (camera on the whole composition).
- **readability:** A camera key with a readTarget must put that string at ≥ 36 px. Where s > 1.15 (the style's crisp limit for DPR-2 captures) the read label is either re-set as live vector text pinned to the hotspot (s08 label + typed text, s11 button) or held under a focus dim with s ≤ 1.25 (s10 IFRO, s11 toast, s15 picker) — flagged in 'upsample'. All other UI text is texture.
- **cursor:** 'to' = a hotspot of the scene's capture, 'rest:x,y' (canvas px, fade-in/rest points) or 'comp:x,y' (canvas px for code-drawn targets). click:true clicks on that frame; the cursor arrives 3–4 f before.
- **sfx:** 'atFrame' = frame where the HIT (transient / loudest point / riser end) lands, relative to the scene; 'fileStartFrame' = atFrame − hit_offset_frames from sfx-manifest.json. Cues that straddle a cut (whips, riser) go on the master audio track. gainDb is relative to the bed (BED = 0.5).
- **reading:** hold ≥ max(24, 2 × chars) from landFrame (last word at spring ≥ 0.9) to outFrame (first exit frame, or the cut). Elements that share the screen (same group) are checked on the SUM of their characters.
- **ubi:** ubiTrack lists which PNG sequence plays on which scene frames: clip dir, startIndex (index = startIndex + (f − from)), loop. Anchors: headCenter (449,303), feet y 797 in the 900-px frame; body ink idle ≈ x 262–624, y 162–797.
- **guards:** Never legible on screen: keys-legend / queue-row-active-hint (they print '1 – 9'), report-1-meta and model-fields (model names), any Gmail/e-mail row. Not used at all: timeline*.png, reports-generated.png, onboarding captures (their footers claim 'Tudo fica no seu Mac'), og.png, brand/ai/*.
- **typography:** Sora 600/700/800 + Inter 400/500/600 only; PT-BR accents kept; brand casing ubiqX / UBI / IA do Ubi; arrows as inline SVG.

## Scenes

### 1. `s01-hook-terca` — abs 0–89 (00.00–03.00 s, bar.beat 1.1 → 2.3) · 90 f · hook

**Purpose.** Stop the scroll on frame 0 with the Friday-timesheet pain: nobody remembers what they did on Tuesday.

**Shot types.** S01 cold-open hook frame (fully set on frame 0), S03-U underline sweep.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **SEXTA-FEIRA · 17:00** | kicker | static | 0 → 0 → 90 | — | Inter 600 24 px, +0.14em, ink-3, centred at y 322. Present from frame 0 (poster frame). |
| **O que você fez na terça?** | hero | static | 0 → 0 → 90 | terça? | Sora 700 128 px, tracking −0.035em, line-height 1.02, two hand-broken lines 'O que você fez' / 'na terça?' (≤ 14 chars/line), centred on y 520. 'terça?' volt + underline sweep f2–14 (E.glide). |

**Visual.** Canvas #0a0d16; Background variant 'plain' with ONE volt orb top-left (Ø1100, 0.18), grain 0.045, vignette 0.55. Behind the type, a GENERIC weekly timesheet grid drawn in code (not a product UI, no brand): 5 columns SEG · TER · QUA · QUI · SEX × 9 rows 08:00…16:00, cells 200×60, headers Inter 600 22 px ink-3, 1.5 px #1f2a40 lines at 45 %, grid 1100×600 centred at (960, 560), blur 6 px, opacity 0.40. The TER column is tinted rose 6 % and holds one sharp rose caret (4×44 px) in its 09:00 cell. Type block centred in title-safe: kicker y 322, headline lines ≈ y 400–530 / 530–660. Frame 0 = designed poster (no fade).

**Assets.**
- none: `code-drawn`. Generic timesheet grid + caret, drawn in React/SVG. No real app, no brand.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | — | `canvas` | 1.0 | — | 0/0 | E.glide | — | Canvas camera (scale about the centre). |
| 89 | — | `canvas` | 1.04 | — | 0/0 | E.glide | — | 1.00 → 1.04 over the whole shot; grid parallax x 0 → −24 px (0.27 px/f). |

**Motion.** f0 everything set (no entrance) — the thumbnail. f2–14 volt underline sweeps under 'terça?' (E.glide). f0–89 camera 1.00→1.04, grid drifts −24 px. f30 (beat 3) the rose caret starts blinking 8 f on / 8 f off. f60 (bar-2 downbeat) the 'TER' header flashes rose for one beat (opacity 0.45→1→0.45) — something new every ≤ 30 f. f89 hard cut while the push is still moving.

**Transitions.** In: cut (0 f) — Film start. The music downbeat on frame 0 is the hit. Out: cut (0 f).

*No SFX (music only).*

**Reading check.** "SEXTA-FEIRA · 17:00" (19) + "O que você fez na terça?" (24): 43 chars → max(24, 2×43) = 86 f; held f0→f90 = 90 f → yes

### 2. `s02-terca-em-pedacos` — abs 90–164 (03.00–05.15 s, bar.beat 2.3 → 3.4) · 75 f · problem

**Purpose.** Tuesday dissolves into tabs, windows and pings: the clock jumps from 09:00 to 17:00 before you notice.

**Shot types.** S05 chaos montage (flash cuts on 8ths), S07 counter (clock, roll mode).

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **TERÇA-FEIRA** | kicker | mask-up | 0 → 6 → 75 | — | Inter 600 26 px +0.14em rose, centred y 372, masked rise 6 f. |
| **17:00** | hero | static | 0 → 30 → 75 | — | Clock counter, Sora 700 220 px tabular-nums, ink, centred y 560, mode 'roll' HH:MM from '09:00' (f0) to '17:00' landing on f30 = abs 120 (bar-3 downbeat), Easing.bezier(.25,.1,.25,1). On land: scale pulse 1→1.05→1 (BOUNCY_SUBTLE) and rose for one beat. |

**Visual.** Four micro-shots of GENERIC, unbranded UI fragments (code-drawn, desaturated 30 %, brightness 0.9, rose accents only, all on the dark canvas so luminance never flips), behind a 50 % canvas scrim disc under the clock: M1 f0–7 browser tab strip, 18 grey tabs squeezing, grey favicon squares; M2 f8–14 window-switcher row of 7 grey tiles, one outlined; M3 f15–22 chat list with grey avatars/bars and rose unread badges '3', '12', '27'; M4 f23–29 inbox rows with rose unread dots and grey subject bars (no names, no addresses). Final shot f30–74: a calendar week packed with overlapping grey blocks at 35 % opacity, blur 4 px. Clock + kicker ride on top of every shot.

**Assets.**
- none: `code-drawn`. Generic fragments only (style.md S05; facts.md §5.15 — no real brand UIs; no keylogger/recording imagery).

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | — | `canvas` | 1.0 | — | 0/0 | linear | — | Each micro-shot pushes 1.00→1.06 linearly with its own rotation ±2° (random(`s02-${n}`)). |
| 29 | — | `canvas` | 1.06 | — | 0/0 | linear | — |  |
| 30 | — | `canvas` | 1.0 | — | 0/0 | linear | — | travel 0 f; Final shot starts fresh. |
| 74 | — | `canvas` | 1.03 | — | 0/0 | linear | — |  |

**Motion.** Cuts on 8ths: f0, f8, f15, f23 (eighth(n) boundaries), then f30 on the downbeat. The clock rolls 09:00→17:00 across f0–30 (minutes as odometer digits, tabular), lands f30 with a pulse; the kicker masks up f0–6. f30–74 calendar blocks drift; f45 the blocks fade to 40 % one column at a time (2 f stagger) — the day blurring out; f74 hard cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `glitch_1.wav` | 0 (90) | 0 | -16 |  |
| `glitch_3.wav` | 8 (98) | 8 | -18 |  |
| `glitch_2.wav` | 15 (105) | 15 | -16 |  |
| `glitch_1.wav` | 23 (113) | 23 | -18 |  |
| `impact_soft_3.wav` | 30 (120) | 30 | -6 | Muffled thud as 17:00 lands (music has a filtered stab on 120). |

**Reading check.** "TERÇA-FEIRA" (11) + "17:00" (5): 16 chars → max(24, 2×16) = 32 f; held f30→f75 = 45 f → yes

### 3. `s03-planilha-de-memoria` — abs 165–224 (05.15–07.15 s, bar.beat 3.4 → 4.4) · 60 f · problem

**Purpose.** Name the chore — the timesheet filled in from memory — and let UBI make his first appearance by shaking his head at it.

**Shot types.** S03 word stagger, S18 cell pops (16ths), S19 mascot entrance, UBI 'no'.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **Planilha de memória?** | headline | stagger-words | 3 → 15 → 60 | memória? | Sora 700 112 px, one line (≈1150 px), left-aligned x 144, cap-top y 190. Words start f3/f6/f9 so 'memória?' lands on f15 = abs 180 (bar-4 downbeat) and turns rose (problem accent). |

**Visual.** The s01 timesheet grid returns SHARP (no blur, opacity 0.7), camera closer (scale 1.25, framing SEG–QUA); rose '?' glyphs (Sora 700 44 px) pop into the TER cells. UBI bottom-right: ubi/no frames at 620 px (k 0.689), frame box left 1170 top 330 → body ≈ x 1346–1601, y 442–879 (clear of the bottom 120 px and of the 200×120 player corner). Ember floor-glow ellipse under his feet (0.25), volt rim light drop-shadow(0 0 30px rgba(77,141,255,.35)), idle float 6 px/120 f.

**Assets.**
- ubi: `ubi/no`. Two head shakes, rest pose at both ends.
- none: `code-drawn`. Same generic grid as s01.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | — | `canvas` | 1.0 | — | 0/0 | linear | — |  |
| 59 | — | `canvas` | 1.05 | — | 0/0 | linear | — | Linear drift; grid parallax −16 px. |

**UBI track.** f0–13 hidden — off-frame below; f14–21 `ubi/no` from index 0 (held) — rest pose during S19 rise; f22–59 `ubi/no` from index 0

**Motion.** f0 cut to the sharp grid. f0–14 eight '?' pop into the TER cells (SNAPPY, 2 f stagger, spread 14 f). f3–15 headline stagger; 'memória?' lands rose on f15. f14 UBI enters (S19, onset B−1: y +320→0, rotate −10°→0, scale 0.7→1, BOUNCY_SUBTLE; lands ≈ f21 with a 3 % squash); he holds ubi/no frame 0 while rising. f22–59 ubi/no 0→37: two head shakes. f45 the '?' marks jitter 1 px (nervous). f59 hard cut to silence.

**Transitions.** In: cut (0 f). Out: cut (0 f) — Hard cut into the drop-out; everything (music, riser) stops on 225.

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `sweep_2.wav` | 0 (165) | 0 | -18 | One tick cluster for the '?' group. |
| `bloop_2.wav` | 21 (186) | 21 | -14 | UBI lands (lower bloop, no servo — problem act stays muted). |
| `riser_1bar_1.wav` | 60 (225) | 0 | -6 | Master-track cue: file starts on f0 (abs 165), its end (end_is_downbeat) lands on abs 225 = the silence. Crescendo −24 → −4 dB; nothing crosses 225. |

**Reading check.** "Planilha de memória?" (20): 20 chars → max(24, 2×20) = 40 f; held f15→f60 = 45 f → yes

### 4. `s04-silencio` — abs 225–239 (07.15–08.00 s, bar.beat 4.4 → 5.1) · 15 f · problem

**Purpose.** Drop-out: half a second of near-black and true silence so the reveal hits.

**Shot types.** T8 drop-out.

*No copy.*

**Visual.** Canvas #0a0d16 + grain 0.05 + vignette only. f8–14: a 6 px volt point at the centre swells into a 60 px soft glow (opacity 0→0.12, E.enter) — the spark the drop explodes from.

**Assets.**
- none: `code-drawn`. Nothing but canvas, grain and one glow.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | — | `canvas` | 1.0 | — | 0/0 | linear | — |  |
| 14 | — | `canvas` | 1.0 | — | 0/0 | linear | — |  |

**Motion.** f0–7 pure canvas (static is intended: a 15 f deliberate freeze). f8–14 volt spark swells. No sound.

**Transitions.** In: cut (0 f). Out: flash (0 f) — The flash lives entirely in s05 f0–4.

*No SFX (music only).*

**Reading check.** No copy in this scene.

### 5. `s05-drop-ubiqx` — abs 240–359 (08.00–12.00 s, bar.beat 5.1 → 7.1) · 120 f · reveal

**Purpose.** The drop: UBI leaps into frame as the ubiqX AI wordmark slams, and one line says what the product is.

**Shot types.** T7 flash cut, S08 logo reveal slam, S19 mascot entrance (jump), S04 masked line.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **ubiqX AI** | hero | slam | 0 → 0 → 120 | X | brand/ubiqx-wordmark-bold.svg at 170 px font (ink box ≈ 165×497 px) + 'AI' pill (Sora 600 56 px, volt on rgba(77,141,255,.14), radius 999). On screen at full opacity on f0 (contact = drop). The X is volt by brand; it pops separately (path #wm-X). |
| **Controle de tempo automático com IA.** | sub | mask-up | 21 → 30 → 120 | — | Inter 500 44 px ink-2, centred, cap-top y 812 (≈ 830 px wide). Mask rise 18 f E.enter, lands f30 = abs 270 (beat). |

**Visual.** Background variant 'grid' (floor fades 0→40 % f0–12), volt orb behind the lockup (0→0.30 by f6, settles 0.18 by f30), ember orb bottom-left 0.07. Horizontal brand lockup centred on x 960 by ink bounds, row centre y 560: UBI left (ubi frames at 600 px, k 0.667, frame box ≈ left 313 top 218 → body ≈ x 488–730, y 326–749), gap 60, wordmark ≈ x 793–1290, 'AI' pill ≈ x 1313–1437. Descriptor centred below. UBI gets a volt floor-glow ellipse and rim light.

**Assets.**
- brand: `brand/ubiqx-wordmark-bold.svg`. Per-letter paths wm-u/b/i/q/X; X pops separately.
- ubi: `ubi/jump-from-idle`. Entered at index 9 (take-off) on the drop.
- ubi: `ubi/idle`. Continues from idle frame 11 after the splice.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | — | `canvas` | 1.0 | — | 0/0 | SLAM | — | Slam shake on f0: noise2D × 6 px × exp(−t/3) for 8 f, ±0.4°. |
| 89 | — | `canvas` | 1.03 | — | 0/0 | E.glide | — |  |
| 119 | — | `canvas` | 1.06 | — | 0/0 | E.glide | — | f90–119 slow push toward UBI, anticipating the cut to his close-up. |

**UBI track.** f0–21 `ubi/jump-from-idle` from index 9 — apex index 15 on f6, contact index 24 on f15, settle index 30 on f21; f22–31 `ubi/jump-from-idle` from index 31 — built-in cross-fade into idle (last frame == idle 10); f32–119 `ubi/idle` from index 11 (loop)

**Motion.** f0 (abs 240, the drop): flash overlay #e8edf9 0.35→0 over 4 f; wordmark on screen at scale 1.10 → 1 (SLAM, ≈1 by f4), tracking +0.02em → −0.03em over 20 f (E.push). UBI starts mid-leap: jump-from-idle index 9 on f0, plus a y-offset +160 → −30 px (f0–6, E.push) → 0 (f6–15, E.exit = gravity); apex f6, feet contact f15 = abs 255 (beat) with the clip's own squash, settle f21. f3 the X path pops (rotate −90°→0, scale 0→1, BOUNCY_SUBTLE). f8 'AI' pill pops. f10–28 glint (140 px diagonal, white 18 %, background-clip text). f21 descriptor masks up, lands f30. f30–59 wordmark text-shadow glow 0.30 fades to 0 (≤ 1 bar of glow). f60 (abs 300 downbeat) orb breathes, grid keeps scrolling. f90–119 camera push 1.03→1.06 toward UBI; hard cut at 120.

**Transitions.** In: flash (5 f) — T7: 1 hit frame + 4 decay frames (0.35 peak, never full white). Only flash in the film. Out: cut (0 f) — Cut to UBI close-up (scale change hides the idle→wave pose difference).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `impact_deep_2.wav` | 0 (240) | 0 | +2 | 45 Hz logo-drop sub + impact on the drop; bed ducks −6 dB. |
| `shimmer_1.wav` | 10 (250) | 10 | -14 | Glint. |
| `bloop_1.wav` | 15 (255) | 15 | -12 | UBI lands (pop + thump + servo). |

**Reading check.** "ubiqX AI" (8) + "Controle de tempo automático com IA." (36): 44 chars → max(24, 2×44) = 88 f; held f30→f120 = 90 f → yes

### 6. `s06-oi-eu-sou-o-ubi` — abs 360–419 (12.00–14.00 s, bar.beat 7.1 → 8.1) · 60 f · reveal

**Purpose.** UBI introduces himself as the host — the voice every later line comes from.

**Shot types.** S19 mascot hero framing, speech bubble, T5 match-cut prep (shrink into the app).

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **Oi, eu sou o UBI.** | headline | static | 9 → 15 → 50 | UBI. | UBI's real onboarding line (onboarding.ts:43). Speech bubble: panel-2 #172033 at 94 %, 1 px rgba(255,255,255,.08), radius 36, box ≈ x 930–1668, y 188–332, tail pointing left to his helmet; text Sora 700 72 px ink, 'UBI.' volt. Bubble scales 0.6→1 from the tail origin (BOUNCY_SUBTLE) f9, text in with the bubble, lands f15 = abs 375 (beat). |

**Visual.** Canvas 'plain' with an ember orb behind UBI (0.08) and a volt orb top-right (0.16); no grid. UBI hero: ubi/wave at 1000 px (k 1.111), frame box left 260 top 20 → body ≈ x 551–955, y 200–905, helmet centre ≈ (759, 356). The wave arm swings to the viewer's left, away from the bubble. f48–59 match-cut prep: UBI shrinks to the in-app size and position of s07's first frame (frame 479 px at left 576, top 305 → body ≈ x 715–926, y 391–729) while the canvas tints to the Hoje hero-card colour #0e1526 and an ember floor glow (the app's) fades in under his feet.

**Assets.**
- ubi: `ubi/wave`. Full 61-frame wave, index = f.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | — | `canvas` | 1.0 | — | 0/0 | linear | — | Static camera; UBI float y = 6·sin(2πf/120). |
| 59 | — | `canvas` | 1.0 | — | 0/0 | linear | — |  |

**UBI track.** f0–59 `ubi/wave` from index 0

**Motion.** f0 cut (abs 360 downbeat). ubi/wave 0→59 (hello peaks ≈ f15–45). f9 bubble pops; text lands f15. f50 bubble scales out (6 f, E.exit). f48–59 UBI transform: frame 1000 → 479 px, box moves to (576, 305) (E.exit, 12 f); canvas tint + app floor glow f48–59.

**Transitions.** In: cut (0 f). Out: match-cut (12 f) — Outgoing half = the 12 f shrink/tint; the next scene opens on the same silhouette inside the real app.

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `pop.wav` | 9 (369) | 9 | -14 | Bubble. |
| `whoosh_out_3.wav` | 51 (411) | 48 | -14 | UBI shrinks away; start with the move. |

**Reading check.** "Oi, eu sou o UBI." (17): 17 chars → max(24, 2×17) = 34 f; held f15→f50 = 35 f → yes

### 7. `s07-voce-trabalha-eu-anoto` — abs 420–539 (14.00–18.00 s, bar.beat 8.1 → 10.1) · 120 f · features

**Purpose.** First look at the real app: UBI lives in 'Hoje', and the day's timeline fills itself in — you work, he records.

**Shot types.** T5 match-cut (3D UBI → in-app UBI), S09 hero product reveal (pull-back, tilt-to-flat), S11 glide, S18 data build (day track), S03 word stagger.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **REGISTRA SOZINHO** | kicker | mask-up | 12 → 18 → 116 | — | Inter 600 24 px +0.14em volt, centred y 84. |
| **Você trabalha. Eu anoto.** | headline | stagger-words | 15 → 30 → 116 | anoto. | Sora 700 96 px, one line ≈ 1180 px, centred, cap-top y 124. 4 words, 3 f stagger from f15 → 'anoto.' lands volt on f30 = abs 450 (beat). UBI's first-person version of facts §6 'Você trabalha. Ele anota.' Canvas scrim 0–280 px (90 % → 0) under the type. |

**Visual.** Screen ui/dashboard.png, width 1440 (s = 0.5 × zoom), 'mac' chrome with neutral #3a4560 dots, volt glow, float 6 px. Background 'grid' (floor 30 %). Wide framing puts the window top at y ≈ 262 so the headline sits above it. Day-track reveal: from f0 a cover in image space hides day-track (554,1178,2236×68) right of x 1300 (≈ 09h) with the empty-track colour; f66–96 it retracts to the now-marker (≈ x 2295) behind a 3 px volt playhead with a 40 px glow.

**Assets.**
- ui: `ui/dashboard.png` — hotspots: `ubi-robot`, `hero`, `day-track-card`, `day-track`, `day-track-legend`. Match target = in-app UBI (body ink ≈ x 2402–2613, y 552–890). The day track is the proof of automatic recording.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | ui/dashboard.png | `ubi-robot` | 2.0 | 1.0 | 0/0 | hard | — | focus (2507, 721); anchor (820, 560); Opening frame = match: in-app UBI body lands at ≈ x 715–926, y 391–729 (338 px), identical to s06 f59. Bitmap 1:1 (upsample 1.0). |
| 36 | ui/dashboard.png | `full` | 1.0 | 0.5 | 6/-6 | E.push | — | travel 36 f; anchor (960, 731); Pull-back = hero reveal (perceptual zoom 2.0 → 1.0), tilt rx 0→6°, ry 0→−6°; window centre y 731 (top ≈ 262). Hero headline of the app (53 px capture) shows at 26 px: texture only, never a read. |
| 90 | ui/dashboard.png | `day-track-card` | 1.6 | 0.8 | 4/-4 | E.glide | — | travel 30 f; anchor (960, 690); Span f60→90 (abs 480→510). Spotlight on day-track-card (dim 0.62) from f60. Target is a graphic (coloured blocks), legend text 24 px capture → 19 px texture. |
| 119 | ui/dashboard.png | `day-track-card` | 1.64 | 0.82 | 4/-4 | linear | — | Micro drift. |

**Motion.** f0 match-cut: same silhouette, now the real app (its own speech bubble and orange floor glow visible). f0–36 pull-back to the full 'Hoje' window (E.push, 97 % done by f18), sheen at f24. f12 kicker, f15–30 headline stagger. f36–59 window float. f60–90 glide to the day-track card, hero dims. f66–96 playhead sweep reveals the day's blocks left→right (E.glide). f96 the playhead parks on the now-marker and pulses on beats (f105). f116–119 whip-out (x 0→−960, E.exit, horizontal blur 0→40).

**Transitions.** In: match-cut (12 f) — Incoming half: the first 12 f of the pull-back continue the shrink from s06. Out: whip-left (4 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `swoosh_long_3.wav` | 18 (438) | 0 | -10 | Deep push/pull for the tilt-to-flat reveal. |
| `sweep_1.wav` | 66 (486) | 66 | -16 | One rising tick cluster for the track build (not per block). |

**Reading check.** "REGISTRA SOZINHO" (16) + "Você trabalha. Eu anoto." (24): 40 chars → max(24, 2×40) = 80 f; held f30→f116 = 86 f → yes

### 8. `s08-nas-suas-categorias` — abs 540–629 (18.00–21.00 s, bar.beat 10.1 → 11.3) · 90 f · features

**Purpose.** The AI classifies into categories YOU define — you describe each front in your own words.

**Shot types.** T2 whip-in, S03 word stagger, S11 glide to element, S15 typing into a field, S12 focus dimming.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **CLASSIFICA COM IA** | kicker | mask-up | 0 → 6 → 90 | — | Inter 600 24 px +0.14em volt, x 144, y 88. |
| **Nas suas categorias.** | headline | stagger-words | 3 → 15 → 90 | suas | Sora 700 96 px, one line ≈ 980 px, left x 144, cap-top y 124; words f3/f6/f9, phrase lands f15 = abs 555 (beat) and 'suas' turns volt on that beat. Canvas scrim 0–300 px. |

**Visual.** Module A: type top-left, UI fills the frame below. Screen ui/categories.png, width 1440, rx 3°, ry −3°, float 4 px. Live re-set (vector) over the bitmap, pinned in image space: (1) the field label 'O que conta como trabalho desta categoria' Inter 500 27 image-px at (1340, 646); (2) a patch in the textarea fill colour over the textarea interior (1356, 705, 1418×170) that hides the captured text, and a typewriter layer (Inter 400 29 image-px, ink) re-typing the capture's own first line. Both stay crisp at 1.35× where the bitmap would be soft.

**Assets.**
- ui: `ui/categories.png` — hotspots: `categories-list-card`, `category-item-1`, `category-item-2`, `category-item-3`, `category-editor`, `category-editor-title`. Opening wide shows 'Suas categorias: IFRO, Incubadora, Cidades Inteligentes' (demo data); then the description field inside category-editor.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | ui/categories.png | `categories-list-card` | 1.5 | 0.75 | 3/-3 | E.push | — | focus (1672, 618); anchor (960, 680); Overview of list + editor (x 512–2832 → 1740 px). Category names 29 px capture → 22 px: texture. |
| 45 | ui/categories.png | `category-editor` | 2.7 | 1.35 | 3/-3 | E.glide | O que conta como trabalho desta categoria (vector re-set): 27 × 1.35 = 36.5 px ✓ | travel 30 f; focus (2040, 700); anchor (960, 700); Span f15→45 (abs 555→585). Visible capture x ≈ 1329–2751, y ≈ 300–1100: label, textarea, helper line. Typed text 29 px → 39 px (vector). Bitmap around it upsampled 1.35 but under a 0.55 focus dim. |
| 89 | ui/categories.png | `category-editor` | 2.76 | 1.38 | 3/-3 | linear | O que conta como trabalho desta categoria (vector re-set): 27 × 1.38 = 37.3 px ✓ | focus (2040, 700); anchor (960, 700); Drift. |

**Motion.** f0–3 whip-in (x +960→0, E.push, blur 40→0). f0 kicker, f3–15 headline. f15–45 glide to the description field; f40 spotlight on (1340, 640, 1450×250) dim 0→0.55 over 12 f. f38 the typewriter starts: 'Docência no IFRO Campus Porto Velho Calama: aulas de Programação Web, orientação de TCC,' at 2 chars/f (84 chars → f38–79), solid caret while typing, then blinking 8/8. f89 hard cut.

**Transitions.** In: whip-left (4 f). Out: cut (0 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `whip_2.wav` | 0 (540) | -2 | -8 | Master-track cue: loudest point on the cut (abs 540), file starts 2 f earlier in s07. |
| `type-1.wav` | 40 (580) | 40 | -22 |  |
| `type-2.wav` | 44 (584) | 44 | -22 |  |
| `type-3.wav` | 48 (588) | 48 | -22 |  |
| `type-4.wav` | 52 (592) | 52 | -22 |  |
| `type-1.wav` | 56 (596) | 56 | -22 |  |
| `type-2.wav` | 60 (600) | 60 | -22 |  |
| `type-3.wav` | 64 (604) | 64 | -22 |  |
| `type-4.wav` | 68 (608) | 68 | -22 |  |
| `type-1.wav` | 72 (612) | 72 | -22 |  |
| `type-2.wav` | 76 (616) | 76 | -22 |  |

**Reading check.** "CLASSIFICA COM IA" (17) + "Nas suas categorias." (20): 37 chars → max(24, 2×37) = 74 f; held f15→f90 = 75 f → yes

### 9. `s09-so-pergunto-o-que-nao-sei` — abs 630–719 (21.00–24.00 s, bar.beat 11.3 → 13.1) · 90 f · features

**Purpose.** UBI reads the review queue and states the deal: he only asks about what he couldn't resolve.

**Shot types.** S19 mascot (look), speech bubble, S10 window float, S12 focus dimming.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **Só pergunto o que não sei.** | headline | static | 24 → 30 → 90 | não sei. | Bubble, two lines 'Só pergunto' / 'o que não sei.' Sora 700 60 px ink, 'não sei.' volt; box ≈ x 1256–1776, y 96–284, tail down-left to the helmet (≈ 1470, 306). First person of facts §6 'Só pergunta o que não sabe.' Pops f24 (BOUNCY_SUBTLE from the tail), lands f30 = abs 660 (downbeat). |

**Visual.** Mirrored module B. Left: Screen ui/review-queue-selected.png, width 1440, rotateY +8° (right edge recedes toward UBI), rotateX 4°, float 6, glow, seen through a rounded viewport (clipPath inset, radius 18, shadow tier 3) at x 150–1260, y 170–880 — it shows the page title and the queue card only; the assign card (and its '1 – 9' legend) and the Gmail row stay outside the viewport. Right: UBI ubi/look at 760 px (k 0.844), frame box left 1100 top 170 → body ≈ x 1315–1628, y 303–844, helmet centre ≈ (1479, 426); ember orb behind him (0.08). UBI never overlaps the UI (viewport ends at x 1260).

**Assets.**
- ui: `ui/review-queue-selected.png` — hotspots: `page-title`, `queue-card`, `queue-list`, `queue-row-2`, `queue-row-3`. Shows the queue: 'Sem categoria' / 'pendente' rows. queue-row-active-hint ('Pressione 1 a 9…') stays ≈ 20 px and under the dim — never legible.
- ubi: `ubi/look`. left (toward the UI) f6–28 → centre → right f44–62 → up-right at the bubble f63–89

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | ui/review-queue-selected.png | `queue-card` | 1.4 | 0.7 | 4/8 | linear | Revisão (page title): 53 × 0.7 = 37.1 px ✓ | focus (1278, 520); anchor (716, 540); Visible capture through the viewport ≈ x 469–2055, y −8–1006: page title (canvas x ≈ 180, y ≈ 235) + queue rows 1–6. Row text 29 px → 20 px: texture. The Gmail row (y ≈ 1040+) is outside the viewport. |
| 89 | ui/review-queue-selected.png | `queue-card` | 1.44 | 0.72 | 4/8 | linear | Revisão (page title): 53 × 0.72 = 38.2 px ✓ | focus (1278, 520); anchor (716, 540); Drift. |

**UBI track.** f0–89 `ubi/look` from index 0

**Motion.** f0 cut. ubi/look index = f. f6–28 UBI turns to the viewer's left — he reads the queue; a spotlight on rows 2–6 (image rect 514, 462, 1529×560 = queue-row-2, queue-row-3 and the three rows below: the 'Sem categoria' / 'pendente' doubts) fades in (dim 0→0.62, 12 f), which also darkens row 1 and its '1 a 9' hint. f30 the whole UI plane racks slightly out (blur 0→2.5 px, SMOOTH 12 f) so the eye goes to UBI and his line. f24 bubble pops, lands f30 (downbeat). f44–62 UBI glances right; f63–89 he looks up-right at his own bubble (clip frames 75–89 hold the glance). f89 cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `pop+1.wav` | 24 (654) | 24 | -14 | Bubble. |

**Reading check.** "Só pergunto o que não sei." (26): 26 chars → max(24, 2×26) = 52 f; held f30→f90 = 60 f → yes

### 10. `s10-uma-tecla` — abs 720–809 (24.00–27.00 s, bar.beat 13.1 → 14.3) · 90 f · features

**Purpose.** One key decides a doubt: press '1' and the group becomes IFRO — and UBI learns the rule.

**Shot types.** S14 keycap press, Module B, S12 focus dimming, UI state change.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **Uma tecla.** | headline | stagger-words | 0 → 9 → 86 | — | Sora 700 104 px, left x 144, cap-top y 236. |
| **E eu aprendo.** | headline | stagger-words | 18 → 30 → 86 | aprendo. | Second line, cap-top y 352. Words f18/f21/f24 → 'aprendo.' lands volt on f30 = abs 750 (beat). Together: facts §6 'Uma tecla. E ele aprende.' in UBI's first person. |

**Visual.** Left column (x 144–760): the two headline lines, and a KeyCap '1' (220 px, top-face gradient #1c2740→#141c2e, Inter 600 84 px legend, rotateX 28° rotateZ −6°) centred at (452, 720). Right: Screen ui/review-queue-selected.png → ui/review-after-assign.png (state swap f17), width 1440, rotateY −10°, rotateX 4°, shown through a rounded viewport (clipPath inset, radius 18, shadow tier 3) at x 860–1860, y 160–920. The viewport keeps keys-legend ('1 – 9 atribuir') out of frame in both states.

**Assets.**
- ui: `ui/review-queue-selected.png` — hotspots: `assign-card`, `assign-card-title`, `assign-picker`, `assign-option-1`, `assign-key-1`. Before: picker with IFRO 1 · Incubadora 2 · Cidades Inteligentes 3 · Distração 4 · Pausa 5.
- ui: `ui/review-after-assign.png` — hotspots: `assign-card`, `assign-picker`, `last-suggestions`, `toast`. After '1': 'Da última decisão — Criar regra: Sempre: Calendário → IFRO' chips appear (the learning).

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | ui/review-queue-selected.png | `assign-option-1` | 2.5 | 1.25 | 4/-10 | hard | IFRO (assign-option-1): 29 × 1.25 = 36.2 px ✓ | focus (2458, 480); anchor (1360, 540); Visible capture y ≈ 176–784 → assign-card-title + picker; keys-legend (y 778+) cut by the viewport. Upsample 1.25 (flagged, row is under the spotlight). |
| 60 | ui/review-after-assign.png | `assign-card` | 2.5 | 1.25 | 4/-10 | E.glide | Incubadora · Cidades Inteligentes · Distração · Pausa (picker rows still in frame): 29 × 1.25 = 36.2 px ✓ | travel 30 f; focus (2458, 700); anchor (1360, 540); Span f30→60 (abs 750→780). Visible y ≈ 396–1004: picker rows 2–5 + the 'Sempre: … → IFRO' chips (25 px capture → 31 px, supporting texture; the headline carries the message). keys-legend moved to y ≈ 1010+ in this capture → out. |
| 89 | ui/review-after-assign.png | `assign-card` | 2.55 | 1.275 | 4/-10 | linear | Incubadora · Cidades Inteligentes · Distração · Pausa (picker rows still in frame): 29 × 1.275 = 37.0 px ✓ | focus (2458, 700); anchor (1360, 540); Drift. |

**Motion.** f0 (abs 720 downbeat) hard cut; KeyCap rises y 40→0 (SNAPPY, lands f6); 'Uma tecla.' f0–9; spotlight on assign-option-1 (dim 0.45). Press C = f15 (abs 735, beat): f12–15 cap down +14 px, skirt 18→4 (E.exit); at C legend turns volt + underglow 0.55 decaying 8 f; the IFRO row flashes volt 20 % for 8 f and a ripple leaves assign-key-1; release f16–23 (SNAPPY). f17 crossfade 6 f to review-after-assign (subtitle 'Finder, 3min'; chips slide in y +8→0). f18–30 'E eu aprendo.' lands. f30–60 spotlight + camera glide down to last-suggestions. f86–89 whip-out.

**Transitions.** In: cut (0 f). Out: whip-left (4 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `key_down_1.wav` | 15 (735) | 15 | -10 |  |
| `success_chime_1.wav` | 18 (738) | 18 | -14 | C6→E6, the row is classified. |
| `key_up_1.wav` | 21 (741) | 21 | -18 |  |

**Reading check.** "Uma tecla." (10) + "E eu aprendo." (13): 23 chars → max(24, 2×23) = 46 f; held f30→f86 = 56 f → yes

### 11. `s11-um-clique-vira-memoria` — abs 810–899 (27.00–30.00 s, bar.beat 14.3 → 16.1) · 90 f · features

**Purpose.** What UBI already got right, you confirm in one click — and it becomes memory for tomorrow.

**Shot types.** T2 whip-in, S03 word stagger, S12 push-in + focus dim, S13 cursor click + state change, S11 glide.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **Um clique vira memória.** | headline | stagger-words | 3 → 15 → 90 | memória. | Facts §3.6 headline, verbatim. Sora 700 96 px, one line ≈ 1130 px, left x 144, cap-top y 110; 2 f stagger from f3 → 'memória.' lands volt on f15 = abs 825 (beat). Canvas scrim 0–280 px. |

**Visual.** Module A. Screen ui/review-settled-expanded.png → ui/review-confirmed.png (swap f32), width 1440, rotateX 4°, ry −4°, float 4. Vector re-set of the 'Confirmar os 19' button (outlined, radius 12, Inter 600 26 image-px ink) pinned over confirm-all-button so the label is crisp at 1.4×. After the click the confirm bar collapses and every origin chip ('IA' / 'memória' / 'regra') becomes a mint 'você' chip with a 100 % bar; the toast '19 grupos confirmados — 21 blocos viraram memória: o Ubi resolve sozinho da próxima vez.' rises in the window's bottom-right corner.

**Assets.**
- ui: `ui/review-settled-expanded.png` — hotspots: `settled-header`, `confirm-bar`, `confirm-all-button`, `reviewed-row-1`. '19 grupos foram o Ubi que decidiu.' + 'Confirmar os 19'.
- ui: `ui/review-confirmed.png` — hotspots: `reviewed-list`, `reviewed-row-1`, `toast`. All rows 'você', toast.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | ui/review-settled-expanded.png | `confirm-bar` | 1.8 | 0.9 | 4/-4 | E.push | — | focus (1278, 560); anchor (960, 640); Overview: settled header, confirm bar, first rows with category + origin chips (texture at 0.9×). |
| 24 | ui/review-settled-expanded.png | `confirm-all-button` | 2.8 | 1.4 | 4/-4 | E.push | Confirmar os 19 (vector re-set): 26 × 1.4 = 36.4 px ✓ | travel 18 f; focus (1898, 478); anchor (1250, 640); Push f6→24. Visible capture x ≈ 1005–2377, y ≈ 21–792: the assign card's '1 – 9' legend part (x ≈ 2555+) stays out. Bitmap upsample 1.4 around the crisp vector button, under a 0.5 dim. |
| 75 | ui/review-confirmed.png | `toast` | 2.5 | 1.25 | 4/-6 | E.glide | 19 grupos confirmados (toast title): 29 × 1.25 = 36.2 px ✓ | travel 30 f; focus (2508, 1688); anchor (1250, 560); Span f45→75 (abs 855→885); the path crosses the chip column so the mint 'você' chips stream past. Toast framed in the window's bottom-right corner (window edge visible at x ≈ 1715, y ≈ 700). Upsample 1.25 (flagged). |
| 89 | ui/review-confirmed.png | `toast` | 2.55 | 1.275 | 4/-6 | linear | 19 grupos confirmados (toast title): 29 × 1.275 = 37.0 px ✓ | focus (2508, 1688); anchor (1250, 560); Drift. |

**Cursor.** f6 `rest:1640,900` (fade in 6 f at rest (screen space)) → f26 `confirm-all-button` (arrives 4 f early (E.cursor, 20 f, arc 0.12); hover state from f24) → f30 `confirm-all-button` **click** (click C = abs 840 (downbeat): cursor 1→0.85→1, button 0.96, ripple 0→44 px) → f40 `rest:1700,820` (drifts off and fades (hideAt 40))

**Motion.** f0–3 whip-in. f3–15 headline. f6–24 push to the button (E.push). f24 hover. Click f30. f32 state change: confirm bar collapses (96→0, 8 f, E.exit) as rows slide up 94 image-px (SNAPPY); top-to-bottom reveal of review-confirmed over 12 f: each row's origin chip flips to mint 'você' with a mint 12 % row flash (2 f stagger, spread ≤ 20 f). f40 toast rises (y +24→0, SNAPPY). f45–75 glide to the toast; f75–89 hold with drift. f89 cut.

**Transitions.** In: whip-left (4 f). Out: cut (0 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `whip_2.wav` | 0 (810) | -2 | -8 | Master-track cue: loudest point on abs 810; file starts in s10. |
| `click.wav` | 30 (840) | 30 | -12 |  |
| `success_chime_2.wav` | 33 (843) | 33 | -14 | C6→G6. |
| `ui_pop_2.wav` | 40 (850) | 40 | -16 | Toast. |

**Reading check.** "Um clique vira memória." (23): 23 chars → max(24, 2×23) = 46 f; held f15→f90 = 75 f → yes

### 12. `s12-nada-em-duvida` — abs 900–989 (30.00–33.00 s, bar.beat 16.1 → 17.3) · 90 f · features

**Purpose.** Payoff: the queue is empty — UBI steps out of the app, thrilled.

**Shot types.** S11 push-in, T5 match-cut inside the shot (in-app UBI → 3D UBI), S19 mascot (excited), rack focus, speech bubble.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **Nada em dúvida!** | headline | static | 39 → 45 → 84 | dúvida! | The app's own empty-state line (review.ts:83). Bubble Sora 700 72 px ink, 'dúvida!' volt; box ≈ x 1000–1620, y 190–330, tail to the helmet. It grows out of the in-app bubble's position (scale/translate 12 f) and lands f45 = abs 945 (beat). |

**Visual.** Screen ui/review-done.png, width 1440, near-flat (rx 2°, ry −2°): the empty queue with the in-app UBI, 'Nada esperando por você' and 'O Ubi resolveu 29 grupos neste dia.' From f30 the in-app UBI is replaced by the 3D UBI at exactly the same size/place (body ≈ x 890–1029, y 431–679 → ubi frame 351 px at left 788 top 368), who then grows to frame 860 px (left 330, top 115 → body ≈ x 580–880, y 270–877) while the app behind racks out (blur 0→8 px, brightness 1→0.45, SMOOTH 12 f). From f30 a soft patch in the empty-state card colour (image space, over review-done-ubi incl. its floor glow and small bubble) hides the bitmap UBI so only one UBI ever exists. Volt rim + ember floor glow on the 3D UBI.

**Assets.**
- ui: `ui/review-done.png` — hotspots: `review-done`, `review-done-ubi`, `queue-card`. Empty queue; review-done-ubi is the match target.
- ubi: `ubi/idle-to-excited`. first frame == idle 0 (rest pose, matches the in-app render)
- ubi: `ubi/excited`. loop

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | ui/review-done.png | `review-done` | 1.6 | 0.8 | 2/-2 | E.push | — | focus (1279, 640); anchor (960, 560); Empty state centred; no read yet. |
| 24 | ui/review-done.png | `review-done-ubi` | 2.0 | 1.0 | 2/-2 | E.push | Nada esperando por você (empty-state heading): 38 × 1.0 = 38.0 px ✓ | travel 24 f; focus (1279, 590); anchor (960, 560); Bitmap 1:1. The heading is readable f24–30, then the scene is about UBI. |
| 89 | ui/review-done.png | `review-done-ubi` | 2.0 | 1.0 | 2/-2 | linear | — | focus (1279, 590); anchor (960, 560); Camera holds; UBI and rack-focus carry the motion. |

**UBI track.** f0–29 hidden — in-app bitmap UBI only; f30–40 `ubi/idle-to-excited` from index 0; f41–65 `ubi/excited` from index 11; f66–89 `ubi/excited` from index 0 (loop)

**Motion.** f0 cut (abs 900 downbeat) on the empty queue. f0–24 push to the in-app UBI (E.push). f30 (abs 930, beat) match-cut: 3D UBI takes the in-app UBI's place (idle-to-excited f30–40), the in-app bubble cross-fades into the big bubble. f30–48 UBI grows and slides left (SMOOTH), app racks out behind him. f39 bubble grows, lands f45. f41–89 excited loop (small bounces, pumping arms). f84–89 blur-dissolve out-half.

**Transitions.** In: cut (0 f). Out: blur-dissolve (6 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `shimmer_2.wav` | 30 (930) | 30 | -12 | UBI steps out (C-major pentatonic twinkle). |
| `pop+2.wav` | 39 (939) | 39 | -14 | Bubble. |

**Reading check.** "Nada em dúvida!" (15): 15 chars → max(24, 2×15) = 30 f; held f45→f84 = 39 f → yes

### 13. `s13-relatorio-sai-pronto` — abs 990–1109 (33.00–37.00 s, bar.beat 17.3 → 19.3) · 120 f · features

**Purpose.** At 18:00 the day's report is written for you, per category, without anyone asking.

**Shot types.** T6 blur dissolve (breather), S07 counter (clock), S09 hero reveal (rise + tilt-to-flat), Module B, S12 focus dimming.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **18:00** | hero | static | 0 → 30 → 120 | — | Clock, Sora 700 240 px tabular-nums, centred (960, 480), roll '17:55' → '18:00' landing f30 = abs 1020 (downbeat); volt for one beat on land; f30–60 docks to the left column (scale 0.4 → 96 px, x 144, cap-top y 250, E.glide). 18:00 is the default report time (facts §4). |
| **O relatório sai pronto.** | headline | stagger-words | 45 → 60 → 120 | pronto. | Facts §3.8 headline verbatim. Sora 700 96 px, two lines 'O relatório' / 'sai pronto.' left x 144, cap-top y 400; words f45/48/51/54 → 'pronto.' lands volt on f60 = abs 1050 (beat). |

**Visual.** Background orbs (volt TL 0.16, ember BR 0.06), no grid — calmer. Right plane: Screen ui/reports.png, width 1440, in a rounded viewport x 860–1860, y 140–940 (radius 18, shadow tier 3), rotateY −10°, rotateX 4°; it rises f30–75 (translateY 260→0, rotateX 32°→4°, opacity 0→1 in 6 f, E.push 45 f). The viewport shows the IFRO report's 'Resumo do dia — IFRO', the summary and 'Destaques' chips. report-1-meta (it names a model version) is kept out of the viewport (visible capture y starts ≈ 523).

**Assets.**
- ui: `ui/reports.png` — hotspots: `report-card-1`, `report-1-summary`, `report-1-summary-heading`, `report-1-highlights`. Never frame report-1-meta (model name, facts §5.18). reports-generated.png is not used (privacy).

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 30 | ui/reports.png | `report-1-summary` | 2.14 | 1.07 | 4/-10 | E.push | Resumo do dia — IFRO: 34 × 1.07 = 36.4 px ✓ | focus (987, 900); anchor (1360, 540); Plane rises into this framing f30–75. Visible capture x ≈ 520–1454, y ≈ 523–1277. Upsample 1.07 (≤ 1.15). |
| 119 | ui/reports.png | `report-1-summary` | 2.2 | 1.1 | 4/-10 | linear | — | focus (987, 900); anchor (1360, 540); Drift. |

**Motion.** f0–5 blur-dissolve in-half (blur 16→0, opacity 0→1, scale 1.03→1). f0–30 clock minutes roll 55→00 (hour flips 17→18 on the last step), land f30 with pulse + volt. f30–60 clock docks left while the Reports window rises (two primary moves, one gesture). f45–60 headline. f75 spotlight on report-1-highlights (dim 0.4, 12 f). f80 sheen across the glass. f119 cut.

**Transitions.** In: blur-dissolve (6 f). Out: cut (0 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `ui_tick_2.wav` | 6 (996) | 6 | -22 |  |
| `ui_tick_2.wav` | 12 (1002) | 12 | -22 |  |
| `ui_tick_2.wav` | 18 (1008) | 18 | -22 |  |
| `ui_tick_2.wav` | 24 (1014) | 24 | -22 |  |
| `ding_2.wav` | 30 (1020) | 30 | -12 | C6 bell as 18:00 lands. |
| `swoosh_long_3.wav` | 48 (1038) | 30 | -12 | Window rise. |

**Reading check.** "18:00" (5) + "O relatório sai pronto." (23): 28 chars → max(24, 2×28) = 56 f; held f60→f120 = 60 f → yes

### 14. `s14-nao-foque` — abs 1110–1199 (37.00–40.00 s, bar.beat 19.3 → 21.1) · 90 f · features

**Purpose.** Focus that defends itself: open a blocked site and UBI closes it — with attitude.

**Shot types.** S06 window slam (centred subject), rack-focused backdrop, S13 cursor click.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **BLOQUEIOS** | kicker | mask-up | 0 → 6 → 90 | — | Inter 600 26 px +0.14em volt, centred y 200. 'Bloqueios' = the app's list of blocked apps/sites. |
| **Não! Foque na sua produtividade.** | ui-caption | static | 0 → 4 → 90 | — | UBI's real line (focus.ts:128), read straight from the captured window: 52 px capture (DPR 4) × 0.826 = 43 px on screen. |

**Visual.** Backdrop: Screen ui/focus.png (width 1440) framed on the Bloqueios card (YouTube, Instagram, Discord), blur 6 px, brightness 0.45. Subject: ui/intervention.png as a floating window (Screen chrome 'none', imageSize 1840×752, width 1520 → s0 0.826) at x 200–1720, y 262–883, shadow tier 3, its own rounded alpha corners; its built-in UBI, 'YouTube' badge and 'Ok, foco!' button all readable (badge 44 px capture → 36 px, button ≈ 48 → 40 px).

**Assets.**
- ui: `ui/intervention.png` — hotspots: `panel`, `message`, `target-badge`, `ok-button`, `ubi`. Aviso do UBI window, DPR 4.
- ui: `ui/focus.png` — hotspots: `targets-card`, `target-2`. Blurred backdrop only.

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | ui/focus.png | `targets-card` | 1.2 | 0.6 | 4/-6 | linear | — | layer backdrop; focus (1278, 1072); anchor (960, 560); BACKDROP layer, blurred 6 px, brightness 0.45; not a read. |
| 0 | ui/intervention.png | `message` | 1.0 | 0.826 | 0/0 | SLAM | Não! Foque na sua produtividade.: 52 × 0.826 = 43.0 px ✓ | layer subject; Window on screen at f0 at scale 1.04 → 1 (SLAM, contact f4), 4 px shake decaying 6 f. Also legible: 'YouTube' badge 44 px capture → 36.3 px, 'Ok, foco!' ≈ 46 → 38 px. |
| 89 | ui/intervention.png | `message` | 1.04 | 0.859 | 0/0 | E.glide | — | layer subject; Slow push on the window; backdrop drifts −12 px. |

**Cursor.** f45 `rest:1560,950` (fade in 6 f) → f71 `ok-button` (arrive 4 f early; button hover) → f75 `ok-button` **click** (click C = abs 1185 (beat); button 0.96 pressed, volt ripple; the window stays up through the cut)

**Motion.** f0 (abs 1110, beat 3 of bar 19) hard cut: window slams (SLAM 4 f) over the blurred Focus page; kicker masks up f0–6. f0–89 push 1.00→1.04. f45 cursor fades in, arcs to 'Ok, foco!' (E.cursor), click f75. f89 cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `impact_soft_2.wav` | 0 (1110) | 0 | -2 | Window slam; bed ducks −5 dB. |
| `glitch_2.wav` | 2 (1112) | 2 | -18 | The blocked tab is gone. |
| `click.wav` | 75 (1185) | 75 | -12 |  |

**Reading check.** "BLOQUEIOS" (9) + "Não! Foque na sua produtividade." (32): 41 chars → max(24, 2×41) = 82 f; held f6→f90 = 84 f → yes

### 15. `s15-ou-deixe-comigo` — abs 1200–1319 (40.00–44.00 s, bar.beat 21.1 → 23.1) · 120 f · proof

**Purpose.** Your AI, your choice: Claude, OpenAI or Grok with your own key — or let UBI handle it (IA do Ubi).

**Shot types.** Module B mirrored, S11 zoom-to-element, S13 cursor click + state change, S19 mascot entrance, UBI 'yes', speech bubble.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **Claude, OpenAI ou Grok.** | headline | stagger-words | 0 → 12 → 60 | — | Provider names as plain text only — no logos, no "powered by". Sora 700 96 px, one line ≈ 1130 px, left x 144, cap-top y 120; 2 f stagger → lands f12; masked exit f60–68. |
| **Ou deixe comigo.** | headline | static | 66 → 72 → 120 | comigo. | UBI's bubble — first person of facts §6 'Ou deixe com o Ubi.' (the managed 'IA do Ubi'). Sora 700 64 px, box ≈ x 1180–1776, y 230–360, tail to the helmet (≈ 1579, 467); lands f72. |

**Visual.** Screen ui/settings-ai.png → ui/settings-ai-ubi.png (swap f62), width 1440, rotateY +6°, rotateX 4°, seen through a rounded viewport x 120–1500, y 300–900 that keeps model-fields (model names) out of frame. UBI bottom-right: frame 600 px (k 0.667) at left 1280, top 360 → body ≈ x 1455–1697, y 468–891 — right of the picker, covering nothing.

**Assets.**
- ui: `ui/settings-ai.png` — hotspots: `section-ia`, `provider-picker`, `provider-anthropic`, `provider-openai`, `provider-xai`, `provider-ia-do-ubi`. Picker: 'Anthropic Claude · OpenAI · xAI Grok · IA do Ubi'. model-fields never framed (facts §5.18).
- ui: `ui/settings-ai-ubi.png` — hotspots: `provider-picker`, `provider-ia-do-ubi`, `provider-active`. 'IA do Ubi' active; the key form becomes the licence note.
- ubi: `ubi/yes-from-idle`. two nods, spliced from idle
- ubi: `ubi/idle`. after the splice

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | ui/settings-ai.png | `provider-picker` | 2.5 | 1.25 | 4/6 | hard | Anthropic Claude · OpenAI · xAI Grok · IA do Ubi: 29 × 1.25 = 36.2 px ✓ | focus (1583, 440); anchor (800, 600); Visible capture x ≈ 1039–2143, y ≈ 200–680 through the viewport. Upsample 1.25 (flagged). |
| 119 | ui/settings-ai-ubi.png | `provider-ia-do-ubi` | 2.55 | 1.275 | 4/6 | linear | IA do Ubi (active provider pill): 29 × 1.275 = 37.0 px ✓ | focus (1583, 440); anchor (800, 600); Drift. |

**Cursor.** f24 `rest:1450,860` (fade in 6 f) → f56 `provider-ia-do-ubi` (arrive 4 f early; pill hover) → f60 `provider-ia-do-ubi` **click** (click C = abs 1260 (downbeat)) → f72 `rest:1180,760` (slides away, fades (hideAt 76))

**UBI track.** f0–58 hidden; f59–106 `ubi/yes-from-idle` from index 0; f107–119 `ubi/idle` from index 11

**Motion.** f0 (abs 1200 downbeat) cut; headline f0–12. Spotlight on provider-picker (dim 0.5). f24 cursor fades in, travels, click f60. f60–68 headline exits up (mask). f62 crossfade 6 f to settings-ai-ubi: the volt 'active' state jumps to 'IA do Ubi'. f59 UBI enters (S19: y +320→0, rotate −10°→0, scale 0.7→1, BOUNCY_SUBTLE, lands f66); yes-from-idle f59–106 (nods ≈ f70–97); f66 bubble pops, lands f72. f107–119 idle. f119 cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `click.wav` | 60 (1260) | 60 | -12 |  |
| `success_chime_3.wav` | 63 (1263) | 63 | -14 | G5→D6. |
| `bloop_1.wav` | 66 (1266) | 66 | -12 | UBI lands. |

**Reading check.** "Claude, OpenAI ou Grok." (23): 23 chars → max(24, 2×23) = 46 f; held f12→f60 = 48 f → yes | "Ou deixe comigo." (16): 16 chars → max(24, 2×16) = 32 f; held f72→f120 = 48 f → yes

### 16. `s16-end-card` — abs 1320–1499 (44.00–50.00 s, bar.beat 23.1 → 26.1) · 180 f · cta

**Purpose.** Land the brand: UBI drops onto the lockup; tagline, download CTA and platforms hold to the last frame.

**Shot types.** S22 end card, S19 mascot (jump landing), S04 masked line, S13 cursor click on the CTA, S20 platform row.

| copy (PT-BR, exact) | role | mode | in → land → out | emphasis | notes |
|---|---|---|---|---|---|
| **ubiqX AI** | hero | slam | 0 → 0 → 180 | X | Wordmark SVG at 150 px font (ink ≈ 145×437) + 'AI' pill (Sora 600 52 px); lands on the final hit (scale 1.06→1, SMOOTH + masked rise). |
| **Retome o controle do seu dia.** | sub | mask-up | 4 → 12 → 180 | — | Tagline (facts §6). Inter 500 48 px ink-2, centred, cap-top y 700. |
| **Baixe em ubiqx.com.br** | label | static | 8 → 14 → 180 | — | CTA pill: Sora 600 44 px, #0a0d16 on volt (6.1:1), radius 999, padding 22×44, centred at (960, 840); pops f8 (BOUNCY_SUBTLE). Download, not buy (checkout not live). |
| **macOS · Windows · Linux** | label | mask-up | 12 → 18 → 180 | — | Inter 500 36 px ink-2 with 32 px monochrome glyphs (brand/os apple, windows, linux) before each name, centred, cap-top y 918 (bottom 120 px stays clear). Rise 14 f E.enter, lands f18. |

**Visual.** Background 'grid' at 20 %, volt orb breathing (scale 1↔1.05 over 120 f), ember orb bottom-left 0.06. Centred lockup (echo of s05): UBI left (frame 620 px, k 0.689, frame box left 343 top 90 → body ≈ x 519–775, y 202–639), wordmark ≈ x 831–1268 (row centre y 420), 'AI' pill ≈ x 1286–1402. Below: tagline, CTA pill, platform row. Everything inside title-safe; the last frame IS the end card (loop-safe, no fade).

**Assets.**
- brand: `brand/ubiqx-wordmark-bold.svg`. Wordmark; X volt.
- brand: `brand/os/apple.svg`. macOS glyph (monochrome).
- brand: `brand/os/windows.svg`. Windows glyph (monochrome).
- brand: `brand/os/linux.svg`. Linux glyph (monochrome).
- ubi: `ubi/jump`. landing half only (index 15→30)
- ubi: `ubi/idle`. segment ends on idle 119 for the wave splice
- ubi: `ubi/wave-from-idle`. goodbye wave

**Camera.**

| at | file | target | zoom | s (upsample) | tilt rx/ry | ease | read target → on screen | note |
|---|---|---|---|---|---|---|---|---|
| 0 | — | `canvas` | 1.0 | — | 0/0 | SMOOTH | — |  |
| 179 | — | `canvas` | 1.02 | — | 0/0 | linear | — | Barely-there drift keeps it alive; text rests at scale 1 (whole-pixel positions). |

**Cursor.** f40 `rest:1500,990` (fade in 6 f) → f56 `comp:960,840` (CTA pill centre; hand cursor on hover) → f60 `comp:960,840` **click** (click C = abs 1380 (downbeat): pill 0.96 + ripple) → f100 `comp:1060,900` (drifts off and fades (hideAt 100) so the final frame is clean)

**UBI track.** f0–5 hidden; f6–21 `ubi/jump` from index 15 — apex→land(index 24 on f15)→settle; f22–89 `ubi/idle` from index 52 — 68 f ending exactly on idle 119; f90–160 `ubi/wave-from-idle` from index 0; f161–179 `ubi/idle` from index 11

**Motion.** f0 (abs 1320, final hit) cut: wordmark lands (SMOOTH + masked rise), glint f10–28. f4 tagline, f8 CTA pop, f12 platform row — all landed by f18 (spread 12 f). UBI drops in from above: hidden f0–5; ubi/jump 15→30 over f6–21 with a y-offset −300→0 px (E.exit = gravity) so the contact (index 24) lands on f15 = abs 1335 (beat), 3 % squash, bloop. f22–89 idle 52→119; f60 cursor clicks the CTA (downbeat); f90 (abs 1410 downbeat) goodbye: wave-from-idle 0→70 (f90–160); f161–179 idle 11→29. Orb breathes throughout; last frame = full lockup.

**Transitions.** In: cut (0 f) — Hard cut on the final hit — the lockup must be crisp on its first frame. Out: cut (0 f) — Last frame of the film; no fade (X loops).

| SFX | hit at f (abs) | file starts f | gain dB | note |
|---|---|---|---|---|
| `impact_deep_1.wav` | 0 (1320) | 0 | +0 | Final hit; bed ducks −4 dB. |
| `shimmer_3.wav` | 10 (1330) | 10 | -14 | Glint. |
| `bloop_1.wav` | 15 (1335) | 15 | -12 | UBI lands on the lockup. |
| `click.wav` | 60 (1380) | 60 | -12 | CTA click. |

**Reading check.** "ubiqX AI" (8) + "Retome o controle do seu dia." (29) + "Baixe em ubiqx.com.br" (21) + "macOS · Windows · Linux" (23): 81 chars → max(24, 2×81) = 162 f; held f18→f180 = 162 f → yes

## Compliance notes

- **Copy provenance (facts.md):** “Controle de tempo automático com IA.” §1 · “Oi, eu sou o UBI.” §6 real line · “Você trabalha. Eu anoto.”, “Só pergunto o que não sei.”, “Uma tecla. E eu aprendo.”, “Ou deixe comigo.” are UBI's first-person versions of the §6 phrase bank · “Um clique vira memória.” §3.6 · “O relatório sai pronto.” §3.8 · 18:00 = default report time §4 · “Nada em dúvida!” and “Não! Foque na sua produtividade.” are real app lines §6 · “Claude, OpenAI ou Grok.” §3.13 (names as plain text, no logos) · tagline, CTA, platform line §6. The trailer's lines are not reused (tagline and CTA excepted).
- **Claims avoided (§5):** no user numbers, no offline/local-AI/encryption claims, no 'grátis', no 'Assine', no provider logos or 'powered by', no model names, no '1–9' in copy (only a '1' key is pressed; the UI's own '1 – 9' legend and hint are kept out of frame, cropped by viewports or dimmed ≤ 20 px), no quantified gains. Demo numbers in the UI are shown as UI, never zoomed as results.
- **Privacy:** timeline*.png and reports-generated.png are not used at all; no Gmail/e-mail row is ever zoomed (s09's viewport ends above the mail.google.com row). report-1-meta and model-fields (model names) are outside every viewport. og.png and onboarding captures (their footers say 'Tudo fica no seu Mac') are not used.
- **Readability:** every camera key with a read target puts it at ≥ 36 px (arithmetic in each camera row: capture font px × 0.5 × zoom). Bitmap upsampling > 1.15 happens only where the read label is live vector text (s08 field label + typing, s11 'Confirmar os 19') or ≤ 1.25 under a focus dim (s10 IFRO, s11 toast, s15 provider pills).
- **Validation:** `validate_storyboard.py` (next to this file) checks parsing, exact tiling, beat-grid starts, transition pairing (≤ 6 types), the reading rule per copy group, banned copy patterns, UI files + hotspot names against ui-manifest.json, UBI clip dirs and index ranges against ubi-manifest.json, SFX files against sfx-manifest.json, the ≤ 3-overlapping-SFX budget and the music map's bar coverage. Result for this draft: VALID.

