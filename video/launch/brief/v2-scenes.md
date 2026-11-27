# v2 pass: direction for each scene

Read `brief/v2-look.md` first: it sets the targets and the look. This file says what each scene has to gain.
"v1" means the frames of `out/launch-v1-master.mp4`. Frames are scene-relative unless marked "abs".

The following do not change, and every other rule in this file works around them:
- the timing (start and duration of each scene);
- the transitions between scenes;
- the SFX cue frames already in place;
- the match-cut contracts listed in each group's `_parts/Gx/common.tsx`.

The copy stays locked to `storyboard.json`, with **one exception, s15**, which gains a headline (see G5).

## Common to every scene

- **Size up the type.** Take each headline, kicker, caption, chip and pill up to the minimums in v2-look §2. Then
  re-flow the layout, because bigger type is not the same layout scaled up. The headline and the subject share
  the frame with no dead band between them.
- **Make the subject big and bright.** The subject is the UI element the line talks about. Zoom until the text you
  want read is at least 34 px on the canvas, or lift the element with `<LiftCard>`. Anything behind the subject
  steps back through depth, blur or the reduced dim, never through a blackout.
- **Keep every hold moving.** A hold longer than 45 f needs secondary motion: parallax, drift, a light sweep, a
  float, a glow breath, or particles.
- **Brighten scene-level values.** Scene code often passes its own dark levels: the backdrop `base`, low orb
  opacities, scrims, `dim` values and `rgba(6, 9, 16, …)` veils. Bring them in line with the v2 stage. The
  look-dev hand-off list in `out/review/v2/LOOK.md` names these sites.
- **Keep the claims safe.** Every guard in v2-look §4 still applies. After zooming, re-check that no price, model
  id, "1–9" legend or contradicting count has become legible. Patch it in image space if it has.
- **Verify before handing back.** Render stills at the key frames of each scene: the poster frame, every landing,
  the middle and end of every hold, and both sides of each cut. Look at every still at full size and at 480×270.
  Run `tools/look-metrics.py` on them. Finish with a clean `npx tsc --noEmit` for your files.

## G1: the problem (s01–s04, abs 0–239)

- **s01 "O que você fez na terça?"**
  - Frame 0 is the poster, and it has to stop the scroll.
  - Headline at 120 px or more; the kicker "SEXTA-FEIRA · 17:00" at 44 px or more.
  - The timesheet is barely visible in v1. It must read as a desk-sized weekly sheet: 1.2× larger, filling the
    lower 55–60 %, with brighter lines, day labels at 34 px or more, and hour labels at 28 px or more.
  - The "ANTES" desaturation stays, but the sheet must be visible.
  - The TER column spotlight should glow volt, not just dim the others.
- **s02 "“Só um minutinho.”" / "40 min"**
  - Headline at 120 px or more. The counter numerals are huge, 260 px or more; "min" is 96 px or more.
  - The distraction chips should read as a real flurry: bigger (40 px text), brighter, more of them, with motion
    blur. Keep them generic, with no brand logos.
  - f0–10 is empty in v1; fill it.
- **s03 "Chutar as horas?"** Headline at 120 px or more. The timesheet is big again. The rose "?" marks are
  bigger (80 px or more) and pop with a spring. The strike-through reads.
  Fix v1's lopsided last frame.
- **s04 (silence):** keep it as the designed black beat with the caret. Change nothing, except the caret's size
  and position if s03 or s05 changes the hand-off.

## G2: the reveal (s05–s06, abs 240–449)

- **s05 "ubiqX AI" / "Controle de tempo automático com IA."**
  - The drop has to explode.
  - The lockup gets 1.15–1.3× larger. The tagline is 64 px or more.
  - A big volt key-light blooms behind the lockup on the drop.
  - The 4-s hold is too static. After the lockup settles (around f30), bring in a product constellation:
    3–5 real UI fragments lifted from the captures (for example the 88 score ring, the day-track strip, a review
    row with its category pill, a report card) float in 3D depth behind and around the lockup. They parallax
    slowly and enter on beats. None of them may carry a price, a model id or a key legend.
  - Keep the UI fragments clear of the s06 match cut. By f110–119 the frame must be back to what the match cut
    expects: UBI at `UBI_MATCH`. Fragments may exit or recede.
- **s06 "Ele registra. Você trabalha."**
  - Headline at 112 px or more.
  - Close v1's dead band (y 170–450 at f40): the window rises higher and larger, or the headline sits closer.
  - Lift the day-track card with `<LiftCard>` as it records 08h→18h.
  - The dashboard is brighter.
  - The match cut at f0 must still land.

## G3: categories and the chain (s07–s08, abs 450–584)

- **s07 "Suas categorias."**
  - Headline at 112 px or more.
  - Zoom the typing field until the typed description is 40 px or more on the canvas. The typing itself is the
    subject.
  - Brighter; the field glows volt while typing.
  - Keep the whip transitions working.
- **s08 "Regras, / memória, / IA."**
  - Slams at 170 px or more. Three lines at that size need a layout: stacked on the left, with the badges on the
    right, and so on.
  - The origin badges (regra / memória / IA) become big lifted cards, one per slam. They are the proof, and they
    must be readable: badge text 40 px or more.

## G4: review and the key (s09–s11, abs 585–779)

- **s09 "Só pergunta o que não sabe."**
  - Headline at 112 px or more.
  - Zoom into the pending groups so row titles are 34 px or more.
  - **Patch the subtitle "…10 grupos esperam sua decisão, 19 min ainda sem categoria."** so no count is legible.
  - Keep the v1 badge patch, and check it against the new grade.
- **s10 "1": the key is the HERO.**
  - The keycap is 420 px or more across, centred, with dramatic key-light and rim light, a reflection or floor
    glow, and a press with an impact ring or shockwave on the contact frame.
  - Keep continuity into s11's keycap-to-chip match cut. Update both sides of that contract together.
- **s11 "Uma tecla. E ele aprende."**
  - Headline at 112 px or more.
  - The category list (IFRO highlighted) and the rule chip "Sempre: … → IFRO" are lifted and readable at 36 px
    or more.
  - The rule chips are the payoff: give them a satisfying pop.

## G5: memory, report, focus (s12–s15, abs 780–1124)

- **s12 "Um clique vira memória."**
  - Headline at 112 px or more.
  - "Confirmar os 19" is the hero button. Lift it (1.4× or more, readable at 40 px or more). The cursor arc,
    press and ripple stay.
  - The mint wave down the rows is bright and satisfying.
  - Check the rows: no price, no model id, no contradicting count.
- **s13 "Nada em dúvida!"**
  - The bubble text is 90 px or more.
  - Fix v1's f0–6 canvas and title-bar strip at the top (zoom around 1.65 or re-anchor).
  - Brighter; UBI larger.
  - Soften v1's f16–18 double image if you can.
- **s14 "18:00" / "O relatório sai pronto." / "Markdown copiado"**
  - The clock is already big; keep it.
  - Headline at 112 px or more.
  - Lift the report card with the "Copiar Markdown" button, and make its text readable (activity rows at 34 px or
    more).
  - The "Markdown copiado" toast is 40 px or more and pops.
  - Keep the model ids removed.
- **s15 (focus intervention).** v1 is a calm hold of a dim modal and has no headline.
  - Add the headline **"Foco que se defende."** (facts §3.11 manchete; 20 chars, 40 f reading budget) at 112 px or
    more. It lands on a beat before the "Ok, foco!" click.
  - Make the modal ("Não! Foque na sua produtividade.", the YouTube chip, "Ok, foco!") big and bright:
    bubble text 40 px or more, the button 44 px or more.
  - Add energy: a push-in, a slight 3D rotation, a light sweep, a pulse on the button before the click, and a
    punch-in or shockwave on the click.
  - The whip-left out-half into s16 stays.

## G6: AI choice, privacy, end card (s16–s18, abs 1125–1559)

- **s16 "Claude, OpenAI ou Grok. / Ou deixe com o Ubi."**
  - Headline lines at 100 px or more.
  - The provider pills are the subject: lifted, pill text 40 px or more, with the hover and the slide to
    "IA do Ubi" clearly visible.
  - **Patch the helper line under the picker in both states.** One state reads "Com Claude, US$ 3–6/mês
    estimados…", the other "…(R$ 49/mês)…". No legible price or estimate is allowed here.
  - No provider logos. The key icon glyphs that are already in the capture are fine.
- **s17 "Seus dados ficam com você." / "A IA vê só o mínimo."**
  - Line 1 at 100 px or more; line 2 at 80 px or more.
  - The redaction is the proof and must be big. Use a panel of fictional samples (redact.rs tokens are exact)
    that flip to their masked form in a beat-locked cascade, each chip readable at 44 px or more:
    - "ana@example.com" → "[email]"
    - "(69) 99999-0000" → "[telefone]"
    - "123.456.789-00" → "[cpf]"
    - a URL with a query → its bare domain
  - Fill the empty right half. UBI stays and is larger.
  - Keep the calm feel and the build into the 1380 hit.
  - Do not claim "nothing leaves your machine".
- **s18 end card**
  - Wordmark 1.2× or larger. Tagline at 64 px or more. CTA pill label at 72 px or more. OS row at 52 px or more,
    with icons scaled to match.
  - Everything stays inside title-safe, and the bottom 120 px stays clear of critical text.
  - It holds 90 f or more fully legible, with no fade.
  - UBI stays.
  - The CTA is exactly "Baixe em ubiqx.com.br".
