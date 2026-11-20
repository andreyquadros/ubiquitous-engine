# FU: integrator follow-ups

Status: **done**. Five changes, each with its tests: decision 19 (section 1), decision 18
(section 2), `plan::reset_targets` (section 3), decision 20 (section 4), and the page and its text
for decisions 17, 22 and 24 (section 5). Commands and their results are in section 6, open points
in section 7.

## 1. Decision 19: the delay of a new privacy-list entry counts observed running time

### What exists

- `ubiqx_core::nofap::ExclusionSeen` has a third field:

  ```rust
  pub struct ExclusionSeen {
      pub key: String,
      pub first_seen_at: DateTime<Utc>,
      #[serde(default)]
      pub observed_secs: u32,
  }
  ```

  `observed_secs` is the running time, in seconds, the app was observed for since the entry was
  first seen. It grows by at most 150 s per verify pass and stops at 900 (the constants of
  `commitment`: `OBSERVE_STEP_MAX_SECS`, `OBSERVED_CAP_SECS`). Data stored before the field
  existed reads as `0` (test `an_exclusion_stored_before_the_observed_time_existed_still_reads`
  in `ubiqx-core`).
- `ubiqx_nofap::exclusions`:
  - `reconcile(lists, seen, now)` and `effective(lists, seen, enabled_at, delay_secs, now)` keep
    their signatures. `reconcile` gives a new entry `observed_secs: 0` and keeps the value of a
    remembered one.
  - **The rule** (`effective`): an entry is effective when it was first seen at or before
    `enabled_at` (unchanged), or when the clock is `delay_secs` past `first_seen_at` **and**
    `observed_secs >= min(delay_secs, 900)`. It is the rule of `commitment::is_ready`.
  - New: `observe(seen: &mut [ExclusionSeen], elapsed_secs: u32) -> bool`. One verify pass: adds
    `min(elapsed_secs, 150)` to every entry, up to 900. Returns whether anything changed.
  - New: `reconcile_since(lists, seen, enabled_at: Option<DateTime<Utc>>, now)`. `reconcile`
    with one more guard, see "Contract notes".

### Contract notes for U5 (engine service)

- In the verify pass, with the same `elapsed_secs` you give `commitment::tick`:

  ```rust
  let changed = exclusions::observe(&mut config.internal.exclusions_seen, elapsed_secs);
  ```

  Store the configuration when it returns `true`. It returns `false` once every entry is at 900,
  so a standing list costs at most a handful of writes per entry, not one per pass for ever.
  Order: `reconcile` first, then `observe` (a brand-new entry is then credited the pass in which
  it was first seen, at most 150 s; calling them the other way round only delays that credit by
  one pass). Both orders are safe.
- Upgrade: entries stored by an earlier build have `observed_secs == 0`. Those first seen at or
  before `enabled_at` stay effective at once. One that was added later and had already waited its
  delay by the clock waits again until `min(delay, 900)` s of running time were observed. That
  is the stricter side and ends by itself.
- **A clock set back before the enabling instant** (found while writing the tests; it is the
  mirror of the hole decision 19 closes). `effective` treats `first_seen_at <= enabled_at` as
  "was there when the module was enabled". If the clock reads earlier than `enabled_at` when a
  new entry is first reconciled, plain `reconcile` records it with that early time and the entry
  is effective at once, and stays so after the clock is put right. `reconcile_since` closes it:
  while `now < enabled_at` it remembers nothing new (remembered entries stay, removed ones are
  dropped), so the new entry is not effective, and its wait starts at the first pass with a
  clock at or past `enabled_at`. **Please call
  `exclusions::reconcile_since(&lists, &seen, config.enabled_at, now)` in the verify pass**
  instead of `reconcile`. At the enabling instant itself `now == enabled_at`, which is not
  "behind": both functions do the same there. With `enabled_at == None` they are the same
  function.
- A sighting between `enabled_at` and the real time (clock set back by less) is covered by the
  running-time rule: after the clock is put right the clock condition is met at once, the 15
  minutes of observed running time are not.

### Fixtures

`UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden` was run: the three fixture files did
not change. `ExclusionSeen` lives in `Internal`, which is never sent to the page, so no IPC value
and no TypeScript type carries it.

## 2. Decision 18: checks on a standing signal are spread over the hour

### What exists (`crates/ubiqx-nofap/src/sampling.rs`)

- `decide`, `record`, `SampleCtx`, `SampleEvent` and `Decision` are unchanged in shape.
- `Cadence` has one more field, `doubles: bool`; it is `true` for `CADENCE_SIGNAL` only. Nothing
  outside `sampling.rs` builds a `Cadence` (checked with a search of the workspace).
- `pub fn spread_gap(every_secs, checks) -> i64`: the wait after the `checks`-th check of a key
  in its hour: `every_secs` after the first, doubled after each later one, never more than
  3,600 s. With 45 s: 45, 90, 180, 360, 720, 1,440.
- Each key remembers "its hour": the instant of the first frame **sent** for it, and how many
  were sent since. The hour is over 3,600 s after it began (or when it seems to begin in the
  future: the clock was set back); the next frame sent begins a new one, and the wait is 45 s
  again.
- The cap is the old one and is still a sliding hour: `PER_KEY_PER_HOUR` (6) frames of a key in
  any 3,600 s. A new hour of the key resets the gap, never the cap.

### The schedule (tested second by second with the fake clock)

- Standing signal, key in front all the time: frames at **5, 50, 140, 320, 680, 1400** s, then
  nothing until **3605**, and the same six offsets again in every following hour (test
  `the_spread_starts_again_when_the_hour_of_the_key_rolls`, three hours).
- After the sixth check the wait would be 24 minutes (`Skip::NotDue` until 2840); from then on
  the answer is `Skip::KeyCap` until the first check is an hour old (test
  `six_checks_per_key_per_hour`).
- A look that sent nothing (`Unchanged`) and a failed call restart the wait and do not lengthen
  it: only a frame that was sent doubles the gap.

### Deviation from the design, and one consequence to know about

- Design 4.3, row "Signal": "5 s, then every 45 s" becomes "5 s, then 45 s, 90 s, 3 min, 6 min,
  12 min apart" (decision 18).
- **The count is of all frames sent for the key in its hour, whatever the cadence was when they
  were sent.** `record` is not told whether a check was made on a signal (the event carries no
  such flag, and its shape was to stay), so a key that was already checked at the plain cadence
  starts the signal cadence further along: with two earlier checks the first look on a new
  signal comes 90 s after the last look, with three 3 minutes, with four 6 minutes, with five 12
  minutes. Before this change it came 45 s after the last look, and was followed by the rest of
  the six within minutes. Three assertions of the existing test `a_signal_cancels_relaxing` moved
  accordingly (150 → 195, 750 → 885, 150 → 195). In the worst case (five checks in the hour,
  relaxed plain cadence of 600 s) a new signal makes the next look come 2 minutes later than no
  signal would have. If the integrator wants the first look after a new signal to stay at 45 s,
  it needs a `signal: bool` on `SampleEvent::Checked` (an engine-facing change), so that only
  checks made on a signal are counted.

## 3. `plan::reset_targets` leaves a switched-off interface alone

- `reset_targets(observation, resolver)` (same signature) now returns only interfaces that are
  `active` (enabled on macOS, up on Windows) **and** whose servers are exactly the provider's
  set. It used to return every interface with exactly that set, active or not (hand-off U2 says
  so in its contract notes; that sentence is out of date).
- `plan::preview` uses the same function for the line `RESOLVER_RESET`, so no reset is promised
  when the only interface with the provider's servers is switched off.
- Consequence to know: a service we set while it was enabled, which the user then switched off,
  keeps the provider's servers after "remove". It is reset by a later "remove" run while it is
  enabled. The Windows back-end only ever lists adapters that are up, so nothing changes there.
- Tests: `reset_targets_leave_an_interface_the_user_changed` (the switched-off rows are left
  out; a reordered set on an active interface is still ours) and
  `a_switched_off_interface_is_neither_reset_nor_promised_a_reset`. The platform crate's tests
  (169) pass unchanged.

## 4. Decision 20: deleted rows are overwritten in the database file

- `crates/ubiqx-storage/src/db.rs`: `PRAGMA secure_delete = ON` is the fifth connection pragma,
  applied by `Db::init`, which both `Db::open` and `Db::open_in_memory` go through. No other
  code of the workspace opens a connection outside tests (searched). It applies to both
  products, and to every table, not only the module's.
- Tests in `db.rs`:
  - `every_connection_overwrites_what_it_deletes`: the setting reads `1` on a first connection, a
    second one on the same file, one opened after both are gone, and an in-memory one.
  - `a_deleted_row_and_a_replaced_value_do_not_stay_in_the_file`: after a `DELETE` and an
    `UPDATE` (no `VACUUM`) and a checkpoint, neither the deleted key, nor the deleted value, nor
    the replaced value is in the file or in the log; the kept text is. The file is read the way
    the erase tests of `tests/nofap.rs` read it.
  - `without_the_pragma_the_deleted_text_stays_in_the_file`: the same steps on a plain
    connection with `secure_delete = OFF` leave all three texts in the file. It is the proof that
    the test above can fail.
- **One existing assertion had to change** (the brief asked for the existing suite unchanged; it
  cannot be): `tests/nofap.rs`, `erased_history_does_not_stay_in_the_file`, asserted that the
  connection's setting is `0` after an erase ("back to what it was"). What it was is now `1`.
  Nothing else of the suite changed: 14 + 23 + 29 + 1 tests pass.
- `repos/nofap.rs::overwriting` (U3) still switches the setting on around its statements and
  restores what it found. With the pragma on everywhere it is a no-op and could go; it was left
  as it is (not this unit's file, and harmless).
- **What the pragma does not do** (unchanged limits, for whoever writes the privacy text):
  in WAL mode the overwritten page is in the log first and reaches the main file at the next
  checkpoint (automatic after about 1,000 pages, or when the last connection closes), and the
  log may still hold an older copy of the page until it is reset. `erase_history` and
  `wipe_user_data` run a `wal_checkpoint(TRUNCATE)` themselves, so they leave nothing; a single
  deleted row (a scrubbed block) is out of the main file after the next checkpoint, not at once.
  It also says nothing about copies the file system or a backup may keep.

## 5. The page and its text: decisions 17, 22 and 24

### Decision 17: "Apagar meu histórico" says what it erases

`data.erase_body` (interface text, not a string of design 5.6):

| | Old | New |
|---|---|---|
| PT-BR | Apaga as contagens e as pausas de regra. Suas listas, sua frase e a proteção ficam como estão. Não tem espera. | Apaga as contagens, as pausas de regra, a sua frase e os seus planos. Suas listas e a proteção ficam como estão. Não tem espera. |
| EN | Erases the counts and the rule snoozes. Your lists, your sentence and the protection stay as they are. There is no wait. | Erases the counts, the rule snoozes, your sentence and your plans. Your lists and the protection stay as they are. There is no wait. |

The mock (`apps/desktop/src/nofap/mock.ts`) follows: its `erase_history` now clears the sentence
and the plans, so the page in `vite dev` and in the tests does what the dialog says. The real
clearing is U5's, in the service.

### Decision 22: a switch whose change is waiting is disabled and says so

- `labels.ts`: `waitingSwitches(config) -> WaitingSwitches`. A switch is waiting when the thing is
  **on** and a request that turns it **off** is in `config.pending` (the page reads the payloads
  there; the status carries kinds only). A switch that is off is never waiting.
- What is covered: "Lista do sistema" (`set_hosts`), "Regras dos navegadores" (`set_policies`),
  the DNS choice (`set_resolver` to none: both options disabled), the switch of the AI check
  (`set_vision`), each scope switch that a waiting `set_vision_scope` turns off (the others stay
  usable), "Incluir meus sites na lista do sistema" on the lists tab (`set_hosts_user_sites`), and
  the button "Desligar o {name}" (`disable`).
- What is not: the waiting time (`set_delay`), the limits (`set_caps`) and the site lists. They
  are not switches and a second request there carries other values, so it is another request.
- The line under a disabled switch (`components/WaitingNote.tsx`), new interface strings:
  - `pending.switch_waiting`: "Pedido em espera: isto continua ligado até você confirmar ou
    cancelar o pedido." / "Request waiting: this stays on until you confirm or cancel the
    request."
  - `pending.scope_waiting`: "Pedido em espera: o que está ligado aqui continua assim até você
    confirmar ou cancelar o pedido." / "Request waiting: what is on here stays on until you
    confirm or cancel the request."
  - next to the disabled "Desligar" button: the existing `overall.pending_hint`.
- The mock returns the waiting request when the same change is asked for again (fields in any
  order), as decision 22 asks of the service.
- **For U5:** the page relies on `view.config.pending[].change` being the change as requested.
  For `set_vision_scope` it compares each flag of the waiting change with the flag in force.

### Decision 24: the consent sentence with the managed provider

`vision.consent` (design 5.6) is **unchanged**, character for character, so the design and the
copy-bank test stand as they are. A variant was added for the managed provider, as an interface
string: `vision.consent_ubi`, chosen by `labels.ts::consentKey(provider)` and used by
`components/VisionConsent.tsx` (the wizard and the consent dialog of the page).

What was wrong with the name "IA do Ubi" in the 5.6 sentence: "pergunta a IA do Ubi" (no
article), "IA do Ubi recebe a imagem, aplica as regras dele … os termos dele" (wrong gender, and
the rules and terms that apply are not the managed route's own).

| | `vision.consent` (5.6, unchanged), the two sentences that name the provider | `vision.consent_ubi` (new), the same two sentences |
|---|---|---|
| PT-BR | … o {name} tira uma foto reduzida só dessa janela e pergunta a {provider} se há pornografia nela. … {provider} recebe a imagem, aplica as regras dele ao que recebe e guarda pelo prazo que os termos dele definem. | … o {name} tira uma foto reduzida só dessa janela e pergunta à {provider} se há pornografia nela. … A {provider} envia a imagem à Anthropic, que aplica as próprias regras ao que recebe e guarda pelo prazo que os termos dela definem. |
| EN | … asks {provider} whether it shows pornography. … {provider} receives the image, applies its own rules to what it receives and keeps it for the period its terms state. | … asks {provider} whether it shows pornography. … {provider} sends the image on to Anthropic, which applies its own rules to what it receives and keeps it for the period its terms state. |

Every other sentence of the variant is the 5.6 text (a test compares them sentence by sentence).
The variant names Anthropic because `vision.managed` (5.6), shown right under it, already says the
managed route goes there. If the managed route ever uses another vendor, both strings change.

The assertion of `NofapWizard.test.tsx` that held the old wording ("pergunta a IA do Ubi") was
updated.

### Tests added or changed in the frontend

- `copy.test.ts`: four tests of the consent sentence (which key per provider, the Portuguese and
  the English reading with the names filled in, sentence-by-sentence comparison with 5.6).
- `NofapPage.test.tsx`: the erase dialog text in both languages and what the mock clears; six
  tests of the waiting switches (the page, every switch, the lists tab, English, the pure
  function, the mock's identical request). A mutation of `waitingSwitches` made three of them
  fail, then it was restored.
- The copy-bank, parity, forbidden-word and naming tests pass unchanged.

## 6. Commands run on the final state

See the report of this unit; the table there lists each command with its result.

## 7. Open points

- **Decision 18, a new signal on a key that was already checked** (section 2): the first look on
  the new signal waits the gap the key's count gives, up to 12 minutes. Counting only checks made
  on a signal needs a flag on `SampleEvent::Checked`.
- **Decision 19, the clock set back before the enabling instant** (section 1): closed by
  `reconcile_since`, which the engine's verify pass now calls. `service.rs` calls plain
  `reconcile(&lists, &[], now)` at enable, which is the documented use.
- **Decision 20 and the write-ahead log** (section 4): a single deleted row leaves the main file
  at the next checkpoint, not at once.
- **`reset_targets` and a service switched off after we set it** (section 3): it keeps the
  provider's servers until a "remove" runs while it is enabled.
- **The Portuguese of `vision.consent` with a vendor name** ("pergunta a Anthropic", "as regras
  dele") was left as the design has it; decision 24 is about the managed provider only.
- **`docs/nofap/DESIGN.md` was not edited** (not this unit's file). Its section 4.3 still says
  "every 45 s" for a signal, and its 7.4 is now what the code and the dialog say.
- **Hand-off U2**, contract note "`plan::reset_targets` returns every interface that is exactly
  ours, active or not", is out of date (section 3).
