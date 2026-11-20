# ubiqX AI launch film: final PT-BR copy edit

> **What this is.** The last copy pass on `brief/storyboard.json` and its generated twin `brief/storyboard.md`:
> every on-screen line checked for accents, crase, concordância, punctuation, casing and truth, then tightened
> where a sharper Brazilian line exists inside the reading-time rule. Before → after, and why.
>
> Done 2026-09-29. Sources: `brief/facts.md` (truth, glossary, approved lines), `brief/style.md` §3.6 (read rule),
> §4.3–4.4 (emphasis, PT-BR specifics). No scene timing, transition, camera, SFX or music event moved.

## Summary

- **33 on-screen copy lines reviewed. 3 rewritten, 30 kept.** 571 characters on screen, down from 575.
- **The film keeps one voice:** short declaratives, “você” for the viewer, “ele” for the product. Questions appear
  only in the problem act, and exclamation marks only inside real app strings.
- **Exact strings, confirmed:** “Retome o controle do seu dia.” · “Baixe em ubiqx.com.br” · “macOS · Windows · Linux”.
  Every quoted UI string was checked against the app source (table below).
- **Both validators pass:** `tools/validate-storyboard.py` reports VALID with 0 errors. The judges'
  `brief/storyboards/validate_storyboard.py` reports VALID with its one known warning (s12 f32 textureReason).
  Both give the same numbers as before the edit.

## The three changes

| Scene | Before | After | Chars | Read rule (need ≤ held) | Why |
|---|---|---|---|---|---|
| `s02` | “Só um minuto.” (emphasis *minuto*) | **“Só um minutinho.”** (emphasis *minutinho*) | 15 → 18 | 36 ≤ 75 f · scene 48 ≤ 75 f | Brazilians say *minutinho* when they are fooling themselves. The diminutive carries the lie, so the counter's **40 min** reads as the punchline, not as a statistic. It is the same idiom as the landing page (“o ‘só um minuto’ que virou quarenta”, `site/src/i18n.ts:143`), in the form people actually say. The line is 3 characters longer. The hold already covers it, s03 and s07 give back 7 characters, and the net change is −4. |
| `s03` | Planilha de memória? (strike on *memória?*) | **Chutar as horas?** (strike on *Chutar*) | 20 → 16 | 32 ≤ 45 f | “Planilha de memória” first parses as a noun (“a memory spreadsheet”). The pain is doing it *from* memory. *Chutar* is the everyday verb for filling in hours by guessing. It makes a verb-led question that rhymes with the hook's “O que você fez na terça?”, and it matches the rose “?” that pop into the cells (the guesses). The empty timesheet under the line already says “planilha”. The rose strike now crosses out the habit and leaves “as horas?” standing, which sets up “Ele registra.” Truth stays with facts §1 (“planilha preenchida de memória na sexta”). The line makes no product claim and does not repeat the trailer. |
| `s07` | As suas categorias. (emphasis *suas*) | **Suas categorias.** (emphasis *Suas*) | 19 → 16 | 32 ≤ 45 f | “As suas” is correct, but in a Brazilian headline the article reads formal, closer to European Portuguese. Dropping it matches “Seus dados ficam com você.” (s17), so the film has one register for possessives. It stays the facts §3.3 manchete, and the trailer's “nas suas categorias” stays out. The stagger drops from 3 words to 2 (f3/f6) and still lands by f15, so the timing is unchanged. |

In both files, every field that quotes these lines was updated: thumbnail, purpose, why, motion, copy note,
readingCheck, the music-map event notes at abs 105 and 180, the synthesis graft and fixes, and the copy-trace
guard. Scene ids (`s02-so-um-minuto`, `s03-planilha-de-memoria`, `s07-as-suas-categorias`) are keys, not copy, so
they stay unchanged for build stability.

## Every line, audited

Reading rule: `hold ≥ max(24, 2 × characters)`, measured from landFrame to outFrame (style §3.6). Groups and
scene budgets are re-checked by the validator.

| # | Scene | Line (as set) | Chars | Need / held (f) | Verdict | Note |
|---|---|---|---|---|---|---|
| 1 | s01 | SEXTA-FEIRA · 17:00 | 19 | 38 / 90 | kept | Reads as a calendar stamp, and “17:00” rhymes with the 17:59 → 18:00 clock in s14. No accent is needed in caps. The middot matches the platform line. |
| 2 | s01 | O que você fez na terça? | 24 | 48 / 90 | kept | This is the hook. It is concrete, second person and one line. “na terça” is the natural contraction. |
| 3 | s02 | “Só um minutinho.” | 18 | 36 / 75 | **changed** | See above. The curly quotes mark it as the viewer's own excuse, and it is the film's only quoted line. |
| 4 | s02 | 40 min | 6 | 24 / 30 | kept | “min” with a space (style §4.4) and no period on a number. It pays off the idiom, never a statistic. |
| 5 | s03 | Chutar as horas? | 16 | 32 / 45 | **changed** | See above. |
| 6 | s05 | ubiqX AI | 8 | 24 / 120 | kept | Brand casing is exact: lower-case “ubiq”, volt “X”, “AI” pill. |
| 7 | s05 | Controle de tempo automático com IA. | 36 | 72 / 105 | kept | The approved short line from facts §1 (landing `<title>`, `site/src/i18n.ts:115`). “IA”, not “AI”, in running copy. The final period fits the declarative voice. |
| 8 | s06 | Ele registra. Você trabalha. | 28 | 56 / 70 | kept | Facts §3.1 manchete. It ends on the viewer's benefit, and “Ele” has its antecedent (brand + UBI) on screen from abs 240. I tested “Você trabalha. Ele registra.” and rejected it: the benefit belongs at the end. |
| 9 | s07 | Suas categorias. | 16 | 32 / 45 | **changed** | See above. |
| 10 | s08 | Regras, / memória, / IA. | 7 + 8 + 3 | 24 each; group 36 / 45 | kept | Facts §3.4 manchete. The commas carry the list across three slams, the period closes it, and each word matches its real origin badge (regra · memória · IA). |
| 11 | s09 | Só pergunta o que não sabe. | 27 | 54 / 59 | kept | From the facts §6 bank. The subject is omitted, which is natural in PT-BR after s08. “o que” is glued with an NBSP. |
| 12 | s10 | 1 | 1 | 24 / 24 | kept | The only key ever shown pressed (the mock has 5 categories). |
| 13 | s11 | Uma tecla. / E ele aprende. | 25 | 50 / 63 | kept | From the facts §6 bank. The “E” adds a beat of anticipation, and “Uma tecla” / “Um clique” (s12) form a deliberate pair. |
| 14 | s12 | Confirmar os 19 | 15 | 30 / 30 | kept | Real button label (`review.ts:80` “Confirmar os {count}”). |
| 15 | s12 | Um clique vira memória. | 23 | 46 / 50 | kept | Facts §3.6 manchete. It echoes “memória” from s08, and memory is the mechanism. “Um clique. Tudo vira memória.” (29 characters needs 58 f against 50 held) fails the rule. |
| 16 | s13 | Nada em dúvida! | 15 | 30 / 33 | kept | UBI's real empty-state line (`review.ts:83`). The exclamation mark is the app's, not the film's. |
| 17 | s14 | 18:00 | 5 | 24 / 90 | kept | Digital-clock format matches the odometer roll from 17:59, and 18:00 is the facts default. |
| 18 | s14 | O relatório sai pronto. | 23 | 46 / 63 | kept | Facts §3.8 manchete. “O relatório se escreve sozinho.” (an approved landing line, 31 characters) would push group g1 to 72 f against 63 available, so it fails. “já sai pronto” would copy the trailer, so “já” stays out. |
| 19 | s14 | Markdown copiado | 16 | 32 / 39 | kept | Real toast (`reports.ts:47`). |
| 20 | s15 | Não! Foque na sua produtividade. | 32 | 64 / 86 | kept | Real UBI line inside the captured window (`focus.ts:128`). This is bitmap text and is not editable. |
| 21 | s15 | Ok, foco! | 9 | 24 / 86 | kept | Real button (`focus.ts:124`). |
| 22 | s16 | Claude, OpenAI ou Grok. | 23 | 46 / 93 | kept | Providers in plain text, with no comma before “ou” (PT-BR has no serial comma). No logos, model names or endorsement wording. |
| 23 | s16 | Ou deixe com o Ubi. | 19 | 38 / 45 | kept | Facts §3.14 manchete. “Ubi”, not “UBI”, is deliberate here: the line names the managed service the cursor clicks (“IA do Ubi”, `settings.ts:333`), which facts §6 spells this way. |
| 24 | s17 | Seus dados ficam com você. | 26 | 52 / 142 | kept | From the facts §6 bank. It says data stays with you, never that nothing leaves (facts §5.4). |
| 25 | s17 | A IA vê só o mínimo. | 20 | 40 / 82 | kept | From the facts §6 bank. “vê só o mínimo” puts the stress on “mínimo”, which is the volt word. |
| 26 | s17 | ana@example.com | 15 | 30 / 45 | kept | A fictional, RFC-2606-reserved address and the only e-mail in the film. A `.com.br` variant would not be reserved. |
| 27 | s17 | [email] | 7 | 24 / 58 | kept | The exact token from `crates/ubiqx-core/src/redact.rs:64`. |
| 28 | s18 | ubiqX AI | 8 | 24 / 168 | kept | Brand. |
| 29 | s18 | Retome o controle do seu dia. | 29 | 58 / 160 | kept, **exact** | Tagline (facts §6). This is the only line shared with the trailer, and that is by design. |
| 30 | s18 | Baixe em ubiqx.com.br | 21 | 42 / 162 | kept, **exact** | CTA. It says download, never buy, because checkout is not live. No period after a URL. |
| 31 | s18 | macOS · Windows · Linux | 23 | 46 / 150 | kept, **exact** | Platform line. NBSPs glue each middot to the name before it, and the validator normalises NBSP for its exact-match check. |

s08 is three copy entries, so the table has 31 rows for 33 lines.

## Checks applied to every line

- **Accents:** all present, including in caps (none of the caps strings needs one). The Latin subsets of Sora and
  Inter cover every glyph used.
- **Crase:** no line needs one. No “às 18h” is set in type; the time is the clock.
- **Concordância:** “Seus dados ficam”, “O relatório sai pronto”, “Um clique vira memória” and “Suas categorias”
  all agree.
- **Punctuation system:** declarative headlines end with a period. Questions (“terça?”, “horas?”) appear only in the
  problem act. Exclamation marks (“Nada em dúvida!”, “Não!”, “Ok, foco!”) appear only in real UI strings. Curly
  quotes appear once, on the quoted excuse. Kickers, labels, numbers and the URL take no final period.
- **Casing:** ubiqX, UBI (the character), Ubi in “IA do Ubi” and in “deixe com o Ubi” (the managed service, UI
  spelling), IA (never “AI” except in the product name), macOS.
- **NBSP glue (style §4.4):** kept on every short word that could strand. “Chutar as horas?” glues “as horas”.
  “Suas categorias.” has no short word to glue.
- **Truth:** no new claims. Every line traces to facts §1, §3, §6 or a real app string. No banned wording (key
  range, grátis, assine/compre, offline, criptografia, parceria/powered by, model names). The validator
  re-checks all of this.
- **Trailer distance:** no trailer line is reused except the tagline. The new lines (“Só um minutinho.”, “Chutar as
  horas?”, “Suas categorias.”) do not appear in the trailer. The trailer's “Nas suas categorias.” is still avoided.

## One voice, hook to CTA

Read as a script, the film is one speaker. It asks the viewer something uncomfortable, answers with the product,
proves each claim with the real UI, and closes on one instruction.

> SEXTA-FEIRA · 17:00 — O que você fez na terça? · “Só um minutinho.” — 40 min · Chutar as horas? ·
> *(silêncio)* · **ubiqX AI** — Controle de tempo automático com IA. · Ele registra. Você trabalha. ·
> Suas categorias. · Regras, memória, IA. · Só pergunta o que não sabe. · **1** · Uma tecla. E ele aprende. ·
> Confirmar os 19 — Um clique vira memória. · *UBI:* Nada em dúvida! · 18:00 — O relatório sai pronto. —
> Markdown copiado · *UBI:* Não! Foque na sua produtividade. — Ok, foco! · Claude, OpenAI ou Grok. Ou deixe com o
> Ubi. · Seus dados ficam com você. A IA vê só o mínimo. (ana@example.com → [email]) · **ubiqX AI** — Retome o
> controle do seu dia. — Baixe em ubiqx.com.br — macOS · Windows · Linux

- **Problem act:** three questions and an excuse, all in the viewer's own words.
- **Features:** short declaratives with “ele” as subject (s06, s09, s11). UBI speaks only real app lines.
- **Proof:** plain facts, no adjectives.
- **CTA:** the imperative “Retome”, then “Baixe”.

## Other on-screen text reviewed (texture, unchanged)

- **s01/s03 timesheet:** “SEG TER QUA QUI SEX” and hour rows “09h…17h”. These are Brazilian abbreviations and the
  style §4.4 time format.
- **s02 window-title chips:** “Nova aba”, “Re: Re: Fwd: reunião”, “Planilha de horas — set”, “(12) Caixa de
  entrada”, “Proposta_v3_final.pdf”, “Sem título — Documento”. They are generic and unbranded, with no address.
  Their job is to be unread.
- **s14 clock:** “17:59”.
- **Vector re-sets of real UI:** “O que conta como trabalho desta categoria” (`categories.ts:41`), “IFRO”,
  “Copiar Markdown” (`reports.ts:29`) and the provider labels. They must match the app byte for byte, so none
  was touched.

## Alternatives considered and rejected

| Scene | Candidate | Why not |
|---|---|---|
| s03 | “Tudo de memória?” | Keeps a callback to “memória” in s08/s12, but it is vaguer and the callback is too far away (11 s) to register. |
| s03 | “Horas no chute?” | Also idiomatic, but noun-first. “Chutar as horas?” mirrors the hook's verb question. |
| s06 | “Você trabalha. Ele registra.” | Ends on the mechanism instead of the benefit, and departs from the facts §3.1 manchete. |
| s07 | “Você descreve. A IA segue.” (26) | The truer differentiator, but it needs 52 f against 45 held. |
| s12 | “Confirmou? Amanhã ele resolve sozinho.” (38) | It needs 76 f against 50 held. |
| s14 | “O relatório se escreve sozinho.” (31) | Group g1 needs 72 f against 63 available. |
| s14 | “O relatório já sai pronto.” | Echoes the trailer verbatim. |
| s01 | “SEXTA, 17H” | More casual, but it breaks the 17:00 → 18:00 clock rhyme with s14. |

## Files touched

- `brief/storyboard.json`: the 3 copy entries (text, emphasis, note) plus every field that quotes them
  (listed above). Edited as data and dumped in the file's own format (2-space indent, UTF-8, trailing newline).
- `brief/storyboard.md`: regenerated from the JSON with the storyboard's own md renderer. Only its hand-written
  “film in one breath” paragraph was updated to the new lines. The diff is limited to the lines that quote the
  changed copy.
- `brief/copy-edit.md`: this file.

## Validator output after the edit

```
brief/storyboard.json
  18 scenes, 1560 frames = 52.0 s (26 bars), 31 shots (avg 1.68 s), transitions ['blur-dissolve', 'cut', 'flash', 'match-cut', 'whip-left'], SFX cues 69, max SFX overlap 3
  VALID (0 errors)
```
