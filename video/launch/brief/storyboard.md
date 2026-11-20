# ubiqX AI — “O que você fez na terça?” (launch film, final)

> Shooting script. Machine-readable twin: `storyboard.json` (the JSON is the source of truth; this file is generated from it). Validate with `python3 tools/validate-storyboard.py`.

**1560 frames = 52.0 s = 26 bars** · 1920×1080 · 30 fps · 120 BPM (beat 15 f, bar 60 f) · 18 scenes · 31 shots · PT-BR on screen, no voice-over, built to work muted.

## The film in one breath

Accountability hook → silence → brand drop → the real app answers Tuesday, one loop step at a time (registra, classifica, revisa, entrega, foco, a sua IA) → trust breather → end card. UBI goes into the app on the drop and steps back out when the review queue is empty. The Hoje, Revisão and Relatórios captures are all dated “Terça-feira, 29 de setembro”, so the product answers the hook’s own question.

Frame 0 is a finished poster: *“SEXTA-FEIRA · 17:00 — O que você fez na terça?”* over an empty timesheet that slams flat. Windows fly past while *“Só um minutinho.”* becomes **40 min**, *“Chutar as horas?”* gets struck through, and one volt caret is left blinking in half a second of total silence. On the drop the caret becomes the **X** of **ubiqX AI**, UBI leaps in beside the wordmark, and *“Controle de tempo automático com IA.”* lands at 8.5 s. UBI shrinks straight into the real **Hoje** screen; the camera pulls back and Tuesday’s day track fills itself up to 18h (*“Ele registra. Você trabalha.”*). Then the loop, on the real UI and on the beat: *“Suas categorias.”* typed live · *“Regras, / memória, / IA.”* slammed onto real origin badges · *“Só pergunta o que não sabe.”* · a keycap **1** pressed alone in a band stop, flying into the app · *“Uma tecla. E ele aprende.”* · one click on **Confirmar os 19** turning the list mint · UBI jumping out of the empty queue: *“Nada em dúvida!”* · **18:00** → *“O relatório sai pronto.”* → **Markdown copiado** · the focus guard’s own window · *“Claude, OpenAI ou Grok. / Ou deixe com o Ubi.”*. A warm breather proves privacy with the app’s real mask token (**ana@example.com → [email]**), builds, and the final hit lands the end card: *“Retome o controle do seu dia.”* · **Baixe em ubiqx.com.br** · macOS · Windows · Linux, with UBI dropping in to wave.

## Beat sheet

| # | Scene | Frames (abs) | Time | Bar.beat | Act | On-screen copy | In → Out |
|---|---|---|---|---|---|---|---|
| 1 | `s01-hook-terca` | 0–89 (90 f) | 00:00.00 | 1.1 | hook | SEXTA-FEIRA · 17:00 / O que você fez na terça? | cut → cut |
| 2 | `s02-so-um-minuto` | 90–179 (90 f) | 00:03.00 | 2.3 | problem | “Só um minutinho.” / 40 min | cut → cut |
| 3 | `s03-planilha-de-memoria` | 180–224 (45 f) | 00:06.00 | 4.1 | problem | Chutar as horas? | cut → cut |
| 4 | `s04-silencio` | 225–239 (15 f) | 00:07.50 | 4.4 | problem | — | cut → flash |
| 5 | `s05-drop-ubiqx` | 240–359 (120 f) | 00:08.00 | 5.1 | reveal | ubiqX AI / Controle de tempo automático com IA. | flash → match-cut |
| 6 | `s06-ele-registra` | 360–449 (90 f) | 00:12.00 | 7.1 | features | Ele registra. Você trabalha. | match-cut → whip-left |
| 7 | `s07-as-suas-categorias` | 450–509 (60 f) | 00:15.00 | 8.3 | features | Suas categorias. | whip-left → cut |
| 8 | `s08-regras-memoria-ia` | 510–584 (75 f) | 00:17.00 | 9.3 | features | Regras, / memória, / IA. | cut → cut |
| 9 | `s09-so-pergunta` | 585–659 (75 f) | 00:19.50 | 10.4 | features | Só pergunta o que não sabe. | cut → cut |
| 10 | `s10-tecla-1` | 660–689 (30 f) | 00:22.00 | 12.1 | features | 1 | cut → match-cut |
| 11 | `s11-e-ele-aprende` | 690–779 (90 f) | 00:23.00 | 12.3 | features | Uma tecla. / E ele aprende. | match-cut → cut |
| 12 | `s12-um-clique` | 780–869 (90 f) | 00:26.00 | 14.1 | features | Confirmar os 19 / Um clique vira memória. | cut → cut |
| 13 | `s13-nada-em-duvida` | 870–929 (60 f) | 00:29.00 | 15.3 | features | Nada em dúvida! | cut → cut |
| 14 | `s14-relatorio` | 930–1034 (105 f) | 00:31.00 | 16.3 | features | 18:00 / O relatório sai pronto. / Markdown copiado | cut → cut |
| 15 | `s15-foco` | 1035–1124 (90 f) | 00:34.50 | 18.2 | features | Não! Foque na sua produtividade. / Ok, foco! | cut → whip-left |
| 16 | `s16-sua-ia` | 1125–1229 (105 f) | 00:37.50 | 19.4 | proof | Claude, OpenAI ou Grok. / Ou deixe com o Ubi. | whip-left → blur-dissolve |
| 17 | `s17-privacidade` | 1230–1379 (150 f) | 00:41.00 | 21.3 | proof | Seus dados ficam com você. / A IA vê só o mínimo. / ana@example.com / [email] | blur-dissolve → cut |
| 18 | `s18-end-card` | 1380–1559 (180 f) | 00:46.00 | 24.1 | cta | ubiqX AI / Retome o controle do seu dia. / Baixe em ubiqx.com.br / macOS · Windows · Linux | cut → cut |

Act lengths: hook 90 f (6 %) · problem 150 f (10 %) · reveal 120 f (8 %) · features 765 f (49 %) · proof 255 f (16 %) · cta 180 f (12 %).

Transition types (5): blur-dissolve, cut, flash, match-cut, whip-left. Hard cuts on ≥ 50 % of boundaries; two whips, both to the left; one flash (the drop); one blur dissolve (into the breather); two match cuts between scenes (UBI into the app, keycap into the chip) plus the in-scene UBI match in s13.

## Synthesis: what came from where

**Base.** Draft A (top summed score, cleanest claims) is the spine: problem → silence → drop → the real loop in order → proof → end card, A’s caret across the silence, A’s same-camera “Confirmar os 19” swap, A’s 18:00 report bookend and A’s “[email]” mask proof.

**Grafts.**

- C s01 hook copy “SEXTA-FEIRA · 17:00 / O que você fez na terça?” replaces A’s 08:00→18:00 draining-day hook (A’s fatal trailer echo); B’s tilt-to-flat slam gives it motion inside the first second.
- C s05 drop lockup with UBI leaping in on beat 2, and C’s match-cut of UBI into the in-app UBI on Hoje (anchored at comp x 1560 so the first frame is inside the app).
- C s08 categories typing, recaptioned with the §3.3 headline, set in Brazilian register as “Suas categorias.”.
- B s04 slam stack on real origin badges, with the badge hops as jump-cuts on the beat.
- B s05–s07 band stop: keycap “1” pressed alone on beat 2, then the keycap → assign-key-1 match with the group leaving the queue.
- B s09 “Nada em dúvida!”: UBI jumps out of the empty queue (B’s index-6 timing so contact lands on the downbeat), bubble grown from the in-app bubble (C).
- B s10 “Markdown copiado” (the app’s real toast string) added to A’s 18:00 report beat.
- C s14 single 3-s intervention slam replaces B’s two focus scenes.
- B s15 privacy breather with UBI’s look clip, carrying A’s s14 “ana@example.com → [email]” proof and the CD’s line “Seus dados ficam com você. A IA vê só o mínimo.”.
- C s16 end card with UBI dropping onto the lockup on the beat after the final hit, re-spliced rest-pose to rest-pose (raw jump → raw wave → idle 0).

**Dropped on purpose.**

- A’s zoom-54 X zoom-through (27× bitmap, production fatal): the UBI match-cut is the single brand → product bridge, so the film has one coherent bridge instead of two.
- A’s standalone OS-row scene (the platforms live only on the end card).
- B’s “Você não deu play.” hook (cryptic, brand at 40 s) and B’s tabletop wall + recap montage (no room inside 52 s; the build into the final hit is the breather’s accelerating push + riser).
- C’s first-person headlines and trailer-kicker echoes; UBI speaks only real app lines.
- Any cursor click on the end-card CTA (flagged as gimmicky).

## UBI moments (3 + the end card)

| Scene | Clip(s) | Beat |
|---|---|---|
| `s05-drop-ubiqx → s06-ele-registra` | ubi/jump-from-idle → ubi/idle | Leaps into the lockup on the drop (contact on beat 2), then shrinks INTO the real Hoje screen (match-cut). |
| `s13-nada-em-duvida` | ubi/jump-from-idle → ubi/idle | Steps OUT of the empty review queue and lands on the downbeat: “Nada em dúvida!” (real line). |
| `s17-privacidade` | ubi/idle → ubi/look | Watches the address collapse into “[email]”, then looks up at “A IA vê só o mínimo.”. |
| `s18-end-card` | ubi/jump → ubi/wave → ubi/idle | Drops onto the lockup on the beat after the final hit and waves goodbye. |

## Music map (the track is composed to this)

- **Key:** A minor (relative of C major): the risers glide to A4, shimmers and sweeps are C-major pentatonic, dings are C6/E6 and chimes C6→E6/G6 — all diatonic here.
- **Tempo:** 120 BPM, 4/4, 26 bars; a downbeat hit on frame 0.
- **Harmony:** Am – F – C – G (one chord per bar) in the grooves; Am(add9) under the problem act and on the final hit, resolving to C on the end card; breakdown Fmaj7 → Am9.
- **Style:** Modern electronic / tech-pop: tight four-on-the-floor kick, round sub, plucked A–C–E–G motif, airy pads, no vocals. Leave the frames where SFX hit free of melodic notes.
- **Mix:** Bed nominal 0.5 (−6 dB). Problem act ≈ −18 LUFS short-term (900 Hz low-pass), drop and features ≈ −12 LUFS. Ducks (1 f attack, 2 f hold, 10 f release): −5 dB at 105, 180, 510, 525, 540, 1035; −6 dB at 240; −4 dB at 1380. Master −14 LUFS integrated, ≤ −1 dBTP.
- **Stems:** Full mix; a low-passed copy of bars 1–4 (or the filter automated); the drop without the crash (in case the SFX stack is enough).

| Section | Bars | Frames (abs) | Energy | Feel |
|---|---|---|---|---|
| Terça — under water | 1–4 | 0–239 | 2 | Whole section through a ~900 Hz low-pass: felt kick on 1 and 3, clock-like closed hats in 8ths, Am9 pad, sub on A1. Snare and filter build from bar 3 (the riser SFX sits on top). Stops dead on abs 225 — no reverb tail, no pickup. |
| Drop — ubiqX | 5–6 | 240–359 | 5 | Full range from abs 240: kick, sidechained sub on A1, claps on 2 and 4, open hats, wide supersaw Am chord; the loudest bar of the film. Bar 6 states the pluck motif. |
| Groove A — registra e classifica | 7–11 | 360–659 | 4 | Main groove: kick, clap 2/4, 16th hats, bass on Am–F–C–G, pluck motif. Stop-time stabs (band hits together) under the three slams, crash on the third. Mostly music-only under the typing. |
| Groove B — revisão, com stop-time | 12–15 | 660–899 | 4 | Bar 12 opens with a band STOP: one stab on the downbeat, then only the sub tail and closed-hat 8ths for two beats so the key-down (675) is heard alone; the groove re-enters on 690 with a counter-melody (bell, C-major pentatonic E5–G5–A5). |
| Groove C — entrega e foco | 16–19 | 900–1139 | 4 | Brighter arp, octave bass, ride on quarters; two beats of thinned drums under the clock (930–959); a low synth “no” stab for the intervention window. Same drum pattern, so it feels like a lift, not a new song. |
| Lift — a sua IA | 20–21 | 1140–1259 | 3 | Drums thin to kick + hats, a warm pad enters, filter slightly closed; the drums drop out on beat 3 of bar 21 under the blur dissolve. |
| Respiro — privacidade, then the build | 22–23 | 1260–1379 | 2 | Kick-less: pad Fmaj7 → Am9, soft plucks, sub pulse on quarters — the calmest moment. On 1320 the kick returns on quarters under the riser SFX; snare roll 8ths → 16ths from 1350, HPF + LPF sweep up into the final hit. |
| Final — end card | 24–26 | 1380–1559 | 4 | Final hit on 1380 (kick + sub + crash, Am(add9) resolving to C), then no more drums: one sustained chord with a slow pluck echo; a two-note goodbye (C5 → A4) as UBI waves on 1440; the tail is fully decayed before the last frame. |

**Events the picture needs (abs frames):**

| Frame | Time | Bar.beat | Type | Note |
|---|---|---|---|---|
| 0 | 00:00.00 | 1.1 | downbeat-hit | First downbeat ON frame 0 (the poster): filtered kick + sub + muted Am stab. No silent lead-in. |
| 90 | 00:03.00 | 2.3 | stinger | Reversed-hat pickup lands on the cut to the chip montage (beat 3). |
| 105 | 00:03.50 | 2.4 | stinger | “Só um minutinho.” slam: muted bass stab; bed duck −5 dB. |
| 105 | 00:03.50 | 2.4 | riser-start | riser-2bar.wav starts; its last sample is abs 224. |
| 150 | 00:05.00 | 3.3 | stinger | Counter lands on 40 min: low piano A1 + E2. |
| 180 | 00:06.00 | 4.1 | downbeat-hit | “Chutar as horas?” slam; bed duck −5 dB. |
| 180 | 00:06.00 | 4.1 | filter-sweep | Low-pass opens 900 Hz → 3 kHz by 224; snare 8ths from 180, 16ths from 210. |
| 225 | 00:07.50 | 4.4 | stop | Hard stop of every stem and the riser (2-f fade max). |
| 225 | 00:07.50 | 4.4 | silence (15 f) | Pre-reveal silence abs 225–239 at −∞: nothing plays, not even a reverse swell. |
| 240 | 00:08.00 | 5.1 | drop | Logo slam + flash; impact_deep_2 stacks on it; bed duck −6 dB. |
| 255 | 00:08.50 | 5.2 | stinger | Snare accent as UBI lands and the descriptor finishes (beat 2). |
| 300 | 00:10.00 | 6.1 | downbeat-hit | Crash; the motif enters with the grid floor. |
| 345 | 00:11.50 | 6.4 | filter-sweep | Quick high-pass lift 345–359 into the match-cut. |
| 360 | 00:12.00 | 7.1 | downbeat-hit | Crash: match-cut into the real app. |
| 420 | 00:14.00 | 8.1 | stinger | Pluck C6 with the playhead landing on 18h (sits with ding_2). |
| 450 | 00:15.00 | 8.3 | stinger | Whip into Categorias. |
| 510 | 00:17.00 | 9.3 | downbeat-hit | Stab under “Regras,” (beat 3); duck −5 dB. |
| 525 | 00:17.50 | 9.4 | downbeat-hit | Stab under “memória,” (beat 4); duck −5 dB. |
| 540 | 00:18.00 | 10.1 | downbeat-hit | Stab + crash under “IA.” (bar 10 downbeat); duck −5 dB. |
| 585 | 00:19.50 | 10.4 | stinger | Tom accent on the cut to Revisão. |
| 645 | 00:21.50 | 11.4 | filter-sweep | One-beat fill that sets up the stop. |
| 660 | 00:22.00 | 12.1 | stop | Stab on the downbeat, then the band stops; nothing melodic on 675 so the key-down sounds alone. |
| 690 | 00:23.00 | 12.3 | downbeat-hit | Groove re-enters on the key → chip morph (beat 3). |
| 720 | 00:24.00 | 13.1 | downbeat-hit | Rule chips appear (bar 13 downbeat). |
| 780 | 00:26.00 | 14.1 | downbeat-hit | Cut to “Confirmar os 19”. |
| 810 | 00:27.00 | 14.3 | stinger | The click: stab + crash, chord lift Am → C — the peak of the features act. |
| 885 | 00:29.50 | 15.4 | stinger | UBI steps out of the app: C-major-pentatonic sparkle (sits with shimmer_2). |
| 900 | 00:30.00 | 16.1 | downbeat-hit | UBI lands: clap + A5 bell (bar 16 downbeat). |
| 945 | 00:31.50 | 16.4 | stinger | Bell E6 as the clock lands on 18:00 (sits with ding_1). |
| 960 | 00:32.00 | 17.1 | downbeat-hit | Soft hit: the report window rises; drums back in. |
| 990 | 00:33.00 | 17.3 | stinger | Pluck with the “Copiar Markdown” click. |
| 1035 | 00:34.50 | 18.2 | stinger | Full-band stab + low “no” synth for the window slam; duck −5 dB. |
| 1080 | 00:36.00 | 19.1 | downbeat-hit | “Ok, foco!” click (bar 19 downbeat). |
| 1125 | 00:37.50 | 19.4 | stinger | Whip into Configurações. |
| 1140 | 00:38.00 | 20.1 | downbeat-hit | Lift starts (bar 20). |
| 1200 | 00:40.00 | 21.1 | downbeat-hit | “IA do Ubi” click (bar 21 downbeat). |
| 1230 | 00:41.00 | 21.3 | stop | Drums out on beat 3 at the blur-dissolve midpoint; pad and sub carry into the breather. |
| 1260 | 00:42.00 | 22.1 | downbeat-hit | Breather bar: pad Fmaj7, sub pulse on quarters, no kick. |
| 1275 | 00:42.50 | 22.2 | stinger | Soft pluck on the internal jump-cut as the address chip appears (beat 2); pad moves to Am9 on 1290. |
| 1320 | 00:44.00 | 23.1 | downbeat-hit | Mask contact (pop); kick returns on quarters. |
| 1320 | 00:44.00 | 23.1 | riser-start | riser_1bar_1.wav (glide to A4) starts; its last sample is abs 1379. |
| 1350 | 00:45.00 | 23.3 | filter-sweep | Snare roll 8ths → 16ths, filters sweep up. |
| 1380 | 00:46.00 | 24.1 | downbeat-hit | FINAL HIT (bar 24 downbeat); bed duck −4 dB. |
| 1395 | 00:46.50 | 24.2 | stinger | Soft thump as UBI lands on the lockup (sits with bloop_1). |
| 1440 | 00:48.00 | 25.1 | stinger | Two-note goodbye C5 → A4 with the wave (bar 25 downbeat). |
| 1500 | 00:50.00 | 26.1 | filter-sweep | Tail: low-pass closes, reverb decays. |
| 1555 | 00:51.83 | 26.4 | stop | Last audible sample ≤ frame 1559; the picture holds the end card to the end. |

## Claim and privacy guards

- No key range on screen: the selected review row’s hint line and the keys legend are covered by solid patches in every review capture that is framed; the only key shown pressed is “1”.
- No model version names: both report meta lines and the settings model fields are patched (and framed out).
- AI providers appear only as plain text (the picker’s own labels and the words Claude, OpenAI, Grok, “IA do Ubi”); no provider logo files are referenced anywhere, and no endorsement wording is used.
- CTA is “Baixe em ubiqx.com.br”: download, never a purchase verb; no price on the end card.
- All UI numbers are demo texture; the only counted number that is a read is the button label “Confirmar os 19”, and no other count is legible in a frame of the same dataset (the empty-state count and “(20)” header are patched or out of frame).
- Privacy: timeline*.png and reports-generated.png are not used; no camera key frames a mail/Gmail row; the only address on screen is the fictional ana@example.com, turned into the real token “[email]”.
- Never used: the OG social image, onboarding captures, worried/sleep UBI next to the product.
- Copy traces to facts.md: §1 (Controle de tempo automático com IA.; the problem lines “Só um minutinho.” and “Chutar as horas?” restate §1’s “o ‘só um minuto’ que virou quarenta” and “planilha preenchida de memória na sexta”), §3.1, §3.3, §3.4, §3.6, §3.8, §3.14 manchetes, §6 bank lines, real UBI lines (Nada em dúvida!, Não! Foque na sua produtividade.), the real toast “Markdown copiado”, tagline and CTA. No trailer line is reused except the tagline. The closest echo is the §3.8 manchete “O relatório sai pronto.” (the trailer’s line is “Corrija com uma tecla. O relatório já sai pronto.”), which both copy judges accepted as the facts headline. Final PT-BR copy edit (before → after, why): brief/copy-edit.md.

## Conventions

- **frames:** Scene-relative unless marked abs. Every scene starts on a beat (multiple of 15); downbeat starts: 0, 180, 240, 360, 660, 780, 1380.
- **transitions:** Not overlapping. Each transition is split into an outgoing half at the end of scene N and an incoming half at the start of scene N+1, named identically; “frames” = the frames of that half inside that scene (0 = empty half). The boundary frame is the cut. Implement as TransitionIn/TransitionOut inside absolute <Sequence from> (premountFor 30), never TransitionSeries (it would shorten the film).
- **camera:** Screen primitive, width 1440 for every 2880-px capture: bitmap scale s = zoom × 1440/2880 = 0.5 × zoom (= the upsample factor). intervention.png (1840 px, DPR 4) is shown at width 1520: s = 0.826 × zoom. “atFrame” = the frame the camera ARRIVES; “duration” = travel frames before it; “focus” = image point (px) placed on “anchor” (canvas px). Zoom interpolates in log space. “canvas” = a scene or layer with no UI plane. Hard cap: s ≤ 1.40 on every key, including drifts.
- **readability:** A camera key with a readTarget puts that string at ≥ 36 px on screen: onScreenPx = captureFontPx × s × lift. captureFontPx was measured from the PNGs (cap height ÷ 0.727). Wherever s > 1.15 the read target is a live vector re-set of the exact UI string (same font, size and colour) glued to the plane as a Screen child. All other UI text is texture, and no text the viewer must read is ever blurred.
- **claimSafety:** patches[] are solid image-space rectangles (Screen children in the sampled card/row colour) over UI text that must never be legible: the key-range hint and the keys legend in every review capture, model-version lines in reports and settings, and continuity counts. scrims[] are solid screen-space bands. tools/validate-storyboard.py maps every sensitive region through every camera key and fails if one is visible and uncovered.
- **cursor:** “to” = a hotspot of the scene’s capture, or “rest:x,y” in canvas px (fade-in/rest points and off-frame entries). click:true clicks on that frame; the cursor arrives 3–4 f earlier. 30–34 px tall in screen space, scale ≤ 1.5× with the camera.
- **sfx:** “atFrame” = the frame where the HIT (transient / loudest point / a riser’s end) lands, relative to the scene; “fileStartFrame” = atFrame − hitOffsetFrames (from sfx-manifest.json). Build ONE master SFX track at absolute frames: cues that straddle a cut (whips, the pull-back swoosh, both risers) must not live inside a scene Sequence. gainDb is relative to the bed (BED = 0.5).
- **reading:** hold ≥ max(24, 2 × characters) from landFrame (last word at spring ≥ 0.9) to outFrame, for every line. Lines that land together carry a group (groupMode “together”) and are checked on the SUM of their characters from the last landing to the first exit. Lines that arrive one after another (a counter under a slam, a second headline line, a callout after a click, the end-card cascade) are read in sequence: each passes on its own, and the scene’s reading budget — max(24, 2 × all characters in the scene) — must fit between the first landing and the last exit.
- **ubi:** ubiTrack lists which PNG sequence plays on which scene frames: index = startIndex + (f − from); “hold” freezes the last index. Anchors in the 900-px frame: headCenter (449, 303), feet y 797, idle body ink ≈ x 262–624, y 157–797.
- **canvasCamera:** A camera key with target “canvas” moves the background, grid, UI planes and UBI. Type layers (headlines, kickers, labels, the CTA) stay pinned in screen space at scale 1 on whole pixels (style §3.5 resting crispness); only slam shakes (≤ 8 f) and masked entrances move them.
- **shots:** shots[] lists the relative frames where a new picture starts inside the scene (first entry 0): hard cuts, jump-cuts, same-camera capture swaps and the flash re-layouts of the montage.
- **typography:** Sora 600/700/800 + Inter 400/500/600 only. PT-BR accents kept; U+00A0 glues short words; brand casing ubiqX / UBI / “IA do Ubi”.

## Scenes

### 1. `s01-hook-terca` — abs 0–89 (90 f · 00:00.00–00:03.00 · bar 1.1 → 2.3) · hook

**Thumbnail.** Near-black frame. A rose kicker “SEXTA-FEIRA · 17:00” over a big white question “O que você fez na terça?” (volt underline under “terça?”). Below it an empty, generic weekly timesheet leans back in 3D like a desk; one volt caret blinks in Tuesday’s column.

**Why it exists.** The hook is the accountability pain (facts §1: “planilha preenchida de memória na sexta”), not the trailer’s 8→18 clock or its “tempo vazando” metaphor. Frame 0 is a finished poster, and the timesheet slams flat inside the first second so there is motion before 1 s. The Hoje, Revisão and Relatórios captures are all dated “Terça-feira, 29 de setembro”, so the film answers its own question.

**Purpose.** Stop the scroll with a question every person who reports hours has been asked: Friday 17:00, and a Tuesday nobody remembers.

**Shot types.** S01 cold-open hook frame, S09 tilt-to-flat (compressed to 24 f, on a generic timesheet), S17 “ANTES” look (empty timesheet)

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| SEXTA-FEIRA · 17:00 | kicker | static | 0 → 0 → 90 | — | Inter 600 26 px +0.14em, rose #f2555a, centred, cap-top y 168. Set on frame 0 (poster). · group g1 (together) |
| O que você fez na terça? | headline | static | 0 → 0 → 90 | terça? | Sora 700 112 px −0.035em ink, one line ≈ 1 330 px, centred, cap-top y 214. Fully set on frame 0. Emphasis = volt underline sweep under “terça?” f2–14 (E.glide). · group g1 (together) |

**Picture.** Background “plain”: canvas #0a0d16, rose orb bottom-left 0.06, faint volt orb top-centre 0.08, vignette 0.6, grain 0.045. Under the type: a generic, unbranded weekly timesheet drawn in vector (no product yet): 5 day columns SEG TER QUA QUI SEX (Inter 600 30 px ink-2, +0.14em) × 9 hour rows 09h…17h (Inter 500 26 px ink-3), 1 440 × 560 px, 1.5 px #1f2a40 cell lines, every cell empty, S17 “ANTES” treatment (desaturate 60 %, brightness 0.8). A volt text caret (4 × 56 px) blinks in cell TER/10h. Frame-0 pose of the timesheet plane: perspective 1 800 px, rotateX 32°, rotateZ −4°, scale 0.90, centre (960, 800) — it leans back under the headline like a desk.

**Assets.**

- `none` `vector`. Timesheet, caret and type are drawn in the composition (tokens: canvas, line, ink-2, ink-3, volt, rose).

**Camera.**

- f0 → `canvas` zoom 1.0, tilt rx 32° ry 0°, hold (poster). Frame 0 = poster: plane at rotateX 32°, rotateZ −4°, scale 0.90, centre y 800. Nothing is mid-transition.
- f24 → `canvas` zoom 1.0, tilt rx 8° ry 0°, E.push. Tilt-to-flat slam of the timesheet: rotateX 32→8°, rotateZ −4→0°, scale 0.90→1, centre y 800→700 (90 % done by f6). DirectionalBlur on the plane only while it moves > 40 px/f; the type is pinned in screen space and never blurs.
- f89 → `canvas` zoom 1.04, tilt rx 8° ry 0°, linear. Camera push 1.00→1.04 toward the TER column, plane parallax −24 px.

**Action, frame by frame.** f0 poster (type set, plane leaning back, caret on). f0–24 the timesheet slams flat (E.push) — the only fast move. f2–14 volt underline sweeps under “terça?”. f30 (beat 3) spotlight on the TER column: the other four columns dim to 40 % over 8 f (S12 dim, no glow). Caret blinks 8 on / 8 off. f24–89 slow push into TER. f90: hard cut while the push is still moving.

**Transitions.** In: cut (0 f) — Film start: frame 0 is a designed, legible poster; the music’s first downbeat hits on frame 0. Out: cut (0 f).

**SFX.** None: the music’s frame-0 downbeat (filtered kick + sub + Am stab) is the hit (style S01).

**Reading check.**

- “SEXTA-FEIRA · 17:00”: 19 chars → max(24, 2×19) = 38 f; held f0→f90 = 90 f → yes
- “O que você fez na terça?”: 24 chars → max(24, 2×24) = 48 f; held f0→f90 = 90 f → yes
- Group g1 read together: 43 chars → 86 f from the last landing (f0) to the first exit (f90) = 90 f → yes
- Scene reading budget: all 43 chars → 86 f from the first landing (f0) to the last exit (f90) = 90 f → yes

### 2. `s02-so-um-minuto` — abs 90–179 (90 f · 00:03.00–00:06.00 · bar 2.3 → 4.1) · problem

**Thumbnail.** Gray, blurred window-title chips streak right-to-left across the dark and re-shuffle on 8th notes; centred, the slam “Só um minutinho.” (“minutinho” in rose); under it a huge counter lands on “40 min”.

**Why it exists.** The landing page’s own idiom (facts §1: “o ‘só um minuto’ que virou quarenta”), in the diminutive Brazilians actually say out loud: “minutinho” is the self-deception, so the 40 lands as the punchline. 40 min is the payoff of the idiom, never a stat. It gives the problem act its only montage (6 micro-shots) and starts the riser toward the silence.

**Purpose.** Say why Tuesday is blank: window after window, every “só um minutinho” turns into forty.

**Shot types.** S05 chaos montage (6 flash re-layouts on 8ths), S02 kinetic word slam, S07 number counter

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| “Só um minutinho.” | headline | slam | 11 → 15 → 90 | minutinho | Sora 800 128 px, one line ≈ 1 200 px (18 chars, so display size rather than the 200 px hero size), centred, cap-top y 330. “minutinho” rose (the diminutive is the lie; the counter is the truth). Contact f15 = abs 105 (beat 4). |
| 40 min | hero | static | 29 → 60 → 90 | — | Counter 1→40 with the “ min” suffix at 50 % (ink-2), Sora 700 240 px tabular-nums, centred, cap-top y 560. Lands f60 = abs 150 (beat 3), turns rose; rose underline sweep under “min”. |

**Picture.** Background “plain” with a rose orb bottom-left 0.08 (problem palette), grain. Layer A — texture, never read: 12 generic, unbranded window-title chips (panel-2 pills, 1 px border, 18 × 18 gray app square + Inter 500 30 px ink-2 at 70 %, desaturated 30 %, 2 px blur) in three depth rows (y 170 / 300 / 900, scale 0.8 / 1 / 0.9) flying right→left. Titles from a generic set: “Nova aba”, “Re: Re: Fwd: reunião”, “Planilha de horas — set”, “(12) Caixa de entrada”, “Proposta_v3_final.pdf”, “Sem título — Documento”. No brands, no addresses, no typing imagery. Layer B: the slam line. Layer C: the counter. Everything stays on the dark canvas (no luminance flips).

**Assets.**

- `none` `vector`. Chips (vector), KineticText slam and Counter primitives.

**Camera.**

- f0 → `canvas` zoom 1.0, tilt rx 0° ry 0°, linear. Each chip re-layout has its own push 1.00→1.06 (linear) and ±2° rotation from random(seed); the text layers sit outside that push.
- f89 → `canvas` zoom 1.02, tilt rx 0° ry 0°, linear

**Action, frame by frame.** f0 (abs 90) hard cut, chips already flying at 38 px/f. Chip-layer flash re-layouts on 8ths: f0 and f8 (abs 90, 98) before the slam, then f30, f38, f45, f53 (abs 120, 128, 135, 143) while the counter runs — a new random chip set each time, no white flash. f11 the slam mounts at scale 1.45, blur 12, opacity 0→1 in 2 f; SLAM spring to CONTACT f15 (abs 105), shake 6 px decaying over 8 f; chips slow to 8 px/f and dim to 40 %. f29 the counter mounts at “1 min” (SNAPPY, y 24→0). f30→60 counts 1→40 (bezier .25,.1,.25,1); lands f60 (abs 150): pulse 1→1.05→1 (the shot’s only bouncy element), ink→rose, underline sweep. f60–89 hold, chips drift at 4 px/f. f90 hard cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 90) `glitch_1.wav` -16 dB, file starts f0. Flash re-layout 1.
- f8 (abs 98) `glitch_3.wav` -16 dB, file starts f8. Flash re-layout 2 (8th).
- f15 (abs 105) `impact.wav` +0 dB, file starts f15. Slam contact; bed duck −5 dB.
- f33 (abs 123) `ui_tick_2.wav` -22 dB, file starts f33. 
- f36 (abs 126) `ui_tick_2.wav` -22 dB, file starts f36. 
- f39 (abs 129) `ui_tick_2.wav` -22 dB, file starts f39. 
- f42 (abs 132) `ui_tick_2.wav` -22 dB, file starts f42. 
- f45 (abs 135) `ui_tick_2.wav` -22 dB, file starts f45. 
- f48 (abs 138) `ui_tick_2.wav` -22 dB, file starts f48. 
- f51 (abs 141) `ui_tick_2.wav` -22 dB, file starts f51. 
- f54 (abs 144) `ui_tick_2.wav` -22 dB, file starts f54. 
- f57 (abs 147) `ui_tick_2.wav` -22 dB, file starts f57. 
- f60 (abs 150) `impact_soft_3.wav` -2 dB, file starts f60. Counter lands on 40 min.
- The 2-bar riser (riser-2bar.wav) starts here at f15 (abs 105) on the master track; it is listed in s04 because its hit — the end of the file — lands on abs 225.

**Reading check.**

- ““Só um minutinho.””: 18 chars → max(24, 2×18) = 36 f; held f15→f90 = 75 f → yes
- “40 min”: 6 chars → max(24, 2×6) = 24 f; held f60→f90 = 30 f → yes
- Scene reading budget: all 24 chars → 48 f from the first landing (f15) to the last exit (f90) = 75 f → yes

### 3. `s03-planilha-de-memoria` — abs 180–224 (45 f · 00:06.00–00:07.50 · bar 4.1 → 4.4) · problem

**Thumbnail.** The empty timesheet again, now flat. “Chutar as horas?” is slammed above it, a rose line strikes through “Chutar”, rose question marks (the guesses) pop into Tuesday’s cells, and the volt caret drifts toward frame centre.

**Why it exists.** Closes the accountability problem (facts §1). “Chutar” is the everyday Brazilian verb for filling hours in from memory, and the rose “?” popping into Tuesday’s cells are the guesses themselves; the empty timesheet under the line says “planilha” without spelling it. It hands the one surviving volt element — the caret — to the silence. The strike-through already says “not anymore” before the brand appears.

**Purpose.** The second pain in three words: Friday’s timesheet gets filled by guesswork (facts §1: “planilha preenchida de memória na sexta”).

**Shot types.** S02 kinetic word slam (the cut is the contact), S17 “ANTES” timesheet

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Chutar as horas? | headline | slam | 0 → 0 → 45 | Chutar | Sora 800 120 px, one line ≈ 1 020 px, centred, cap-top y 170. On screen at full opacity on f0 (scale 1.06→1, SLAM): the cut is the contact on the downbeat abs 180. Emphasis = rose strike-through (0.07em bar) drawn across “Chutar” f20–30: the bad habit is struck, the hours stay. |

**Picture.** The s01 timesheet returns flat (rotateX 6°), scale 1.0, centred y 640. Rose “?” glyphs (Inter 600 44 px) pop into empty cells. The timesheet layer drifts (linear, 0.5 px/f) so that on f44 the volt caret in TER/10h sits exactly at screen (960, 520) — the spot where the wordmark’s X lands in s05.

**Assets.**

- `none` `vector`. Same vector timesheet as s01.

**Camera.**

- f0 → `canvas` zoom 1.0, tilt rx 0° ry 0°, linear
- f44 → `canvas` zoom 1.03, tilt rx 0° ry 0°, linear. Slow push; the caret ends at (960, 520).

**Action, frame by frame.** f0 (abs 180, downbeat) hard cut into the slam contact + 6 px shake decaying over 8 f. f8, f15, f23, f30 (8ths): rose “?” pop into TER/11h, SEG/15h, QUI/09h, TER/14h (opacity 0→1, scale 0.9→1, SNAPPY — no bounce in the problem act). f20–30 rose strike-through across “Chutar”. Caret solid f30–44. f45: hard cut to the drop-out — everything disappears except the caret.

**Transitions.** In: cut (0 f). Out: cut (0 f) — T8 drop-out: hard cut to canvas + caret; music and riser end dead on abs 225.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 180) `impact_soft_2.wav` +0 dB, file starts f0. Slam contact; bed duck −5 dB.
- f8 (abs 188) `ui_tick_1.wav` -22 dB, file starts f8. “?” pops.
- f15 (abs 195) `ui_tick_1.wav` -22 dB, file starts f15. “?” pops.
- f23 (abs 203) `ui_tick_1.wav` -22 dB, file starts f23. “?” pops.
- f30 (abs 210) `ui_tick_1.wav` -22 dB, file starts f30. “?” pops.

**Reading check.**

- “Chutar as horas?”: 16 chars → max(24, 2×16) = 32 f; held f0→f45 = 45 f → yes

### 4. `s04-silencio` — abs 225–239 (15 f · 00:07.50–00:08.00 · bar 4.4 → 5.1) · problem

**Thumbnail.** Dark frame, grain only, a single volt caret at the centre.

**Why it exists.** The pre-reveal silence (≥ 15 f at −∞) that makes the drop land. The surviving caret makes the silence read as intent, not a black frame, and primes the X.

**Purpose.** The drop-out: everything stops; one volt caret blinks dead centre in total silence, exactly where the X will land.

**Shot types.** T8 drop-out (cut to silence)

**On-screen copy.** None.

**Picture.** Canvas #0a0d16 + grain 0.045 + vignette only. The 4 × 56 px volt caret at (960, 520), same position as s03’s last frame.

**Assets.**

- `none` `vector`. Caret only.

**Camera.**

- f0 → `canvas` zoom 1.0, tilt rx 0° ry 0°, linear. The film’s only deliberate freeze; grain keeps it alive.

**Action, frame by frame.** f0–7 caret on, f8–14 off. Nothing else moves. Music −∞ dB, no SFX, no reverb tail.

**Transitions.** In: cut (0 f) — T8 drop-out. Out: flash (0 f) — The outgoing half of T7 is empty: the flash lives in s05 f0–3.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 225) `riser-2bar.wav` -6 dB, file starts f-120. Placed by its END: the riser’s last sample is abs 224, so it plays abs 105–224 (starts at s02 f15) and ends exactly as the silence begins. Master-track cue; nothing is audible at abs 225–239.

**Reading check.**

- No copy in this scene.

### 5. `s05-drop-ubiqx` — abs 240–359 (120 f · 00:08.00–00:12.00 · bar 5.1 → 7.1) · reveal

**Thumbnail.** A big white “ubiq” + volt “X” wordmark with the “AI” pill; UBI stands to its right on a volt floor glow; “Controle de tempo automático com IA.” is centred underneath; a volt orb blooms behind.

**Why it exists.** Brand and descriptor land at 8.5 s (must be ≤ 12 s). The X pops out of the caret from the silence. UBI’s leap starts his arc: here he goes into the app, and in s13 he comes back out when the work is done.

**Purpose.** The answer on the loudest frame: ubiqX AI, what it is in one line, and UBI landing beside it.

**Shot types.** S08 logo reveal slam, T7 flash cut on the hit, S19 mascot entrance, S04 masked line reveal

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| ubiqX AI | hero | slam | 0 → 0 → 120 | X | brand/ubiqx-wordmark-bold.svg at height 194 px (= Sora 700 200 px), ≈ x 440–1024, y 439–633, plus an “AI” pill (Sora 600 56 px, volt on rgba(77,141,255,0.14), radius 999) at x 1048–1194. Full opacity from f0 (contact = the drop). The X (path wm-X) is centred on (960, 520), the caret’s spot. · group g1 (together) |
| Controle de tempo automático com IA. | sub | mask-up | 3 → 15 → 120 | — | Facts §1 short line. Inter 500 44 px ink-2, one line ≈ 830 px, centred x 960, cap-top y 752. 12-f mask (E.enter) from f3, landed f15 = abs 255 (8.5 s). · group g1 (together) |

**Picture.** Background “orbs”: volt orb Ø1100 behind the lockup (0→0.30 over 6 f, settles 0.18 by f30), ember orb bottom-right 0.07; grid floor fades in from f60 (0→35 %). Horizontal lockup centred by ink bounds (x 440–1491): wordmark · 24 px · “AI” pill · 56 px · UBI (frames at 600 px, k 0.667, frame box left 1075 top 129 → body ≈ x 1250–1491, y 234–660, feet on a soft volt floor-glow ellipse at y 660, volt rim light drop-shadow(0 0 30px rgba(77,141,255,0.35))). Descriptor below. Wordmark text-shadow glow 0.30 for this bar only (f0–59). f105–119 match-cut prep: the type stays crisp while the canvas behind it tints from #0a0d16 to the Hoje hero-card colour #0d1424, the app’s ember floor glow fades in under UBI, and UBI shrinks and moves onto the exact rect of the in-app UBI in s06’s first frame (frame 600→475 px, box (1075,129)→(1317,308), body → x 1455–1668, y 391–729).

**Assets.**

- `brand` `brand/ubiqx-wordmark-bold.svg`. Per-letter paths wm-u/b/i/q/X; the X pops separately.
- `ubi` `ubi/jump-from-idle`. Entered at index 6 so ground contact (index 21) lands on f15.
- `ubi` `ubi/idle`. Continues from idle 11 (jump-from-idle’s last frame equals idle 10).

**Camera.**

- f0 → `canvas` zoom 1.0, tilt rx 0° ry 0°, SLAM. Slam shake on f0: noise2D × 6 px × exp(−t/3) for 8 f, ±0.4°.
- f60 → `canvas` zoom 1.0, tilt rx 0° ry 0°, linear
- f104 → `canvas` zoom 1.04, tilt rx 0° ry 0°, E.glide. Slow push f60→104 (span) as the grid floor arrives.
- f119 → `canvas` zoom 1.04, tilt rx 0° ry 0°, hold

**UBI.**

- f0–f34: `ubi/jump-from-idle` frames 6–40. Launches in from below: y-offset +260→−30 px f0–9 (E.push), then −30→0 f9–15 (E.exit = gravity). Apex (index 15) on f9, ground contact (index 21) on f15 = abs 255 (beat 2), squash (index 24) on f18, index 40 (== idle 10) on f34.
- f35–f119: `ubi/idle` frames 11–95. Idle 11→95. f105–119: scale 0.667→0.528 and move onto the in-app UBI rect (E.exit, 15 f).

**Action, frame by frame.** f0 (abs 240, THE DROP): flash overlay #e8edf9 0.35→0 over 4 f (T7); wordmark on screen at full opacity, scale 1.10→1 (SLAM), tracking +0.02em→−0.03em over 20 f (E.push). f0–2 the caret is still at the X’s centre; f3 the X pops out of it (rotate −90°→0, scale 0→1, BOUNCY_SUBTLE — the shot’s one bouncy element). f8 “AI” pill (SNAPPY). f0–15 UBI launches in and lands on the beat (contact f15 = abs 255), 3 % squash baked into the clip. f3–15 descriptor masks up. f10–28 glint (140 px diagonal band, white 18 %, masked to the glyphs). f60 (abs 300, downbeat) grid floor fades in and a slow push begins. f105–119 match-cut prep (tint + UBI shrinks into the in-app position). f120 cut.

**Transitions.** In: flash (4 f) — T7 on the drop: #e8edf9 at 0.35 decaying to 0 over f0–3. Out: match-cut (15 f) — Out-half f105–119: UBI shrinks onto the in-app UBI’s rect while the canvas tints to the hero-card colour.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 240) `impact_deep_2.wav` +2 dB, file starts f0. The drop (impact + 45 Hz sub in one file); bed duck −6 dB.
- f10 (abs 250) `shimmer_1.wav` -14 dB, file starts f10. Glint.
- f15 (abs 255) `bloop_1.wav` -12 dB, file starts f15. UBI lands.
- f108 (abs 348) `whoosh_out_3.wav` -14 dB, file starts f105. UBI shrinks away into the app.

**Reading check.**

- “ubiqX AI”: 8 chars → max(24, 2×8) = 24 f; held f0→f120 = 120 f → yes
- “Controle de tempo automático com IA.”: 36 chars → max(24, 2×36) = 72 f; held f15→f120 = 105 f → yes
- Group g1 read together: 44 chars → 88 f from the last landing (f15) to the first exit (f120) = 105 f → yes
- Scene reading budget: all 44 chars → 88 f from the first landing (f0) to the last exit (f120) = 120 f → yes

### 6. `s06-ele-registra` — abs 360–449 (90 f · 00:12.00–00:15.00 · bar 7.1 → 8.3) · features

**Thumbnail.** The real “Hoje” screen, slightly tilted, under “Ele registra. Você trabalha.”; below it the day track fills with blue, orange and green category blocks behind a volt playhead that stops on the 18h tick.

**Why it exists.** The hero product reveal and module 1 (registra, facts §3.1). The sweep ends on 18:00, which the report scene pays off (facts §3.8 default time), instead of a clock hook.

**Purpose.** First look at the real product — UBI lives in “Hoje” — and the first promise proved: Tuesday recorded itself, block by block, up to 18:00.

**Shot types.** T5 match-cut (3D UBI → in-app UBI), S09 hero reveal (pull-back, tilt-to-flat), S11 glide, S18 data build (day track), S03 word stagger

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Ele registra. Você trabalha. | headline | stagger-words | 5 → 20 → 90 | registra. | Facts §3.1 manchete. Sora 700 80 px −0.03em ink, one line ≈ 1 190 px, centred, cap-top y 88, over a top canvas scrim (solid 0–200 → 0 at 260). 4 words, 3 f stagger from f5 → landed f20. “registra.” volt. |

**Picture.** Screen ui/dashboard.png, width 1440 (bitmap scale s = 0.5 × zoom), “mac” chrome with neutral #3a4560 dots, radius 18, shadow tier 3 + volt under-glow, float 4 px / 120 f, sheen at f24. Background “grid” (floor 30 %). f0: the frame is inside the Hoje hero card at bitmap 1:1 — the in-app UBI (with its own speech bubble and ember floor glow) sits exactly where s05’s 3D UBI stopped; a 4-f crossfade (3D → bitmap) hides any residual pose difference. Day track: an image-space cover (Screen child, empty-track colour) hides day-track (554,1178,2236×68) right of image x 1299 (08h) until f36, then retracts to x 2231 (18h) behind a 3 px volt playhead with a 24 px glow; everything right of 18h (incl. the app’s now-marker) stays covered for the rest of the scene. Legend and axis labels are texture.

**Assets.**

- `ui` `ui/dashboard.png` — hotspots: `ubi-robot`, `ubi-speech-bubble`, `hero`, `day-track-card`, `day-track`, `day-track-legend`. Match target = in-app UBI (body ink ≈ image x 2402–2615, y 552–890). The day track is the proof of automatic recording. Hero numbers are demo texture, never a read.

**Camera.**

- f0 → `ubi-robot` zoom 2.0, tilt rx 0° ry 0°, hard (file `ui/dashboard.png`; width 1440; bitmap scale 1.0; focus (2507, 721) → anchor (1560, 560)). Match frame: in-app UBI body at ≈ x 1455–1668, y 391–729 (bitmap 1:1). Image right edge at comp x 1933 and top at y −161, so the whole frame is inside the app (no canvas beside the window).
- f24 → `full` zoom 1.0, tilt rx 6° ry -6°, E.push (file `ui/dashboard.png`; width 1440; bitmap scale 0.5; focus (1440, 900) → anchor (960, 680); travel 24 f). Pull-back = hero reveal (perceptual zoom 2.0→1.0); window top ≈ y 186. The app’s own headline (53 px capture) shows at 27 px: texture only.
- f60 → `day-track-card` zoom 1.5, tilt rx 4° ry -4°, E.glide (file `ui/dashboard.png`; width 1440; bitmap scale 0.75; focus (1672, 1207) → anchor (960, 690); travel 30 f). Span f30→60. The top scrim thickens to solid 0–420 so the hero recedes; the card spans ≈ x 90–1830, y 555–825; spotlight on day-track-card (dim 0.62) from f45.
- f89 → `day-track-card` zoom 1.53, tilt rx 4° ry -4°, linear (file `ui/dashboard.png`; width 1440; bitmap scale 0.765; focus (1672, 1207) → anchor (960, 690)). Drift.

**Spotlights.** `day-track-card` f45–f89 dim 0.62

**Action, frame by frame.** f0 (abs 360, downbeat, crash) match-cut: same silhouette, now the real app; f0–4 crossfade 3D → bitmap. f0–24 pull-back to the whole Hoje window (E.push, 97 % done by f12); tilt settles to rx 6°, ry −6°. f5–20 headline stagger (“registra.” turns volt). f24 sheen. f30–60 glide to the day-track card (E.glide span), scrim thickens, spotlight from f45. f36–60 build: the cover retracts 08h→18h behind the volt playhead (E.glide 24 f) and the real category blocks appear left→right. f60 (abs 420, downbeat) the playhead lands on the 18h tick and pulses once. f60–85 drift; playhead glow breathes on f75. f86–89 whip-left out-half (x 0→−960, E.exit, horizontal blur 0→40).

**Transitions.** In: match-cut (12 f) — In-half: the pull-back continues s05’s shrink; 4-f 3D→bitmap crossfade on the UBI. Out: whip-left (4 f) — T2 out-half f86–89; cut at peak velocity on abs 450.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f12 (abs 372) `swoosh_long_3.wav` -10 dB, file starts f-6. Deep pull-back for the hero reveal; the file starts 6 f before the scene (master track).
- f36 (abs 396) `sweep_1.wav` -16 dB, file starts f36. One tick cluster for the whole track build.
- f60 (abs 420) `ding_2.wav` -12 dB, file starts f60. Playhead lands on 18h (C6).

**Reading check.**

- “Ele registra. Você trabalha.”: 28 chars → max(24, 2×28) = 56 f; held f20→f90 = 70 f → yes

### 7. `s07-as-suas-categorias` — abs 450–509 (60 f · 00:15.00–00:17.00 · bar 8.3 → 9.3) · features

**Thumbnail.** Close on the real Categorias editor: the field “O que conta como trabalho desta categoria” being typed live — “Docência no IFRO Campus Porto Velho Calama…” — under “Suas categorias.” (“Suas” in volt). “Horário do relatório 18:00” sits soft at the top of the frame.

**Why it exists.** Module 2a, the §3.3 differentiator (the AI reads your description before classifying). It is 2 s long and folds into the classify module rather than being a separate feature.

**Purpose.** Classification starts from your own words: you describe what counts as work for each category, and the AI follows that description.

**Shot types.** S15 typing (typewriter, vector re-set), S11 zoom, S03 word stagger, T2 whip-in

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Suas categorias. | headline | stagger-words | 3 → 15 → 60 | Suas | Facts §3.3 manchete in Brazilian headline register: the article goes, like “Seus dados” in s17 (not the trailer’s “nas suas categorias”). Sora 700 96 px, left x 144, cap-top y 110, over a top canvas scrim (90 % 0–240 px → 0 at 300). 2 words at f3/6 → landed by f15 (abs 465). “Suas” volt. |

**Picture.** Screen ui/categories.png, width 1440, rx 3° ry −3°, float 3 px. Close on the IFRO editor: “Horário do relatório 18:00” soft at the top, the field label and the description textarea in the middle. Vector re-sets pinned in image space (Screen children): (1) the label “O que conta como trabalho desta categoria” (Inter 500, 26 image px, ink-2) at (1342, 646); (2) a #121a2b patch (the textarea fill) over the textarea interior (1356,705,1418×170) hiding the captured text, and a live typewriter layer (Inter 400, 29 image px, ink) re-typing the capture’s own value: “Docência no IFRO Campus Porto Velho Calama: aulas de Programação Web, orientação de TCC,” (88 chars). Spotlight on the field block (1330,630,1470×280), dim 0.55, from f6.

**Assets.**

- `ui` `ui/categories.png` — hotspots: `category-editor`, `category-editor-title`, `category-item-1`. Demo categories IFRO, Incubadora, Cidades Inteligentes; the typed text is the capture’s own description value.

**Camera.**

- f0 → `category-editor` zoom 2.8, tilt rx 3° ry -3°, arrives with the whip-in (file `ui/categories.png`; width 1440; bitmap scale 1.4; focus (2040, 740) → anchor (1060, 660)). **Read:** “O que conta como trabalho desta categoria (vector re-set)” 26 capture px × 1.4 = **36.4 px**, vector ✓. Visible image ≈ x 1283–2654, y 269–1040; image right edge at comp x 2236 (off-frame). Typed text 29 image px → 40.6 px (vector). The 1.4× bitmap around it sits under a 0.55 dim.
- f59 → `category-editor` zoom 2.8, tilt rx 3° ry -3°, linear (file `ui/categories.png`; width 1440; bitmap scale 1.4; focus (2040, 740) → anchor (1040, 660)). **Read:** “O que conta como trabalho desta categoria (vector re-set)” 26 capture px × 1.4 = **36.4 px**, vector ✓. Drift by a 20 px pan; zoom stays at the 1.4 cap.

**Spotlights.** (1330, 630, 1470×280) f6–f59 dim 0.55

**Action, frame by frame.** f0–3 whip in-half (x +960→0, E.push, blur 40→0). f3–15 headline. f6 spotlight (dim 0→0.55, 12 f). f8 caret appears; f8–52 typewriter at 2 chars/f with a solid caret; then the caret blinks 8/8. f60 hard cut.

**Transitions.** In: whip-left (4 f) — T2 in-half f0–3. Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 450) `whip_2.wav` -8 dB, file starts f-2. Whip pass-by on the cut abs 450; file starts 2 f earlier (master track).
- f8 (abs 458) `type-1.wav` -22 dB, file starts f8. 
- f12 (abs 462) `type-2.wav` -22 dB, file starts f12. 
- f16 (abs 466) `type-3.wav` -22 dB, file starts f16. 
- f20 (abs 470) `type-4.wav` -22 dB, file starts f20. 
- f24 (abs 474) `type-1.wav` -22 dB, file starts f24. 
- f28 (abs 478) `type-2.wav` -22 dB, file starts f28. 
- f32 (abs 482) `type-3.wav` -22 dB, file starts f32. 
- f36 (abs 486) `type-4.wav` -22 dB, file starts f36. 
- f40 (abs 490) `type-1.wav` -22 dB, file starts f40. 
- f44 (abs 494) `type-2.wav` -22 dB, file starts f44. 
- f48 (abs 498) `type-3.wav` -22 dB, file starts f48. 

**Reading check.**

- “Suas categorias.”: 16 chars → max(24, 2×16) = 32 f; held f15→f60 = 45 f → yes

### 8. `s08-regras-memoria-ia` — abs 510–584 (75 f · 00:17.00–00:19.50 · bar 9.3 → 10.4) · features

**Thumbnail.** Left: a stacked slam “Regras, / memória, / IA.” (“IA.” in volt). Right: a close, tilted crop of real classified rows, a volt ring around the origin badge “IA” on the Terminal row.

**Why it exists.** Module 2b (facts §3.4). Three hits on three beats, with “IA.” on the bar-10 downbeat; the right side jump-cuts badge to badge, so it is also three shots of rhythm.

**Purpose.** How it decides: rules and memory first, AI for the rest — each slammed word lands on the matching origin badge of a real classified row.

**Shot types.** S02 slam stack (3 words on 3 beats, accumulating), S12 push-in with focus dimming, jump-cuts on the beat

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Regras, | hero | slam | 0 → 0 → 75 | — | Sora 800 150 px, line 1 of a left stack x 144, cap-top y 250; ink → ink-2 (70 %) when the next word lands. · group g1 (together) |
| memória, | hero | slam | 15 → 15 → 75 | — | Line 2, cap-top y 393; ink → ink-2 at f30. · group g1 (together) |
| IA. | hero | slam | 30 → 30 → 75 | IA. | Line 3, cap-top y 536; lands in volt on the downbeat abs 540. · group g1 (together) |

**Picture.** Screen ui/review-settled-expanded.png (“Classificados neste dia (19)” open), width 1440, rx 4° ry −8°. The right-hand column of real rows: category chip, confidence bar, origin badge. Badge rects (image px, reviewed-row-1 + 114 px per row): row 1 sei.ifro.edu.br “regra” ≈ (1826,566,114,40); row 3 docs.google.com “memória” ≈ (1788,792,152,40); row 5 Terminal “IA” ≈ (1858,1020,82,40). The left 55 % carries the slam stack on a canvas scrim (85 % → 0 at x 1000). Spotlight inside the plane: badge rect + 16 px padding, dim 0.62, 2 px volt ring + 40 px glow 0.3, radius 14.

**Assets.**

- `ui` `ui/review-settled-expanded.png` — hotspots: `reviewed-row-1`, `reviewed-list`, `keys-legend`. Rows 3 and 5 are reviewed-row-1 + 228 / + 456 image px. keys-legend is listed because it carries a patch.

**Claim-safety patches / scrims.**

- patch on `ui/review-settled-expanded.png` (2136, 662, 640×32) in #0c1220, f0–f74: keys legend. 

**Camera.**

- f0 → `reviewed-row-1` zoom 2.3, tilt rx 4° ry -8°, hard (file `ui/review-settled-expanded.png`; width 1440; bitmap scale 1.15; focus (1883, 586) → anchor (1400, 560)). Texture only: badge words ≈ 26 capture px → 30 px; the 150-px slam word states each badge. Visible ≈ x 666–2335, y 99–1038.
- f15 → `reviewed-list` zoom 2.3, tilt rx 3° ry -7°, hard (jump-cut on the beat) (file `ui/review-settled-expanded.png`; width 1440; bitmap scale 1.15; focus (1864, 812) → anchor (1400, 560))
- f30 → `reviewed-list` zoom 2.3, tilt rx 4° ry -8°, hard (jump-cut on the downbeat) (file `ui/review-settled-expanded.png`; width 1440; bitmap scale 1.15; focus (1899, 1040) → anchor (1400, 560))
- f74 → `reviewed-list` zoom 2.3, tilt rx 3° ry -8°, linear (file `ui/review-settled-expanded.png`; width 1440; bitmap scale 1.15; focus (1899, 1028) → anchor (1400, 560)). Drift: plane y −12 px, rx 4→3°.

**Spotlights.** (1810, 550, 146×72) f0–f14 dim 0.62; (1772, 776, 184×72) f15–f29 dim 0.62; (1842, 1004, 114×72) f30–f74 dim 0.62

**Action, frame by frame.** f0 (abs 510) hard cut straight into the first contact: “Regras,” on screen at full opacity, scale 1.06→1 (SLAM), 6 px shake over 8 f; spotlight on “regra” (d 0→1 over 8 f). f15 (abs 525) hard jump-cut down the list to the “memória” badge as “memória,” slams in on line 2; “Regras,” dims to ink-2. f30 (abs 540, downbeat) jump-cut to the “IA” badge as “IA.” slams in volt, with a canvas lift #0a0d16→#121a2c→#0a0d16 over 3 f. Each badge ring stays drawn at 60 % after its hop (row 3’s ring is still visible in the last framing). f30–74 hold on row 5, glow breathes 0.3→0.22, drift. f75 hard cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 510) `impact_soft_1.wav` +0 dB, file starts f0. “Regras,” — duck −5 dB.
- f15 (abs 525) `impact_soft_2.wav` +0 dB, file starts f15. “memória,” — duck −5 dB.
- f30 (abs 540) `impact_soft_3.wav` +2 dB, file starts f30. “IA.” on the downbeat — duck −5 dB.

**Reading check.**

- “Regras,”: 7 chars → max(24, 2×7) = 24 f; held f0→f75 = 75 f → yes
- “memória,”: 8 chars → max(24, 2×8) = 24 f; held f15→f75 = 60 f → yes
- “IA.”: 3 chars → max(24, 2×3) = 24 f; held f30→f75 = 45 f → yes
- Group g1 read together: 18 chars → 36 f from the last landing (f30) to the first exit (f75) = 45 f → yes
- Scene reading budget: all 18 chars → 36 f from the first landing (f0) to the last exit (f75) = 75 f → yes

### 9. `s09-so-pergunta` — abs 585–659 (75 f · 00:19.50–00:22.00 · bar 10.4 → 12.1) · features

**Thumbnail.** The real Revisão page, tilted: the selected Calendário group and three Finder groups all reading “Sem categoria · 0 % · pendente”, the “Atribuir ao grupo selecionado” card on the right; “Só pergunta o que não sabe.” across the bottom band.

**Why it exists.** Module 3a (facts §3.5: review only of the doubts). It sets up the key: the push ends on the picker whose first row is IFRO.

**Purpose.** Revisão: the queue only holds what the chain could not settle — rows of “Sem categoria · pendente”.

**Shot types.** S11 zoom (medium → close, internal jump-cut), S12 dim, headline in the bottom band (Module A variant)

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Só pergunta o que não sabe. | headline | stagger-words | 2 → 16 → 75 | não sabe. | Facts §6 bank. Sora 700 96 px, left x 144, baseline y 905 (above the 120-px player strip), over the bottom scrim. 5 units (“o que” glued), 2 f stagger from f2 → landed f16. Emphasis = volt marker highlight (16 %) swept behind “não sabe.” f18–28. |

**Picture.** Screen ui/review-queue-selected.png, width 1440, rx 4° ry −6°. Shot A (f0–29) medium: page title “Revisão”, the queue with the selected Calendário group and three Finder groups (Sem categoria · 0 % · pendente), and the assign card with the five named categories. Shot B (f30–74) closer on rows 1–4, then a push toward the picker. Claim patches (solid Screen children): the selected row’s key-range hint line and the keys legend. A solid bottom scrim (comp y 800–1080, gradient from 700) carries the headline and hides rows 6 and below, including the mail.google.com group, which is never framed.

**Assets.**

- `ui` `ui/review-queue-selected.png` — hotspots: `queue-card`, `queue-row-1`, `queue-row-2`, `queue-row-3`, `assign-card`, `assign-option-1`, `queue-row-active-hint`, `keys-legend`. queue-row-active-hint and keys-legend are listed because they carry patches.

**Claim-safety patches / scrims.**

- patch on `ui/review-queue-selected.png` (630, 400, 1222×32) in #15233f, f0–f74: key-range hint. selected row: the second line that names a key range; row colour #15233f
- patch on `ui/review-queue-selected.png` (2136, 806, 640×32) in #0c1220, f0–f74: keys legend. 
- solid screen scrim (0, 800, 1920×280), f0–f74. Bottom band under the headline (canvas 100 %, gradient 700→800).

**Camera.**

- f0 → `queue-card` zoom 1.6, tilt rx 4° ry -6°, hard (file `ui/review-queue-selected.png`; width 1440; bitmap scale 0.8; focus (1400, 560) → anchor (960, 460)). Medium. Visible ≈ x 200–2600, y −15–1335 (the title bar covers the top); the solid bottom scrim hides image y ≥ 985.
- f30 → `queue-row-1` zoom 2.1, tilt rx 4° ry -5°, hard (internal jump-cut on beat 3) (file `ui/review-queue-selected.png`; width 1440; bitmap scale 1.05; focus (1500, 520) → anchor (960, 420)). Closer on rows 1–4 (their “pendente” pills pulse an ember ring once on f45). Texture: row text ≈ 29 capture px → 30 px; the headline carries the message.
- f74 → `assign-option-1` zoom 2.3, tilt rx 3° ry -5°, E.glide (file `ui/review-queue-selected.png`; width 1440; bitmap scale 1.15; focus (2000, 500) → anchor (960, 480); travel 30 f). Span f44→74: anticipation push toward the picker (IFRO · 1 on top); cut while moving. Image right edge at comp x 1972.

**Spotlights.** (514, 238, 1529×560) f34–f74 dim 0.5

**Action, frame by frame.** f0 (abs 585) hard cut, medium framing. f2–16 headline stagger; f18–28 marker under “não sabe.”. f30 (abs 615) jump-cut closer on rows 1–4; f34 the other rows dim (0.5); f45 the four “pendente” pills pulse an ember ring (one beat). f44–74 span push toward the picker. Cut at f75 while still moving.

**Transitions.** In: cut (0 f). Out: cut (0 f) — Cut on the downbeat abs 660 into the band stop.

**SFX.** None: music only (≥ 30 % of the features act stays SFX-free).

**Reading check.**

- “Só pergunta o que não sabe.”: 27 chars → max(24, 2×27) = 54 f; held f16→f75 = 59 f → yes

### 10. `s10-tecla-1` — abs 660–689 (30 f · 00:22.00–00:23.00 · bar 12.1 → 12.3) · features

**Thumbnail.** A big 3D keycap “1” centred over the blurred Revisão page; its legend flashes volt as it goes down.

**Why it exists.** B’s stop-time moment: the band stops on the downbeat and the key-down is heard alone on beat 2. It is the only key ever shown pressed.

**Purpose.** Keyboard speed: one key decides the group — the “1”, pressed alone in the band stop.

**Shot types.** S14 keycap press close-up

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| 1 | label | static | 0 → 6 → 30 | — | KeyCap legend, Inter 600 92 px; latched volt after the press. |

**Picture.** Behind: the review-queue-selected plane exactly where s09 left it, rack-blurred 14 px (static whole-plane CSS blur) and dimmed 0.55, patches still applied. KeyCap centred at (960, 560): 240 × 240 px, radius 34, top face #1c2740→#141c2e, 1 px rgba(255,255,255,0.10) border, 18 px skirt #0c111d, floor shadow; perspective 1200 px, rotateX 28°, rotateZ −6°.

**Assets.**

- `ui` `ui/review-queue-selected.png` — hotspots: `assign-option-1`, `queue-row-active-hint`, `keys-legend`. Blurred backdrop only.

**Claim-safety patches / scrims.**

- patch on `ui/review-queue-selected.png` (630, 400, 1222×32) in #15233f, f0–f29: key-range hint. selected row: the second line that names a key range; row colour #15233f
- patch on `ui/review-queue-selected.png` (2136, 806, 640×32) in #0c1220, f0–f29: keys legend. 

**Camera.**

- f0 → `assign-option-1` zoom 2.3, tilt rx 3° ry -5°, hold (file `ui/review-queue-selected.png`; width 1440; bitmap scale 1.15; focus (2000, 500) → anchor (960, 480)). Backdrop only (blur 14 px). Composition push 1.00→1.04 linear over the scene.
- f29 → `assign-option-1` zoom 2.3, tilt rx 3° ry -5°, hold (file `ui/review-queue-selected.png`; width 1440; bitmap scale 1.15; focus (2000, 500) → anchor (960, 480))

**Action, frame by frame.** f0 (abs 660, band stop) cut; the key mounts scale 0.94→1, y +20→0 (SNAPPY, f0–8). f12–15 press: translateY +14 px, skirt 18→4 px (E.exit 3 f). CONTACT f15 (abs 675, beat 2): legend turns volt, underglow 0 0 60px rgba(77,141,255,0.55) decaying over 8 f, 2 px micro-shake. f16–23 release (SNAPPY), legend stays volt (latched). f30 match-cut.

**Transitions.** In: cut (0 f). Out: match-cut (0 f) — The keycap’s top face keeps its rect, radius and colour on s11’s first frame.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f15 (abs 675) `key_down_1.wav` -8 dB, file starts f15. Contact, heard alone in the band stop.
- f21 (abs 681) `key_up_1.wav` -18 dB, file starts f21. Release.

**Reading check.**

- “1”: 1 chars → max(24, 2×1) = 24 f; held f6→f30 = 24 f → yes

### 11. `s11-e-ele-aprende` — abs 690–779 (90 f · 00:23.00–00:26.00 · bar 12.3 → 14.1) · features

**Thumbnail.** A tight, tilted crop of the real assign card: the keycap has become the “1” chip beside IFRO, the queue on the left has lost its top group, and a new chip “Sempre: Calendário → IFRO” pops in; “Uma tecla. / E ele aprende.” on the left.

**Why it exists.** Module 3b (facts §3.5 + §3.7). The T5 match cut turns the physical key into UI. The rule chip is the app’s own proof of learning.

**Purpose.** The press lands in the real UI: the key flies into “IFRO · 1”, the group leaves the queue, and the app offers a rule — it learns.

**Shot types.** T5 match cut (keycap → assign-key-1), S13 UI state change, S03 word stagger

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Uma tecla. / E ele aprende. | headline | stagger-words | 12 → 27 → 90 | aprende. | Facts §6 bank. Sora 700 88 px, two lines left x 144, cap-tops 300 / 400, over a left canvas scrim (85 % → 0 at x 900). Units “Uma”, “tecla.”, “E ele”, “aprende.” at f12/15/18/21 → landed f27. “aprende.” volt. |

**Picture.** Screen ui/review-queue-selected.png → ui/review-after-assign.png (6-f crossfade at f15), width 1440, rx 3° ry −6°. Framing on the assign card, the picker and the right half of the queue rows. Vector re-sets: the “IFRO” option label (Inter 500, 29 image px, ink) and the key chip “1” (the morph target). Claim patches on both captures (hint line, keys legend). Until f30 a #0c1220 patch covers last-suggestions (2127,800,663,150) so the rule chips can appear on the downbeat. The after-assign toast and its “Classificados neste dia (20)” header stay out of frame.

**Assets.**

- `ui` `ui/review-queue-selected.png` — hotspots: `assign-key-1`, `assign-option-1`, `queue-row-active-hint`, `keys-legend`. Before the press.
- `ui` `ui/review-after-assign.png` — hotspots: `assign-picker`, `assign-option-1`, `last-suggestions`, `queue-row-active-hint`, `keys-legend`. After “1”: the Calendário group left the queue; “Criar regra: Sempre: Calendário → IFRO” chips.

**Claim-safety patches / scrims.**

- patch on `ui/review-queue-selected.png` (630, 400, 1222×32) in #15233f, f0–f20: key-range hint. selected row: the second line that names a key range; row colour #15233f
- patch on `ui/review-queue-selected.png` (2136, 806, 640×32) in #0c1220, f0–f20: keys legend. 
- patch on `ui/review-after-assign.png` (630, 400, 1222×32) in #15233f, f15–f89: key-range hint. 
- patch on `ui/review-after-assign.png` (2136, 1016, 640×32) in #0c1220, f15–f89: keys legend. measured at y 1023–1041 in this capture (the manifest rect for keys-legend here is wrong)
- patch on `ui/review-after-assign.png` (2127, 800, 663×150) in #0c1220, f15–f30: rule chips (reveal). Lifts on f30 so the chips appear on the downbeat abs 720.

**Camera.**

- f0 → `assign-key-1` zoom 2.6, tilt rx 3° ry -6°, hard (file `ui/review-queue-selected.png`; width 1440; bitmap scale 1.3; focus (2100, 560) → anchor (960, 420)). **Read:** “IFRO (assign-option-1, vector re-set)” 29 capture px × 1.3 = **37.7 px**, vector ✓. Visible ≈ x 1362–2838, y 237–1068; image right edge at comp x 1974 (no canvas beside the window on the match frame). The chip assign-key-1 lands at comp ≈ (1800, 215), 52 px.
- f15 → `assign-picker` zoom 2.6, tilt rx 3° ry -6°, hard (state swap) (file `ui/review-after-assign.png`; width 1440; bitmap scale 1.3; focus (2100, 560) → anchor (960, 420)). **Read:** “IFRO (assign-option-1, vector re-set)” 29 capture px × 1.3 = **37.7 px**, vector ✓
- f60 → `last-suggestions` zoom 2.3, tilt rx 3° ry -6°, E.glide (file `ui/review-after-assign.png`; width 1440; bitmap scale 1.15; focus (2080, 640) → anchor (1010, 420); travel 30 f). Span f30→60: a slight pull-out (scale 1.3→1.15) down to the rule chips (≈ 25 capture px → 29 px: supporting texture; the headline says it). Visible ≈ x 1202–2871, y 275–1214; image right edge at comp x 1930. Legend at image y 1023 is patched; toast and the “(20)” header are out of frame.
- f89 → `last-suggestions` zoom 2.3, tilt rx 3° ry -6°, linear (file `ui/review-after-assign.png`; width 1440; bitmap scale 1.15; focus (2080, 650) → anchor (1010, 420)). Drift.

**Spotlights.** `assign-option-1` f12–f30 dim 0.45; `last-suggestions` f30–f89 dim 0.45

**Action, frame by frame.** f0 (abs 690, beat 3) match-cut: the s10 keycap is drawn at its s10 rect over the now-sharp UI and flies/flattens into assign-key-1 over 12 f (SNAPPY: rect, rotation → plane tilt, radius 34→8, fill → chip); backdrop blur 12→0 and dim 0.4→0 (SMOOTH 12 f); the groove re-enters. f12 the IFRO row takes the pressed tint (volt 16 %, 6 f). f15 (abs 705, beat) state change: 6-f crossfade to review-after-assign — the Calendário group slides out left (x −140 px, fades 6 f) and the list closes up; the new active row flashes mint 12 % for 8 f. f12–27 headline. f30 (abs 720, downbeat) the patch lifts: “Criar regra: Sempre: Calendário → IFRO” chips morph in (y +8→0, opacity, SNAPPY); spotlight glides from the IFRO row to the chips (f30–60 span). f60–89 hold + drift.

**Transitions.** In: match-cut (12 f) — Keycap → assign-key-1 morph (T5), 12 f SNAPPY. Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f12 (abs 702) `ui_tick_1.wav` -18 dB, file starts f12. Chip lights.
- f18 (abs 708) `success_chime_1.wav` -14 dB, file starts f18. Group classified (C6→E6).
- f30 (abs 720) `pop.wav` -16 dB, file starts f30. Rule chips appear.

**Reading check.**

- “Uma tecla. / E ele aprende.”: 25 chars → max(24, 2×25) = 50 f; held f27→f90 = 63 f → yes

### 12. `s12-um-clique` — abs 780–869 (90 f · 00:26.00–00:29.00 · bar 14.1 → 15.3) · features

**Thumbnail.** Tight on the real “Confirmar os 19” button, the cursor clicking it; then the rows below flip one by one to mint “você” badges as the camera pulls back, under “Um clique vira memória.”.

**Why it exists.** Module 3c (facts §3.6). The click is the features-act peak. The same-camera capture swap (A’s fix) avoids C’s off-image glide to the toast.

**Purpose.** One click turns everything the Ubi decided into your own answers — “Confirmar os 19” — and the list flips to “você”.

**Shot types.** S12 push-in with dimming, S13 cursor click + state change, S11 pull-back

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Confirmar os 19 | ui-caption | static | 0 → 0 → 30 | — | The real button label as a crisp vector replica (Inter 500, 24 image px) that lifts 1.08× above the plane on hover: 24 × 1.4 × 1.08 = 36.3 px on screen. Counted as read until the click on f30 (abs 810, beat); the state change replaces it on f32. |
| Um clique vira memória. | headline | stagger-words | 30 → 40 → 90 | memória. | Facts §3.6 manchete. Sora 700 88 px, left x 144, cap-top y 110, over a top-left scrim (85 % → 0). 3 units (“Um clique” glued) at f30/32/34 → landed f40. “memória.” volt. |

**Picture.** Screen ui/review-settled-expanded.png, hard swap to ui/review-confirmed.png at f32 with the identical camera: row 1’s mint “você” badge lands about 50 px from where the button was. The shot opens tight at 1.4× bitmap under a 0.62 dim; the button is a vector replica (same rect, radius 16, 1 px line border, label). After the swap the camera pulls back to 1.0× to show the column of mint “você” badges. Claim patch: keys legend in both captures. The toast (bottom-right) stays out of frame. The settled header visible after the pull-back reads “(19)”, the same number as the button.

**Assets.**

- `ui` `ui/review-settled-expanded.png` — hotspots: `confirm-bar`, `confirm-all-button`, `keys-legend`. Before the click.
- `ui` `ui/review-confirmed.png` — hotspots: `reviewed-list`, `reviewed-row-1`, `keys-legend`. After: every origin badge reads “você”.

**Claim-safety patches / scrims.**

- patch on `ui/review-settled-expanded.png` (2136, 662, 640×32) in #0c1220, f0–f31: keys legend. 
- patch on `ui/review-confirmed.png` (2136, 662, 640×32) in #0c1220, f32–f89: keys legend. 

**Camera.**

- f0 → `confirm-all-button` zoom 2.8, tilt rx 4° ry -6°, hard (file `ui/review-settled-expanded.png`; width 1440; bitmap scale 1.4; focus (1898, 478) → anchor (1250, 600)). **Read:** “Confirmar os 19 (vector replica, lifted 1.08×)” 24 capture px × 1.4 × lift 1.08 = **36.3 px**, vector ✓. Visible ≈ x 1005–2377, y 49–821: the bar’s left text and the header are off-frame left; the legend’s right end is off-frame right (and patched).
- f32 → `reviewed-row-1` zoom 2.8, tilt rx 4° ry -6°, hard (capture swap, same camera) (file `ui/review-confirmed.png`; width 1440; bitmap scale 1.4; focus (1898, 478) → anchor (1250, 600)). Same camera as f0 across the capture swap.
- f50 → `reviewed-list` zoom 2.0, tilt rx 4° ry -6°, E.push (file `ui/review-confirmed.png`; width 1440; bitmap scale 1.0; focus (1500, 700) → anchor (1150, 640); travel 18 f). Pull-back f32→50. Visible ≈ x 350–2270, y 60–1140: rows 1–6 all mint “você”.
- f89 → `reviewed-list` zoom 2.04, tilt rx 4° ry -6°, linear (file `ui/review-confirmed.png`; width 1440; bitmap scale 1.02; focus (1500, 700) → anchor (1150, 640)). Drift.

**Spotlights.** `confirm-bar` f0–f31 dim 0.62

**Cursor.**

- f4 → `rest:1990,1140`. Enters from off-frame bottom-right; never teleports.
- f26 → `confirm-all-button`. Arc 12 %, E.cursor 22 f; hover from f24 (the replica lifts 1.08×).
- f30 → `confirm-all-button` **CLICK**. CLICK abs 810 (beat 3): cursor 1→0.85→1, replica 0.96 + pressed colour, ripple 0→44 px + second ring 64 px.
- f44 → `rest:1700,900`. Drifts off and fades (f38–44).

**Action, frame by frame.** f0 (abs 780, downbeat) cut in tight on “Confirmar os 19” (crisp replica, dimmed surroundings). f4–26 cursor arcs in; f24 hover lift. f30 CLICK (abs 810). f32 (C+2) hard swap: the bar is gone and row 1 reads “você” in mint almost where the button was; f33–51 a mint wave runs down the rows (row flash 12 %, 2 f stagger, spread 18 f). f30–40 headline. f32–50 pull-back (E.push) reveals the whole column of “você”. f50–89 hold, drift.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f30 (abs 810) `click.wav` -12 dB, file starts f30. Confirmar os 19.
- f33 (abs 813) `shimmer_1.wav` -14 dB, file starts f33. All badges flip to “você” (C+3).

**Reading check.**

- “Confirmar os 19”: 15 chars → max(24, 2×15) = 30 f; held f0→f30 = 30 f → yes
- “Um clique vira memória.”: 23 chars → max(24, 2×23) = 46 f; held f40→f90 = 50 f → yes
- Scene reading budget: all 38 chars → 76 f from the first landing (f0) to the last exit (f90) = 90 f → yes

### 13. `s13-nada-em-duvida` — abs 870–929 (60 f · 00:29.00–00:31.00 · bar 15.3 → 16.3) · features

**Thumbnail.** The empty Revisão page racks out of focus while a big 3D UBI jumps out of it, and his real line “Nada em dúvida!” grows from the in-app bubble.

**Why it exists.** The best UBI beat in the drafts (B), and the end of his arc: he went into the app on the drop and comes out when the work is done (C). The resolved-count text is patched out, so no count contradicts “19”.

**Purpose.** Payoff: the queue is empty, and UBI steps out of the app to celebrate — “Nada em dúvida!”.

**Shot types.** S11 push, T5 match (in-app UBI → 3D UBI), S19 mascot + speech bubble

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Nada em dúvida! | headline | static | 15 → 27 → 60 | dúvida! | UBI’s real empty-state line (review.ts:83). Speech bubble: Sora 700 72 px ink, “dúvida!” volt; panel-2 #172033 at 94 %, 1 px rgba(255,255,255,0.08), radius 36, tail to his head. It grows out of the in-app bubble’s own position (scale 0.3→1 + translate, SNAPPY 12 f) and lands f27. |

**Picture.** Screen ui/review-done.png, width 1440, near-flat (rx 2° ry −2°): the empty queue, the in-app UBI with its small “Nada em dúvida!” bubble, “Nada esperando por você”, and the subtitle “Terça-feira, 29 de setembro: a fila está vazia.”. Continuity/claim patches (#0c1220): the paragraph with the day’s resolved count, the settled header with its count, and the keys legend. From f15 a patch in the card colour hides the bitmap UBI and its bubble so only one UBI exists, and the app racks out (whole-plane blur 0→8 px, brightness 1→0.45, SMOOTH 12 f). 3D UBI: volt rim + ember floor glow.

**Assets.**

- `ui` `ui/review-done.png` — hotspots: `review-done`, `review-done-ubi`, `settled-header`, `keys-legend`. Empty-state card; review-done-ubi is the match target (ink ≈ x 1209–1350, y 461–711).
- `ubi` `ubi/jump-from-idle`. From index 6 so ground contact (21) lands on the downbeat.
- `ubi` `ubi/idle`. Continues from idle 11.

**Claim-safety patches / scrims.**

- patch on `ui/review-done.png` (890, 840, 780×80) in #0c1220, f0–f59: resolved count. The empty-state paragraph that states how many groups the Ubi resolved (a different count than “19”).
- patch on `ui/review-done.png` (520, 1138, 1500×44) in #0c1220, f0–f59: settled header count. 
- patch on `ui/review-done.png` (2136, 1016, 640×32) in #0c1220, f0–f59: keys legend. Measured at y 1023–1041 in this capture.
- patch on `ui/review-done.png` (1147, 330, 264×420) in #0c1220, f15–f59: bitmap UBI + bubble. Only one UBI on screen after the match.

**Camera.**

- f0 → `review-done` zoom 1.6, tilt rx 2° ry -2°, hard (file `ui/review-done.png`; width 1440; bitmap scale 0.8; focus (1279, 560) → anchor (960, 520)). Empty state centred; visible ≈ x 79–2479, y −90–1260.
- f15 → `review-done-ubi` zoom 2.0, tilt rx 2° ry -2°, E.push (file `ui/review-done.png`; width 1440; bitmap scale 1.0; focus (1279, 540) → anchor (960, 600); travel 15 f). Bitmap 1:1. In-app UBI body at comp ≈ x 890–1031, y 521–771 (the 3D match rect: frame 352 px, box left 788 top 460).
- f59 → `review-done-ubi` zoom 2.0, tilt rx 2° ry -2°, hold (file `ui/review-done.png`; width 1440; bitmap scale 1.0; focus (1279, 540) → anchor (960, 600)). The plane is now a blurred backdrop; a composition push 1.00→1.10 (E.push f15→35) follows UBI.

**UBI.**

- f0–f14: hidden. Only the in-app bitmap UBI.
- f15–f49: `ubi/jump-from-idle` frames 6–40. Match on f15: drawn at frame 352 px over the in-app body, then grows to frame 760 px with body centre x 640, feet y 900 (SMOOTH f15–35). 3-f crossfade bitmap→3D. Apex (15) on f24, ground contact (21) on f30 = abs 900 (downbeat), squash (24) on f33, index 40 on f49.
- f50–f59: `ubi/idle` frames 11–20. 

**Action, frame by frame.** f0 (abs 870) cut on the empty queue; f0–15 push onto the in-app UBI (E.push). f15 (abs 885, beat 2) match: the 3D UBI takes the in-app UBI’s place, the app racks out, UBI grows and slides left while he crouches and jumps; the small in-app bubble grows into the big bubble (f15–27). f30 (abs 900, downbeat) feet contact, 3 px camera shake, squash baked in. f35–59 settle into idle, float 8 px. f60 hard cut.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f15 (abs 885) `shimmer_2.wav` -12 dB, file starts f15. UBI steps out of the app.
- f27 (abs 897) `pop+2.wav` -16 dB, file starts f27. Bubble lands.
- f30 (abs 900) `bloop_1.wav` -12 dB, file starts f30. UBI lands on the downbeat.

**Reading check.**

- “Nada em dúvida!”: 15 chars → max(24, 2×15) = 30 f; held f27→f60 = 33 f → yes

### 14. `s14-relatorio` — abs 930–1034 (105 f · 00:31.00–00:34.50 · bar 16.3 → 18.2) · features

**Thumbnail.** A huge “18:00” lands and flies up into a volt kicker pill as the real Relatórios window rises; then tight on the “Copiar Markdown” button, a cursor click and a mint “Markdown copiado” pill. Headline: “O relatório sai pronto.”.

**Why it exists.** Module 4 (facts §3.8–3.9, Markdown export). 18:00 is used only as the report time (the default), rhyming with the day track’s 18h; the app’s own toast string proves the deliverable.

**Purpose.** Entrega: at 18:00 the daily report per category is already written, and one click copies it as Markdown.

**Shot types.** S07 number (clock), clock → kicker morph, S09 rise, S12 push-in, S13 click + success, S03 word stagger

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| 18:00 | kicker | static | 0 → 15 → 105 | — | f0–29: Sora 700 240 px tabular, centred (960, 470), reads “17:59”, odometer-rolls to “18:00” landing f15 (abs 945, beat 2). f30–41 morphs (SNAPPY) into a kicker pill at x 144, y 88–136 (Inter 600 40 px, volt on volt 14 %) and stays. · group g1 (together) |
| O relatório sai pronto. | headline | stagger-words | 32 → 42 → 105 | pronto. | Facts §3.8 manchete. Sora 700 88 px, left x 144, cap-top y 170, over a top-left scrim. 3 units (“O relatório” glued) at f32/34/36 → landed f42. “pronto.” volt. · group g1 (together) |
| Markdown copiado | ui-caption | static | 62 → 66 → 105 | — | The app’s own copy toast string (reports.ts:47) set as a callout pill under the button: Inter 600 36 px ink on rgba(46,204,143,0.16), radius 999, 1.5 px leader line drawn over 10 f. |

**Picture.** Part A (f0–29): Background “orbs”, the big clock. Part B (f30–104): Screen ui/reports.png (Relatórios › Diário, card “IFRO”) rising tilt-to-flat, width 1440. Claim patches (#0c1220): report-1-meta (it names a model version) and the second card’s meta line. Vector re-set of the “Copiar Markdown” button (label Inter 500 28 image px, copy icon). The item row whose evidence line shows the mail domain (image y ≈ 1455) stays below the frame in every key.

**Assets.**

- `ui` `ui/reports.png` — hotspots: `report-card-1`, `report-1-summary`, `report-1-items`, `report-1-copy`, `report-1-meta`. report-1-meta is listed because it carries a patch.

**Claim-safety patches / scrims.**

- patch on `ui/reports.png` (650, 436, 1620×30) in #0c1220, f30–f104: model version (meta line 1). 
- patch on `ui/reports.png` (650, 1696, 1620×30) in #0c1220, f30–f104: model version (meta line 2). 

**Camera.**

- f0 → `canvas` zoom 1.0, tilt rx 0° ry 0°, linear. Part A: the clock card, drift 1.00→1.02.
- f45 → `report-card-1` zoom 1.6, tilt rx 6° ry -3°, E.push (file `ui/reports.png`; width 1440; bitmap scale 0.8; focus (1672, 840) → anchor (1100, 640); travel 15 f). Enter “rise” f30→50 (y +220→0, rotateX 20→6°, E.push). Wide on the whole IFRO report: “Resumo do dia — IFRO”, Destaques, Atividade / Minutos / Tipo / Horário (texture ≈ 21 px). Visible ≈ x 297–2697, y 40–1390.
- f57 → `report-1-copy` zoom 2.6, tilt rx 4° ry -3°, E.push (file `ui/reports.png`; width 1440; bitmap scale 1.3; focus (2426, 424) → anchor (1340, 600); travel 12 f). **Read:** “Copiar Markdown (vector re-set)” 28 capture px × 1.3 = **36.4 px**, vector ✓. Punch-in f45→57. Visible ≈ x 1395–2880 (image right edge at comp 1930), y −38–793 (the mac title bar covers the top): header buttons + table rows 1–2 only.
- f104 → `report-1-copy` zoom 2.6, tilt rx 4° ry -3°, linear (file `ui/reports.png`; width 1440; bitmap scale 1.3; focus (2426, 424) → anchor (1340, 600)). **Read:** “Copiar Markdown (vector re-set)” 28 capture px × 1.3 = **36.4 px**, vector ✓. Hold; float only (zoom stays below the cap).

**Spotlights.** `report-1-copy` f45–f104 dim 0.62

**Cursor.**

- f40 → `rest:1750,940`. Fades in (6 f) at a rest point.
- f56 → `report-1-copy`. Arc, E.cursor; hover from f54.
- f60 → `report-1-copy` **CLICK**. CLICK abs 990 (beat 3): replica 0.96, ripple; f62 the copy icon becomes a mint check (evolvePath 10 f).
- f80 → `rest:1700,900`. Drifts off and fades.

**Action, frame by frame.** f0 (abs 930) hard cut: “17:59” set. f10–15 the last digits roll; contact f15 (abs 945) + pulse 1→1.05→1 + volt for one beat. f30 (abs 960, downbeat) “18:00” flies to the kicker pill (SNAPPY 12 f) while the report window rises (E.push 20 f). f32–42 headline. f30–45 wide on the report. f45–57 punch-in to “Copiar Markdown” + spotlight. f40 cursor fades in, arrives f56, CLICK f60. f62 mint check; f62–66 the “Markdown copiado” callout pops (SNAPPY) and its leader line draws. f66–104 hold, float.

**Transitions.** In: cut (0 f). Out: cut (0 f).

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f15 (abs 945) `ding_1.wav` -12 dB, file starts f15. 18:00 lands (E6); rhymes with s06’s ding.
- f36 (abs 966) `whoosh-soft.wav` -14 dB, file starts f27. Report window rises; loudest point ≈ fastest frame of the rise.
- f60 (abs 990) `click.wav` -12 dB, file starts f60. Copiar Markdown.
- f63 (abs 993) `success_chime_2.wav` -14 dB, file starts f63. Mint check.

**Reading check.**

- “18:00”: 5 chars → max(24, 2×5) = 24 f; held f15→f105 = 90 f → yes
- “O relatório sai pronto.”: 23 chars → max(24, 2×23) = 46 f; held f42→f105 = 63 f → yes
- “Markdown copiado”: 16 chars → max(24, 2×16) = 32 f; held f66→f105 = 39 f → yes
- Group g1 read together: 28 chars → 56 f from the last landing (f42) to the first exit (f105) = 63 f → yes
- Scene reading budget: all 44 chars → 88 f from the first landing (f15) to the last exit (f105) = 90 f → yes

### 15. `s15-foco` — abs 1035–1124 (90 f · 00:34.50–00:37.50 · bar 18.2 → 19.4) · features

**Thumbnail.** The real “Aviso do UBI” window slams in over the blurred Foco page: the in-app UBI, “Não! Foque na sua produtividade.”, a rose “YouTube” badge, and a cursor clicking the volt “Ok, foco!”.

**Why it exists.** Module 5 (facts §3.11) as a single 3-s beat (CD fix: B’s two focus scenes compressed). The window is a DPR-4 capture shown at 0.83×, the sharpest UI in the film.

**Purpose.** Focus that defends itself: YouTube showed up during a session, UBI closed it and says why; you answer “Ok, foco!”.

**Shot types.** S06 pop-in (window as subject, slammed), S13 cursor click

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Não! Foque na sua produtividade. | ui-caption | static | 0 → 4 → 90 | — | UBI’s real line inside the captured window (focus.ts:128): 52 capture px (DPR 4) × 0.826 = 43 px on screen. · group g1 (together) |
| Ok, foco! | ui-caption | static | 0 → 4 → 90 | — | The real button label: ≈ 47 capture px × 0.826 = 39 px. · group g1 (together) |

**Picture.** Backdrop: Screen ui/focus.png framed on the Bloqueios card (YouTube, Instagram, Discord), whole-plane blur 6 px, brightness 0.45 (context, never a read). Subject: ui/intervention.png as a floating window (Screen chrome “none”, imageSize 1840×752, width 1520 → s 0.826) centred at (960, 560) → x 200–1720, y 249–871, shadow tier 3 + volt glow 0.25.

**Assets.**

- `ui` `ui/intervention.png` — hotspots: `panel`, `ubi`, `message`, `target-badge`, `ok-button`. Aviso do UBI window (DPR 4, transparent).
- `ui` `ui/focus.png` — hotspots: `targets-card`. Blurred backdrop only.

**Camera.**

- f0 → `targets-card` zoom 1.2, tilt rx 4° ry -6°, linear (file `ui/focus.png`; width 1440; bitmap scale 0.6; focus (1278, 1072) → anchor (960, 560); layer backdrop). Backdrop, blurred 6 px, brightness 0.45; drifts −12 px over the scene.
- f0 → `message` zoom 1.0, tilt rx 0° ry 0°, SLAM (file `ui/intervention.png`; width 1520; bitmap scale 0.8261; focus (920, 376) → anchor (960, 560); layer subject). **Read:** “Não! Foque na sua produtividade.” 52 capture px × 0.8261 = **43.0 px**, bitmap ✓. Window on screen at f0 at scale 1.04 → 1 (SLAM, landed f4), 4 px shake over 6 f. Also legible: “YouTube” badge (≈ 44 → 36 px) and “Ok, foco!” (≈ 47 → 39 px).
- f89 → `message` zoom 1.04, tilt rx 0° ry 0°, E.glide (file `ui/intervention.png`; width 1520; bitmap scale 0.8591; focus (920, 376) → anchor (960, 560); layer subject). **Read:** “Não! Foque na sua produtividade.” 52 capture px × 0.8591 = **44.7 px**, bitmap ✓. Slow push on the window.

**Cursor.**

- f20 → `rest:1560,930`. Fades in (6 f).
- f41 → `ok-button`. Arrives 4 f early; hover.
- f45 → `ok-button` **CLICK**. CLICK abs 1080 (downbeat): button 0.96 pressed, volt ripple; the window stays up to the cut.
- f70 → `rest:1600,920`. Drifts off and fades.

**Action, frame by frame.** f0 (abs 1035, beat 4) hard cut: the window slams in over the blurred Foco page (SLAM 4 f, shake); backdrop drifts −12 px. f20 cursor fades in, arcs to “Ok, foco!”, CLICK f45 (downbeat abs 1080). f45–89 window push 1.00→1.04, the in-app UBI’s glow breathes. f90 cut.

**Transitions.** In: cut (0 f). Out: whip-left (4 f) — T2 out-half f86–89.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 1035) `impact_soft_2.wav` -2 dB, file starts f0. Window slam; bed duck −5 dB.
- f2 (abs 1037) `glitch_2.wav` -18 dB, file starts f2. The blocked tab is gone.
- f45 (abs 1080) `click.wav` -12 dB, file starts f45. Ok, foco!

**Reading check.**

- “Não! Foque na sua produtividade.”: 32 chars → max(24, 2×32) = 64 f; held f4→f90 = 86 f → yes
- “Ok, foco!”: 9 chars → max(24, 2×9) = 24 f; held f4→f90 = 86 f → yes
- Group g1 read together: 41 chars → 82 f from the last landing (f4) to the first exit (f90) = 86 f → yes
- Scene reading budget: all 41 chars → 82 f from the first landing (f4) to the last exit (f90) = 86 f → yes

### 16. `s16-sua-ia` — abs 1125–1229 (105 f · 00:37.50–00:41.00 · bar 19.4 → 21.3) · proof

**Thumbnail.** A tight, tilted crop of the real “Provedor de IA” picker (Anthropic Claude · OpenAI · xAI Grok · IA do Ubi); the cursor hovers each name and clicks “IA do Ubi”; bottom band: “Claude, OpenAI ou Grok. / Ou deixe com o Ubi.”.

**Why it exists.** Module 6 (facts §3.13–3.14). Providers appear only as the picker’s plain text and as words; no logos, no endorsement wording, no model names.

**Purpose.** Your AI or ours: Claude, OpenAI or Grok with your own key — or leave it to the Ubi — chosen in the real settings, providers named in plain text.

**Shot types.** S11 zoom, S13 hover path + click + state change, S03 word stagger (two lines), T2 whip-in

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Claude, OpenAI ou Grok. | headline | stagger-words | 2 → 12 → 105 | — | Facts §6 bank. Sora 700 88 px, left x 144, cap-top y 736 (bottom band) over the bottom scrim. 3 units at f2/4/6 → landed f12. No emphasis: neutral naming. |
| Ou deixe com o Ubi. | headline | stagger-words | 48 → 60 → 105 | Ubi. | Facts §3.14 manchete. Line 2, cap-top y 840 (baseline ≈ 904, above the player strip). 4 units at f48/50/52/54 → landed f60. “Ubi.” volt. |

**Picture.** Screen ui/settings-ai.png (Configurações › IA, “Anthropic Claude” active) → hard swap to ui/settings-ai-ubi.png at f77 with the identical camera. Vector re-set of the four provider labels (“Anthropic Claude”, “OpenAI”, “xAI Grok”, “IA do Ubi”, Inter 500 28 image px) and the active-pill background, so hover and active states are crisp. Claim patches (#0c1220): the model fields in both captures. A solid bottom scrim (comp y 700–1080) hides everything below image y ≈ 630 (key box, models section). The helper line under the picker (a provider cost estimate, then the managed-plan line with its price) is texture.

**Assets.**

- `ui` `ui/settings-ai.png` — hotspots: `provider-picker`, `provider-anthropic`, `provider-openai`, `provider-xai`, `provider-ia-do-ubi`, `model-fields`. Before the click.
- `ui` `ui/settings-ai-ubi.png` — hotspots: `provider-picker`, `provider-ia-do-ubi`, `provider-active`, `model-fields`. After the click: IA do Ubi active.

**Claim-safety patches / scrims.**

- patch on `ui/settings-ai.png` (1146, 961, 1644×312) in #0c1220, f0–f76: model names. 
- patch on `ui/settings-ai-ubi.png` (1146, 761, 1644×312) in #0c1220, f77–f104: model names. 
- solid screen scrim (0, 700, 1920×380), f0–f104. Bottom band under the two lines.

**Camera.**

- f0 → `provider-picker` zoom 2.6, tilt rx 3° ry -7°, arrives with the whip-in (file `ui/settings-ai.png`; width 1440; bitmap scale 1.3; focus (1583, 453) → anchor (1010, 470)). **Read:** “Anthropic Claude · OpenAI · xAI Grok · IA do Ubi (vector re-set)” 28 capture px × 1.3 = **36.4 px**, vector ✓. Visible ≈ x 806–2283, y 91–922.
- f77 → `provider-picker` zoom 2.6, tilt rx 3° ry -7°, hard (state swap) (file `ui/settings-ai-ubi.png`; width 1440; bitmap scale 1.3; focus (1583, 453) → anchor (1010, 470)). **Read:** “Anthropic Claude · OpenAI · xAI Grok · IA do Ubi (vector re-set)” 28 capture px × 1.3 = **36.4 px**, vector ✓
- f104 → `provider-picker` zoom 2.6, tilt rx 3° ry -7°, linear (file `ui/settings-ai-ubi.png`; width 1440; bitmap scale 1.3; focus (1583, 453) → anchor (1000, 470)). **Read:** “Anthropic Claude · OpenAI · xAI Grok · IA do Ubi (vector re-set)” 28 capture px × 1.3 = **36.4 px**, vector ✓. Drift by a 10 px pan.

**Cursor.**

- f8 → `rest:1500,660`. Fades in (6 f) just below the picker.
- f15 → `provider-anthropic`. Hover (already active).
- f30 → `provider-openai`. Hover on the beat (abs 1155).
- f45 → `provider-xai`. Hover on the beat (abs 1170).
- f71 → `provider-ia-do-ubi`. Arrives 4 f early; hover.
- f75 → `provider-ia-do-ubi` **CLICK**. CLICK abs 1200 (downbeat); ripple; f77 swap: “IA do Ubi” becomes the active pill (the app’s own state).

**Action, frame by frame.** f0–3 whip in-half (x +960→0, blur 40→0). f2–12 line 1. The cursor hovers the pills on the beats (f15 Claude, f30 OpenAI, f45 xAI Grok): hover background fades in/out over 4 f. f48–60 line 2. f71 cursor on “IA do Ubi”; CLICK f75 (abs 1200). f77 hard swap to settings-ai-ubi. f78 shimmer. f78–98 hold, drift. f99–104 blur-dissolve out-half on the UI/background (blur 0→16, opacity 1→0, scale 1→1.03); the type hard-cuts at f105.

**Transitions.** In: whip-left (4 f) — T2 in-half f0–3. Out: blur-dissolve (6 f) — Out-half f99–104; mood change into the breather.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 1125) `whip_3.wav` -8 dB, file starts f-3. Whip on the cut abs 1125; file starts 3 f earlier (master track).
- f30 (abs 1155) `ui_tick_2.wav` -22 dB, file starts f30. Hover.
- f45 (abs 1170) `ui_tick_3.wav` -22 dB, file starts f45. Hover.
- f75 (abs 1200) `click.wav` -12 dB, file starts f75. IA do Ubi.
- f78 (abs 1203) `shimmer_2.wav` -14 dB, file starts f78. State change.

**Reading check.**

- “Claude, OpenAI ou Grok.”: 23 chars → max(24, 2×23) = 46 f; held f12→f105 = 93 f → yes
- “Ou deixe com o Ubi.”: 19 chars → max(24, 2×19) = 38 f; held f60→f105 = 45 f → yes
- Scene reading budget: all 42 chars → 84 f from the first landing (f12) to the last exit (f105) = 93 f → yes

### 17. `s17-privacidade` — abs 1230–1379 (150 f · 00:41.00–00:46.00 · bar 21.3 → 24.1) · proof

**Thumbnail.** Calm, warm frame with no UI: UBI on the left glancing around; on the right “Seus dados ficam com você. / A IA vê só o mínimo.”; a window-title chip reading “ana@example.com” collapses into a volt “[email]” token, and a thin volt line runs from it up to the word “IA”.

**Why it exists.** The trust beat that C lacked: B’s calm breather with UBI’s look, carrying A’s concrete proof — the exact token from crates/ubiqx-core/src/redact.rs. The copy says data goes out masked, never that nothing leaves. The last second is the build into the final hit.

**Purpose.** The breather and the trust answer: your data stays with you and the AI only sees the minimum — shown with the app’s real mask token.

**Shot types.** S04 masked lines, S19 mascot (idle → look), S13 state change (address → token), T6 blur dissolve in, internal jump-cut

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| Seus dados ficam com você. | headline | mask-up | 0 → 8 → 150 | — | Facts §6 bank. Sora 700 72 px −0.03em ink, left x 740, cap-top y 300 (≈ 963 px wide, ends ≈ x 1703 < 1776); masked line (padding .16em/.12em), 18 f E.enter, ≥ 0.9 by f8. Kept out of the blur-dissolve (crisp). |
| A IA vê só o mínimo. | headline | mask-up | 60 → 68 → 150 | mínimo. | Second line, cap-top y 396. “mínimo.” volt. |
| ana@example.com | label | static | 45 → 45 → 90 | — | Fictional, RFC-2606-reserved address inside a generic window-title chip (panel-2, radius 22, 1.5 px border, 96 px tall, 28-px window glyph), Inter 500 48 px ink, x 740–1300, y 520–616. The only e-mail string in the film. |
| [email] | label | static | 90 → 92 → 150 | — | The exact token written by crates/ubiqx-core/src/redact.rs. Inter 600 48 px, volt on volt 14 %; same chip, width morphs (SNAPPY). |

**Picture.** Background “plain” + orbs (volt top-right 0.16, ember bottom-left 0.07 — a warm scene), grain 0.045, vignette 0.55. No UI: the calmest frame of the film. UBI 3D on the left: frame 780 px (k 0.867), centre x 420, feet y 930 → body ≈ x 257–571, y 375–930; volt floor glow + soft shadow; float 8 px / 126 f. Text column x 740–1776. After the contact a 2 px volt line draws from the “[email]” chip up to the word “IA” in line 2: the AI receives only the token. Shot B (from f45): jump-cut to a composition scale 1.08 centred on the text column (UBI larger at the left edge).

**Assets.**

- `ubi` `ubi/idle`. Idle 75→119, so it ends on 119 before the look clip.
- `ubi` `ubi/look`. Left → centre → right (toward the chip) → up-right (toward the words).

**Camera.**

- f0 → `canvas` zoom 1.0, tilt rx 0° ry 0°, linear
- f45 → `canvas` zoom 1.08, tilt rx 0° ry 0°, hard (internal jump-cut on beat 2)
- f119 → `canvas` zoom 1.1, tilt rx 0° ry 0°, linear
- f149 → `canvas` zoom 1.16, tilt rx 0° ry 0°, E.exit. The build: the push accelerates into the final hit.

**UBI.**

- f0–f44: `ubi/idle` frames 75–119. Idle 75→119.
- f45–f134: `ubi/look` frames 0–89. Right glance ≈ look frame 43 → f88 (the address collapses on f90); up-right glance ≈ look frame 62 → f107, as the volt line reaches “IA”.
- f135–f149: `ubi/look` frames 89–89 (held). Holds the settled up-right glance (look frames 75–89 are the held glance); float keeps him alive.

**Action, frame by frame.** f0–5 blur-dissolve in-half (background + UBI: blur 16→0, opacity 0→1, scale 1.03→1); line 1 masks up from f0 (≥ 0.9 by f8). f0–44 UBI idles. f45 (abs 1275, beat 2) jump-cut tighter; the chip “ana@example.com” is set below line 1; UBI’s look clip starts (left, centre, then right toward the chip). f60 line 2 masks up (≥ 0.9 by f68). f76–88 a volt marker sweeps across the address (E.glide 12 f). f90 (abs 1320, downbeat) CONTACT: the address collapses into “[email]” (SNAPPY width morph; the old glyphs blur 0→6 px and fade over 4 f). f92–104 the volt line draws from the token to “IA” (evolvePath 12 f); “mínimo.” turns volt. f105–149 the build: the composition push accelerates (E.exit), the grid floor fades 0→30 %, the volt orb goes 0.16→0.24; UBI ends looking up-right at the words. f150: hard cut on the final hit.

**Transitions.** In: blur-dissolve (6 f) — In-half f0–5 on background + UBI only. Out: cut (0 f) — Hard cut on the final hit abs 1380.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f76 (abs 1306) `ui_tick_1.wav` -22 dB, file starts f76. Marker sweep starts.
- f90 (abs 1320) `pop.wav` -14 dB, file starts f90. Mask contact.
- riser_1bar_1.wav (1 bar, ends on A4) starts at f90 (abs 1320) on the master track; it is listed in s18 because its end is the final hit.

**Reading check.**

- “Seus dados ficam com você.”: 26 chars → max(24, 2×26) = 52 f; held f8→f150 = 142 f → yes
- “A IA vê só o mínimo.”: 20 chars → max(24, 2×20) = 40 f; held f68→f150 = 82 f → yes
- “ana@example.com”: 15 chars → max(24, 2×15) = 30 f; held f45→f90 = 45 f → yes
- “[email]”: 7 chars → max(24, 2×7) = 24 f; held f92→f150 = 58 f → yes
- Scene reading budget: all 68 chars → 136 f from the first landing (f8) to the last exit (f150) = 142 f → yes

### 18. `s18-end-card` — abs 1380–1559 (180 f · 00:46.00–00:52.00 · bar 24.1 → 27.1) · cta

**Thumbnail.** Centred lockup: white “ubiq” + volt “X” + “AI” pill with UBI standing to the right; “Retome o controle do seu dia.” below; a volt pill “Baixe em ubiqx.com.br”; “macOS · Windows · Linux” with small monochrome glyphs.

**Why it exists.** S22 end card with C’s landing (on the beat after the final hit) fixed to splice cleanly. It is the only place the platform row appears, and it is legible for 150 f after the last element lands. No fade: the last frame is the end card.

**Purpose.** Close: the brand, the tagline, one action — “Baixe em ubiqx.com.br” — the three systems, and UBI landing beside the lockup to wave goodbye.

**Shot types.** S22 end card (CTA), S19 mascot (drop + wave), S20 platform row

**On-screen copy (exact PT-BR).**

| Text | Role | Mode | In → land → out (rel) | Emphasis | Notes |
|---|---|---|---|---|---|
| ubiqX AI | hero | mask-up | 0 → 12 → 180 | X | Wordmark SVG at height 165 (font 170) ≈ x 501–998 + “AI” pill (Sora 600 52 px) x 1022–1146, row centre y 432; SMOOTH 1.06→1 + masked rise 12 f. |
| Retome o controle do seu dia. | sub | mask-up | 6 → 20 → 180 | — | Tagline (facts §6). Inter 500 48 px ink-2, centred, cap-top y 610. |
| Baixe em ubiqx.com.br | headline | static | 12 → 18 → 180 | — | CTA pill: Sora 600 44 px, #0a0d16 on volt #4d8dff (6.1:1), radius 999, padding 22 × 44, centred at y 720 (674–766). Pops with BOUNCY_SUBTLE (the shot’s one bouncy element). Download, not buy (checkout is not live). |
| macOS · Windows · Linux | label | mask-up | 18 → 30 → 180 | — | Inter 500 36 px ink-2 with 30-px monochrome glyphs (brand/os apple, windows, linux) before each name, centred y 850 (bottom ≈ 870 < 960). |

**Picture.** Background “orbs” + grid 20 %, volt orb breathing (1↔1.05 over 120 f), ember orb bottom-left 0.06. Centred lockup (rhymes with s05): wordmark · “AI” pill · UBI (frame 560 px, k 0.622, box left 1031 → body ≈ x 1194–1419, feet on the lockup’s floor line y 540 with a volt floor glow). Tagline, CTA pill and platform row stacked below; everything inside title-safe, nothing in the bottom 120 px or the corner boxes. The last frame is the end card (loop-safe, no fade).

**Assets.**

- `brand` `brand/ubiqx-wordmark-bold.svg`. Wordmark; X volt.
- `brand` `brand/os/apple.svg`. macOS glyph (monochrome).
- `brand` `brand/os/windows.svg`. Windows glyph (monochrome).
- `brand` `brand/os/linux.svg`. Linux glyph (monochrome).
- `ubi` `ubi/jump`. Landing half only (index 15→30); starts and ends on the rest pose.
- `ubi` `ubi/wave`. Raw wave: starts and ends on the rest pose.
- `ubi` `ubi/idle`. Idle 0 == rest pose.

**Camera.**

- f0 → `canvas` zoom 1.0, tilt rx 0° ry 0°, SMOOTH
- f179 → `canvas` zoom 1.02, tilt rx 0° ry 0°, linear. Barely-there drift keeps it alive; text rests at scale 1 on whole pixels.

**UBI.**

- f0–f8: hidden. 
- f9–f24: `ubi/jump` frames 15–30. Drops in from above the frame: y-offset −340→0 px f9–15 (E.exit = gravity) with the clip from its apex (15); ground contact (21) on f15 = abs 1395, squash (24) on f18, rest pose (30) on f24.
- f25–f85: `ubi/wave` frames 0–60. Rest pose → rest pose, so there is no splice pop; the arm is up across the abs-1440 downbeat (two-note goodbye).
- f86–f179: `ubi/idle` frames 0–93. Idle 0→93.

**Action, frame by frame.** f0 (abs 1380, FINAL HIT): the wordmark lands (SMOOTH 1.06→1 + masked rise), orb 0→0.22, glint f10–28. f6 tagline mask (landed f20). f12 CTA pill pops (landed f18). f18 platform row rises (y 16→0, blur 6→0, E.enter 14 f, glyphs 3 f apart; landed f30). f9–15 UBI drops onto the floor line, contact f15 (abs 1395, the beat after the hit). f25–85 wave (the arm peaks across abs 1440). f86–179 idle; only the orb breathes and UBI idles; the last frame is the full lockup.

**Transitions.** In: cut (0 f) — Hard cut on the final hit. Out: cut (0 f) — End of film; the last frame is the end card.

**SFX** (hit frame rel / abs, gain vs bed, file start rel).

- f0 (abs 1380) `riser_1bar_1.wav` -6 dB, file starts f-60. Placed by its END (tonal glide landing on A4): plays abs 1320–1379 and ends exactly on the final hit. Master-track cue.
- f0 (abs 1380) `impact_deep_1.wav` +2 dB, file starts f0. Final hit; bed duck −4 dB.
- f10 (abs 1390) `shimmer_3.wav` -14 dB, file starts f10. Glint.
- f15 (abs 1395) `bloop_1.wav` -12 dB, file starts f15. UBI lands on the lockup.

**Reading check.**

- “ubiqX AI”: 8 chars → max(24, 2×8) = 24 f; held f12→f180 = 168 f → yes
- “Retome o controle do seu dia.”: 29 chars → max(24, 2×29) = 58 f; held f20→f180 = 160 f → yes
- “Baixe em ubiqx.com.br”: 21 chars → max(24, 2×21) = 42 f; held f18→f180 = 162 f → yes
- “macOS · Windows · Linux”: 23 chars → max(24, 2×23) = 46 f; held f30→f180 = 150 f → yes
- Scene reading budget: all 81 chars → 162 f from the first landing (f12) to the last exit (f180) = 168 f → yes

## How every judge must-fix and fatal item was resolved

| From | Item | Resolution |
|---|---|---|
| CD + truth | Brand and descriptor on screen by 12 s; “Ele” needs an antecedent. | Wordmark slams at abs 240 (8.0 s); “Controle de tempo automático com IA.” lands at abs 255 (8.5 s). The first “Ele” is at abs 380, after the brand and UBI are both on screen. |
| CD + production | Validator with 0 errors; every camera key carries its file; ≤ 3 SFX at once. | tools/validate-storyboard.py: VALID. Every UI camera key has file, screenWidth, bitmap scale and readTarget; the max SFX overlap is 3. |
| CD + truth (fatal A) | No 08:00→18:00 / 09:00→17:00 clock or “time leaking” hook; 18:00 only as the report time. | Hook = C’s accountability pair on an empty timesheet. 18:00 appears only as the end of the recorded day (s06 playhead) and as the report time (s14). |
| CD | ≤ 6 modules, one benefit each; fold the categories typewriter; privacy in the breather. | registra (s06) · classifica (s07 categorias + s08 regras/memória/IA) · revisa (s09–s13) · relatório (s14) · foco (s15, one 3-s beat) · sua IA (s16); privacy is the s17 breather. |
| CD + truth | A concrete trust beat that says data goes out masked. | s17: “Seus dados ficam com você. / A IA vê só o mínimo.” with “ana@example.com” collapsing into the real “[email]” token and a line drawn to “IA”. |
| CD | Vary shot lengths, reach 28–38 shots, break the 90/90/90/120 cadence, add a stop-time moment. | 31 shots (avg 1.68 s); scene lengths 15, 30, 45, 60, 75, 90, 105, 120, 150, 180; B’s keycap band stop at abs 660–689; jump-cuts on beats in s08, s09. |
| CD | One true silence before the brand drop; a clear build into the final hit; frame 0 is a poster with a hit. | Silence abs 225–239 (−∞, caret only); build abs 1320–1379 (riser + snare roll + accelerating push); frame 0 = the finished hook poster on the first downbeat. |
| CD + production | UI text the camera reads ≥ 36 px and crisp; bitmap scale ≤ 1.40 including drifts. | Pixel-measured font sizes; every read target ≥ 36 px (36.3–43.0); vector re-sets wherever scale > 1.15 (categories label, IFRO, Confirmar os 19 with a 1.08× lift, Copiar Markdown, provider labels). Max scale 1.40. |
| truth + production | Never legible: the key-range legend/hint, model names, mail rows, the OG image, provider logos; the only key pressed is “1”. | Solid image-space patches in every framed review, report and settings capture (legend re-measured at y 1023–1041 in review-after-assign/review-done, where the manifest rect is wrong); the validator maps every sensitive region through every camera key. |
| CD + truth | One voice: UBI third person; first-person only in real app lines; ≤ 4 UBI beats; no worried/sleep near the product. | Headlines use “Ele”/“o Ubi”; UBI’s only bubble is the real “Nada em dúvida!”; the intervention window carries the real “Não! Foque na sua produtividade.”. UBI beats: drop → into the app, out of the empty queue, privacy look, plus the end card. |
| CD + truth (fatal C) | UI continuity: “Confirmar os 19” must not share a frame with other counts of the same dataset. | review-done’s resolved-count paragraph and “(29)” header are patched; review-after-assign’s “(20)” header and toast stay out of frame; s12’s header after the pull-back reads “(19)”, like the button. |
| CD + truth | Exact end card, ≥ 90 f legible, nothing in the bottom 120 px, no standalone OS row, no fade. | s18: wordmark, tagline, volt pill “Baixe em ubiqx.com.br”, “macOS · Windows · Linux” with glyphs; last landing f30, 150 f fully legible; last frame is the end card. |
| truth (fatal C) | No trailer lines (C’s kickers “REGISTRA SOZINHO”, “CLASSIFICA COM IA”, “Nas suas categorias.”). | None used; categories use the §3.3 headline, set as “Suas categorias.”. Only the tagline repeats the trailer. |
| truth | Demo numbers never the hero or read target. | No push to “88 de foco” or the app headline; “40 min” is the payoff of “Só um minutinho.”; the only counted read is the button label “Confirmar os 19”. |
| production (fatal A) | Zoom-54 X zoom-through, 1.425 drift, legible hint in A s09. | The X zoom-through is dropped (the UBI match-cut is the single brand → product bridge); the report drift stays at 2.6 (s 1.3); the hint is patched. |
| production (fatal C) | C s07 match frame showed 727 px of canvas; C s11 toast glide ran off the capture; C s13 showed a model id; C s16 raw-jump → idle pop. | Match anchor (1560, 560) puts the image right edge at comp x 1933; s12 uses A’s same-camera swap + pull-back; report meta lines are patched; the end card splices raw jump → raw wave → idle 0, all on the rest pose. |
| production | Audio on one master track with file start = hit − offset; transitions as halves in absolute Sequences; whole-plane blur only; still probes before the full render. | Every SFX cue carries fileStartFrame/hitOffsetFrames; conventions spell out the Sequence build; blur is whole-plane (s10, s13, s15 backdrops) or a ≤ 4-f whip; the QA list names the still probes. |

## Build and QA notes

- Still probes before the full render (≈ 5 s each): s05 f119 vs s06 f0 (UBI match, ±4 px); s06 f60 (playhead on the 18h tick); s07 f0 (label + typewriter re-set over the bitmap); s08 f0/f15/f30 (badge spotlight rects); s09 f0 and f30 (patches over the hint and legend, mail row under the scrim); s10 f29 vs s11 f0 (keycap rect) and s11 f12 (chip); s12 f0 (replica over the button) and f32 (swap: “você” ≈ 50 px from the button); s13 f15 (UBI match) and f27 (bubble); s14 f57; s15 f4; s16 f0 and f77; s17 f90; s18 f179.
- Measure on a still before locking: the wm-X centre of brand/ubiqx-wordmark-bold.svg at 194 px height (planned (960, 520), the caret’s spot); the in-app UBI ink boxes (dashboard ≈ 2402–2615 × 552–890, review-done ≈ 1209–1350 × 461–711); the hero-card colour (#0d1424 sampled).
- Recapture opportunity: review-* captures at DPR 3 would let every review read run at bitmap scale ≤ 0.94 without vector re-sets; a capture with a 1–5 legend would remove the need for legend patches. Neither is required by this board.
- Music: the stop at abs 225 must be dry (no tail); nothing may sound in 225–239; the riser SFX ends by itself on 224; the final tail must end by frame 1559.
- Render budget: 1 560 frames at ≈ 4.5 fps ≈ 6 min. Blur is whole-plane CSS only (no masked DOF, no backdrop-filter); whips use DirectionalBlur for ≤ 4 f per half.
- Both validators pass: tools/validate-storyboard.py (VALID, 0 warnings) and the judges’ brief/storyboards/validate_storyboard.py (VALID; its one warning is s12 f32, the 9-frame capture-swap key at scale 1.4, documented as textureReason).

## Validator output

```
brief/storyboard.json
  18 scenes, 1560 frames = 52.0 s (26 bars), 31 shots (avg 1.68 s), transitions ['blur-dissolve', 'cut', 'flash', 'match-cut', 'whip-left'], SFX cues 69, max SFX overlap 3
  VALID (0 errors)
```
